/// Módulo para notificaciones de Windows
/// Maneja toast notifications usando notify-rust

use notify_rust::{Notification, Timeout};
use anyhow::Result;
use tracing::{info, error};

/// Mostrar notificación de backup completado exitosamente
pub fn show_backup_success(files_copied: Option<u32>, duration: Option<&str>) -> Result<()> {
    let title = "✅ Backup Completado";
    
    let message = match (files_copied, duration) {
        (Some(count), Some(time)) => format!("✨ {} archivos copiados en {}", count, time),
        (Some(count), None) => format!("✨ {} archivos copiados", count),
        (None, Some(time)) => format!("✨ Sin cambios detectados ({})", time),
        (None, None) => "✨ Sin cambios detectados".to_string(),
    };

    show_notification(title, &message, NotificationType::Success)
}

/// Mostrar notificación de backup con advertencias
pub fn show_backup_warning(warning_msg: &str) -> Result<()> {
    let title = "⚠️ Backup con Advertencias";
    let message = format!("🔶 {}", warning_msg);
    
    show_notification(title, &message, NotificationType::Warning)
}

/// Mostrar notificación de backup fallido
pub fn show_backup_failed(error_msg: &str) -> Result<()> {
    let title = "❌ Backup Falló";
    let message = format!("💥 {}", error_msg);
    
    show_notification(title, &message, NotificationType::Error)
}

/// Mostrar notificación de daemon iniciado
pub fn show_daemon_started(interval: u64) -> Result<()> {
    let title = "🤖 Daemon Iniciado";
    let hours = interval / 3600;
    let message = if hours >= 1 {
        format!("⏰ Backup automático cada {} horas", hours)
    } else {
        format!("⏰ Backup automático cada {} segundos", interval)
    };
    
    show_notification(title, &message, NotificationType::Info)
}

/// Mostrar notificación de daemon detenido
pub fn show_daemon_stopped() -> Result<()> {
    let title = "⏹️ Daemon Detenido";
    let message = "🔕 Backup automático deshabilitado";
    
    show_notification(title, message, NotificationType::Info)
}

/// Mostrar notificación cuando se minimiza al tray
pub fn show_tray_minimized() -> Result<()> {
    let title = "📦 Minimizado al Tray";
    let message = "🖥️ La aplicación sigue funcionando en segundo plano\n💡 Click derecho en el icono del tray para abrir menú";
    
    show_notification(title, message, NotificationType::Info)
}

/// Tipos de notificación para diferentes estilos
enum NotificationType {
    Success,
    Warning,
    Error,
    Info,
}

/// Función central para mostrar notificaciones
fn show_notification(title: &str, message: &str, notification_type: NotificationType) -> Result<()> {
    info!("🔔 Showing notification: {} - {}", title, message);
    
    let mut notification = Notification::new();
    
    notification
        .summary(title)
        .body(message)
        .timeout(Timeout::Milliseconds(5000)) // 5 segundos
        .appname("RustyVault");
    
    // Configurar icon según el tipo (opcional)
    match notification_type {
        NotificationType::Success => {
            // Icono de éxito (si tienes un archivo .ico)
            // notification.icon("success.ico");
        }
        NotificationType::Warning => {
            // Icono de advertencia
            // notification.icon("warning.ico");  
        }
        NotificationType::Error => {
            // Icono de error
            // notification.icon("error.ico");
        }
        NotificationType::Info => {
            // Icono de información
            // notification.icon("info.ico");
        }
    }
    
    match notification.show() {
        Ok(_) => {
            info!("✅ Notification shown successfully");
            Ok(())
        }
        Err(e) => {
            error!("❌ Failed to show notification: {}", e);
            Err(anyhow::anyhow!("Failed to show notification: {}", e))
        }
    }
}

/// Inicializar sistema de notificaciones (si es necesario)
pub fn initialize() -> Result<()> {
    // En Windows con notify-rust generalmente no se necesita inicialización especial
    info!("🔔 Notification system initialized");
    Ok(())
} 