mod commands;
mod config;
mod printing;
mod server;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tauri::{Emitter, Listener, Manager};

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

const BADGE_RADIUS_FRAC: f64 = 0.20;
const BADGE_RADIUS_MIN: f64 = 3.0;
const BORDER_WIDTH_FRAC: f64 = 0.04;
const BORDER_WIDTH_MIN: f64 = 1.0;
const BADGE_MARGIN_FRAC: f64 = 0.05;
const COLOR_RUNNING: (u8, u8, u8) = (76, 175, 80);
const COLOR_STOPPED: (u8, u8, u8) = (244, 67, 54);
const COLOR_BORDER: (u8, u8, u8) = (255, 255, 255);

/// Create a copy of the base icon with a colored status badge in the bottom-right corner.
fn create_status_icon(
    base: &tauri::image::Image<'_>,
    running: bool,
) -> tauri::image::Image<'static> {
    let width = base.width();
    let height = base.height();
    let mut rgba = base.rgba().to_vec();

    let size = width.min(height) as f64;
    let badge_r = (size * BADGE_RADIUS_FRAC).max(BADGE_RADIUS_MIN);
    let border_w = (size * BORDER_WIDTH_FRAC).max(BORDER_WIDTH_MIN);
    let total_r = badge_r + border_w;
    let margin = size * BADGE_MARGIN_FRAC;
    let cx = width as f64 - total_r - margin;
    let cy = height as f64 - total_r - margin;

    let fill = if running { COLOR_RUNNING } else { COLOR_STOPPED };

    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 + 0.5 - cx;
            let dy = y as f64 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= total_r + 0.5 {
                let idx = ((y * width + x) * 4) as usize;
                let aa = (total_r + 0.5 - dist).clamp(0.0, 1.0);

                let (r, g, b) = if dist <= badge_r { fill } else { COLOR_BORDER };

                // Alpha-composite badge over existing pixel
                let src_a = aa;
                let dst_a = rgba[idx + 3] as f64 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);

                if out_a > 0.0 {
                    let blend = |src_c: u8, dst_c: u8| {
                        ((src_c as f64 * src_a + dst_c as f64 * dst_a * (1.0 - src_a)) / out_a)
                            as u8
                    };
                    rgba[idx] = blend(r, rgba[idx]);
                    rgba[idx + 1] = blend(g, rgba[idx + 1]);
                    rgba[idx + 2] = blend(b, rgba[idx + 2]);
                    rgba[idx + 3] = (out_a * 255.0) as u8;
                }
            }
        }
    }

    tauri::image::Image::new_owned(rgba, width, height)
}

fn update_tray_status(app: &tauri::AppHandle, running: bool) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Some(base_icon) = app.default_window_icon() {
            let icon = create_status_icon(base_icon, running);
            let _ = tray.set_icon(Some(icon));
        }
        let tooltip = if running {
            "Dazzle — Server running"
        } else {
            "Dazzle — Server stopped"
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    // Start with a red badge — switches to green once the server binds
    let icon = app
        .default_window_icon()
        .map(|base| create_status_icon(base, false))
        .unwrap();

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("Dazzle — Starting…")
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
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second instance was launched — just show the existing window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
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

            // On Windows, always start hidden (runs as a background service).
            // On macOS/Linux, hide only when launched with --hidden (autostart via system mechanism).
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

            setup_tray(app)?;

            // Update tray icon when server status changes
            let handle = app.handle().clone();
            app.listen("server-status", move |event| {
                if let Ok(running) = serde_json::from_str::<bool>(event.payload()) {
                    update_tray_status(&handle, running);
                }
            });

            // Start the HTTP print server (after listener is registered to avoid race)
            tauri::async_runtime::spawn(async move {
                if let Err(e) = restart_server(&state).await {
                    log::error!("Failed to start server: {e}");
                    state.app_handle.emit("server-status", false).ok();
                    state.app_handle.emit("server-error", &e).ok();
                }
            });

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
