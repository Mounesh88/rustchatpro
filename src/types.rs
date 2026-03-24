use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use crate::crypto::RoomKey;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageKind {
    Chat,
    DirectMessage,
    Command,
    System,
    Ack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub kind:      MessageKind,
    pub content:   String,
    pub room:      String,
    pub sender_id: String,
    pub timestamp: String,
    pub target_id: String,
}

impl ChatMessage {
    pub fn chat(content: &str, room: &str, sender_id: &str) -> Self {
        Self {
            kind:      MessageKind::Chat,
            content:   content.to_string(),
            room:      room.to_string(),
            sender_id: sender_id.to_string(),
            timestamp: Utc::now().format("%H:%M:%S").to_string(),
            target_id: String::new(),
        }
    }

    pub fn dm(content: &str, sender_id: &str, target_id: &str) -> Self {
        Self {
            kind:      MessageKind::DirectMessage,
            content:   content.to_string(),
            room:      String::new(),
            sender_id: sender_id.to_string(),
            timestamp: Utc::now().format("%H:%M:%S").to_string(),
            target_id: target_id.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        Self {
            kind:      MessageKind::System,
            content:   content.to_string(),
            room:      String::new(),
            sender_id: String::from("server"),
            timestamp: Utc::now().format("%H:%M:%S").to_string(),
            target_id: String::new(),
        }
    }

    pub fn ack(content: &str) -> Self {
        Self {
            kind:      MessageKind::Ack,
            content:   content.to_string(),
            room:      String::new(),
            sender_id: String::from("server"),
            timestamp: Utc::now().format("%H:%M:%S").to_string(),
            target_id: String::new(),
        }
    }

    pub fn display(&self) -> String {
        match self.kind {
            MessageKind::Chat => format!(
                "[{}][{}][{}]: {}\n",
                self.timestamp,
                self.room,
                &self.sender_id[..8.min(self.sender_id.len())],
                self.content
            ),
            MessageKind::DirectMessage => format!(
                "[{}][DM from {}]: {}\n",
                self.timestamp,
                &self.sender_id[..8.min(self.sender_id.len())],
                self.content
            ),
            MessageKind::System => format!(
                "[{}] *** {} ***\n",
                self.timestamp,
                self.content
            ),
            MessageKind::Ack => format!(
                "[{}] > {}\n",
                self.timestamp,
                self.content
            ),
            MessageKind::Command => String::new(),
        }
    }
}

#[allow(dead_code)]
pub fn encode(msg: &ChatMessage) -> anyhow::Result<Vec<u8>> {
    let data = bincode::serialize(msg)?;
    let len  = data.len() as u32;
    let mut buf = Vec::with_capacity(4 + data.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&data);
    Ok(buf)
}

#[allow(dead_code)]
pub fn decode(bytes: &[u8]) -> anyhow::Result<ChatMessage> {
    Ok(bincode::deserialize(bytes)?)
}

#[derive(Clone)]
pub struct ClientInfo {
    pub id:           Uuid,
    pub current_room: String,
    pub last_seen:    Arc<Mutex<Instant>>,
}

impl ClientInfo {
    pub fn new() -> Self {
        Self {
            id:           Uuid::new_v4(),
            current_room: "lobby".to_string(),
            last_seen:    Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn update_last_seen(&mut self) {
        if let Ok(mut guard) = self.last_seen.lock() {
            *guard = Instant::now();
        }
    }
}

pub type ClientMessage = String;
pub type SharedClients = Arc<DashMap<Uuid, mpsc::Sender<ClientMessage>>>;
pub type SharedRooms = Arc<DashMap<String, HashSet<Uuid>>>;
pub type SharedRoomKeys = Arc<DashMap<String, RoomKey>>;

// Convenience constructors
pub fn new_shared_clients() -> SharedClients {
    Arc::new(DashMap::new())
}

pub fn new_shared_rooms() -> SharedRooms {
    Arc::new(DashMap::new())
}

pub fn new_shared_room_keys() -> SharedRoomKeys {
    Arc::new(DashMap::new())
}
