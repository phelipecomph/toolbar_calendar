use rusqlite::Connection;
use std::sync::Mutex;

/// Estado global gerenciado pelo Tauri. Conexão SQLite única sob Mutex
/// (operações rusqlite são síncronas; nunca seguramos o lock através de `.await`).
pub struct AppState {
    pub db: Mutex<Connection>,
}
