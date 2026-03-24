// db.rs
// All database operations — creating tables, saving messages,
// loading history. This file is the only place in the whole
// codebase that touches SQLite directly.

use anyhow::Result;
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use chrono::Utc;

// DbPool is a type alias for our SQLite connection pool.
// A pool maintains multiple database connections so many
// async tasks can query the database simultaneously without
// waiting for each other.
pub type DbPool = Pool<Sqlite>;

// init_db() creates the database file and sets up all tables.
// Called once at server startup from main().
// Returns a pool that we share across all client tasks.
pub async fn init_db(database_url: &str) -> Result<DbPool> {
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await?;

    // CREATE TABLE IF NOT EXISTS means this is safe to run
    // every startup — it only creates the table once.
    // If the table already exists it does nothing.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            room        TEXT    NOT NULL,
            sender_id   TEXT    NOT NULL,
            content     TEXT    NOT NULL,
            timestamp   TEXT    NOT NULL,
            kind        TEXT    NOT NULL DEFAULT 'chat'
        )"
    )
    .execute(&pool)
    .await?;

    // Create an index on room + timestamp so history queries
    // are fast even with millions of messages stored.
    // An index is like a book index — lets SQLite find rows
    // without scanning the entire table.
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_messages_room
         ON messages(room, timestamp)"
    )
    .execute(&pool)
    .await?;

    tracing::info!("Database initialized — chat.db ready");
    Ok(pool)
}

// save_message() inserts one chat message into the database.
// Called every time a client sends a chat message to a room.
// Uses .await so it never blocks the Tokio runtime.
pub async fn save_message(
    pool: &DbPool,
    room: &str,
    sender_id: &str,
    content: &str,
    kind: &str,
) -> Result<()> {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO messages (room, sender_id, content, timestamp, kind)
         VALUES (?, ?, ?, ?, ?)"
    )
    // Each .bind() fills in one ? placeholder in order.
    // This is called a parameterized query — it prevents
    // SQL injection attacks automatically.
    .bind(room)
    .bind(sender_id)
    .bind(content)
    .bind(timestamp)
    .bind(kind)
    .execute(pool)
    .await?;

    Ok(())
}

// get_room_history() loads the last N messages from a room.
// Called when a client uses the /history command.
// Returns a Vec of formatted strings ready to display.
pub async fn get_room_history(
    pool: &DbPool,
    room: &str,
    limit: i64,
) -> Result<Vec<String>> {
    // This subquery trick gets the LAST N messages in
    // chronological order. Without it we would get the
    // first N messages instead of the most recent ones.
    let rows = sqlx::query(
        "SELECT sender_id, content, timestamp, kind
         FROM (
             SELECT sender_id, content, timestamp, kind
             FROM messages
             WHERE room = ?
             ORDER BY id DESC
             LIMIT ?
         )
         ORDER BY timestamp ASC"
    )
    .bind(room)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    // Convert each database row into a formatted display string
    let mut history = Vec::new();

    for row in rows {
        // row.get() extracts a column value by name
        let sender_id: String = row.get("sender_id");
        let content: String   = row.get("content");
        let timestamp: String = row.get("timestamp");
        let kind: String      = row.get("kind");

        // Format based on message kind
        let line = match kind.as_str() {
            "chat" => format!(
                "[{}][{}]: {}\n",
                timestamp,
                &sender_id[..8.min(sender_id.len())],
                content
            ),
            "system" => format!(
                "[{}] *** {} ***\n",
                timestamp,
                content
            ),
            _ => format!("[{}]: {}\n", timestamp, content),
        };

        history.push(line);
    }

    Ok(history)
}

// get_all_rooms() returns a list of rooms that have messages.
// Useful for showing active rooms with message counts.
pub async fn get_all_rooms(pool: &DbPool) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT room, COUNT(*) as msg_count
         FROM messages
         WHERE kind = 'chat'
         GROUP BY room
         ORDER BY msg_count DESC"
    )
    .fetch_all(pool)
    .await?;

    let mut rooms = Vec::new();
    for row in rows {
        let room: String = row.get("room");
        let count: i64   = row.get("msg_count");
        rooms.push(format!("  {} ({} messages)\n", room, count));
    }

    Ok(rooms)
}