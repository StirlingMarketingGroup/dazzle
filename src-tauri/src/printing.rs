use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Printer {
    pub name: String,
    pub is_default: bool,
}

/// Discover printers using `lpstat` (macOS/Linux).
pub fn discover() -> Result<Vec<Printer>, String> {
    let output = Command::new("lpstat")
        .arg("-p")
        .output()
        .map_err(|e| format!("Failed to run lpstat: {e}"))?;

    // No printers configured returns non-zero — that's fine, just empty list
    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let default_printer = Command::new("lpstat")
        .arg("-d")
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            s.split(": ").nth(1).map(|s| s.trim().to_string())
        });

    let printers = stdout
        .lines()
        .filter_map(|line| {
            // lpstat -p output: "printer NAME is idle." or "printer NAME disabled ..."
            let name = line
                .strip_prefix("printer ")?
                .split_whitespace()
                .next()?
                .to_string();
            let is_default = default_printer.as_deref() == Some(&name);
            Some(Printer { name, is_default })
        })
        .collect();

    Ok(printers)
}

/// Send raw bytes to a printer using `lp -o raw` (macOS/Linux).
pub fn send_raw(printer: &str, data: &[u8]) -> Result<(), String> {
    use std::io::Write;

    let mut child = Command::new("lp")
        .args(["-d", printer, "-o", "raw"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn lp: {e}"))?;

    child
        .stdin
        .take()
        .unwrap()
        .write_all(data)
        .map_err(|e| format!("Failed to write to lp: {e}"))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for lp: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "lp failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
