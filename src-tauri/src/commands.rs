use crate::model::{mock_events, Account, AccountKind, AppConfig, Edge, NormalizedEvent};
use crate::{db, oauth, state::AppState, sync};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_events(
    state: State<AppState>,
    from: String,
    to: String,
) -> Result<Vec<NormalizedEvent>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    // No accounts yet -> show mocks so the strip isn't empty on first run.
    if db::count_accounts(&conn)? == 0 {
        return Ok(mock_events());
    }
    db::query_range(&conn, &from, &to)
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<AppConfig, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::load_config(&conn)
}

#[tauri::command]
pub fn set_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::save_config(&conn, &config)
}

#[tauri::command]
pub fn list_accounts(state: State<AppState>) -> Result<Vec<Account>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::list_accounts(&conn)
}

#[tauri::command]
pub fn add_ics_account(
    state: State<AppState>,
    display_name: String,
    color: String,
    url: String,
) -> Result<Account, String> {
    let acc = Account {
        id: uuid::Uuid::new_v4().to_string(),
        kind: AccountKind::Ics,
        display_name,
        color,
        config: serde_json::json!({ "url": url }),
        sync_token: None,
        enabled: true,
    };
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::add_account(&conn, &acc)?;
    Ok(acc)
}

#[tauri::command]
pub fn remove_account(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::remove_account(&conn, &id)
}

#[tauri::command]
pub fn set_account_color(state: State<AppState>, id: String, color: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_account_color(&conn, &id, &color)
}

#[tauri::command]
pub async fn trigger_sync(app: AppHandle) -> Result<(), String> {
    sync::sync_once(&app).await
}

/// Runs the Google OAuth flow (opens the browser), stores the account + refresh token
/// in the keyring and triggers a sync. Reused by the UI and by startup auto-seed.
pub async fn add_google_account_flow(
    app: AppHandle,
    client_id: String,
    client_secret: String,
) -> Result<Account, String> {
    // authorize() is blocking (tiny_http + reqwest::blocking) -> run it on its own
    // thread, outside the tokio runtime; poll for the result without blocking the executor.
    let cid = client_id.clone();
    let csec = client_secret.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(oauth::authorize(&cid, &csec));
    });
    let tokens = loop {
        match rx.try_recv() {
            Ok(res) => break res?,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err("OAuth flow interrupted".into());
            }
        }
    };

    let refresh = tokens
        .refresh_token
        .ok_or("Google returned no refresh_token (revoke access and try again)")?;

    let acc = Account {
        id: uuid::Uuid::new_v4().to_string(),
        kind: AccountKind::Google,
        display_name: "Google".into(),
        color: "#4285f4".into(),
        config: serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "calendar_id": "primary"
        }),
        sync_token: None,
        enabled: true,
    };

    // Store the token FIRST so any sync that sees the account already finds the refresh
    // token in the keyring.
    oauth::tokens::store_refresh(&acc.id, &refresh)?;
    {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::add_account(&conn, &acc)?;
    }

    let _ = sync::sync_once(&app).await;
    Ok(acc)
}

#[tauri::command]
pub async fn add_google_account(
    app: AppHandle,
    client_id: String,
    client_secret: String,
) -> Result<Account, String> {
    add_google_account_flow(app, client_id, client_secret).await
}

#[tauri::command]
pub fn open_in_browser(app: AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct AnchorRect {
    pub x: f64,
    pub y: f64,
    #[allow(dead_code)]
    pub width: f64,
    #[allow(dead_code)]
    pub height: f64,
}

/// Places a popup adjacent to the strip on its screen-interior side, then clamps it
/// fully inside the monitor so it's never off-screen. Anchored to the monitor rect
/// (not the strip window, which can be stale right after a redock).
///
/// `along_px` = offset along the main axis from the monitor's near corner; `None`
/// means "far end" (where the clock sits). `thickness_px` = strip thickness. Physical px.
fn place_popup(
    strip: &WebviewWindow,
    edge: Edge,
    monitor_index: usize,
    thickness_px: i32,
    along_px: Option<i32>,
    w: i32,
    h: i32,
) -> Result<PhysicalPosition<i32>, String> {
    let mons = strip.available_monitors().map_err(|e| e.to_string())?;
    let (ml, mt, mr, mb) = match mons.get(monitor_index).or_else(|| mons.first()) {
        Some(m) => {
            let p = m.position();
            let s = m.size();
            (p.x, p.y, p.x + s.width as i32, p.y + s.height as i32)
        }
        None => {
            let sp = strip.outer_position().map_err(|e| e.to_string())?;
            let ss = strip.outer_size().map_err(|e| e.to_string())?;
            (sp.x, sp.y, sp.x + ss.width as i32, sp.y + ss.height as i32)
        }
    };

    let horizontal = matches!(edge, Edge::Top | Edge::Bottom);
    let main_len = if horizontal { mr - ml } else { mb - mt };
    let along = along_px.unwrap_or(main_len);

    // Open toward the screen interior; align to the anchor along the main axis.
    let (mut x, mut y) = match edge {
        Edge::Bottom => (ml + along - w / 2, mb - thickness_px - h),
        Edge::Top => (ml + along - w / 2, mt + thickness_px),
        Edge::Left => (ml + thickness_px, mt + along - h / 2),
        Edge::Right => (mr - thickness_px - w, mt + along - h / 2),
    };

    // Clamp fully inside the monitor.
    x = x.clamp(ml, (mr - w).max(ml));
    y = y.clamp(mt, (mb - h).max(mt));
    Ok(PhysicalPosition::new(x, y))
}

/// Opens the `detail` window positioned above the clicked block and sends the event.
#[tauri::command]
pub fn open_detail(
    app: AppHandle,
    state: State<AppState>,
    event_id: String,
    anchor: AnchorRect,
) -> Result<(), String> {
    let (event, cfg) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (db::get_event(&conn, &event_id)?, db::load_config(&conn)?)
    };
    let Some(event) = event else {
        return Err("event not found".into());
    };

    let strip = app.get_webview_window("strip").ok_or("strip window missing")?;
    let detail = app.get_webview_window("detail").ok_or("detail window missing")?;

    let scale = strip.scale_factor().unwrap_or(1.0);
    let (w, h) = ((320.0 * scale) as i32, (200.0 * scale) as i32);
    let thickness = (cfg.strip_height_logical as f64 * scale) as i32;
    // The block's main-axis offset: X for horizontal strips, Y for vertical ones.
    let along = match cfg.edge {
        Edge::Left | Edge::Right => (anchor.y * scale) as i32,
        _ => (anchor.x * scale) as i32,
    };
    let pos = place_popup(&strip, cfg.edge, cfg.monitor_index, thickness, Some(along), w, h)?;

    detail.set_position(pos).map_err(|e| e.to_string())?;
    detail
        .emit("detail://event", &event)
        .map_err(|e| e.to_string())?;
    detail.show().map_err(|e| e.to_string())?;
    detail.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn close_detail(app: AppHandle) -> Result<(), String> {
    if let Some(detail) = app.get_webview_window("detail") {
        let _ = detail.hide();
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct MonitorDto {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn get_monitors(app: AppHandle) -> Result<Vec<MonitorDto>, String> {
    let strip = app.get_webview_window("strip").ok_or("strip window missing")?;
    let mons = strip.available_monitors().map_err(|e| e.to_string())?;
    Ok(mons
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let s = m.size();
            MonitorDto {
                index: i,
                name: m.name().cloned().unwrap_or_else(|| format!("Monitor {}", i + 1)),
                width: s.width,
                height: s.height,
            }
        })
        .collect())
}

/// Persists edge + monitor, re-docks the strip and notifies the frontend (orientation).
#[tauri::command]
pub fn set_dock(
    app: AppHandle,
    state: State<AppState>,
    monitor_index: usize,
    edge: String,
) -> Result<AppConfig, String> {
    let edge = Edge::from_str(&edge).ok_or("invalid edge")?;
    let mut cfg = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::load_config(&conn)?
    };
    cfg.edge = edge;
    cfg.monitor_index = monitor_index;
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::save_config(&conn, &cfg)?;
    }

    #[cfg(windows)]
    {
        if let Some(strip) = app.get_webview_window("strip") {
            if let Err(e) = crate::platform::appbar::redock(
                &strip,
                cfg.edge,
                cfg.monitor_index,
                cfg.strip_height_logical,
            ) {
                eprintln!("[dock] redock failed: {e}");
            }
        }
    }

    let _ = app.emit("dock://changed", &cfg);
    Ok(cfg)
}

/// Opens the small settings window near the clock (far end of the strip).
#[tauri::command]
pub fn open_settings(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let cfg = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::load_config(&conn)?
    };
    let strip = app.get_webview_window("strip").ok_or("strip window missing")?;
    let settings = app
        .get_webview_window("settings")
        .ok_or("settings window missing")?;

    let scale = strip.scale_factor().unwrap_or(1.0);
    let w = (240.0 * scale) as i32;
    let h = (230.0 * scale) as i32;
    let thickness = (cfg.strip_height_logical as f64 * scale) as i32;

    // The clock sits at the far end of the strip (None = far end). place_popup opens
    // toward the interior and clamps to the monitor.
    let pos = place_popup(&strip, cfg.edge, cfg.monitor_index, thickness, None, w, h)?;

    settings.set_position(pos).map_err(|e| e.to_string())?;
    let _ = app.emit("settings://open", &cfg);
    settings.show().map_err(|e| e.to_string())?;
    settings.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
