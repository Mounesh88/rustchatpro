use crate::client::handle_client;
use crate::db::DbPool;
use crate::types::{SharedClients, SharedRoomKeys, SharedRooms};
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{info, warn};

pub async fn run_server_with_state(
    _addr: &str,
    pool: DbPool,
    clients: SharedClients,
    rooms: SharedRooms,
    room_keys: SharedRoomKeys,
) -> Result<()> {
    let pool = Arc::new(pool);

    let http_listener = TcpListener::bind("127.0.0.1:8080").await?;
    let ws_listener   = TcpListener::bind("127.0.0.1:8081").await?;
    let tcp_listener  = TcpListener::bind("127.0.0.1:8082").await?;

    info!("HTTP server on http://127.0.0.1:8080");
    info!("WebSocket server on ws://127.0.0.1:8081");
    info!("TCP server on 127.0.0.1:8082");
    info!("Open http://127.0.0.1:8080 in your browser");

    let ws_clients  = Arc::clone(&clients);
    let ws_rooms    = Arc::clone(&rooms);
    let ws_keys     = Arc::clone(&room_keys);
    let ws_pool     = Arc::clone(&pool);

    let tcp_clients = Arc::clone(&clients);
    let tcp_rooms   = Arc::clone(&rooms);
    let tcp_keys    = Arc::clone(&room_keys);
    let tcp_pool    = Arc::clone(&pool);

    // WebSocket listener
    tokio::spawn(async move {
        loop {
            match ws_listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!(
                        peer_addr = %peer_addr,
                        protocol = "websocket",
                        "new connection"
                    );
                    let c = Arc::clone(&ws_clients);
                    let r = Arc::clone(&ws_rooms);
                    let k = Arc::clone(&ws_keys);
                    let p = Arc::clone(&ws_pool);
                    tokio::spawn(async move {
                        if let Err(e) = handle_websocket(
                            stream, c, r, k, p
                        ).await {
                            warn!(error = %e, "WS error");
                        }
                    });
                }
                Err(e) => warn!(error = %e, "WS accept error"),
            }
        }
    });

    // TCP/telnet listener
    tokio::spawn(async move {
        loop {
            match tcp_listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!(
                        peer_addr = %peer_addr,
                        protocol = "tcp",
                        "new connection"
                    );
                    let c = Arc::clone(&tcp_clients);
                    let r = Arc::clone(&tcp_rooms);
                    let k = Arc::clone(&tcp_keys);
                    let p = Arc::clone(&tcp_pool);
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(
                            stream, c, r, k, p
                        ).await {
                            warn!(error = %e, "TCP error");
                        }
                    });
                }
                Err(e) => warn!(error = %e, "TCP accept error"),
            }
        }
    });

    // HTTP listener — main task
    loop {
        match http_listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!(
                    peer_addr = %peer_addr,
                    protocol = "http",
                    "new connection"
                );
                tokio::spawn(async move {
                    if let Err(e) = serve_http(stream).await {
                        warn!(error = %e, "HTTP error");
                    }
                });
            }
            Err(e) => warn!(error = %e, "HTTP accept error"),
        }
    }
}

async fn serve_http(
    mut stream: tokio::net::TcpStream
) -> Result<()> {
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    
    if n == 0 {
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&buf[..n]);
    
    // Parse the HTTP request line
    let request_line = match request_str.lines().next() {
        Some(line) => line,
        None => {
            send_404(&mut stream).await?;
            return Ok(());
        }
    };

    // Extract the path from "GET /path HTTP/1.1"
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        send_404(&mut stream).await?;
        return Ok(());
    }

    let request_path = parts[1];
    
    // Determine file to serve
    let file_path = if request_path == "/" {
        "static/index.html".to_string()
    } else {
        format!("static{}", request_path)
    };

    // Sanitize path to prevent directory traversal
    let file_path = file_path.replace("../", "").replace("..\\", "");

    // Try to read the file
    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/html; charset=utf-8\r\n\
                 Content-Length: {}\r\n\
                 Cache-Control: no-cache, no-store, must-revalidate\r\n\
                 Pragma: no-cache\r\n\
                 Expires: 0\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
                content.len(),
                content
            );
            stream.write_all(response.as_bytes()).await?;
        }
        Err(_) => {
            send_404(&mut stream).await?;
        }
    }
    stream.flush().await?;
    Ok(())
}

async fn send_404(stream: &mut tokio::net::TcpStream) -> Result<()> {
    let response = "HTTP/1.1 404 Not Found\r\n\
                    Content-Type: text/plain\r\n\
                    Content-Length: 9\r\n\
                    Connection: close\r\n\
                    \r\n\
                    Not Found";
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn handle_websocket(
    stream: tokio::net::TcpStream,
    clients: SharedClients,
    rooms: SharedRooms,
    room_keys: SharedRoomKeys,
    pool: Arc<DbPool>,
) -> Result<()> {
    use crate::crypto::RoomKey;
    use crate::room::{broadcast_to_room, leave_room};
    use crate::types::{ChatMessage, ClientInfo, ClientMessage};
    use crate::ws_handler::handle_ws_input_pub;
    use futures_util::{SinkExt, StreamExt};
    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::Message;

    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    info!("WebSocket handshake complete");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut info = ClientInfo::new();
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
         ===================================",
        info.id,
        info.current_room,
        lobby_key.to_hex()
    );
    ws_sender.send(Message::Text(welcome)).await?;

    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender
                .send(Message::Text(msg))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    while let Some(result) = ws_receiver.next().await {
        let msg = match result {
            Ok(m)  => m,
            Err(_) => break,
        };
        match msg {
            Message::Text(text) => {
                let input = text.trim().to_string();
                if input.is_empty() { continue; }
                info.update_last_seen();
                if input == "PONG" { continue; }
                let cont = handle_ws_input_pub(
                    &input,
                    &mut info,
                    &mut current_key,
                    &clients,
                    &rooms,
                    &room_keys,
                    &pool,
                ).await;
                if !cont { break; }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    leave_room(&rooms, info.id, &info.current_room).await;
    let bye = ChatMessage::system(&format!(
        "[{}] left", &info.id.to_string()[..8]
    ));
    broadcast_to_room(
        &rooms, &clients,
        &info.current_room, &bye, Some(info.id)
    ).await;
    clients.remove(&info.id);
    info!(
        client_id = %info.id,
        remaining = clients.len(),
        "WS client removed"
    );
    write_task.abort();
    Ok(())
}