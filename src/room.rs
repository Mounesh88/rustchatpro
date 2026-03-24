use crate::crypto::RoomKey;
use crate::types::{ChatMessage, SharedClients, SharedRoomKeys, SharedRooms};
use tracing::info;
use uuid::Uuid;

pub async fn join_room(
    rooms: &SharedRooms,
    clients: &SharedClients,
    room_keys: &SharedRoomKeys,
    client_id: Uuid,
    room_name: &str,
    old_room: &str,
) -> String {
    // Leave old room first
    leave_room(rooms, client_id, old_room).await;

    // Add client to new room
    rooms
        .entry(room_name.to_string())
        .or_insert_with(Default::default)
        .insert(client_id);

    // Get or create room key
    let key = room_keys
        .entry(room_name.to_string())
        .or_insert_with(RoomKey::generate)
        .clone();

    info!(
        client_id = %client_id,
        room = %room_name,
        "client joined room"
    );

    let count = rooms.get(room_name).map(|r| r.len()).unwrap_or(0);

    // Notify others about join
    let notify = ChatMessage::system(&format!(
        "[{}] joined room '{}'",
        &client_id.to_string()[..8],
        room_name
    ));
    broadcast_to_room(rooms, clients, room_name, &notify, Some(client_id)).await;

    // 🔥 CRITICAL FIX: Broadcast ROOM KEY to ALL clients in room
    let key_msg = ChatMessage::system(&format!(
        "ROOM_KEY:{}:{}",
        room_name,
        key.to_hex()
    ));

    broadcast_to_room(rooms, clients, room_name, &key_msg, None).await;

    // Return response to joining client
    format!(
        "Joined room '{}' ({} member{})\nROOM_KEY:{}\n",
        room_name,
        count,
        if count == 1 { "" } else { "s" },
        key.to_hex()
    )
}

pub async fn leave_room(
    rooms: &SharedRooms,
    client_id: Uuid,
    room_name: &str,
) {
    if let Some(mut room) = rooms.get_mut(room_name) {
        room.remove(&client_id);
    }
}

pub async fn broadcast_to_room(
    rooms: &SharedRooms,
    clients: &SharedClients,
    room_name: &str,
    msg: &ChatMessage,
    exclude_id: Option<Uuid>,
) {
    let member_ids: Vec<Uuid> = match rooms.get(room_name) {
        Some(room) => room.iter().cloned().collect(),
        None => return,
    };

    let display = msg.display();

    for member_id in member_ids {
        if Some(member_id) == exclude_id {
            continue;
        }

        if let Some(sender) = clients.get(&member_id) {
            let _ = sender.send(display.clone()).await;
        }
    }
}

pub fn list_rooms(rooms: &SharedRooms) -> String {
    let mut result = String::from("--- Active rooms ---\n");

    for entry in rooms.iter() {
        result.push_str(&format!(
            "  {} ({} member{})\n",
            entry.key(),
            entry.value().len(),
            if entry.value().len() == 1 { "" } else { "s" }
        ));
    }

    result
}