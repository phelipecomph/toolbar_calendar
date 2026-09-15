use crate::model::{mock_events, Account, AccountKind, AppConfig, NormalizedEvent};
use crate::{db, oauth, state::AppState, sync};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_events(
    state: State<AppState>,
    from: String,
    to: String,
) -> Result<Vec<NormalizedEvent>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    // Sem contas ainda → mostra mocks (DX na Fase 1/2).
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

/// Roda o fluxo OAuth do Google (abre navegador), grava conta + refresh token no keyring
/// e dispara um sync. Reusável pela UI (Fase 4) e pelo auto-seed no startup.
pub async fn add_google_account_flow(
    app: AppHandle,
    client_id: String,
    client_secret: String,
) -> Result<Account, String> {
    // authorize() é bloqueante (tiny_http + reqwest::blocking) — roda em thread própria,
    // fora do runtime tokio; aguardamos por polling sem travar o executor.
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
                return Err("fluxo OAuth interrompido".into());
            }
        }
    };

    let refresh = tokens
        .refresh_token
        .ok_or("Google não devolveu refresh_token (revogue o acesso e refaça)")?;

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

    // Token PRIMEIRO: garante que qualquer sync que veja a conta já ache o refresh no keyring.
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
    #[allow(dead_code)]
    pub y: f64,
    #[allow(dead_code)]
    pub width: f64,
    #[allow(dead_code)]
    pub height: f64,
}

/// Abre a janela `detail` posicionada acima do bloco clicado e envia o evento.
#[tauri::command]
pub fn open_detail(
    app: AppHandle,
    state: State<AppState>,
    event_id: String,
    anchor: AnchorRect,
) -> Result<(), String> {
    let event = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_event(&conn, &event_id)?
    };
    let Some(event) = event else {
        return Err("evento não encontrado".into());
    };

    let strip = app.get_webview_window("strip").ok_or("janela strip ausente")?;
    let detail = app.get_webview_window("detail").ok_or("janela detail ausente")?;

    let scale = strip.scale_factor().unwrap_or(1.0);
    let strip_pos = strip.outer_position().map_err(|e| e.to_string())?;
    let detail_h_logical = 200.0_f64;
    let x = strip_pos.x + (anchor.x * scale) as i32;
    let y = strip_pos.y - (detail_h_logical * scale) as i32;

    detail
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
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
