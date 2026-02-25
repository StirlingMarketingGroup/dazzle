use crate::{config, printing, server, AppState};
use std::sync::Arc;

#[tauri::command]
pub fn list_printers() -> Result<Vec<printing::Printer>, String> {
    printing::discover()
}

#[tauri::command]
pub fn print_zpl(printer: String, zpl: String) -> Result<(), String> {
    printing::send_raw(&printer, zpl.as_bytes())?;
    log::info!("Printed {} bytes to {printer}", zpl.len());
    Ok(())
}

#[tauri::command]
pub fn get_config(state: tauri::State<'_, Arc<AppState>>) -> Result<config::AppConfig, String> {
    state
        .config
        .read()
        .map(|c| c.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_config(
    new_config: config::AppConfig,
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let (port_changed, autostart_changed) = {
        let current = state.config.read().map_err(|e| e.to_string())?;
        (
            current.port != new_config.port,
            current.auto_start != new_config.auto_start,
        )
    };

    config::save(&new_config)?;

    if autostart_changed {
        use tauri_plugin_autostart::ManagerExt;
        let autostart = app.autolaunch();
        if new_config.auto_start {
            autostart.enable().map_err(|e| e.to_string())?;
        } else {
            autostart.disable().map_err(|e| e.to_string())?;
        }
    }

    {
        let mut config = state.config.write().map_err(|e| e.to_string())?;
        *config = new_config;
    }

    if port_changed {
        crate::restart_server(state.inner()).await?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_server_running(state: tauri::State<'_, Arc<AppState>>) -> Result<bool, String> {
    let handle = state.server_handle.lock().await;
    Ok(handle.as_ref().is_some_and(|h| !h.is_finished()))
}

#[tauri::command]
pub async fn restart_server(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::restart_server(state.inner()).await
}

#[tauri::command]
pub fn get_print_jobs(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<server::PrintJob>, String> {
    state
        .print_jobs
        .read()
        .map(|jobs| jobs.clone())
        .map_err(|e| e.to_string())
}
