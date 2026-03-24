use crate::crypto::{encrypt, RoomKey};
use crate::db::{self, DbPool};
use crate::room::{broadcast_to_room, join_room, leave_room, list_rooms};
use crate::types::{
    ChatMessage, ClientInfo, ClientMessage,
    SharedClients, SharedRoomKeys, SharedRooms,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::info;

pub async fn handle_client(
    stream: TcpStream,
    clients: SharedClients,
    rooms: SharedRooms,
    room_keys: SharedRoomKeys,
    pool: Arc<DbPool>,
) -> Result<()> {
    let mut info = ClientInfo::new();

    info!(
        client_id = %info.id,
        "TCP client connected"
    );

    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let (tx, mut rx) = mpsc::channel::<ClientMessage>(32);

    clients.insert(info.id, tx);
    rooms
        .entry("lobby".to_string())
        .or_insert_with(Default::default)
        .insert(info.id);

    let lobby_key = room_keys
        .entry("lobby".to_string())
        .or_insert_with(RoomKey::generate)
        .clone();

    let mut current_key = lobby_key.clone();

    let welcome = format!(
        "=== Welcome to RustChatPro v0.9 ===\n\
         ID      : {}\n\
         Room    : {}\n\
         RoomKey : {}\n\
         Cmds    : /join <room> · /msg <id> <text>\n\
                   /rooms · /history · /quit\n\
         ===================================\n",
        info.id,
        info.current_room,
        lobby_key.to_hex()
    );
    writer.write_all(welcome.as_bytes()).await?;

    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if writer.write_all(msg.as_bytes()).await.is_err() {
                break;
            }
        }
    });

    let mut line = String::new();
    loop {
        line.clear();
        let n = buf_reader.read_line(&mut line).await?;
        if n == 0 {
            info!(
                client_id = %info.id,
                "TCP client disconnected"
            );
            break;
        }

        let input = line.trim().to_string();
        if input.is_empty() { continue; }

        info.update_last_seen();

        if input == "PONG" || input == "/pong" {
            info!(
                client_id = %info.id,
                "PONG received — client alive"
            );
            continue;
        }

        let ack = ChatMessage::ack(&format!("received: {}", input));
        send_to_client(&clients, info.id, &ack.display()).await;

        let should_continue = handle_input(
            &input,
            &mut info,
            &mut current_key,
            &clients,
            &rooms,
            &room_keys,
            &pool,
        ).await;

        if !should_continue { break; }
    }

    leave_room(&rooms, info.id, &info.current_room).await;
    let bye = ChatMessage::system(&format!(
        "[{}] left the room", &info.id.to_string()[..8]
    ));
    broadcast_to_room(
        &rooms, &clients,
        &info.current_room, &bye, Some(info.id)
    ).await;
    clients.remove(&info.id);
    info!(
        client_id = %info.id,
        remaining = clients.len(),
        "TCP client removed"
    );
    write_task.abort();
    Ok(())
}

async fn handle_input(
    input: &str,
    info: &mut ClientInfo,
    current_key: &mut RoomKey,
    clients: &SharedClients,
    rooms: &SharedRooms,
    room_keys: &SharedRoomKeys,
    pool: &Arc<DbPool>,
) -> bool {
    if input.starts_with('/') {
        let parts: Vec<&str> = input.splitn(3, ' ').collect();
        match parts[0] {
            "/join" => {
                if parts.len() < 2 {
                    send_to_client(clients, info.id,
                        "Usage: /join <roomname>\n").await;
                    return true;
                }
                let room_name = parts[1].to_lowercase();
                let response  = join_room(
                    rooms, clients, room_keys,
                    info.id, &room_name, &info.current_room
                ).await;
                if let Some(key) = room_keys.get(&room_name) {
                    *current_key = key.clone();
                }
                info!(
                    client_id = %info.id,
                    old_room = %info.current_room,
                    new_room = %room_name,
                    "client joined room"
                );
                info.current_room = room_name;
                send_to_client(clients, info.id, &response).await;
            }
            "/rooms" => {
                let live = list_rooms(rooms);
                send_to_client(clients, info.id, &live).await;
                match db::get_all_rooms(pool).await {
                    Ok(db_rooms) if !db_rooms.is_empty() => {
                        let mut msg = String::from(
                            "--- Rooms with history ---\n"
                        );
                        for r in db_rooms { msg.push_str(&r); }
                        send_to_client(clients, info.id, &msg).await;
                    }
                    _ => {}
                }
            }
            "/history" => {
                send_to_client(clients, info.id,
                    "--- Loading history ---\n").await;
                match db::get_room_history(
                    pool, &info.current_room, 50
                ).await {
                    Ok(messages) if messages.is_empty() => {
                        send_to_client(clients, info.id,
                            "No history found.\n").await;
                    }
                    Ok(messages) => {
                        for msg in messages {
                            send_to_client(clients, info.id,
                                &msg).await;
                        }
                        send_to_client(clients, info.id,
                            "--- End of history ---\n").await;
                    }
                    Err(e) => {
                        send_to_client(clients, info.id,
                            &format!("History error: {}\n", e)
                        ).await;
                    }
                }
            }
            "/msg" => {
                if parts.len() < 3 {
                    send_to_client(clients, info.id,
                        "Usage: /msg <id> <message>\n").await;
                    return true;
                }
                handle_private_msg(
                    clients, info.id, parts[1], parts[2]
                ).await;
            }
            "/quit" => {
                send_to_client(clients, info.id,
                    "Goodbye!\n").await;
                return false;
            }
            _ => {
                send_to_client(clients, info.id,
                    &format!("Unknown command: {}\n",
                        parts[0])).await;
            }
        }
    } else {
        match encrypt(input, current_key) {
            Ok(encrypted) => {
                info!(
                    room = %info.current_room,
                    client_id = %info.id,
                    content_len = input.len(),
                    "message broadcast"
                );
                let enc_msg = ChatMessage::chat(
                    &encrypted,
                    &info.current_room,
                    &info.id.to_string(),
                );
                broadcast_to_room(
                    rooms, clients,
                    &info.current_room, &enc_msg, None
                ).await;

                let pool_clone   = Arc::clone(pool);
                let room_clone   = info.current_room.clone();
                let sender_clone = info.id.to_string();
                let enc_clone    = encrypted.clone();
                tokio::spawn(async move {
                    if let Err(e) = db::save_message(
                        &pool_clone, &room_clone,
                        &sender_clone, &enc_clone, "chat",
                    ).await {
                        tracing::warn!(
                            error = %e,
                            "DB save failed"
                        );
                    }
                });

                let confirm = format!(
                    "[you → {}]: {}\n",
                    info.current_room, input
                );
                send_to_client(clients, info.id, &confirm).await;
            }
            Err(e) => {
                send_to_client(clients, info.id,
                    &format!("Encryption error: {}\n", e)).await;
            }
        }
    }
    true
}

async fn handle_private_msg(
    clients: &SharedClients,
    sender_id: uuid::Uuid,
    target_prefix: &str,
    text: &str,
) {
    let target = clients.iter().find(|entry| {
        entry.key().to_string().starts_with(target_prefix)
    });
    match target {
        Some(entry) => {
            let dm = ChatMessage::dm(
                text,
                &sender_id.to_string(),
                &entry.key().to_string(),
            );
            let _ = entry.value().send(dm.display()).await;
            let confirm = ChatMessage::ack(&format!(
                "DM delivered to {}",
                &entry.key().to_string()[..8]
            ));
            send_to_client(
                clients, sender_id, &confirm.display()
            ).await;
        }
        None => {
            send_to_client(clients, sender_id,
                &format!("No client found: '{}'\n",
                    target_prefix)).await;
        }
    }
}

async fn send_to_client(
    clients: &SharedClients,
    client_id: uuid::Uuid,
    msg: &str,
) {
    if let Some(sender) = clients.get(&client_id) {
        let _ = sender.send(msg.to_string()).await;
    }
}