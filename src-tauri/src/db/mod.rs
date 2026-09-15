use crate::model::{Account, AccountKind, AppConfig, Attendee, NormalizedEvent};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);

CREATE TABLE IF NOT EXISTS accounts (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    color         TEXT NOT NULL,
    config_json   TEXT NOT NULL,
    sync_token    TEXT,
    enabled       INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id             TEXT PRIMARY KEY,
    account_id     TEXT NOT NULL,
    title          TEXT NOT NULL,
    start_utc      TEXT NOT NULL,
    end_utc        TEXT NOT NULL,
    all_day        INTEGER NOT NULL DEFAULT 0,
    description    TEXT,
    location       TEXT,
    meeting_link   TEXT,
    html_link      TEXT,
    attendees_json TEXT,
    color          TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_range ON events(start_utc, end_utc);
CREATE INDEX IF NOT EXISTS idx_events_account ON events(account_id);

CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT NOT NULL);
"#;

pub fn open(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| e.to_string())?;
    conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
    Ok(conn)
}

// ---------- accounts ----------

pub fn add_account(conn: &Connection, acc: &Account) -> Result<(), String> {
    conn.execute(
        "INSERT INTO accounts (id, kind, display_name, color, config_json, sync_token, enabled, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))",
        params![
            acc.id,
            acc.kind.as_str(),
            acc.display_name,
            acc.color,
            acc.config.to_string(),
            acc.sync_token,
            acc.enabled as i64,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_accounts(conn: &Connection) -> Result<Vec<Account>, String> {
    let mut stmt = conn
        .prepare("SELECT id, kind, display_name, color, config_json, sync_token, enabled FROM accounts")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            let kind_s: String = r.get(1)?;
            let config_s: String = r.get(4)?;
            Ok(Account {
                id: r.get(0)?,
                kind: AccountKind::from_str(&kind_s).unwrap_or(AccountKind::Ics),
                display_name: r.get(2)?,
                color: r.get(3)?,
                config: serde_json::from_str(&config_s).unwrap_or(serde_json::Value::Null),
                sync_token: r.get(5)?,
                enabled: r.get::<_, i64>(6)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn count_accounts(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))
        .map_err(|e| e.to_string())
}

pub fn remove_account(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM events WHERE account_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_account_color(conn: &Connection, id: &str, color: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE accounts SET color = ?2 WHERE id = ?1",
        params![id, color],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- events ----------

/// Substitui todos os eventos de uma conta pelos recém-sincronizados (delete + insert).
pub fn replace_account_events(
    conn: &mut Connection,
    account_id: &str,
    events: &[NormalizedEvent],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM events WHERE account_id = ?1", params![account_id])
        .map_err(|e| e.to_string())?;
    for ev in events {
        let attendees_json = serde_json::to_string(&ev.attendees).unwrap_or_else(|_| "[]".into());
        tx.execute(
            "INSERT OR REPLACE INTO events
             (id, account_id, title, start_utc, end_utc, all_day, description, location, meeting_link, html_link, attendees_json, color, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12, datetime('now'))",
            params![
                ev.id, ev.account_id, ev.title, ev.start, ev.end, ev.all_day as i64,
                ev.description, ev.location, ev.meeting_link, ev.html_link, attendees_json, ev.color,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn row_to_event(r: &rusqlite::Row) -> rusqlite::Result<NormalizedEvent> {
    let attendees_json: Option<String> = r.get(10)?;
    let attendees: Vec<Attendee> = attendees_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let account_id: String = r.get(1)?;
    Ok(NormalizedEvent {
        id: r.get(0)?,
        source_id: account_id.clone(),
        account_id,
        title: r.get(2)?,
        start: r.get(3)?,
        end: r.get(4)?,
        all_day: r.get::<_, i64>(5)? != 0,
        description: r.get(6)?,
        location: r.get(7)?,
        meeting_link: r.get(8)?,
        html_link: r.get(9)?,
        attendees,
        color: r.get(11)?,
    })
}

const EVENT_COLS: &str =
    "id, account_id, title, start_utc, end_utc, all_day, description, location, meeting_link, html_link, attendees_json, color";

/// Eventos que intersectam a janela [from, to] (ISO8601 UTC).
pub fn query_range(conn: &Connection, from: &str, to: &str) -> Result<Vec<NormalizedEvent>, String> {
    let sql = format!(
        "SELECT {EVENT_COLS} FROM events WHERE end_utc > ?1 AND start_utc < ?2 ORDER BY start_utc"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![from, to], row_to_event)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn get_event(conn: &Connection, id: &str) -> Result<Option<NormalizedEvent>, String> {
    let sql = format!("SELECT {EVENT_COLS} FROM events WHERE id = ?1");
    conn.query_row(&sql, params![id], row_to_event)
        .optional()
        .map_err(|e| e.to_string())
}

// ---------- config ----------

pub fn load_config(conn: &Connection) -> Result<AppConfig, String> {
    let val: Option<String> = conn
        .query_row("SELECT value FROM config WHERE key = 'app'", [], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    match val {
        Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        None => Ok(AppConfig::default()),
    }
}

pub fn save_config(conn: &Connection, cfg: &AppConfig) -> Result<(), String> {
    let s = serde_json::to_string(cfg).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES ('app', ?1)",
        params![s],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
