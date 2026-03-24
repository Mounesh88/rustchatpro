use crate::types::{SharedClients, SharedRooms};
use tokio::time::{interval, Duration};
use tracing::{info, warn};

#[allow(dead_code)]
const TIMEOUT_SECS: u64 = 90;

const PING_INTERVAL: Duration = Duration::from_secs(30);

pub async fn run_heartbeat(
    clients: SharedClients,
    rooms: SharedRooms,
) {
    let mut ticker = interval(PING_INTERVAL);
    info!("Heartbeat task started — pinging every {}s",
        PING_INTERVAL.as_secs());

    loop {
        ticker.tick().await;

        info!(
            client_count = clients.len(),
            "heartbeat tick"
        );

        let mut dead_clients = Vec::new();

        for entry in clients.iter() {
            let client_id = *entry.key();
            let sender    = entry.value();
            let ping_msg  = format!("[server] PING\n");

            if sender.send(ping_msg).await.is_err() {
                warn!(
                    client_id = %client_id,
                    "client channel closed — marking dead"
                );
                dead_clients.push(client_id);
            }
        }

        for client_id in dead_clients {
            clients.remove(&client_id);
            for mut room in rooms.iter_mut() {
                room.value_mut().remove(&client_id);
            }
            info!(
                client_id = %client_id,
                "dead client removed"
            );
        }

        info!(
            alive_count = clients.len(),
            "heartbeat complete"
        );
    }
}