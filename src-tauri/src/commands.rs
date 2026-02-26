use crate::{autostart, config, printing, server, AppState};
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
) -> Result<(), String> {
    let port_changed = {
        let current = state.config.read().map_err(|e| e.to_string())?;
        current.port != new_config.port
    };

    config::save(&new_config)?;

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
pub fn get_autostart() -> Result<bool, String> {
    autostart::is_enabled()
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        autostart::enable()
    } else {
        autostart::disable()
    }
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
