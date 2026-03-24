mod client;
mod crypto;
mod db;
mod heartbeat;
mod logging;
mod room;
mod server;
mod types;
mod ws_handler;

#[cfg(test)]
mod tests;

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    logging::init_logging()?;

    info!(version = "0.9", "Starting RustChatPro");

    let pool      = db::init_db("sqlite:chat.db").await?;
    let clients   = types::new_shared_clients();
    let rooms     = types::new_shared_rooms();
    let room_keys = types::new_shared_room_keys();

    info!("All systems initialized — starting server");

    let hb_clients = std::sync::Arc::clone(&clients);
    let hb_rooms   = std::sync::Arc::clone(&rooms);
    tokio::spawn(async move {
        heartbeat::run_heartbeat(hb_clients, hb_rooms).await;
    });

    server::run_server_with_state(
        "127.0.0.1:8080",
        pool,
        clients,
        rooms,
        room_keys,
    ).await?;

    Ok(())
}