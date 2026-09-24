use crate::model::{Account, AccountKind, NormalizedEvent};
use crate::{db, oauth, providers, state::AppState};
use chrono::{Duration, Utc};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};

#[derive(serde::Serialize, Clone)]
struct EventsPayload {
    events: Vec<NormalizedEvent>,
    generated_at: String,
}

fn emit_status(app: &AppHandle, state: &str, error: Option<String>) {
    let _ = app.emit(
        "sync://status",
        serde_json::json!({ "state": state, "error": error }),
    );
}

/// Runs one sync cycle: fetch all providers, write to SQLite and emit to the frontend.
pub async fn sync_once(app: &AppHandle) -> Result<(), String> {
    emit_status(app, "syncing", None);
    let state = app.state::<AppState>();

    // Snapshot config + accounts (short lock, released before any await).
    let (cfg, accounts) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (db::load_config(&conn)?, db::list_accounts(&conn)?)
    };

    let now = Utc::now();
    let from = now - Duration::minutes(cfg.window_before_minutes);
    let to = now + Duration::minutes(cfg.window_after_minutes);

    eprintln!(
        "[sync] {} account(s), window {} .. {}",
        accounts.len(),
        from.to_rfc3339(),
        to.to_rfc3339()
    );

    for acc in accounts.iter().filter(|a| a.enabled) {
        let provider = match providers::build(acc) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[sync] {}: {e}", acc.display_name);
                continue;
            }
        };
        match provider.list_events(from, to).await {
            Ok(events) => {
                eprintln!("[sync] {}: {} event(s) in window", acc.display_name, events.len());
                let mut conn = state.db.lock().map_err(|e| e.to_string())?;
                if let Err(e) = db::replace_account_events(&mut conn, &acc.id, &events) {
                    eprintln!("[sync] write {}: {e}", acc.display_name);
                }
            }
            Err(e) => {
                eprintln!("[sync] {}: ERROR {e}", acc.display_name);
                // Google token revoked/expired (Testing mode ~7 days) -> reopen login
                // using the credentials stored on the account (no env needed).
                if acc.kind == AccountKind::Google && e.contains("invalid_grant") {
                    maybe_reconnect_google(app, acc);
                }
            }
        }
    }

    let events = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::query_range(&conn, &from.to_rfc3339(), &to.to_rfc3339())?
    };
    let _ = app.emit(
        "calendar://events-updated",
        EventsPayload {
            events,
            generated_at: now.to_rfc3339(),
        },
    );
    emit_status(app, "idle", None);
    Ok(())
}

/// Reopens the Google OAuth flow (once per process) reusing the client_id/secret stored
/// on the account, saves the new refresh token in the keyring and re-syncs.
fn maybe_reconnect_google(app: &AppHandle, acc: &Account) {
    static BUSY: AtomicBool = AtomicBool::new(false);
    if BUSY.swap(true, Ordering::SeqCst) {
        return; // already reconnecting
    }

    let cid = acc.config.get("client_id").and_then(|v| v.as_str()).map(String::from);
    let csec = acc.config.get("client_secret").and_then(|v| v.as_str()).map(String::from);
    let (Some(cid), Some(csec)) = (cid, csec) else {
        BUSY.store(false, Ordering::SeqCst);
        eprintln!("[google] no stored credentials to reconnect");
        return;
    };
    let account_id = acc.id.clone();
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        eprintln!("[google] token expired -> reopening login...");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(oauth::authorize(&cid, &csec));
        });
        let res = loop {
            match rx.try_recv() {
                Ok(r) => break r,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break Err("OAuth thread ended".into())
                }
            }
        };
        match res {
            Ok(tokens) => match tokens.refresh_token {
                Some(rt) => {
                    if let Err(e) = oauth::tokens::store_refresh(&account_id, &rt) {
                        eprintln!("[google] store_refresh: {e}");
                    } else {
                        eprintln!("[google] reconnected.");
                        let _ = sync_once(&app).await;
                    }
                }
                None => eprintln!("[google] reconnect returned no refresh_token"),
            },
            Err(e) => eprintln!("[google] reconnect failed: {e}"),
        }
        BUSY.store(false, Ordering::SeqCst);
    });
}

/// Sync loop every N minutes (N comes from config, re-read each iteration).
pub fn start_timer(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(e) = sync_once(&app).await {
                eprintln!("[sync] error: {e}");
            }
            let mins = {
                let state = app.state::<AppState>();
                let guard = state.db.lock().ok();
                guard
                    .and_then(|c| db::load_config(&c).ok())
                    .map(|c| c.sync_interval_minutes)
                    .unwrap_or(3)
            };
            tokio::time::sleep(std::time::Duration::from_secs(mins.max(1) * 60)).await;
        }
    });
}
