use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Printer {
    pub name: String,
    pub is_default: bool,
}

// ─── macOS / Linux ──────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::Printer;
    use std::process::Command;

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
}

// ─── Windows ────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use super::Printer;
    use std::ffi::c_void;
    use windows::core::{HSTRING, PCWSTR, PWSTR};
    use windows::Win32::Graphics::Printing::{
        ClosePrinter, EndDocPrinter, EndPagePrinter, EnumPrintersW, GetDefaultPrinterW,
        OpenPrinterW, StartDocPrinterW, StartPagePrinter, WritePrinter, DOC_INFO_1W,
        PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_HANDLE, PRINTER_INFO_2W,
    };

    /// Discover printers via the Win32 `EnumPrintersW` API.
    pub fn discover() -> Result<Vec<Printer>, String> {
        let default_name = get_default_printer_name();

        // First call: determine required buffer size.
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let mut bytes_needed: u32 = 0;
        let mut count: u32 = 0;

        unsafe {
            let _ = EnumPrintersW(
                flags,
                None,
                2, // level 2 → PRINTER_INFO_2W
                None,
                &mut bytes_needed,
                &mut count,
            );
        }

        if bytes_needed == 0 {
            return Ok(vec![]);
        }

        // Second call: fill the buffer.
        let mut buf: Vec<u8> = vec![0u8; bytes_needed as usize];
        unsafe {
            EnumPrintersW(
                flags,
                None,
                2,
                Some(&mut buf),
                &mut bytes_needed,
                &mut count,
            )
            .map_err(|e| format!("EnumPrintersW failed: {e}"))?;
        }

        let infos = unsafe {
            std::slice::from_raw_parts(buf.as_ptr() as *const PRINTER_INFO_2W, count as usize)
        };

        let printers = infos
            .iter()
            .filter_map(|info| {
                let name = unsafe { info.pPrinterName.to_string().ok()? };
                let is_default = default_name.as_deref() == Some(name.as_str());
                Some(Printer { name, is_default })
            })
            .collect();

        Ok(printers)
    }

    /// Send raw bytes to a named printer via the Win32 spooler API.
    pub fn send_raw(printer: &str, data: &[u8]) -> Result<(), String> {
        let printer_hstring = HSTRING::from(printer);
        let mut handle = PRINTER_HANDLE::default();

        // Open the printer.
        unsafe {
            OpenPrinterW(PCWSTR(printer_hstring.as_ptr()), &mut handle, None)
                .map_err(|e| format!("OpenPrinterW failed: {e}"))?;
        }

        // Guard to ensure ClosePrinter runs even on early errors.
        let result = (|| -> Result<(), String> {
            // DOC_INFO_1W fields are PWSTR. We cast from HSTRING's const pointer;
            // the spooler only reads through these pointers so this is safe.
            let doc_name = HSTRING::from("Dazzle Raw Print");
            let datatype = HSTRING::from("RAW");

            let doc_info = DOC_INFO_1W {
                pDocName: PWSTR(doc_name.as_ptr() as *mut u16),
                pOutputFile: PWSTR::null(),
                pDatatype: PWSTR(datatype.as_ptr() as *mut u16),
            };

            // StartDocPrinterW returns a job ID (non-zero on success).
            let job_id = unsafe { StartDocPrinterW(handle, 1, &doc_info) };
            if job_id == 0 {
                return Err("StartDocPrinterW failed (returned 0)".into());
            }

            unsafe {
                StartPagePrinter(handle)
                    .ok()
                    .map_err(|e| format!("StartPagePrinter failed: {e}"))?;
            }

            // Write the data in full.
            let mut bytes_written: u32 = 0;
            unsafe {
                WritePrinter(
                    handle,
                    data.as_ptr() as *const c_void,
                    data.len() as u32,
                    &mut bytes_written,
                )
                .ok()
                .map_err(|e| format!("WritePrinter failed: {e}"))?;
            }

            if (bytes_written as usize) != data.len() {
                return Err(format!(
                    "WritePrinter: only wrote {bytes_written} of {} bytes",
                    data.len()
                ));
            }

            unsafe {
                EndPagePrinter(handle)
                    .ok()
                    .map_err(|e| format!("EndPagePrinter failed: {e}"))?;
                EndDocPrinter(handle)
                    .ok()
                    .map_err(|e| format!("EndDocPrinter failed: {e}"))?;
            }

            Ok(())
        })();

        unsafe {
            let _ = ClosePrinter(handle);
        }

        result
    }

    /// Retrieve the name of the default printer, if any.
    fn get_default_printer_name() -> Option<String> {
        // First call: get required buffer length (in chars, including null).
        let mut len: u32 = 0;
        unsafe {
            let _ = GetDefaultPrinterW(None, &mut len);
        }

        if len == 0 {
            return None;
        }

        let mut buf: Vec<u16> = vec![0u16; len as usize];
        let ok = unsafe { GetDefaultPrinterW(Some(PWSTR(buf.as_mut_ptr())), &mut len) };

        if !ok.as_bool() {
            return None;
        }

        // Trim the trailing null.
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16(&buf[..end]).ok()
    }
}

// ─── Public re-exports ──────────────────────────────────────────────────────

pub fn discover() -> Result<Vec<Printer>, String> {
    platform::discover()
}

pub fn send_raw(printer: &str, data: &[u8]) -> Result<(), String> {
    platform::send_raw(printer, data)
}
