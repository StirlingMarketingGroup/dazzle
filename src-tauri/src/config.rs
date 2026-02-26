use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub selected_printer: Option<String>,
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
    match std::fs::read_to_string(&path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(config) => config,
            Err(e) => {
                log::warn!("Failed to parse config at {}: {e}", path.display());
                AppConfig::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => AppConfig::default(),
        Err(e) => {
            log::warn!("Failed to read config at {}: {e}", path.display());
            AppConfig::default()
        }
    }
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
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
    fn old_config_with_auto_start_field_still_loads() {
        // Existing config files from before the auto_start removal should still deserialize
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
}
