//! Settings an administrator sets for the whole PC through Group Policy, under
//! HKLM\SOFTWARE\Policies\StirlingMarketingGroup\Dazzle. Read on every use, so
//! a policy refresh applies without restarting Dazzle.

#[cfg(target_os = "windows")]
mod platform {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};

    const POLICY_KEY: &str = r"SOFTWARE\Policies\StirlingMarketingGroup\Dazzle";
    const PRINTER_VALUE: &str = "Printer";

    /// The printer every user's Dazzle on this PC prints to (REG_SZ value `Printer`).
    pub fn printer() -> Option<String> {
        unsafe {
            let subkey = HSTRING::from(POLICY_KEY);
            let name = HSTRING::from(PRINTER_VALUE);
            let mut size: u32 = 0;
            let status = RegGetValueW(
                HKEY_LOCAL_MACHINE,
                &subkey,
                &name,
                RRF_RT_REG_SZ,
                None,
                None,
                Some(&mut size),
            );
            if status != ERROR_SUCCESS || size < 2 {
                return None;
            }

            let mut buf = vec![0u8; size as usize];
            let status = RegGetValueW(
                HKEY_LOCAL_MACHINE,
                &subkey,
                &name,
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut size),
            );
            if status != ERROR_SUCCESS || size < 2 {
                return None;
            }

            let wide: Vec<u16> = buf[..size as usize]
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .take_while(|unit| *unit != 0)
                .collect();
            let printer = String::from_utf16_lossy(&wide);
            if printer.is_empty() {
                None
            } else {
                Some(printer)
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    /// The printer every user's Dazzle on this PC prints to (REG_SZ value `Printer`).
    pub fn printer() -> Option<String> {
        None
    }
}

pub use platform::*;
