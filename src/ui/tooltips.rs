#![allow(dead_code)]
/// Sistema de tooltips educativos para parámetros de Robocopy
/// Cada tooltip explica MUY claramente qué hace cada opción para usuarios no técnicos
/// 
/// Formato: Explicación simple + Warning/Tip + Recomendación
/// Iconos: ⚠️ warnings, 💡 tips, 🔧 técnico, 🔄 repetición, ⏱️ tiempo

/// Tooltip para Mirror Mode (/MIR)
pub const MIRROR_MODE_TOOLTIP: &str = r#"/MIR - Modo Espejo: Crea una copia EXACTA del origen en destino.
⚠️ ATENCIÓN: Elimina archivos del destino que no existan en origen.
Útil para: Backup completo idéntico
Cuidado con: Puede borrar archivos si cambias la carpeta origen"#;

/// Tooltip para Multithreading (/MT:X)
pub const MULTITHREADING_TOOLTIP: &str = r#"/MT:X - Hilos simultáneos para copiar archivos.
💡 Más hilos = Más velocidad (hasta cierto punto)
Recomendado: 8 hilos para discos duros, 16+ para SSDs
Rango: 1-128 hilos"#;

/// Tooltip para FAT File Timing (/FFT)
pub const FAT_TIMING_TOOLTIP: &str = r#"/FFT - Compatibilidad con discos FAT32/exFAT.
🔧 Soluciona problemas con USBs, discos externos, NAS antiguos
Usa timing de 2 segundos en lugar de precisión de NTFS
Recomendado: Activar siempre por compatibilidad"#;

/// Tooltip para Retry Count (/R:X)
pub const RETRY_COUNT_TOOLTIP: &str = r#"/R:X - Reintentos por archivo fallido.
🔄 Por defecto robocopy reintenta 1,000,000 de veces (!)
Recomendado: 3-5 reintentos para uso normal
Para red inestable: 10+ reintentos"#;

/// Tooltip para Retry Wait (/W:X)
pub const RETRY_WAIT_TOOLTIP: &str = r#"/W:X - Segundos que espera entre cada reintento.
⏱️ Por defecto robocopy espera 30 segundos (!)
Recomendado: 2-5 segundos para uso normal
Para red lenta: 10+ segundos"#;

/// Tooltip para Check Interval
pub const CHECK_INTERVAL_TOOLTIP: &str = r#"Intervalo entre verificaciones automáticas de backup.
⏱️ Define cada cuántos segundos el daemon revisa si necesita hacer backup

📋 Configuraciones recomendadas:
- 3600s (1 hora): Ideal para documentos de trabajo
- 7200s (2 horas): Uso normal para archivos que cambian poco  
- 18000s (5 horas): Archivos grandes o que cambian raramente
- 300s (5 min): Solo para archivos muy críticos

💡 Tip: Intervalos muy cortos (<60s) aumentan el uso de CPU"#;

/// Tooltip para Start with Windows
pub const START_WITH_WINDOWS_TOOLTIP: &str = r#"Iniciar automáticamente con Windows.
🚀 La aplicación se abrirá al iniciar el sistema
Se registra en Windows Registry (HKCU\Run)
Útil para: Backups automáticos sin intervención manual"#;

/// Tooltip para Source Folder
pub const SOURCE_FOLDER_TOOLTIP: &str = r#"Carpeta de origen que se va a respaldar.
📁 Debe existir y ser accesible
Se copiarán todos los archivos y subcarpetas
Ejemplo: C:\Users\TuUsuario\Documents"#;

/// Tooltip para Destination Folder
pub const DESTINATION_FOLDER_TOOLTIP: &str = r#"Carpeta de destino donde se guardará el backup.
📦 Se creará automáticamente si no existe
Recomendado: Usar un disco diferente al origen
Ejemplo: D:\Backup\Documents"#;

/// Helper function para mostrar tooltips con icono de ayuda
pub fn show_tooltip_with_icon(ui: &mut egui::Ui, text: &str, tooltip: &str) -> egui::Response {
    #![allow(dead_code)]
    ui.horizontal(|ui| {
        ui.label(text);
        ui.label("❔")
            .on_hover_text(tooltip)
    }).response
}

/// Helper function para mostrar tooltip solo en texto (sin icono)
pub fn show_tooltip_text(ui: &mut egui::Ui, text: &str, tooltip: &str) -> egui::Response {
    ui.label(text)
        .on_hover_text(tooltip)
}

/// Tooltip helper para checkbox con explicación
pub fn tooltip_checkbox(ui: &mut egui::Ui, value: &mut bool, text: &str, tooltip: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let response = ui.checkbox(value, text);
        ui.label("❔")
            .on_hover_text(tooltip);
        response
    }).inner
}

/// Tooltip helper para slider con explicación
pub fn tooltip_slider<T>(
    ui: &mut egui::Ui, 
    value: &mut T, 
    range: std::ops::RangeInclusive<T>,
    text: &str,
    tooltip: &str
) -> egui::Response 
where
    T: egui::emath::Numeric,
{
    ui.horizontal(|ui| {
        ui.label(text);
        let response = ui.add(egui::Slider::new(value, range));
        ui.label("❔")
            .on_hover_text(tooltip);
        response
    }).inner
}

/// Preset buttons para check interval con tooltips
pub fn interval_preset_buttons(ui: &mut egui::Ui, interval: &mut u64) {
    ui.horizontal(|ui| {
        if ui.button("1h")
            .on_hover_text("3600 segundos - Ideal para documentos")
            .clicked() 
        {
            *interval = 3600;
        }
        
        if ui.button("2h")
            .on_hover_text("7200 segundos - Uso normal")
            .clicked() 
        {
            *interval = 7200;
        }
        
        if ui.button("5h")
            .on_hover_text("18000 segundos - Archivos que cambian poco")
            .clicked() 
        {
            *interval = 18000;
        }
    });
} 