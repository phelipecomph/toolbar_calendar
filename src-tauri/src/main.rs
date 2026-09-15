// Evita janela de console extra no Windows em release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod model;
mod oauth;
mod platform;
mod providers;
mod state;
mod sync;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_events,
            commands::get_config,
            commands::set_config,
            commands::list_accounts,
            commands::add_ics_account,
            commands::remove_account,
            commands::set_account_color,
            commands::trigger_sync,
            commands::add_google_account,
            commands::open_in_browser,
            commands::open_detail,
            commands::close_detail,
        ])
        .setup(|app| {
            // Banco em app_data_dir.
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_path = dir.join("agenda.db");
            let conn = db::open(&db_path)?;

            match std::env::var("AGENDA_ICS_URL") {
                Ok(u) => eprintln!("[startup] AGENDA_ICS_URL={u}"),
                Err(_) => eprintln!("[startup] AGENDA_ICS_URL nao definida"),
            }

            // Seed opcional p/ teste: variável de ambiente AGENDA_ICS_URL.
            if db::count_accounts(&conn).unwrap_or(0) == 0 {
                if let Ok(url) = std::env::var("AGENDA_ICS_URL") {
                    if !url.is_empty() {
                        let acc = model::Account {
                            id: uuid::Uuid::new_v4().to_string(),
                            kind: model::AccountKind::Ics,
                            display_name: "ICS".into(),
                            color: "#4285f4".into(),
                            config: serde_json::json!({ "url": url }),
                            sync_token: None,
                            enabled: true,
                        };
                        let _ = db::add_account(&conn, &acc);
                    }
                }
            }

            let n_acc = db::count_accounts(&conn).unwrap_or(-1);
            eprintln!("[startup] db={} contas={n_acc}", db_path.display());

            app.manage(AppState {
                db: Mutex::new(conn),
            });

            let strip = app
                .get_webview_window("strip")
                .expect("strip window missing");

            #[cfg(windows)]
            {
                if let Err(e) = platform::appbar::register(&strip, 24) {
                    eprintln!("appbar register failed: {e}");
                }
                let on_exit = strip.clone();
                strip.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Destroyed) {
                        let _ = platform::appbar::unregister(&on_exit);
                    }
                });
            }
            #[cfg(not(windows))]
            {
                let _ = &strip;
            }

            // Auto-reparo no startup:
            //  - conta Google sem token no keyring → remove (força re-login);
            //  - conta ICS apontando p/ arquivo local (sample de teste) → remove.
            let broken: Vec<String> = {
                let st = app.state::<AppState>();
                let guard = st.db.lock();
                match guard {
                    Ok(conn) => db::list_accounts(&conn)
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|a| match a.kind {
                            model::AccountKind::Google => {
                                oauth::tokens::load_refresh(&a.id).is_err()
                            }
                            model::AccountKind::Ics => a
                                .config
                                .get("url")
                                .and_then(|v| v.as_str())
                                .map(|u| !u.starts_with("http"))
                                .unwrap_or(true),
                            _ => false,
                        })
                        .map(|a| a.id)
                        .collect(),
                    Err(_) => Vec::new(),
                }
            };
            for id in broken {
                eprintln!("[startup] removendo conta inválida/teste {id}");
                let st = app.state::<AppState>();
                let guard = st.db.lock();
                if let Ok(conn) = guard {
                    let _ = db::remove_account(&conn, &id);
                }
            }

            // Auto-seed Google (uma vez): se há credenciais no ambiente e nenhuma conta Google,
            // dispara o OAuth interativo. Nas próximas execuções usa o keyring.
            let need_google = {
                let st = app.state::<AppState>();
                let conn = st.db.lock().map_err(|e| e.to_string())?;
                db::list_accounts(&conn)
                    .map(|v| v.iter().all(|a| a.kind != model::AccountKind::Google))
                    .unwrap_or(false)
            };
            if need_google {
                if let (Ok(cid), Ok(csec)) = (
                    std::env::var("AGENDA_GOOGLE_CLIENT_ID"),
                    std::env::var("AGENDA_GOOGLE_CLIENT_SECRET"),
                ) {
                    if !cid.is_empty() && !csec.is_empty() {
                        let h = app.handle().clone();
                        tauri::async_runtime::spawn(async move {
                            eprintln!("[google] iniciando OAuth (abrindo navegador)...");
                            match commands::add_google_account_flow(h, cid, csec).await {
                                Ok(a) => eprintln!("[google] conta adicionada: {}", a.id),
                                Err(e) => eprintln!("[google] OAuth falhou: {e}"),
                            }
                        });
                    }
                }
            }

            // Autostart no login do Windows (idempotente).
            let _ = app.autolaunch().enable();

            // Sync inicial + timer.
            sync::start_timer(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
