use rusqlite::Connection;
use std::sync::Mutex;

/// Global state managed by Tauri. A single SQLite connection behind a Mutex
/// (rusqlite ops are synchronous; the lock is never held across an `.await`).
pub struct AppState {
    pub db: Mutex<Connection>,
}
