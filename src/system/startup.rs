use anyhow::{Context, Result};
use std::path::Path;

/// Create or remove a shortcut in the current user's Startup folder.
/// This approach is less intrusive than copying the exe and is visible to users.
#[cfg(target_os = "windows")]
pub fn set_startup_shortcut(enabled: bool, exe: &Path) -> Result<()> {
    use std::process::Command;

    let exe = exe.canonicalize().context("failed to canonicalize exe path")?;

    // Resolve Startup folder: %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
    let startup = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("Startup"))
        .context("APPDATA not set; cannot determine Startup folder")?;

    let lnk = startup.join("RustyVault.lnk");

    if enabled {
        // Copy exe to a fixed location in APPDATA to ensure the path always exists
        let appdata = std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .map(|p| p.join("RustyVault"))
            .context("APPDATA not set; cannot determine app data folder")?;

        std::fs::create_dir_all(&appdata).context("failed to create app data directory")?;

        let exe_copy = appdata.join("rusty-vault.exe");

        // Copy the exe to the fixed location
        std::fs::copy(&exe, &exe_copy).context("failed to copy exe to app data directory")?;

        tracing::info!("📋 Copied exe to: {}", exe_copy.display());

        // PowerShell script to create a .lnk using WScript.Shell
        let exe_str = exe_copy.display().to_string();
        let lnk_str = lnk.display().to_string();
        let cwd = exe_copy.parent().map(|p| p.display().to_string()).unwrap_or_else(|| "".to_string());

        tracing::info!("🔗 Creating startup shortcut:");
        tracing::info!("   TargetPath: {}", exe_str);
        tracing::info!("   Arguments: --start-daemon");
        tracing::info!("   WorkingDirectory: {}", cwd);
        tracing::info!("   Shortcut: {}", lnk_str);

        let ps = format!(
            "$shell = New-Object -ComObject WScript.Shell; $sc = $shell.CreateShortcut('{lnk}'); $sc.TargetPath = '{exe}'; $sc.Arguments = '--start-daemon'; $sc.WorkingDirectory = '{cwd}'; $sc.Save(); Write-Host 'Shortcut created successfully'",
            lnk = lnk_str,
            exe = exe_str,
            cwd = cwd
        );

        // Ejecutar PowerShell de forma asíncrona para no bloquear la UI
        let ps_clone = ps.clone();
        let exe_clone = exe_str.clone();
        std::thread::spawn(move || {
            use std::process::Command;
            match Command::new("powershell")
                .args(&["-NoProfile", "-NonInteractive", "-Command", &ps_clone])
                .status()
            {
                Ok(status) if status.success() => {
                    tracing::info!("✅ Startup shortcut created successfully for: {}", exe_clone);
                }
                Ok(status) => {
                    tracing::error!("❌ PowerShell failed to create shortcut: exit code {}", status);
                }
                Err(e) => {
                    tracing::error!("❌ Failed to launch PowerShell for shortcut creation: {}", e);
                }
            }
        });
    } else {
        if lnk.exists() {
            std::fs::remove_file(&lnk).context("failed to remove existing shortcut")?;
            tracing::info!("🗑️ Startup shortcut removed");
        }

        // Also try to remove the copied exe
        let appdata = std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .map(|p| p.join("RustyVault").join("rusty-vault.exe"));

        if let Some(exe_copy) = appdata {
            if exe_copy.exists() {
                let _ = std::fs::remove_file(&exe_copy); // Ignore errors
                tracing::info!("🗑️ Copied exe removed: {}", exe_copy.display());
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_startup_shortcut(_enabled: bool, _exe: &Path) -> Result<()> {
    // No-op on non-windows systems
    Ok(())
}
