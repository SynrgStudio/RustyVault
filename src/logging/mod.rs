use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    Layer,
};

/// Setup del sistema de logging multi-target según PRD
/// - Console: Todos los niveles (development)
/// - File: Solo errores (production) 
/// - UI: Todos los niveles (via channel - implementar después)
pub fn setup_logging() -> Result<()> {
    // Determinar directorio de logs
    let log_dir = get_log_directory()?;
    std::fs::create_dir_all(&log_dir)?;
    
    // File appender solo para errores
    let log_file = log_dir.join("daemon_backup_ui.log");
    let file_appender = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;
    
    // Layer para archivos - solo errores y warnings
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false) // No ANSI codes en archivos
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(EnvFilter::new("warn")); // Solo warnings y errores
    
    // Layer para console - todos los niveles en desarrollo
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true) // Colores en console
        .with_target(false) // Más limpio en console
        .with_filter(get_console_filter());
    
    // Configurar subscriber con múltiples layers
    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();
    
    tracing::info!("📋 Sistema de logging configurado:");
    tracing::info!("  📁 Logs de errores: {}", log_dir.display());
    tracing::info!("  🖥️ Console logging: {}", get_console_level());
    
    Ok(())
}

/// Determina el directorio para logs
/// Prioridad: carpeta del ejecutable > carpeta temp
fn get_log_directory() -> Result<PathBuf> {
    // Intentar usar carpeta del ejecutable primero
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return Ok(exe_dir.to_path_buf());
        }
    }
    
    // Fallback a carpeta temporal del usuario
    if let Some(temp_dir) = dirs::cache_dir() {
        let log_dir = temp_dir.join("RobocopyBackupTool");
        return Ok(log_dir);
    }
    
    // Último fallback a directorio actual
    Ok(PathBuf::from("."))
}

/// Filtro para console según environment
fn get_console_filter() -> EnvFilter {
    // En development: debug, en production: info
    let default_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level))
}

/// Obtiene nivel de logging para console (para mostrar en UI)
fn get_console_level() -> &'static str {
    if cfg!(debug_assertions) {
        "DEBUG (development build)"
    } else {
        "INFO (release build)"
    }
} 