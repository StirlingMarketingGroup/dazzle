//! Toggle the "Launch at login" registry entry (HKCU Run key) on Windows.
//! On other platforms these are no-ops — autostart is managed by the system.

#[cfg(target_os = "windows")]
mod platform {
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE,
        REG_SZ,
    };
    use windows::core::HSTRING;

    const RUN_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "Dazzle";

    fn exe_path() -> Result<String, String> {
        std::env::current_exe()
            .map(|p| format!("\"{}\" --hidden", p.display()))
            .map_err(|e| e.to_string())
    }

    fn win32_ok(err: windows::Win32::Foundation::WIN32_ERROR) -> Result<(), String> {
        if err.0 == 0 {
            Ok(())
        } else {
            Err(format!("Registry error: {err:?}"))
        }
    }

    pub fn is_enabled() -> Result<bool, String> {
        unsafe {
            let mut hkey = Default::default();
            let subkey = HSTRING::from(RUN_KEY);
            let status =
                RegOpenKeyExW(HKEY_CURRENT_USER, &subkey, None, KEY_READ, &mut hkey);
            if status == ERROR_FILE_NOT_FOUND {
                return Ok(false);
            }
            win32_ok(status)?;

            let name = HSTRING::from(VALUE_NAME);
            let result = RegQueryValueExW(hkey, &name, None, None, None, None);
            let _ = RegCloseKey(hkey);

            Ok(result.0 == 0)
        }
    }

    pub fn enable() -> Result<(), String> {
        let value = exe_path()?;
        let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2)
        };

        unsafe {
            let mut hkey = Default::default();
            let subkey = HSTRING::from(RUN_KEY);
            win32_ok(RegCreateKeyExW(
                HKEY_CURRENT_USER,
                &subkey,
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            ))?;

            let name = HSTRING::from(VALUE_NAME);
            let result = RegSetValueExW(hkey, &name, None, REG_SZ, Some(bytes));
            let _ = RegCloseKey(hkey);

            win32_ok(result)
        }
    }

    pub fn disable() -> Result<(), String> {
        unsafe {
            let mut hkey = Default::default();
            let subkey = HSTRING::from(RUN_KEY);
            let status =
                RegOpenKeyExW(HKEY_CURRENT_USER, &subkey, None, KEY_WRITE, &mut hkey);
            if status == ERROR_FILE_NOT_FOUND {
                return Ok(()); // Key doesn't exist, so it's already disabled
            }
            win32_ok(status)?;

            let name = HSTRING::from(VALUE_NAME);
            let result = RegDeleteValueW(hkey, &name);
            let _ = RegCloseKey(hkey);

            // Not found is fine — already disabled
            if result == ERROR_FILE_NOT_FOUND {
                return Ok(());
            }
            win32_ok(result)
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub fn is_enabled() -> Result<bool, String> {
        Ok(false)
    }

    pub fn enable() -> Result<(), String> {
        Ok(())
    }

    pub fn disable() -> Result<(), String> {
        Ok(())
    }
}

pub use platform::*;
