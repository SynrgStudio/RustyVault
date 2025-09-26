fn main() {
    // Only try to embed resources on Windows
    #[cfg(target_os = "windows")]
    {
        if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
            // Use winres to set the application icon if ico.ico exists next to Cargo.toml
            let icon_path = std::path::Path::new("ico.ico");
            if icon_path.exists() {
                let mut res = winres::WindowsResource::new();
                res.set_icon("ico.ico");
                res.compile()?;
                println!("cargo:rerun-if-changed=ico.ico");
            } else {
                // If ico.ico missing, try ico.png (not all linkers accept png as resource)
                println!("cargo:warning=ico.ico not found; build will continue without embedded icon");
            }
            Ok(())
        })() {
            println!("cargo:warning=Failed to embed Windows resources: {}", e);
        }
    }
}
