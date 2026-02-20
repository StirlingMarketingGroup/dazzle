use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Printer {
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub selected_printer: Option<String>,
    pub auto_start: bool,
}

#[tauri::command]
pub fn list_printers() -> Result<Vec<Printer>, String> {
    // TODO: Platform-specific printer discovery
    Ok(vec![])
}

#[tauri::command]
pub fn print_zpl(printer: String, zpl: String) -> Result<(), String> {
    // TODO: Platform-specific raw printing
    log::info!("Printing to {}: {} bytes of ZPL", printer, zpl.len());
    Ok(())
}

#[tauri::command]
pub fn get_config() -> Result<AppConfig, String> {
    Ok(AppConfig {
        port: 9100,
        selected_printer: None,
        auto_start: false,
    })
}

#[tauri::command]
pub fn set_config(_config: AppConfig) -> Result<(), String> {
    // TODO: Persist config
    Ok(())
}
