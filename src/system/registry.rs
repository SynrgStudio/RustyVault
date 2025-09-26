#![allow(dead_code)]
use anyhow::{Context, Result};
use std::path::Path;
use tracing::{info, debug};

// Value name used in the Run registry key
const RUN_VALUE_NAME: &str = "RustyVault";

/// Configure auto-start with Windows via Registry
#[cfg(target_os = "windows")]
pub fn set_windows_startup(enabled: bool, exe_path: &Path) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    info!("🚀 Configuring Windows startup: enabled={}, path={}", enabled, exe_path.display());

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _disp) = hkcu
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .context("Failed to open HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run key")?;

    if enabled {
        // Command with a flag so the app can detect startup mode if needed
        let command = format!("\"{}\" --start-daemon", exe_path.display());
        key.set_value(RUN_VALUE_NAME, &command)
            .context("Failed to set Run value in registry")?;
        info!("✅ Registered {} to start with Windows", RUN_VALUE_NAME);
    } else {
        match key.delete_value(RUN_VALUE_NAME) {
            Ok(_) => info!("✅ Unregistered {} from Windows startup", RUN_VALUE_NAME),
            Err(e) => debug!("Value not present or failed to delete: {}", e),
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_windows_startup(_enabled: bool, _exe_path: &Path) -> Result<()> {
    debug!("set_windows_startup called on non-windows OS - noop");
    Ok(())
}

/// Check if app is registered to start with Windows
#[cfg(target_os = "windows")]
pub fn is_windows_startup_enabled() -> Result<bool> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .context("Failed to open HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run key")?;

    match key.get_value::<String, &str>(RUN_VALUE_NAME) {
        Ok(val) => {
            debug!("Found Run value: {}", val);
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_windows_startup_enabled() -> Result<bool> {
    debug!("is_windows_startup_enabled called on non-windows OS - returning false");
    Ok(false)
}

/// Get current exe path
pub fn get_current_exe_path() -> Result<std::path::PathBuf> {
    std::env::current_exe().context("Failed to get current exe path")
}