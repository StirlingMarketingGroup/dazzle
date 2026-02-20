mod commands;
mod config;
mod printing;
mod server;

use std::sync::{Arc, RwLock};
use tauri::Manager;

pub struct AppState {
    pub config: RwLock<config::AppConfig>,
    pub print_jobs: RwLock<Vec<server::PrintJob>>,
    pub server_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub app_handle: tauri::AppHandle,
}

pub async fn restart_server(state: &Arc<AppState>) -> Result<(), String> {
    let mut handle_guard = state.server_handle.lock().await;

    if let Some(h) = handle_guard.take() {
        h.abort();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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
            window.hide().unwrap();
        } else {
            window.show().unwrap();
            window.set_focus().unwrap();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        ])
        .setup(|app| {
            let cfg = config::load();
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
                }
            });

            setup_tray(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Close to tray instead of quitting
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().unwrap();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
