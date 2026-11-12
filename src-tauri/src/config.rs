use crate::printing;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub selected_printer: Option<String>,
}

/// The file on disk. `version` stays here so the window's `AppConfig` does not carry it.
#[derive(Serialize, Deserialize)]
struct ConfigFile {
    /// 0 when the file has none. See `upgrade`.
    #[serde(default)]
    version: u32,
    #[serde(flatten)]
    config: AppConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: 29100,
            selected_printer: None,
        }
    }
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("dazzle");
    if let Err(e) = std::fs::create_dir_all(&path) {
        log::warn!("Failed to create config directory: {e}");
    }
    path.push("config.json");
    path
}

pub fn load() -> AppConfig {
    let path = config_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return AppConfig::default(),
        Err(e) => {
            log::warn!("Failed to read config at {}: {e}", path.display());
            return AppConfig::default();
        }
    };

    let file: ConfigFile = match serde_json::from_str(&text) {
        Ok(file) => file,
        Err(e) => {
            log::warn!("Failed to parse config at {}: {e}", path.display());
            return AppConfig::default();
        }
    };

    let config = file.config;
    if file.version != 0 || config.selected_printer.is_none() {
        return config;
    }

    let printers = match printing::discover() {
        Ok(printers) => printers,
        Err(e) => {
            log::warn!(
                "Failed to list printers while reading config at {}: {e}",
                path.display()
            );
            return config;
        }
    };
    let Some(upgraded) = upgrade(&config, &printers) else {
        return config;
    };
    if let Err(e) = save(&upgraded) {
        log::warn!("Failed to write config at {}: {e}", path.display());
    }
    upgraded
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(&ConfigFile {
        version: VERSION,
        config: config.clone(),
    })
    .map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// The version 1 config for a version 0 file.
///
/// A version 0 file may hold the system default printer, saved without the user
/// choosing it. So a saved printer that is installed and does not look like a
/// ZPL printer is forgotten. That needs a ZPL-looking printer to take its place:
/// while none is installed this returns `None`, and the file stays at version 0.
fn upgrade(config: &AppConfig, printers: &[printing::Printer]) -> Option<AppConfig> {
    if !printers.iter().any(printing::looks_like_zpl) {
        return None;
    }

    let selected_printer = config.selected_printer.clone().filter(|saved| {
        !printers
            .iter()
            .any(|printer| printer.name == *saved && !printing::looks_like_zpl(printer))
    });
    Some(AppConfig {
        selected_printer,
        ..config.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let config = AppConfig::default();
        assert_eq!(config.port, 29100);
        assert!(config.selected_printer.is_none());
    }

    #[test]
    fn config_roundtrips_through_json() {
        let config = AppConfig {
            port: 8080,
            selected_printer: Some("Zebra ZD420".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.port, 8080);
        assert_eq!(
            deserialized.selected_printer.as_deref(),
            Some("Zebra ZD420")
        );
    }

    #[test]
    fn config_deserializes_with_null_printer() {
        let json = r#"{"port":29100,"selected_printer":null}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.port, 29100);
        assert!(config.selected_printer.is_none());
    }

    #[test]
    fn save_and_load_via_filesystem() {
        let dir = std::env::temp_dir().join(format!("dazzle-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        let config = AppConfig {
            port: 3000,
            selected_printer: Some("Test Printer".to_string()),
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(&path, &json).unwrap();

        let loaded: AppConfig =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(loaded.port, 3000);
        assert_eq!(loaded.selected_printer.as_deref(), Some("Test Printer"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_json_does_not_crash() {
        let result = serde_json::from_str::<AppConfig>("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn partial_json_with_missing_fields_fails() {
        // Missing required field (port) should fail deserialization
        let result = serde_json::from_str::<AppConfig>(r#"{"selected_printer":null}"#);
        assert!(result.is_err());
    }

    #[test]
    fn config_serializes_to_expected_json_structure() {
        let config = AppConfig {
            port: 29100,
            selected_printer: None,
        };

        let json: serde_json::Value = serde_json::to_value(&config).unwrap();
        assert_eq!(json["port"], 29100);
        assert!(json["selected_printer"].is_null());
    }

    #[test]
    fn port_boundary_values() {
        // Port 1 (minimum valid)
        let config = AppConfig {
            port: 1,
            selected_printer: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.port, 1);

        // Port 65535 (maximum valid)
        let config = AppConfig {
            port: 65535,
            selected_printer: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.port, 65535);
    }

    #[test]
    fn config_ignores_unknown_fields() {
        let json = r#"{"port":29100,"selected_printer":"Zebra ZD420","auto_start":true}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.port, 29100);
        assert_eq!(config.selected_printer.as_deref(), Some("Zebra ZD420"));
    }

    #[test]
    fn config_with_unicode_printer_name() {
        let config = AppConfig {
            port: 29100,
            selected_printer: Some("Druckerei-Schreibmaschine".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            loaded.selected_printer.as_deref(),
            Some("Druckerei-Schreibmaschine")
        );
    }

    fn printer(name: &str) -> printing::Printer {
        printing::Printer {
            name: name.to_string(),
            is_default: false,
            model: String::new(),
            offline: false,
        }
    }

    fn saved(name: &str) -> AppConfig {
        AppConfig {
            port: 29100,
            selected_printer: Some(name.to_string()),
        }
    }

    #[test]
    fn upgrade_keeps_a_zpl_looking_saved_printer() {
        let printers = [
            printer("Zebra ZP 450"),
            printer("Brother HL-L2350DW series Printer"),
        ];
        let upgraded = upgrade(&saved("Zebra ZP 450"), &printers).unwrap();
        assert_eq!(upgraded.selected_printer.as_deref(), Some("Zebra ZP 450"));
    }

    #[test]
    fn upgrade_forgets_a_non_zpl_saved_printer_when_a_zpl_printer_is_installed() {
        let printers = [
            printer("Brother HL-L2350DW series Printer"),
            printer("Zebra ZP 450"),
        ];
        let upgraded = upgrade(&saved("Brother HL-L2350DW series Printer"), &printers).unwrap();
        assert_eq!(upgraded.selected_printer, None);
        assert_eq!(upgraded.port, 29100);
    }

    #[test]
    fn upgrade_keeps_a_saved_printer_that_is_not_installed() {
        let printers = [printer("Zebra ZP 450")];
        let upgraded = upgrade(&saved("Office Laser"), &printers).unwrap();
        assert_eq!(upgraded.selected_printer.as_deref(), Some("Office Laser"));
    }

    #[test]
    fn upgrade_waits_while_no_zpl_printer_is_installed() {
        let printers = [
            printer("Brother HL-L2350DW series Printer"),
            printer("Microsoft Print to PDF"),
        ];
        assert!(upgrade(&saved("Brother HL-L2350DW series Printer"), &printers).is_none());
    }

    #[test]
    fn version_0_json_parses_with_version_0() {
        let json = r#"{"port":29100,"selected_printer":"Office Laser"}"#;
        let file: ConfigFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.version, 0);
        assert_eq!(file.config.port, 29100);
        assert_eq!(
            file.config.selected_printer.as_deref(),
            Some("Office Laser")
        );
    }

    #[test]
    fn saved_file_roundtrips_with_version_1() {
        let config = AppConfig {
            port: 3000,
            selected_printer: Some("Test Printer".to_string()),
        };
        let json = serde_json::to_string_pretty(&ConfigFile {
            version: VERSION,
            config: config.clone(),
        })
        .unwrap();

        let loaded: ConfigFile = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.config.port, config.port);
        assert_eq!(loaded.config.selected_printer, config.selected_printer);
    }
}
