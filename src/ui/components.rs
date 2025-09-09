/// Componentes UI reutilizables para la aplicación
/// Por ahora básicos, se expandirán según necesidades futuras

use eframe::egui;

/// Status indicator component - muestra estado con color y emoji
pub fn status_indicator(ui: &mut egui::Ui, is_active: bool, active_text: &str, inactive_text: &str) {
    if is_active {
        ui.colored_label(egui::Color32::GREEN, format!("✅ {}", active_text));
    } else {
        ui.colored_label(egui::Color32::GRAY, format!("⏸️ {}", inactive_text));
    }
}

/// Folder path display with truncation if too long
pub fn folder_path_display(ui: &mut egui::Ui, path: &str, _max_width: f32) -> egui::Response {
    let truncated = if path.len() > 50 {
        format!("...{}", &path[path.len().saturating_sub(47)..])
    } else {
        path.to_string()
    };
    
    ui.add(egui::Label::new(truncated))
        .on_hover_text(path)
}

/// Progress bar component para mostrar progreso de backup (futuro)
pub fn backup_progress_bar(ui: &mut egui::Ui, progress: f32, current_file: Option<&str>) {
    ui.add(egui::ProgressBar::new(progress).show_percentage());
    
    if let Some(file) = current_file {
        ui.small(format!("Processing: {}", file));
    }
} 