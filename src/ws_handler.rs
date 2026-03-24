// ws_handler.rs
// Phase 9 COMPLETE — Structured logging + spans + propagation

use crate::db::DbPool;
use crate::room::{broadcast_to_room, join_room, leave_room, list_rooms};
use crate::crypto::{encrypt, RoomKey};
use crate::types::{
    ChatMessage, ClientInfo, ClientMessage,
    SharedClients, SharedRoomKeys, SharedRooms,
};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn, info_span, Instrument};

#[allow(dead_code)]
pub async fn handle_ws_client(
    raw_stream: tokio::net::TcpStream,
    clients: SharedClients,
    rooms: SharedRooms,
    room_keys: SharedRoomKeys,
    pool: Arc<DbPool>,
) -> Result<()> {
    // Upgrade TCP → WebSocket
    let ws_stream = tokio_tungstenite::accept_async(raw_stream).await?;

    // Create client identity
    let mut info = ClientInfo::new();

    // ✅ SPAN (Phase 9)
    let span = info_span!(
        "ws_client",
        client_id = %info.id,
        protocol = "websocket"
    );
    let _enter = span.enter();

    info!(
        client_id = %info.id,
        event = "connected",
        "websocket client connected"
    );

    // Split socket
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Channel setup
    let (tx, mut rx) = mpsc::channel::<ClientMessage>(32);

    clients.insert(info.id, tx);
    rooms
        .entry("lobby".to_string())
        .or_insert_with(Default::default)
        .insert(info.id);

    // Room key
    let lobby_key = room_keys
        .entry("lobby".to_string())
        .or_insert_with(RoomKey::generate)
        .clone();

    let mut current_key = lobby_key.clone();

    // Welcome message
    let welcome = format!(
        "=== Welcome to RustChatPro v0.9 ===\n\
         ID      : {}\n\
         Room    : {}\n\
         RoomKey : {}\n\
         ===================================",
        info.id,
        info.current_room,
        lobby_key.to_hex()
    );
    ws_sender.send(Message::Text(welcome)).await?;

    // Writer task
    let write_task = tokio::spawn(
        async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        }
        .instrument(tracing::Span::current())
    );

    // Read loop
    while let Some(msg) = ws_receiver.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "websocket receive error");
                break;
            }
        };

        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            Message::Ping(_) => continue,
            _ => continue,
        };

        let input = text.trim().to_string();
        if input.is_empty() {
            continue;
        }

        info.update_last_seen();

        if input == "PONG" || input == "/pong" {
            info!(
                client_id = %info.id,
                event = "pong",
                "client alive"
            );
            continue;
        }

        let should_continue = handle_ws_input_pub(
            &input,
            &mut info,
            &mut current_key,
            &clients,
            &rooms,
            &room_keys,
            &pool,
        ).await;

        if !should_continue {
            break;
        }
    }

    // Cleanup
    leave_room(&rooms, info.id, &info.current_room).await;

    let bye = ChatMessage::system(&format!(
        "[{}] left the room",
        &info.id.to_string()[..8]
    ));

    broadcast_to_room(
        &rooms,
        &clients,
        &info.current_room,
        &bye,
        Some(info.id),
    ).await;

    clients.remove(&info.id);

    info!(
        client_id = %info.id,
        remaining = clients.len(),
        event = "disconnected",
        "websocket client removed"
    );

    write_task.abort();

    Ok(())
}

// =========================
// INPUT HANDLER
// =========================

pub async fn handle_ws_input_pub(
    input: &str,
    info: &mut ClientInfo,
    current_key: &mut RoomKey,
    clients: &SharedClients,
    rooms: &SharedRooms,
    room_keys: &SharedRoomKeys,
    pool: &Arc<DbPool>,
) -> bool {
    use crate::db;

    if input.starts_with('/') {
        let parts: Vec<&str> = input.splitn(3, ' ').collect();

        match parts[0] {
            "/join" => {
                if parts.len() < 2 {
                    ws_send(clients, info.id, "Usage: /join <room>").await;
                    return true;
                }

                let room_name = parts[1].to_lowercase();

                let response = join_room(
                    rooms,
                    clients,
                    room_keys,
                    info.id,
                    &room_name,
                    &info.current_room,
                ).await;

                if let Some(key) = room_keys.get(&room_name) {
                    *current_key = key.clone();
                }

                info!(
                    client_id = %info.id,
                    old_room = %info.current_room,
                    new_room = %room_name,
                    event = "room_join",
                    "client joined room"
                );

                info.current_room = room_name;

                ws_send(clients, info.id, &response).await;
            }

            "/rooms" => {
                let list = list_rooms(rooms);
                ws_send(clients, info.id, &list).await;
            }

            "/history" => {
                ws_send(clients, info.id, "--- Loading history ---").await;

                match db::get_room_history(pool, &info.current_room, 50).await {
                    Ok(msgs) if msgs.is_empty() => {
                        ws_send(clients, info.id, "No history found.").await;
                    }
                    Ok(msgs) => {
                        for m in msgs {
                            ws_send(clients, info.id, &m).await;
                        }
                        ws_send(clients, info.id, "--- End of history ---").await;
                    }
                    Err(e) => {
                        warn!(error = %e, "history fetch failed");
                    }
                }
            }

            "/msg" => {
                if parts.len() < 3 {
                    ws_send(clients, info.id, "Usage: /msg <id> <text>").await;
                    return true;
                }

                handle_ws_dm(clients, info.id, parts[1], parts[2]).await;
            }

            "/quit" => {
                ws_send(clients, info.id, "Goodbye!").await;
                return false;
            }

            _ => {
                ws_send(
                    clients,
                    info.id,
                    &format!("Unknown command: {}", parts[0]),
                ).await;
            }
        }
    } else {
        match encrypt(input, current_key) {
            Ok(encrypted) => {
                // ✅ STRUCTURED LOG (core Phase 9)
                info!(
                    client_id = %info.id,
                    room = %info.current_room,
                    content_len = input.len(),
                    encrypted = true,
                    event = "message_received",
                    "websocket message received"
                );

                let enc_msg = ChatMessage::chat(
                    &encrypted,
                    &info.current_room,
                    &info.id.to_string(),
                );

                broadcast_to_room(
                    rooms,
                    clients,
                    &info.current_room,
                    &enc_msg,
                    None,
                ).await;

                // Save to DB with span propagation
                let pool_c = Arc::clone(pool);
                let room_c = info.current_room.clone();
                let sender_c = info.id.to_string();
                let enc_c = encrypted.clone();

                tokio::spawn(
                    async move {
                        if let Err(e) = db::save_message(
                            &pool_c,
                            &room_c,
                            &sender_c,
                            &enc_c,
                            "chat",
                        ).await {
                            warn!(error = %e, "DB save failed");
                        }
                    }
                    .instrument(tracing::Span::current())
                );

                let confirm = format!(
                    "[you → {}]: {}",
                    info.current_room,
                    input
                );

                ws_send(clients, info.id, &confirm).await;
            }

            Err(e) => {
                warn!(error = %e, "encryption failed");
            }
        }
    }

    true
}

// =========================
// DM HANDLER
// =========================

async fn handle_ws_dm(
    clients: &SharedClients,
    sender_id: uuid::Uuid,
    target_prefix: &str,
    text: &str,
) {
    let target = clients.iter().find(|e| {
        e.key().to_string().starts_with(target_prefix)
    });

    match target {
        Some(entry) => {
            let dm = ChatMessage::dm(
                text,
                &sender_id.to_string(),
                &entry.key().to_string(),
            );

            let _ = entry.value().send(dm.display()).await;

            ws_send(
                clients,
                sender_id,
                &format!(
                    "DM sent to {}",
                    &entry.key().to_string()[..8]
                ),
            ).await;
        }

        None => {
            ws_send(
                clients,
                sender_id,
                &format!("No client: {}", target_prefix),
            ).await;
        }
    }
}

// =========================
// SEND HELPER
// =========================

async fn ws_send(
    clients: &SharedClients,
    client_id: uuid::Uuid,
    msg: &str,
) {
    if let Some(sender) = clients.get(&client_id) {
        let _ = sender.send(msg.to_string()).await;
    }
}