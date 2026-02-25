mod commands;
mod config;
mod printing;
mod server;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tauri::{Emitter, Manager};

pub struct AppState {
    pub config: RwLock<config::AppConfig>,
    pub print_jobs: RwLock<Vec<server::PrintJob>>,
    pub server_handle: tokio::sync::Mutex<Option<server::ServerHandle>>,
    pub app_handle: tauri::AppHandle,
}

pub async fn restart_server(state: &Arc<AppState>) -> Result<(), String> {
    let mut handle_guard = state.server_handle.lock().await;

    if let Some(h) = handle_guard.take() {
        h.shutdown().await;
    }

    let handle = server::start(state.clone()).await?;
    *handle_guard = Some(handle);
    Ok(())
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Dazzle — ZPL Print Server")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => toggle_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--hidden"])
                .build(),
        )
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::list_printers,
            commands::print_zpl,
            commands::get_config,
            commands::set_config,
            commands::get_print_jobs,
            commands::get_server_running,
            commands::restart_server,
        ])
        .setup(|app| {
            let cfg = config::load();

            // Sync autostart with config
            use tauri_plugin_autostart::ManagerExt;
            let autostart = app.autolaunch();
            if cfg.auto_start {
                autostart.enable().ok();
            } else {
                autostart.disable().ok();
            }

            // On Windows, always start hidden (runs as a background service).
            // On macOS/Linux, hide only when launched with --hidden (autostart).
            let hidden = if cfg!(target_os = "windows") {
                true
            } else {
                std::env::args().any(|a| a == "--hidden")
            };
            if hidden {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            let state = Arc::new(AppState {
                config: RwLock::new(cfg),
                print_jobs: RwLock::new(Vec::new()),
                server_handle: tokio::sync::Mutex::new(None),
                app_handle: app.handle().clone(),
            });

            app.manage(state.clone());

            // Start the HTTP print server
            tauri::async_runtime::spawn(async move {
                if let Err(e) = restart_server(&state).await {
                    log::error!("Failed to start server: {e}");
                    state.app_handle.emit("server-error", &e).ok();
                }
            });

            setup_tray(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                static NOTIFIED: AtomicBool = AtomicBool::new(false);

                api.prevent_close();
                let _ = window.hide();

                // Show a one-time notification so the user knows we're still running
                if !NOTIFIED.swap(true, Ordering::Relaxed) {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = window
                        .app_handle()
                        .notification()
                        .builder()
                        .title("Dazzle is still running")
                        .body("The print server is running in the background. Right-click the tray icon to quit.")
                        .show();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
