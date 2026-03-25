# RustChatPro+

[![CI](https://github.com/YOUR_USERNAME/rustchatpro/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/rustchatpro/actions/workflows/ci.yml)

# 🚀 RustChatPro — High-Performance Real-Time Chat System

## 📌 Project Overview

**RustChatPro** is a high-performance, real-time chat server built using **Rust**, designed to demonstrate modern backend engineering principles such as **asynchronous programming**, **low-latency communication**, and **scalable system design**.

The system leverages **WebSockets** for bi-directional communication and the **Tokio async runtime** to efficiently manage multiple concurrent clients. It includes production-oriented features like **heartbeat monitoring**, **structured logging**, and **room-based messaging**.

This project reflects a strong focus on **performance, reliability, and observability**—key aspects of real-world backend systems.

---

## 🔥 Key Features

### ⚡ High Performance

* Built with Rust for memory safety and zero-cost abstractions
* Efficient async execution using Tokio
* Low-latency real-time message delivery

### 🛡️ Safety & Reliability

* Strong type system ensures compile-time guarantees
* Safe concurrency without data races
* Graceful handling of client connections and disconnections

### 🧵 Concurrency & Scalability

* Handles multiple clients concurrently using async/await
* Non-blocking I/O for optimal resource utilization
* Designed to scale with increasing client load

### 🔌 Real-Time Communication

* WebSocket-based full-duplex communication
* Instant message broadcasting across connected clients

### 🏠 Room-Based Messaging

* Logical grouping of users into rooms (e.g., `lobby`)
* Scoped message delivery within rooms

### ❤️ Heartbeat Monitoring

* Periodic health checks for connected clients
* Automatic detection and cleanup of inactive connections

### 📊 Observability & Logging

* Structured logging with timestamps and thread IDs
* File-based logs (`logs/server.log`) for debugging and monitoring
* Real-time logs available via console output

### 🗄️ Persistence Layer

* SQLite integration (`chat.db`) for extendable data storage
* Foundation for message history and user tracking

### ⚙️ CI/CD Integration

* GitHub Actions pipeline for automated builds and tests
* Ensures code quality and reliability

---

## 🛠️ Technology Stack

| Component     | Technology                   |
| ------------- | ---------------------------- |
| Language      | Rust                         |
| Async Runtime | Tokio                        |
| Communication | WebSockets                   |
| Database      | SQLite                       |
| Logging       | Tracing / Structured Logging |
| Frontend      | HTML (Minimal UI)            |
| CI/CD         | GitHub Actions               |

---

## 🚀 Quick Start

### 🔹 Prerequisites

* Install Rust: https://www.rust-lang.org/tools/install

---

### 🔹 Clone the Repository

```bash
git clone https://github.com/Mounesh88/rustchatpro.git
cd rustchatpro
```

---

### 🔹 Run the Server

```bash
cargo run
```

You should see:

```
Open http://127.0.0.1:8080 in your browser
```

---

### 🔹 Access the Application

Open your browser:

```
http://127.0.0.1:8080
```

---

### 🔹 Test Real-Time Chat

1. Open multiple browser tabs
2. Send messages from different tabs
3. Observe real-time updates across clients

---

## 🧠 Architecture

```
           ┌────────────────────┐
           │     Browser UI     │
           │  (Multiple Tabs)   │
           └─────────┬──────────┘
                     │
                     ▼
           ┌────────────────────┐
           │   WebSocket Layer  │
           │  (Full Duplex I/O) │
           └─────────┬──────────┘
                     │
                     ▼
        ┌─────────────────────────────┐
        │     Tokio Async Runtime     │
        │ (Concurrent Task Handling)  │
        └───────┬─────────┬───────────┘
                │         │
                ▼         ▼
        ┌──────────┐  ┌──────────────┐
        │  Rooms   │  │  Heartbeat   │
        │ Manager  │  │  Monitoring  │
        └────┬─────┘  └──────┬───────┘
             │               │
             ▼               ▼
        ┌──────────┐   ┌────────────┐
        │ Shared   │   │  Logging   │
        │ State    │   │  System    │
        └──────────┘   └────────────┘
               │
               ▼
        ┌──────────────┐
        │   SQLite DB  │
        │  (chat.db)   │
        └──────────────┘
```

---

## 📂 Project Structure

```
rustchatpro/
├── src/
│   ├── main.rs
│   ├── server.rs
│   ├── ws_handler.rs
│   ├── room.rs
│   ├── heartbeat.rs
│   ├── db.rs
│   ├── logging.rs
│   └── types.rs
│
├── static/
│   └── index.html
│
├── logs/
│   └── server.log
│
├── chat.db
├── Cargo.toml
└── README.md
```

---

## 📊 Example Logs

```
heartbeat tick client_count=4
heartbeat complete alive_count=4
WebSocket handshake complete
client joined room
```

---

## ⚠️ Notes

* Log file output may be **buffered**; real-time logs are visible in the terminal
* Ensure port `8080` is available before running the server

---

## 🔮 Future Enhancements

* 🔐 JWT-based authentication
* 🌐 Cloud deployment (AWS / Azure)
* 📦 Docker containerization
* ⚡ Redis for pub/sub scaling
* 📊 Admin dashboard for monitoring
* 💬 Message persistence and history APIs

---

## 💼 Author

**Mounesh Rayalla**
Master’s in Computer Science
Focus: Cloud, AI/ML, Backend Engineering

---

## ⭐ Support

If you find this project useful, consider giving it a ⭐ on GitHub!


