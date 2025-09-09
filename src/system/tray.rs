/// Sistema de system tray para Windows - COMUNICACIÓN CON BACKGROUND THREAD
/// El thread del tray envía comandos al background thread que maneja el estado

use anyhow::Result;

use tracing::{info, debug, warn};
use tray_icon::{TrayIcon, TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}, Icon, TrayIconEvent};

/// Acciones que puede enviar el system tray (mantenidas para compatibilidad)
#[derive(Debug, Clone)]
pub enum TrayAction {
    ShowApplication,
    StartDaemon,
    StopDaemon,
    Exit,
}

/// Manejador del system tray - COMUNICACIÓN CON BACKGROUND
pub struct SystemTray {
    _tray_icon: TrayIcon,
}

impl SystemTray {
    /// Crear e inicializar system tray - COMUNICACIÓN CON BACKGROUND
    pub fn new(_egui_ctx: eframe::egui::Context) -> Result<Self> {
        info!("Inicializando system tray...");
        
        // Cargar icono
        let icon = Self::load_icon()?;
        
        // Crear menu
        let tray_menu = Menu::new();
        let show_item = MenuItem::with_id("show_app", "Mostrar Aplicacion", true, None);
        let start_daemon_item = MenuItem::with_id("start_daemon", "Iniciar Daemon", true, None);
        let stop_daemon_item = MenuItem::with_id("stop_daemon", "Detener Daemon", true, None);
        let exit_item = MenuItem::with_id("exit_app", "Salir", true, None);
        
        tray_menu.append(&show_item)?;
        tray_menu.append(&start_daemon_item)?;
        tray_menu.append(&stop_daemon_item)?;
        tray_menu.append(&exit_item)?;
        
        // Crear tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("RustyVault - Click derecho para opciones")
            .with_icon(icon)
            .with_menu_on_left_click(true)
            .build()?;
        
        // Thread del tray - envía comandos al background thread
        std::thread::spawn(move || {
            
            loop {
                // Procesar menu events
                while let Ok(menu_event) = MenuEvent::receiver().try_recv() {
                    match menu_event.id.0.as_str() {
                        "show_app" => {
                            let _ = crate::system::window::try_restore_main_window_by_title("RustyVault v2.0");
                            crate::app::send_background_command(crate::app::BackgroundCommand::ShowWindow);
                        }
                        "start_daemon" => {
                            crate::app::send_background_command(crate::app::BackgroundCommand::StartDaemon);
                        }
                        "stop_daemon" => {
                            crate::app::send_background_command(crate::app::BackgroundCommand::StopDaemon);
                        }
                        "exit_app" => {
                            crate::app::send_background_command(crate::app::BackgroundCommand::Exit);
                        }
                        _ => {
                            warn!("Unknown menu ID: '{}'", menu_event.id.0);
                        }
                    }
                }
                
                // Procesar icon events
                while let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
                    match tray_event {
                        TrayIconEvent::Click { .. } => {
                            // Click izquierdo abre el menú (configurado en builder)
                        }
                        TrayIconEvent::DoubleClick { .. } => {
                            let _ = crate::system::window::try_restore_main_window_by_title("RustyVault v2.0");
                            crate::app::send_background_command(crate::app::BackgroundCommand::ShowWindow);
                        }
                        _ => {
                            // Ignore other events
                        }
                    }
                }
                
                // Sleep
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });
        
        info!("System tray inicializado");
        
        Ok(Self {
            _tray_icon: tray_icon,
        })
    }
    
    /// Cargar icono
    fn load_icon() -> Result<Icon> {
        let icon_path = std::env::current_exe()
            .map(|exe_path| exe_path.parent().unwrap_or_else(|| std::path::Path::new(".")).join("ico.ico"))
            .unwrap_or_else(|_| std::path::Path::new("ico.ico").to_path_buf());
        
        if icon_path.exists() {
            debug!("📁 Cargando icono desde: {}", icon_path.display());
            Icon::from_path(&icon_path, None)
                .map_err(|e| anyhow::anyhow!("Error cargando icono: {}", e))
        } else {
            warn!("⚠️ Usando icono por defecto");
            let mut icon_rgba = Vec::new();
            for _ in 0..(16 * 16) {
                icon_rgba.extend_from_slice(&[0, 100, 200, 255]);
            }
            Icon::from_rgba(icon_rgba, 16, 16)
                .map_err(|e| anyhow::anyhow!("Error creando icono: {}", e))
        }
    }
    
    /// Minimizar al tray
    pub fn minimize_to_tray(&self) -> Result<()> {
        
        if let Err(e) = crate::system::notifications::show_tray_minimized() {
            warn!("⚠️ Error mostrando notificación: {}", e);
        }
        
        Ok(())
    }
} 