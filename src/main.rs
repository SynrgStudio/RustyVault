use anyhow::Result;
use std::env;
use tracing::{info, error, warn};

mod app;
mod ui;
mod core;
mod system;
mod logging;

use app::BackupApp;
use logging::setup_logging;

/// Entry point principal de la aplicación RustyVault
fn main() -> Result<()> {
    // Setup logging system
    setup_logging()?;
    
    // CRÍTICO: Inicializar tray-icon event loop antes de crear la GUI
    // Esto es necesario para que funcionen los eventos del system tray en Windows
    info!("🔧 Inicializando sistema de eventos del tray...");
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let start_daemon = args.contains(&"--start-daemon".to_string());
    let show_help = args.contains(&"--help".to_string());
    
    if show_help {
        show_help_message();
        return Ok(());
    }
    
    info!("🚀 Iniciando RustyVault v2.0");
    info!("👤 Desarrollado por Alexis Texas - Rust Senior Developer");
    
    // Configurar egui para Windows
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 800.0])  // Suficiente para ver todo
            .with_min_inner_size([700.0, 800.0])
            .with_max_inner_size([700.0, 800.0])
            .with_resizable(false)
            .with_icon(load_icon())
            .with_title("RustyVault v2.0"),
        ..Default::default()
    };
    
    // Iniciar aplicación GUI
    if let Err(e) = eframe::run_native(
        "RustyVault",
        native_options,
        Box::new(move |cc| {
            // Setup egui styling para mejor apariencia
            setup_custom_style(&cc.egui_ctx);
            
            // Crear app con flag de auto-start daemon
            Ok(Box::new(BackupApp::new(cc, start_daemon)))
        }),
    ) {
        error!("❌ Error al iniciar la aplicación GUI: {}", e);
        return Err(anyhow::anyhow!("Failed to start GUI application: {}", e));
    }
    
    info!("👋 RustyVault cerrado correctamente");
    Ok(())
}

/// Muestra mensaje de ayuda CLI
fn show_help_message() {
    println!("🔧 RustyVault v2.0 - Modern Backup Automation");
    println!("👤 Desarrollado por Alexis Texas");
    println!();
    println!("USO:");
    println!("  rusty-vault.exe [OPTIONS]");
    println!();
    println!("OPCIONES:");
    println!("  --start-daemon    Auto-inicia el daemon de backup al abrir");
    println!("  --help           Muestra este mensaje de ayuda");
    println!();
    println!("CONFIGURACIÓN:");
    println!("  La configuración se guarda en config.json junto al ejecutable");
    println!("  Edita manualmente el archivo para configuraciones avanzadas");
    println!();
    println!("EJEMPLOS:");
    println!("  rusty-vault.exe                  # Abrir GUI normal");
    println!("  rusty-vault.exe --start-daemon   # Auto-start daemon");
}

/// Carga el icono desde archivo ico.ico
fn load_icon() -> egui::IconData {
    // Intentar cargar ico.ico desde el directorio del ejecutable
    let icon_path = std::env::current_exe()
        .map(|exe_path| exe_path.parent().unwrap_or_else(|| std::path::Path::new(".")).join("ico.ico"))
        .unwrap_or_else(|_| std::path::Path::new("ico.ico").to_path_buf());
    
    if icon_path.exists() {
        info!("📁 Cargando icono para ventana desde: {}", icon_path.display());
        
        // Leer archivo ico
        if let Ok(icon_bytes) = std::fs::read(&icon_path) {
            // Para egui necesitamos convertir .ico a RGBA
            // Por simplicidad, usamos el icono por defecto si hay problemas
            if let Ok(icon_image) = image::load_from_memory(&icon_bytes) {
                let rgba_image = icon_image.to_rgba8();
                let (width, height) = rgba_image.dimensions();
                
                return egui::IconData {
                    rgba: rgba_image.into_raw(),
                    width: width as u32,
                    height: height as u32,
                };
            }
        }
        
        warn!("⚠️ Error procesando ico.ico, usando icono por defecto");
    } else {
        warn!("⚠️ ico.ico no encontrado en: {}", icon_path.display());
    }
    
    // Fallback: icono por defecto
    egui::IconData::default()
}

/// Configurar estilo custom para egui - Dark Mode elegante
fn setup_custom_style(ctx: &egui::Context) {
    // 🎨 CAMBIO DE TEMA: Cambia esta línea para probar diferentes temas
    setup_theme_elegant_dark(ctx);
    
    // 🌟 TEMAS DISPONIBLES:
    // setup_theme_elegant_dark(ctx);   // ✨ Gris violeta suave (ACTUAL)
    // setup_theme_forest_green(ctx);   // 🟢 Verde oscuro profesional
    // setup_theme_steel_blue(ctx);     // 🔵 Azul acero suave
}

/// 🌙 TEMA: Elegant Dark - Gris violeta suave (Recomendado)
fn setup_theme_elegant_dark(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    // Dark mode base
    style.visuals.dark_mode = true;
    
    // Colores base oscuros y elegantes
    style.visuals.window_fill = egui::Color32::from_rgb(32, 32, 32);      // Gris oscuro principal
    style.visuals.panel_fill = egui::Color32::from_rgb(40, 40, 40);       // Gris un poco más claro para panels
    style.visuals.faint_bg_color = egui::Color32::from_rgb(24, 24, 24);   // Background más oscuro
    
    // Widgets con tonos violeta sutiles
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(50, 50, 50);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(55, 55, 55);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 90); // ✨ Gris violeta suave
    
    // Texto claro y legible
    style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(220, 220, 220);
    style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);
    style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
    style.visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
    
    // Selection con violeta elegante
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(90, 90, 100); // ✨ Gris violeta para selección
    style.visuals.selection.stroke.color = egui::Color32::WHITE; // 🔥 TEXTO BLANCO para elementos seleccionados
    
    // 🔥 FIX: Texto blanco brillante para elementos seleccionados
    style.visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(255, 255, 255); // Blanco puro
    style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(255, 255, 255); // Blanco puro también para hover
    
    // Accents sutiles
    style.visuals.hyperlink_color = egui::Color32::from_rgb(140, 140, 180); // ✨ Violeta suave para links
    style.visuals.warn_fg_color = egui::Color32::from_rgb(255, 140, 0);     // Orange para warnings
    style.visuals.error_fg_color = egui::Color32::from_rgb(255, 80, 80);    // Red para errors
    
    apply_common_style_settings(&mut style);
    ctx.set_style(style);
}

/// 🟢 TEMA: Forest Green - Verde oscuro profesional (Alternativa)
#[allow(dead_code)]
fn setup_theme_forest_green(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    style.visuals.dark_mode = true;
    
    // Base colors con tonos verdes
    style.visuals.window_fill = egui::Color32::from_rgb(25, 35, 25);       // Verde muy oscuro
    style.visuals.panel_fill = egui::Color32::from_rgb(30, 40, 30);        // Verde oscuro para panels
    style.visuals.faint_bg_color = egui::Color32::from_rgb(20, 25, 20);    // Background más oscuro
    
    // Widgets con accent verde
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(45, 50, 45);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 55, 50);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 70, 60);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(70, 85, 70); // Verde suave
    
    // Selection verde elegante
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(80, 100, 80);
    style.visuals.selection.stroke.color = egui::Color32::WHITE; // 🔥 TEXTO BLANCO para elementos seleccionados
    
    // Texto claro y legible
    style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(220, 220, 220);
    style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);
    style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(255, 255, 255);
    style.visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(255, 255, 255);
    
    // Accents verdes
    style.visuals.hyperlink_color = egui::Color32::from_rgb(120, 160, 120);
    style.visuals.warn_fg_color = egui::Color32::from_rgb(255, 140, 0);
    style.visuals.error_fg_color = egui::Color32::from_rgb(255, 80, 80);
    
    apply_common_style_settings(&mut style);
    ctx.set_style(style);
}

/// 🔵 TEMA: Steel Blue - Azul acero suave (Sin el azul molesto)
#[allow(dead_code)]
fn setup_theme_steel_blue(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    style.visuals.dark_mode = true;
    
    // Base colors con tonos azul acero
    style.visuals.window_fill = egui::Color32::from_rgb(28, 32, 38);       // Azul gris oscuro
    style.visuals.panel_fill = egui::Color32::from_rgb(35, 40, 45);        // Azul gris para panels
    style.visuals.faint_bg_color = egui::Color32::from_rgb(22, 25, 30);    // Background más oscuro
    
    // Widgets con accent azul suave
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(45, 50, 55);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 55, 60);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(65, 70, 75);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(75, 80, 90); // Azul acero suave
    
    // Selection azul elegante (NO brillante)
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(85, 90, 100);
    style.visuals.selection.stroke.color = egui::Color32::WHITE; // 🔥 TEXTO BLANCO para elementos seleccionados
    
    // Texto claro y legible
    style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(220, 220, 220);
    style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);
    style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(255, 255, 255);
    style.visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(255, 255, 255);
    
    // Accents azules suaves
    style.visuals.hyperlink_color = egui::Color32::from_rgb(130, 140, 170);
    style.visuals.warn_fg_color = egui::Color32::from_rgb(255, 140, 0);
    style.visuals.error_fg_color = egui::Color32::from_rgb(255, 80, 80);
    
    apply_common_style_settings(&mut style);
    ctx.set_style(style);
}

/// Configuración común para todos los temas
fn apply_common_style_settings(style: &mut egui::Style) {
    // Spacing cómodo
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.indent = 18.0;
    
    // Bordes más suaves
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
} 