# RustChatPro+

[![CI](https://github.com/YOUR_USERNAME/rustchatpro/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/rustchatpro/actions/workflows/ci.yml)

A modern, multi-client chat platform built entirely in Rust.

## Features

- Real-time messaging with Tokio async runtime
- Multiple chat rooms with isolation
- Private direct messaging
- End-to-end encryption with AES-256-GCM
- SQLite message persistence
- WebSocket browser client
- Connection heartbeat monitoring
- Structured logging with tracing

## Architecture

| Component | Technology |
|-----------|------------|
| Async runtime | Tokio |
| Concurrency | DashMap |
| Serialization | Bincode + Serde |
| Database | SQLite + SQLx |
| Encryption | AES-256-GCM |
| WebSocket | tokio-tungstenite |
| Logging | tracing + tracing-subscriber |

## Running
```bash
# Start the server
RUST_LOG=info cargo run

# Open browser client
open http://127.0.0.1:8080

# Connect telnet client
telnet 127.0.0.1 8082
```

## Testing
```bash
cargo test
```

## Project Structure
```
src/
├── main.rs         # Entry point
├── server.rs       # TCP/WS/HTTP listeners
├── client.rs       # TCP client handler
├── ws_handler.rs   # WebSocket handler
├── room.rs         # Room management
├── types.rs        # Shared data types
├── crypto.rs       # AES-256-GCM encryption
├── db.rs           # SQLite persistence
├── heartbeat.rs    # Connection health
├── logging.rs      # Structured logging
└── tests.rs        # Unit tests
static/
└── index.html      # Browser chat UI
logs/
└── server.log      # Server log file
```

## Commands

| Command | Description |
|---------|-------------|
| `/join <room>` | Join a chat room |
| `/msg <id> <text>` | Send private message |
| `/rooms` | List active rooms |
| `/history` | Load message history |
| `/quit` | Disconnect |