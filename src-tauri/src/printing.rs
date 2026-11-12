use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Printer {
    pub name: String,
    pub is_default: bool,
    /// The Windows driver name. Empty on other platforms.
    #[serde(skip)]
    pub model: String,
    /// Windows shows the printer as offline, as it does for an unplugged USB printer.
    /// Always false on other platforms.
    #[serde(skip)]
    pub offline: bool,
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
                Some(Printer {
                    name,
                    is_default,
                    model: String::new(),
                    offline: false,
                })
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
        PRINTER_ATTRIBUTE_WORK_OFFLINE, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
        PRINTER_HANDLE, PRINTER_INFO_2W, PRINTER_STATUS_OFFLINE,
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
                // A missing driver name is not a reason to hide the printer.
                let model = unsafe {
                    if info.pDriverName.is_null() {
                        String::new()
                    } else {
                        info.pDriverName.to_string().unwrap_or_default()
                    }
                };
                let is_default = default_name.as_deref() == Some(name.as_str());
                let offline = info.Attributes & PRINTER_ATTRIBUTE_WORK_OFFLINE != 0
                    || info.Status & PRINTER_STATUS_OFFLINE != 0;
                Some(Printer {
                    name,
                    is_default,
                    model,
                    offline,
                })
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

/// Windows cannot say which printers speak ZPL, so go by the name and the driver:
/// Zebra's drivers and Bixolon's label printers.
const ZPL_MARKS: [&str; 4] = ["zpl", "zebra", "zdesigner", "bixolon"];

pub fn looks_like_zpl(printer: &Printer) -> bool {
    let name = printer.name.to_lowercase();
    let model = printer.model.to_lowercase();
    ZPL_MARKS
        .iter()
        .any(|mark| name.contains(mark) || model.contains(mark))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrinterSource {
    /// An administrator set it for this PC.
    Policy,
    /// This user chose it in the window.
    User,
    /// Dazzle picked it.
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PrinterChoice {
    pub name: String,
    pub source: PrinterSource,
}

/// The printer a job goes to when the request names none.
pub fn choice(selected: Option<&str>) -> Result<Option<PrinterChoice>, String> {
    if let Some(name) = crate::policy::printer() {
        return Ok(Some(PrinterChoice {
            name,
            source: PrinterSource::Policy,
        }));
    }

    Ok(choose_from(selected, &discover()?))
}

fn choose_from(selected: Option<&str>, printers: &[Printer]) -> Option<PrinterChoice> {
    if let Some(name) = selected {
        if printers.iter().any(|printer| printer.name == name) {
            return Some(PrinterChoice {
                name: name.to_string(),
                source: PrinterSource::User,
            });
        }
    }

    let printer = printers
        .iter()
        .filter(|printer| looks_like_zpl(printer))
        // A USB printer moved to another port leaves its old queue behind, offline.
        .min_by_key(|printer| (printer.offline, printer.name.to_lowercase()))
        .or_else(|| printers.iter().find(|printer| printer.is_default))
        .or_else(|| printers.first())?;
    Some(PrinterChoice {
        name: printer.name.clone(),
        source: PrinterSource::Automatic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printer(name: &str, is_default: bool, model: &str) -> Printer {
        Printer {
            name: name.to_string(),
            is_default,
            model: model.to_string(),
            offline: false,
        }
    }

    #[test]
    fn looks_like_zpl_matches_a_zebra_name() {
        assert!(looks_like_zpl(&printer("Zebra ZP 450", false, "")));
    }

    #[test]
    fn looks_like_zpl_matches_a_zdesigner_driver() {
        assert!(looks_like_zpl(&printer(
            "Shipping",
            false,
            "ZDesigner ZD420-203dpi ZPL"
        )));
    }

    #[test]
    fn looks_like_zpl_matches_a_bixolon_name() {
        assert!(looks_like_zpl(&printer(
            "BIXOLON SRP-770III - BPL-Z",
            false,
            ""
        )));
        assert!(looks_like_zpl(&printer("Bixolon SRP-770III", false, "")));
    }

    #[test]
    fn looks_like_zpl_rejects_an_office_printer() {
        assert!(!looks_like_zpl(&printer(
            "Brother HL-L2350DW series Printer",
            false,
            ""
        )));
        assert!(!looks_like_zpl(&printer(
            "Microsoft Print to PDF",
            false,
            ""
        )));
    }

    #[test]
    fn choose_from_uses_the_installed_printer_the_user_chose() {
        let printers = [
            printer("HP LaserJet", true, ""),
            printer("Zebra ZP 450", false, ""),
        ];

        assert_eq!(
            choose_from(Some("HP LaserJet"), &printers),
            Some(PrinterChoice {
                name: "HP LaserJet".to_string(),
                source: PrinterSource::User,
            })
        );
    }

    #[test]
    fn choose_from_falls_to_automatic_when_the_choice_is_not_installed() {
        let printers = [
            printer("HP LaserJet", true, ""),
            printer("Zebra ZP 450", false, ""),
        ];

        assert_eq!(
            choose_from(Some("Warehouse Zebra"), &printers),
            Some(PrinterChoice {
                name: "Zebra ZP 450".to_string(),
                source: PrinterSource::Automatic,
            })
        );
    }

    #[test]
    fn choose_from_prefers_a_zpl_printer_over_the_system_default() {
        let printers = [
            printer("HP LaserJet", true, ""),
            printer("Zebra ZP 450", false, ""),
        ];

        assert_eq!(
            choose_from(None, &printers),
            Some(PrinterChoice {
                name: "Zebra ZP 450".to_string(),
                source: PrinterSource::Automatic,
            })
        );
    }

    #[test]
    fn choose_from_orders_zpl_printers_by_name_ignoring_case() {
        let printers = [
            printer("Zebra ZP 450", false, ""),
            printer("alpha ZPL", true, ""),
        ];

        assert_eq!(
            choose_from(None, &printers),
            Some(PrinterChoice {
                name: "alpha ZPL".to_string(),
                source: PrinterSource::Automatic,
            })
        );
    }

    #[test]
    fn choose_from_takes_an_online_zpl_printer_before_an_offline_one() {
        let printers = [
            Printer {
                offline: true,
                ..printer("ZDesigner ZD420", false, "")
            },
            printer("ZDesigner ZD420 (Copy 1)", false, ""),
        ];

        assert_eq!(
            choose_from(None, &printers),
            Some(PrinterChoice {
                name: "ZDesigner ZD420 (Copy 1)".to_string(),
                source: PrinterSource::Automatic,
            })
        );
    }

    #[test]
    fn choose_from_uses_the_system_default_when_none_look_like_zpl() {
        let printers = [
            printer("Brother HL-L2350DW series Printer", false, ""),
            printer("Microsoft Print to PDF", true, ""),
        ];

        assert_eq!(
            choose_from(None, &printers),
            Some(PrinterChoice {
                name: "Microsoft Print to PDF".to_string(),
                source: PrinterSource::Automatic,
            })
        );
    }

    #[test]
    fn choose_from_uses_the_first_printer_when_none_is_the_default() {
        let printers = [
            printer("Brother HL-L2350DW series Printer", false, ""),
            printer("Microsoft Print to PDF", false, ""),
        ];

        assert_eq!(
            choose_from(None, &printers),
            Some(PrinterChoice {
                name: "Brother HL-L2350DW series Printer".to_string(),
                source: PrinterSource::Automatic,
            })
        );
    }

    #[test]
    fn choose_from_returns_none_when_no_printer_is_installed() {
        assert_eq!(choose_from(None, &[]), None);
    }
}
