#![allow(dead_code)]
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;
use crate::ui::icons::SafeIcons;

/// Acciones que puede realizar la UI
#[derive(Debug, Clone)]
pub enum UIAction {
    StartDaemon,
    StopDaemon,
    RunBackupNow,
    MinimizeToTray,
    Exit,
    ConfigChanged,
    OpenSettings,
    UpdateAutoStart(bool),
    
    // === NEW CARDS ACTIONS ===
    AddBackupPair { source: String, destination: String },
    UpdateBackupPair { index: usize, source: String, destination: String },
    RemoveBackupPair(usize),
    EditBackupPair(usize),
    MoveBackupPairUp(usize),
    MoveBackupPairDown(usize),

    // === ADVANCED MANAGEMENT ACTIONS ===
    ToggleBackupPairEnabled(usize, bool),
}
use crate::core::{AppConfig, RobocopyConfig};
use crate::ui::tooltips::*;

/// Ventana principal con interfaz minimalista según PRD
/// Layout: Daemon Control + Backup Cards + Robocopy Settings + Window Actions
pub struct MainWindow {
    // === LEGACY UI (to be removed) ===
    /// Buffer temporal para editar source folder
    pub source_folder_buffer: String,
    /// Buffer temporal para editar destination folder  
    pub destination_folder_buffer: String,
    /// Buffer temporal para editar check interval
    pub interval_buffer: String,
    /// Flag para indicar si ya se inicializó desde config
    initialized_from_config: bool,
    
    // === NEW CARDS UI ===
    /// Modal para agregar/editar backup pairs
    pub show_add_modal: bool,
    pub editing_pair_index: Option<usize>,
    /// Buffers para modal add/edit
    pub temp_source_buffer: String,
    pub temp_destination_buffer: String,

    // === DELETE CONFIRMATION MODAL ===
    /// Modal de confirmación para eliminar backup pairs
    pub show_delete_confirmation: bool,
    pub delete_pair_index: Option<usize>,

    // === PATH VALIDATION ===
    /// Resultado de validación en tiempo real
    pub current_validation: Option<crate::core::BackupPairValidation>,

    // === ADVANCED BACKUP PAIR MANAGEMENT ===
    /// Modo de selección múltiple para bulk operations
    pub bulk_selection_mode: bool,
    /// IDs de backup pairs seleccionados para bulk operations
    pub selected_pairs: std::collections::HashSet<String>,
    /// Estado de drag & drop
    pub drag_state: Option<DragState>,
    /// Índice del target de drop
    pub drop_target: Option<usize>,
    /// Modal de confirmación para bulk operations
    pub show_bulk_confirmation: bool,
    /// Tipo de operación bulk pendiente
    pub bulk_operation_type: BulkOperationType,
    
    // === SHARED CONFIG ===
    /// Config temporal para editar parámetros robocopy
    pub temp_robocopy_config: RobocopyConfig,
    /// Flag temporal para start with windows
    pub temp_start_with_windows: bool,
    /// Mostrar preview del comando robocopy
    show_command_preview: bool,
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            // Legacy UI
            source_folder_buffer: String::new(),
            destination_folder_buffer: String::new(),
            interval_buffer: String::new(), // Vacío inicialmente para detectar primera carga
            initialized_from_config: false,
            
            // New Cards UI
            show_add_modal: false,
            editing_pair_index: None,
            temp_source_buffer: String::new(),
            temp_destination_buffer: String::new(),

            // Delete confirmation modal
            show_delete_confirmation: false,
            delete_pair_index: None,

            // Path validation
            current_validation: None,

            // Advanced backup pair management
            bulk_selection_mode: false,
            selected_pairs: std::collections::HashSet::new(),
            drag_state: None,
            drop_target: None,
            show_bulk_confirmation: false,
            bulk_operation_type: BulkOperationType::Enable,
            
            // Shared config
            temp_robocopy_config: RobocopyConfig::default(),
            temp_start_with_windows: false,
            show_command_preview: false,
        }
    }
    
    /// Show main window - llamado desde BackupApp
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &Arc<Mutex<AppConfig>>,
        daemon_running: &Arc<AtomicBool>,
        background_state: &Arc<Mutex<crate::app::AppState>>,
        action_callback: &mut dyn FnMut(UIAction),
    ) {
        // Sincronizar buffers con configuración
        self.sync_buffers_with_config(config);
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🔧 RustyVault v2.0");
            ui.separator();
            
            // Section 1: Daemon Control - Simple y claro
            self.render_daemon_control_section(ui, daemon_running, action_callback);
            ui.add_space(10.0);
            
            // Section 2: Backup Progress Status Dashboard  
            self.render_backup_status_section(ui, config, background_state);
            ui.add_space(10.0);
            
            // Section 3: NEW Backup Cards UI
            self.render_backup_cards_section(ui, config, background_state, action_callback);
            ui.add_space(10.0);
            
            // Section 4: LEGACY Folder Paths (TO BE REMOVED)
            ui.collapsing("🔧 Legacy Single Backup (Dev Only)", |ui| {
                self.render_folder_paths_section(ui, action_callback);
            });
            ui.add_space(10.0);
            
            // Section 5: Robocopy Settings con tooltips explicativos
            self.render_robocopy_settings_section(ui, action_callback);
            ui.add_space(10.0);
            
            // Section 6: Window Actions - Opción A (botón explícito)
            self.render_window_actions_section(ui, action_callback);
            
            // Espacio final + Auto-sizing dinámico
            ui.add_space(5.0); // Padding inferior
            
            // DYNAMIC WINDOW SIZING: Ajustar altura basándose en contenido real
            let final_bottom = ui.min_rect().bottom();
            let target_height = final_bottom + 5.0; // Pequeño buffer extra
            
            // Solo ajustar si la diferencia es significativa
            if (target_height - ctx.screen_rect().height()).abs() > 10.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(700.0, target_height)));
            }
            
            // Command preview (opcional)
            if self.show_command_preview {
                ui.add_space(10.0);
                self.render_command_preview_section(ui);
            }
        });
    }
    
    /// Section 1: Control básico del daemon
    fn render_daemon_control_section(
        &mut self,
        ui: &mut egui::Ui,
        daemon_running: &Arc<AtomicBool>,
        action_callback: &mut dyn FnMut(UIAction),
    ) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label("⚙ Control del Daemon");
            
            ui.horizontal(|ui| {
                // Start with Windows checkbox
                if tooltip_checkbox(
                    ui,
                    &mut self.temp_start_with_windows,
                    "Start with Windows",
                    START_WITH_WINDOWS_TOOLTIP,
                ).changed() {
                    action_callback(UIAction::UpdateAutoStart(self.temp_start_with_windows));
                }
                
                ui.separator();
                
                // Check interval con preset buttons
                ui.label("Check interval:");
                if ui.add(egui::TextEdit::singleline(&mut self.interval_buffer)
                    .desired_width(80.0))
                    .on_hover_text(CHECK_INTERVAL_TOOLTIP)
                    .lost_focus() {
                    action_callback(UIAction::ConfigChanged);
                }
                ui.label("seconds");
                
                // Preset buttons con tooltips
                if ui.button("1h")
                    .on_hover_text("3600 segundos - Ideal para documentos")
                    .clicked() 
                {
                    self.interval_buffer = "3600".to_string();
                    action_callback(UIAction::ConfigChanged);
                }
                
                if ui.button("2h")
                    .on_hover_text("7200 segundos - Uso normal")
                    .clicked() 
                {
                    self.interval_buffer = "7200".to_string();
                    action_callback(UIAction::ConfigChanged);
                }
                
                if ui.button("5h")
                    .on_hover_text("18000 segundos - Archivos grandes")
                    .clicked() 
                {
                    self.interval_buffer = "18000".to_string();
                    action_callback(UIAction::ConfigChanged);
                }
            });
            
            ui.horizontal(|ui| {
                let is_running = daemon_running.load(Ordering::Relaxed);
                
                if is_running {
                    if ui.button("⏹ Stop Daemon").clicked() {
                        action_callback(UIAction::StopDaemon);
                    }
                    ui.label("✅ Daemon running");
                } else {
                    if ui.button("▶ Start Daemon").clicked() {
                        action_callback(UIAction::StartDaemon);
                    }
                    ui.label("⏸ Daemon stopped");
                }
                
                ui.separator();
                
                if ui.button("↻ Run Backup Now").clicked() {
                    action_callback(UIAction::RunBackupNow);
                }
            });
        });
    }
    
    /// Section 2: Source & Destination folders
    fn render_folder_paths_section(
        &mut self,
        ui: &mut egui::Ui,
        action_callback: &mut dyn FnMut(UIAction),
    ) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label("📁 Source & Destination");
            
            // Source folder
            ui.horizontal(|ui| {
                show_tooltip_with_icon(ui, "Source:", SOURCE_FOLDER_TOOLTIP);
                ui.add(egui::TextEdit::singleline(&mut self.source_folder_buffer)
                    .desired_width(300.0));
                if ui.button("📁 Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.source_folder_buffer = path.to_string_lossy().to_string();
                        action_callback(UIAction::ConfigChanged);
                    }
                }
            });
            
            // Destination folder
            ui.horizontal(|ui| {
                show_tooltip_with_icon(ui, "Dest:", DESTINATION_FOLDER_TOOLTIP);
                ui.add(egui::TextEdit::singleline(&mut self.destination_folder_buffer)
                    .desired_width(300.0));
                if ui.button("📁 Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.destination_folder_buffer = path.to_string_lossy().to_string();
                        action_callback(UIAction::ConfigChanged);
                    }
                }
            });
        });
    }
    
    /// Section 3: Robocopy Settings con tooltips MUY explicativos
    fn render_robocopy_settings_section(
        &mut self,
        ui: &mut egui::Ui,
        action_callback: &mut dyn FnMut(UIAction),
    ) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label("🔧 Robocopy Settings");
            
            // Primera fila: Mirror Mode y FAT Timing
            ui.horizontal(|ui| {
                if tooltip_checkbox(
                    ui,
                    &mut self.temp_robocopy_config.mirror_mode,
                    "Mirror Mode",
                    MIRROR_MODE_TOOLTIP,
                ).clicked() {
                    action_callback(UIAction::ConfigChanged);
                }
                
                ui.separator();
                
                if tooltip_checkbox(
                    ui,
                    &mut self.temp_robocopy_config.fat_file_timing,
                    "FAT Timing",
                    FAT_TIMING_TOOLTIP,
                ).clicked() {
                    action_callback(UIAction::ConfigChanged);
                }
            });
            
            // Segunda fila: Threads y Retries
            ui.horizontal(|ui| {
                if tooltip_slider(
                    ui,
                    &mut self.temp_robocopy_config.multithreading,
                    1..=128,
                    "Threads:",
                    MULTITHREADING_TOOLTIP,
                ).drag_stopped() {
                    action_callback(UIAction::ConfigChanged);
                }
                
                ui.separator();
                
                if tooltip_slider(
                    ui,
                    &mut self.temp_robocopy_config.retry_count,
                    0..=20,
                    "Retries:",
                    RETRY_COUNT_TOOLTIP,
                ).drag_stopped() {
                    action_callback(UIAction::ConfigChanged);
                }
            });
            
            // Tercera fila: Wait time
            ui.horizontal(|ui| {
                if tooltip_slider(
                    ui,
                    &mut self.temp_robocopy_config.retry_wait,
                    1..=60,
                    "Wait:",
                    RETRY_WAIT_TOOLTIP,
                ).drag_stopped() {
                    action_callback(UIAction::ConfigChanged);
                }
                ui.label("seconds");
                
                ui.separator();
                
                // Toggle para mostrar preview del comando
                ui.checkbox(&mut self.show_command_preview, "Show Command Preview");
            });
        });
    }
    
    /// Section 4: Window Actions - Opción A (botón explícito)
    fn render_window_actions_section(
        &mut self,
        ui: &mut egui::Ui,
        action_callback: &mut dyn FnMut(UIAction),
    ) {
        ui.horizontal(|ui| {
            if ui.button("⬇ Minimize to Tray")
                .on_hover_text("Minimiza la aplicación al system tray (sigue funcionando en segundo plano)")
                .clicked()
            {
                action_callback(UIAction::MinimizeToTray);
            }

            // 🚧 SETTINGS TEMPORALMENTE DESHABILITADO - UNCOMMENT PARA HABILITAR:
            /*
            if ui.button("⚙ Settings")
                .on_hover_text("Abrir ventana de configuración avanzada")
                .clicked()
            {
                action_callback(UIAction::OpenSettings);
            }
            */
            
            // 🔧 DUMMY SETTINGS BUTTON (remove when enabling real settings above)
            if ui.add_enabled(false, egui::Button::new("⚙ Settings (WIP)"))
                .on_hover_text("Settings en desarrollo - temporalmente deshabilitado")
                .clicked()
            {
                // No action - dummy button
            }

            if ui.button("❌ Exit")
                .on_hover_text("Cerrar completamente la aplicación")
                .clicked()
            {
                action_callback(UIAction::Exit);
            }
        });
    }
    
    /// Command preview section (opcional)
    fn render_command_preview_section(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label("💾 Command Preview");
            let preview = self.temp_robocopy_config.preview_command(
                &self.source_folder_buffer,
                &self.destination_folder_buffer,
            );
            ui.code(&preview);
        });
    }
    
    /// Sincronizar buffers de UI con configuración actual
    fn sync_buffers_with_config(&mut self, config: &Arc<Mutex<AppConfig>>) {
        if let Ok(cfg) = config.lock() {
            // Solo sincronizar la primera vez que se carga la configuración
            if !self.initialized_from_config {
                self.source_folder_buffer = cfg.source_folder.clone();
                self.destination_folder_buffer = cfg.destination_folder.clone();
                self.temp_robocopy_config = cfg.robocopy.clone();
                self.temp_start_with_windows = cfg.start_with_windows;
                self.interval_buffer = cfg.check_interval_seconds.to_string();
                self.initialized_from_config = true;
            }
        }
    }
    
    /// Parse interval desde string buffer
    fn parse_interval(&mut self) -> u64 {
        self.interval_buffer.parse::<u64>().unwrap_or(3600)
    }
    
    // === BACKUP STATUS DASHBOARD ===
    
    /// Renderizar dashboard de progreso de backups con barra segmentada
    fn render_backup_status_section(&self, ui: &mut egui::Ui, config: &Arc<Mutex<AppConfig>>, background_state: &Arc<Mutex<crate::app::AppState>>) {
        // Solo mostrar si hay backup pairs configurados
        let backup_pairs = if let Ok(cfg) = config.lock() {
            cfg.backup_pairs.clone()
        } else {
            return;
        };
        
        if backup_pairs.is_empty() {
            return; // No mostrar dashboard si no hay backups configurados
        }
        
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label("📊 Backup Progress Status");
            
            ui.add_space(8.0);
            
            // Progress bar segmentada
            self.render_segmented_progress_bar(ui, &backup_pairs, background_state);
            
            ui.add_space(8.0);
            
            // Leyenda de colores
            self.render_status_legend(ui, &backup_pairs, background_state);
            
            ui.add_space(8.0);
            
            // Stats resumen
            self.render_backup_stats(ui, &backup_pairs, background_state);
        });
    }
    
    /// Renderizar barra de progreso segmentada (solo backup pairs activos)
    fn render_segmented_progress_bar(&self, ui: &mut egui::Ui, backup_pairs: &[crate::core::config::BackupPair], background_state: &Arc<Mutex<crate::app::AppState>>) {
        // Filtrar solo backup pairs activos
        let active_pairs: Vec<_> = backup_pairs.iter().filter(|pair| pair.enabled).collect();
        let total_active_pairs = active_pairs.len();

        if total_active_pairs == 0 {
            ui.horizontal(|ui| {
                ui.label("Overall Progress:");
                ui.colored_label(egui::Color32::GRAY, "No active backup pairs");
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Overall Progress:");
            ui.colored_label(egui::Color32::GRAY, format!("({} active)", total_active_pairs));

            // Calcular ancho disponible para la barra
            let available_width = ui.available_width() - 20.0; // Margin
            let segment_width = available_width / total_active_pairs as f32;

            for (active_index, pair) in active_pairs.iter().enumerate() {
                // Determinar color del segmento basado en último estado
                let (color, status_char) = self.get_backup_pair_status_visual_real(pair, background_state);

                // Renderizar segmento de la barra
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(segment_width - 2.0, 20.0),
                    egui::Sense::hover()
                );

                ui.painter().rect_filled(rect, 2.0, color);

                // Texto del estado en el centro del segmento
                let text_color = if color == egui::Color32::WHITE {
                    egui::Color32::BLACK
                } else {
                    egui::Color32::WHITE
                };

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    status_char,
                    egui::FontId::default(),
                    text_color,
                );

                // Tooltip con detalles
                if response.hovered() {
                    response.on_hover_ui(|ui| {
                        ui.label(format!("Active Backup Pair #{}", active_index + 1));
                        ui.label(format!("Source: {}", pair.source.display()));
                        ui.label(format!("Destination: {}", pair.destination.display()));
                        ui.label(format!("Status: {}", self.get_backup_pair_status_text_real(pair, background_state)));
                    });
                }
            }
        });
    }
    
    /// Renderizar leyenda de colores (solo backup pairs activos)
    fn render_status_legend(&self, ui: &mut egui::Ui, backup_pairs: &[crate::core::config::BackupPair], background_state: &Arc<Mutex<crate::app::AppState>>) {
        let mut success_count = 0;
        let mut warning_count = 0;
        let mut error_count = 0;
        let mut pending_count = 0;

        // Contar estados usando datos reales - solo backup pairs activos
        for pair in backup_pairs.iter().filter(|pair| pair.enabled) {
            match self.get_backup_pair_status_text_real(pair, background_state).as_str() {
                "Exitoso" => success_count += 1,
                "Advertencia" => warning_count += 1,
                "Error" => error_count += 1,
                _ => pending_count += 1,
            }
        }
        
        ui.horizontal(|ui| {
            ui.label("Legend:");
            
            if success_count > 0 {
                ui.colored_label(egui::Color32::from_rgb(76, 175, 80), "■");
                ui.label(format!("Exitoso ({})", success_count));
            }
            
            if warning_count > 0 {
                ui.colored_label(egui::Color32::from_rgb(255, 152, 0), "■");
                ui.label(format!("Advertencia ({})", warning_count));
            }
            
            if error_count > 0 {
                ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "■");
                ui.label(format!("Error ({})", error_count));
            }
            
            if pending_count > 0 {
                ui.colored_label(egui::Color32::from_rgb(158, 158, 158), "■");
                ui.label(format!("Pendiente ({})", pending_count));
            }
        });
    }
    
    /// Renderizar estadísticas de backup (solo backup pairs activos)
    fn render_backup_stats(&self, ui: &mut egui::Ui, backup_pairs: &[crate::core::config::BackupPair], background_state: &Arc<Mutex<crate::app::AppState>>) {
        // Filtrar solo backup pairs activos
        let active_pairs: Vec<_> = backup_pairs.iter().filter(|pair| pair.enabled).collect();
        let total_active = active_pairs.len();

        if total_active == 0 {
            ui.horizontal(|ui| {
                ui.label("Status: No active backup pairs");
            });
            return;
        }

        // Contar estados completados vs pendientes usando datos reales - solo activos
        let completed_count = active_pairs.iter()
            .filter(|pair| {
                let status = self.get_backup_pair_status_text_real(pair, background_state);
                status != "Pendiente"
            })
            .count();

        ui.horizontal(|ui| {
            ui.label(format!("Status: {}/{} active completed", completed_count, total_active));
            ui.separator();

            // Obtener timestamp del último backup ejecutado (solo activos)
            let last_backup_text = self.get_last_backup_timestamp_active(&active_pairs, background_state);
            ui.weak(format!("Último backup: {}", last_backup_text));
        });
    }
    
    /// Obtener color y carácter visual para el estado de un backup pair (REAL)
    fn get_backup_pair_status_visual_real(&self, pair: &crate::core::config::BackupPair, background_state: &Arc<Mutex<crate::app::AppState>>) -> (egui::Color32, &str) {
        // Obtener estado real del backup pair
        if let Ok(state) = background_state.lock() {
            if let Some(backup_status) = state.backup_statuses.get(&pair.id) {
                match &backup_status.status {
                    crate::app::BackupStatus::Success(_) => (egui::Color32::from_rgb(76, 175, 80), "✅"),   // Success - verde
                    crate::app::BackupStatus::Warning(_) => (egui::Color32::from_rgb(255, 152, 0), "⚠"), // Warning - naranja  
                    crate::app::BackupStatus::Error(_) => (egui::Color32::from_rgb(244, 67, 54), "❌"),   // Error - rojo
                    crate::app::BackupStatus::Running => (egui::Color32::from_rgb(33, 150, 243), "●"),   // Running - azul
                    crate::app::BackupStatus::Pending => (egui::Color32::from_rgb(158, 158, 158), "○"),  // Pending - gris
                }
            } else {
                // No hay estado registrado = pendiente
                (egui::Color32::from_rgb(158, 158, 158), "○")
            }
        } else {
            // Error accediendo al estado = gris
            (egui::Color32::from_rgb(158, 158, 158), "○")
        }
    }
    
    /// Obtener color y carácter visual para el estado de un backup pair (DEMO/FALLBACK)
    fn get_backup_pair_status_visual(&self, pair: &crate::core::config::BackupPair) -> (egui::Color32, &str) {
        // DEMO: Simular estados diversos para mostrar la progress bar
        // TODO: Reemplazar con estado real de backups
        
        // Usar el índice basado en el nombre para simular estados
        let demo_state = pair.source.file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.len() % 4)
            .unwrap_or(0);
            
        match demo_state {
            0 => (egui::Color32::from_rgb(76, 175, 80), "✅"),   // Success - verde
            1 => (egui::Color32::from_rgb(255, 152, 0), "⚠"),   // Warning - naranja  
            2 => (egui::Color32::from_rgb(244, 67, 54), "❌"),   // Error - rojo
            _ => (egui::Color32::from_rgb(158, 158, 158), "○"),  // Pending - gris
        }
    }
    
    /// Obtener texto descriptivo del estado de un backup pair (REAL)
    fn get_backup_pair_status_text_real(&self, pair: &crate::core::config::BackupPair, background_state: &Arc<Mutex<crate::app::AppState>>) -> String {
        // Obtener estado real del backup pair
        if let Ok(state) = background_state.lock() {
            if let Some(backup_status) = state.backup_statuses.get(&pair.id) {
                match &backup_status.status {
                    crate::app::BackupStatus::Success(_) => "Exitoso".to_string(),
                    crate::app::BackupStatus::Warning(msg) => format!("Advertencia: {}", msg),
                    crate::app::BackupStatus::Error(msg) => format!("Error: {}", msg),
                    crate::app::BackupStatus::Running => "En ejecución".to_string(),
                    crate::app::BackupStatus::Pending => "Pendiente".to_string(),
                }
            } else {
                // No hay estado registrado = pendiente
                "Pendiente".to_string()
            }
        } else {
            // Error accediendo al estado
            "Error: Estado no disponible".to_string()
        }
    }
    
    /// Obtener texto descriptivo del estado de un backup pair (DEMO/FALLBACK)
    fn get_backup_pair_status_text(&self, pair: &crate::core::config::BackupPair) -> String {
        // DEMO: Simular estados diversos (mismo algoritmo que visual)
        // TODO: Reemplazar con estado real de backups
        
        let demo_state = pair.source.file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.len() % 4)
            .unwrap_or(0);
            
        match demo_state {
            0 => "Exitoso".to_string(),
            1 => "Advertencia".to_string(), 
            2 => "Error".to_string(),
            _ => "Pendiente".to_string(),
        }
    }
    
    /// Obtener timestamp del último backup ejecutado
    fn get_last_backup_timestamp(&self, backup_pairs: &[crate::core::config::BackupPair], background_state: &Arc<Mutex<crate::app::AppState>>) -> String {
        if let Ok(state) = background_state.lock() {
            // Encontrar el timestamp más reciente de todos los backup pairs
            let most_recent_timestamp = backup_pairs.iter()
                .filter_map(|pair| {
                    state.backup_statuses.get(&pair.id)
                        .and_then(|status| status.last_execution)
                })
                .max();
                
            if let Some(timestamp) = most_recent_timestamp {
                // Convertir timestamp Unix a fecha legible
                if let Some(datetime) = std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(timestamp)) {
                    if let Ok(local_time) = std::time::SystemTime::now().duration_since(datetime) {
                        let seconds_ago = local_time.as_secs();
                        
                        if seconds_ago < 60 {
                            format!("hace {} segundos", seconds_ago)
                        } else if seconds_ago < 3600 {
                            format!("hace {} minutos", seconds_ago / 60)
                        } else if seconds_ago < 86400 {
                            format!("hace {} horas", seconds_ago / 3600)
                        } else {
                            format!("hace {} días", seconds_ago / 86400)
                        }
                    } else {
                        // Backup fue en el futuro (raro pero posible)
                        format!("hace pocos segundos")
                    }
                } else {
                    "Error de timestamp".to_string()
                }
            } else {
                "Nunca ejecutado".to_string()
            }
        } else {
            "Estado no disponible".to_string()
        }
    }

    /// Obtener timestamp del último backup ejecutado (solo backup pairs activos)
    fn get_last_backup_timestamp_active(&self, active_pairs: &[&crate::core::config::BackupPair], background_state: &Arc<Mutex<crate::app::AppState>>) -> String {
        if let Ok(state) = background_state.lock() {
            // Encontrar el timestamp más reciente de todos los backup pairs activos
            let most_recent_timestamp = active_pairs.iter()
                .filter_map(|pair| {
                    state.backup_statuses.get(&pair.id)
                        .and_then(|status| status.last_execution)
                })
                .max();

            if let Some(timestamp) = most_recent_timestamp {
                // Convertir timestamp Unix a fecha legible
                if let Some(datetime) = std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(timestamp)) {
                    if let Ok(local_time) = std::time::SystemTime::now().duration_since(datetime) {
                        let seconds_ago = local_time.as_secs();

                        if seconds_ago < 60 {
                            format!("hace {} segundos", seconds_ago)
                        } else if seconds_ago < 3600 {
                            format!("hace {} minutos", seconds_ago / 60)
                        } else if seconds_ago < 86400 {
                            format!("hace {} horas", seconds_ago / 3600)
                        } else {
                            format!("hace {} días", seconds_ago / 86400)
                        }
                    } else {
                        // Backup fue en el futuro (raro pero posible)
                        format!("hace pocos segundos")
                    }
                } else {
                    "Error de timestamp".to_string()
                }
            } else {
                "Nunca ejecutado".to_string()
            }
        } else {
            "Estado no disponible".to_string()
        }
    }

    /// Obtener estadísticas de un backup pair para mostrar en la card
    fn get_backup_pair_stats(&self, pair: &crate::core::config::BackupPair, background_state: &Arc<Mutex<crate::app::AppState>>) -> (u32, u32, String, String) {
        if let Ok(state) = background_state.lock() {
            if let Some(status) = state.backup_statuses.get(&pair.id) {
                let execution_count = status.execution_count;
                let success_rate = status.success_rate();
                let last_execution = status.format_last_execution();
                let files_copied = match status.files_copied_last {
                    Some(count) => format!("{}", count),
                    None => "0".to_string(),
                };
                
                return (execution_count, success_rate, last_execution, files_copied);
            }
        }
        
        // Valores por defecto si no hay datos
        (0, 0, "nunca".to_string(), "0".to_string())
    }

    // === NEW CARDS UI FUNCTIONS ===
    
    /// Renderizar cards de backup pairs 
    fn render_backup_cards_section(&mut self, ui: &mut egui::Ui, config: &Arc<Mutex<AppConfig>>, background_state: &Arc<Mutex<crate::app::AppState>>, action_callback: &mut dyn FnMut(UIAction)) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label("📂 Backup Directories");
            
            // Leer backup pairs de la config
            let backup_pairs = if let Ok(cfg) = config.lock() {
                cfg.backup_pairs.clone()
            } else {
                vec![]
            };
            
            if backup_pairs.is_empty() {
                // Empty state
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label("📂 Sin backups configurados");
                    ui.weak("Haz click en 'Agregar Backup' para comenzar");
                    ui.add_space(20.0);
                });
            } else {
                // Separar backup pairs en activos y deshabilitados
                let (active_pairs, disabled_pairs): (Vec<_>, Vec<_>) = backup_pairs
                    .iter()
                    .enumerate()
                    .partition(|(_, pair)| pair.enabled);

                // === SECCIÓN DE BACKUP PAIRS ACTIVOS ===
                if !active_pairs.is_empty() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(0, 150, 0), "✅");
                            ui.strong("Active Backup Pairs");
                            ui.colored_label(egui::Color32::GRAY, format!("({})", active_pairs.len()));
                        });

                        ui.add_space(5.0);

                        // Renderizar backup pairs activos
                        for (active_index, (original_index, pair)) in active_pairs.iter().enumerate() {
                            self.render_active_backup_card(ui, *original_index, pair, active_index, active_pairs.len(), &backup_pairs, background_state, action_callback);
                        }
                    });
                }

                // Espacio entre secciones
                if !active_pairs.is_empty() && !disabled_pairs.is_empty() {
                    ui.add_space(10.0);
                }

                // === SECCIÓN DE BACKUP PAIRS DESHABILITADOS ===
                if !disabled_pairs.is_empty() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::GRAY, "❌");
                            ui.colored_label(egui::Color32::GRAY, "Disabled Backup Pairs");
                            ui.colored_label(egui::Color32::GRAY, format!("({})", disabled_pairs.len()));
                        });

                        ui.add_space(5.0);

                        // Renderizar backup pairs deshabilitados
                        for (_, (original_index, pair)) in disabled_pairs.iter().enumerate() {
                            self.render_disabled_backup_card(ui, *original_index, pair, &backup_pairs, action_callback);
                        }
                    });
                }
            }
            
            // Add button
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("+ Agregar Nuevo Backup").clicked() {
                    self.show_add_modal = true;
                    self.editing_pair_index = None;
                    self.temp_source_buffer.clear();
                    self.temp_destination_buffer.clear();
                }
            });
        });
        
        // Show modals if needed
        if self.show_add_modal {
            self.render_add_edit_modal(ui, config, action_callback);
        }

        if self.show_delete_confirmation {
            self.render_delete_confirmation_modal(ui, config, action_callback);
        }
    }
    
    /// Modal para agregar/editar backup pair con validación avanzada
    fn render_add_edit_modal(&mut self, ui: &mut egui::Ui, config: &Arc<Mutex<AppConfig>>, action_callback: &mut dyn FnMut(UIAction)) {
        let modal_title = if self.editing_pair_index.is_some() {
            "Editar Backup"
        } else {
            "Agregar Nuevo Backup"
        };
        
        egui::Window::new(modal_title)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(500.0);

                    // Realizar validación en tiempo real
                    let existing_pairs = if let Ok(cfg) = config.lock() {
                        cfg.backup_pairs.clone()
                    } else {
                        vec![]
                    };

                    let validation = crate::core::PathValidator::validate_backup_pair(
                        &self.temp_source_buffer,
                        &self.temp_destination_buffer,
                        &existing_pairs,
                        self.editing_pair_index
                    );
                    self.current_validation = Some(validation.clone());

                    ui.label("Directorio Origen:");
                    ui.horizontal(|ui| {
                        let _source_response = ui.text_edit_singleline(&mut self.temp_source_buffer);

                        // Mostrar estado de validación del origen
                        self.render_validation_icon(ui, &validation.source_result);

                        if ui.button("📂 Browse").clicked() {
                            // Abrir file dialog para seleccionar directorio origen
                            let mut dialog = rfd::FileDialog::new()
                                .set_title("Seleccionar Directorio Origen");
                            
                            // Si ya hay un path, usarlo como directorio inicial
                            if !self.temp_source_buffer.trim().is_empty() {
                                if let Some(parent) = std::path::Path::new(&self.temp_source_buffer).parent() {
                                    dialog = dialog.set_directory(parent);
                                }
                            }
                            
                            if let Some(folder) = dialog.pick_folder() {
                                self.temp_source_buffer = folder.to_string_lossy().to_string();
                                info!("📂 Source folder selected: {}", self.temp_source_buffer);
                            }
                        }
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.label("Directorio Destino:");
                    ui.horizontal(|ui| {
                        let _dest_response = ui.text_edit_singleline(&mut self.temp_destination_buffer);

                        // Mostrar estado de validación del destino
                        self.render_validation_icon(ui, &validation.destination_result);

                        if ui.button("📂 Browse").clicked() {
                            // Abrir file dialog para seleccionar directorio destino
                            let mut dialog = rfd::FileDialog::new()
                                .set_title("Seleccionar Directorio Destino");
                            
                            // Si ya hay un path, usarlo como directorio inicial
                            if !self.temp_destination_buffer.trim().is_empty() {
                                if let Some(parent) = std::path::Path::new(&self.temp_destination_buffer).parent() {
                                    dialog = dialog.set_directory(parent);
                                }
                            }
                            
                            if let Some(folder) = dialog.pick_folder() {
                                self.temp_destination_buffer = folder.to_string_lossy().to_string();
                                info!("📂 Destination folder selected: {}", self.temp_destination_buffer);
                            }
                        }
                    });
                    
                    ui.add_space(15.0);

                    // Panel de validación
                    self.render_validation_panel(ui, &validation);

                    ui.add_space(15.0);

                    ui.horizontal(|ui| {
                        if ui.button("❌ Cancelar").clicked() {
                            self.show_add_modal = false;
                            self.editing_pair_index = None;
                            self.temp_source_buffer.clear();
                            self.temp_destination_buffer.clear();
                        }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let can_save = !self.temp_source_buffer.trim().is_empty()
                                && !self.temp_destination_buffer.trim().is_empty()
                                && !validation.has_errors();

                            let save_button = if validation.has_errors() {
                                egui::Button::new("❌ Corregir Errores")
                            } else if !validation.get_warning_messages().is_empty() {
                                egui::Button::new("⚠ Guardar con Advertencias")
                            } else {
                                egui::Button::new("✅ Guardar")
                            };

                            if ui.add_enabled(can_save, save_button).clicked() {
                                if let Some(index) = self.editing_pair_index {
                                    // Modo edición
                                    info!("✏️ UI: Actualizando backup pair #{}: {} → {}", 
                                         index + 1, self.temp_source_buffer, self.temp_destination_buffer);
                                    action_callback(UIAction::UpdateBackupPair {
                                        index,
                                        source: self.temp_source_buffer.clone(),
                                        destination: self.temp_destination_buffer.clone(),
                                    });
                                } else {
                                    // Modo agregar
                                    info!("➕ UI: Agregando backup pair: {} → {}", 
                                         self.temp_source_buffer, self.temp_destination_buffer);
                                    action_callback(UIAction::AddBackupPair {
                                        source: self.temp_source_buffer.clone(),
                                        destination: self.temp_destination_buffer.clone(),
                                    });
                                }
                                
                                // Cerrar modal y limpiar estado
                                self.show_add_modal = false;
                                self.editing_pair_index = None;
                                self.temp_source_buffer.clear();
                                self.temp_destination_buffer.clear();
                            }
                        });
                    });
                });
            });
    }
    
    /// Renderizar backup pair activo con funcionalidad completa
    fn render_active_backup_card(
        &mut self,
        ui: &mut egui::Ui,
        original_index: usize,
        pair: &crate::core::config::BackupPair,
        active_index: usize,
        total_active_pairs: usize,
    _existing_pairs: &[crate::core::config::BackupPair],
        background_state: &Arc<Mutex<crate::app::AppState>>,
        action_callback: &mut dyn FnMut(UIAction)
    ) {
        // Validar este backup pair
        let validation = crate::core::PathValidator::validate_backup_pair(
            &pair.source.display().to_string(),
            &pair.destination.display().to_string(),
            _existing_pairs,
            Some(original_index)
        );

        ui.group(|ui| {
            // LÍNEA ÚNICA COMPACTA - Todo en una sola línea horizontal
            ui.horizontal(|ui| {
                // Enable/Disable Toggle - PRIMERA POSICIÓN para fácil acceso
                let mut enabled = pair.enabled;
                if ui.checkbox(&mut enabled, "").clicked() {
                    self.toggle_backup_pair_enabled(original_index, enabled, action_callback);
                }

                // Priority badge para backup pairs activos
                ui.colored_label(
                    egui::Color32::from_rgb(100, 150, 200),
                    format!("#{}", active_index + 1)
                );

                // Mostrar estado de validación con colores apropiados
                if validation.has_errors() {
                    ui.colored_label(egui::Color32::from_rgb(255, 80, 80), SafeIcons::WARNING)
                        .on_hover_text("Hay problemas de configuración que requieren atención");
                } else if !validation.get_warning_messages().is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(100, 150, 255), SafeIcons::INFO)
                        .on_hover_text("Configuración funcional con notas informativas");
                }

                // Source y Destination con estilo normal (backup pair activo)
                ui.label("📁");
                ui.strong(
                    pair.source.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );

                ui.label("->");

                ui.label("📁");
                ui.strong(
                    pair.destination.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );

                // Status indicator para activos
                ui.colored_label(egui::Color32::GREEN, SafeIcons::SUCCESS);

                // ICONO DE DIRECTORIOS con tooltip hover (reemplaza la línea de rutas completas)
                ui.colored_label(egui::Color32::from_rgb(120, 120, 120), "📂")
                    .on_hover_text(format!(
                        "Rutas completas:\n📁 Origen: {}\n📁 Destino: {}",
                        pair.source.display(),
                        pair.destination.display()
                    ));

                // BOTONES DE ACCIÓN - Funcionalidad completa para backup pairs activos
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Move down button
                    if active_index < total_active_pairs - 1 {
                        if ui.small_button("⬇").clicked() {
                            info!("⬇️ UI: Moviendo backup pair activo #{} hacia abajo", active_index + 1);
                            action_callback(UIAction::MoveBackupPairDown(original_index));
                        }
                    }

                    // Move up button
                    if active_index > 0 {
                        if ui.small_button("⬆").clicked() {
                            info!("⬆️ UI: Moviendo backup pair activo #{} hacia arriba", active_index + 1);
                            action_callback(UIAction::MoveBackupPairUp(original_index));
                        }
                    }

                    // Delete button
                    if ui.small_button("🗑").clicked() {
                        info!("🗑️ UI: Abriendo confirmación para eliminar backup pair #{}", original_index + 1);
                        self.show_delete_confirmation = true;
                        self.delete_pair_index = Some(original_index);
                    }

                    // Edit button
                    if ui.small_button("✏").clicked() {
                        info!("✏️ UI: Editando backup pair #{}", original_index + 1);
                        action_callback(UIAction::EditBackupPair(original_index));
                    }
                });
            });
            
            // LÍNEA 2: Estadísticas con font pequeña
            ui.horizontal(|ui| {
                // Obtener estadísticas del backup pair
                let (execution_count, success_rate, last_execution, files_copied) = 
                    self.get_backup_pair_stats(pair, background_state);
                
                // Aplicar font más pequeña
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::new(11.0, egui::FontFamily::Proportional)
                );
                
                // Mostrar estadísticas compactas
                ui.colored_label(
                    egui::Color32::from_rgb(120, 120, 120),
                    format!(
                        "📊 {} ejecuciones • ✅ {}% éxito • ⏱ {} • 📄 {} archivos",
                        execution_count,
                        success_rate,
                        last_execution,
                        files_copied
                    )
                );
            });
        });
        ui.add_space(5.0);
    }

    /// Renderizar backup pair deshabilitado con funcionalidad limitada
    fn render_disabled_backup_card(
        &mut self,
        ui: &mut egui::Ui,
        original_index: usize,
        pair: &crate::core::config::BackupPair,
    _existing_pairs: &[crate::core::config::BackupPair],
        action_callback: &mut dyn FnMut(UIAction)
    ) {
        ui.group(|ui| {
            // Aplicar estilo gris para toda la card
            ui.style_mut().visuals.override_text_color = Some(egui::Color32::GRAY);

            // LÍNEA ÚNICA COMPACTA - Estilo deshabilitado
            ui.horizontal(|ui| {
                // Enable/Disable Toggle - PRIMERA POSICIÓN para fácil acceso
                let mut enabled = pair.enabled;
                if ui.checkbox(&mut enabled, "").clicked() {
                    self.toggle_backup_pair_enabled(original_index, enabled, action_callback);
                }

                // Icono de pausa en lugar de priority badge
                ui.colored_label(egui::Color32::GRAY, "⏸");

                // Source y Destination con estilo gris
                ui.colored_label(egui::Color32::GRAY, "📁");
                ui.colored_label(egui::Color32::GRAY,
                    pair.source.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );

                ui.colored_label(egui::Color32::GRAY, "->");

                ui.colored_label(egui::Color32::GRAY, "📁");
                ui.colored_label(egui::Color32::GRAY,
                    pair.destination.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );

                // ICONO DE DIRECTORIOS con tooltip hover
                ui.colored_label(egui::Color32::GRAY, "📂")
                    .on_hover_text(format!(
                        "Rutas completas:\n📁 Origen: {}\n📁 Destino: {}",
                        pair.source.display(),
                        pair.destination.display()
                    ));

                // BOTONES DE ACCIÓN - Solo delete para backup pairs deshabilitados
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Solo delete button para deshabilitados
                    if ui.small_button("🗑").clicked() {
                        info!("🗑️ UI: Abriendo confirmación para eliminar backup pair deshabilitado #{}", original_index + 1);
                        self.show_delete_confirmation = true;
                        self.delete_pair_index = Some(original_index);
                    }
                });
            });
        });

        ui.add_space(5.0);
    }

    /// Modal de confirmación para eliminar backup pairs con validaciones de seguridad
    fn render_delete_confirmation_modal(
        &mut self,
        ui: &mut egui::Ui,
        config: &Arc<Mutex<AppConfig>>,
        action_callback: &mut dyn FnMut(UIAction)
    ) {
        if let Some(delete_index) = self.delete_pair_index {
            // Obtener información del backup pair a eliminar
            let backup_pair_info = if let Ok(cfg) = config.lock() {
                if let Some(pair) = cfg.backup_pairs.get(delete_index) {
                    Some((
                        pair.source.display().to_string(),
                        pair.destination.display().to_string(),
                        pair.source.clone(),
                        pair.destination.clone()
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((source_str, dest_str, source_path, dest_path)) = backup_pair_info {
                // Detectar rutas críticas del sistema
                let is_critical_source = self.is_critical_system_path(&source_path);
                let is_critical_dest = self.is_critical_system_path(&dest_path);
                let has_critical_paths = is_critical_source || is_critical_dest;

                egui::Window::new("⚠ Confirmar Eliminación")
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .collapsible(false)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        ui.vertical(|ui| {
                            ui.set_min_width(500.0);
                            ui.add_space(10.0);

                            // Título de advertencia
                            ui.horizontal(|ui| {
                                ui.label("⚠");
                                ui.heading("¿Estás seguro de eliminar este backup?");
                            });

                            ui.add_space(15.0);

                            // Información del backup pair
                            ui.group(|ui| {
                                ui.label(format!("📂 Backup #{}", delete_index + 1));
                                ui.add_space(5.0);

                                ui.horizontal(|ui| {
                                    ui.label("Origen:");
                                    ui.monospace(&source_str);
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Destino:");
                                    ui.monospace(&dest_str);
                                });
                            });

                            ui.add_space(10.0);

                            // Advertencias de rutas críticas
                            if has_critical_paths {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("🔥");
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 100, 100),
                                            "ADVERTENCIA: Ruta crítica del sistema detectada"
                                        );
                                    });

                                    if is_critical_source {
                                        ui.label("• El directorio origen contiene archivos críticos del sistema");
                                    }
                                    if is_critical_dest {
                                        ui.label("• El directorio destino está en una ubicación crítica del sistema");
                                    }

                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 140, 0),
                                        "Eliminar este backup podría afectar la protección de archivos importantes."
                                    );
                                });
                                ui.add_space(10.0);
                            }

                            // Información adicional
                            ui.group(|ui| {
                                ui.label("ℹ Esta acción:");
                                ui.label("• Eliminará permanentemente la configuración de backup");
                                ui.label("• NO eliminará los archivos respaldados existentes");
                                ui.label("• NO se puede deshacer");
                            });

                            ui.add_space(20.0);

                            // Botones de acción
                            ui.horizontal(|ui| {
                                if ui.button("❌ Cancelar").clicked() {
                                    self.show_delete_confirmation = false;
                                    self.delete_pair_index = None;
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // Botón de eliminar con color de advertencia
                                    let delete_button = if has_critical_paths {
                                        egui::Button::new("🔥 Eliminar (Ruta Crítica)")
                                            .fill(egui::Color32::from_rgb(200, 50, 50))
                                    } else {
                                        egui::Button::new("🗑 Sí, Eliminar")
                                            .fill(egui::Color32::from_rgb(150, 50, 50))
                                    };

                                    if ui.add(delete_button).clicked() {
                                        info!("🗑️ UI: Confirmada eliminación de backup pair #{}", delete_index + 1);
                                        action_callback(UIAction::RemoveBackupPair(delete_index));
                                        self.show_delete_confirmation = false;
                                        self.delete_pair_index = None;
                                    }
                                });
                            });

                            ui.add_space(10.0);
                        });
                    });
            } else {
                // Si no se puede obtener la información, cerrar el modal
                self.show_delete_confirmation = false;
                self.delete_pair_index = None;
            }
        }
    }

    /// Detectar si una ruta es crítica del sistema
    fn is_critical_system_path(&self, path: &std::path::Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        // Rutas críticas de Windows
        let critical_paths = [
            "c:\\windows",
            "c:\\program files",
            "c:\\program files (x86)",
            "c:\\programdata",
            "c:\\system volume information",
            "c:\\$recycle.bin",
            "c:\\recovery",
            "c:\\boot",
            "c:\\efi",
        ];

        // Verificar si la ruta comienza con alguna ruta crítica
        for critical_path in &critical_paths {
            if path_str.starts_with(critical_path) {
                return true;
            }
        }

        // Verificar rutas de usuario críticas
        if let Some(user_profile) = std::env::var("USERPROFILE").ok() {
            let user_profile = user_profile.to_lowercase();
            let critical_user_paths = [
                format!("{}\\appdata", user_profile),
                format!("{}\\ntuser.dat", user_profile),
            ];

            for critical_path in &critical_user_paths {
                if path_str.starts_with(critical_path) {
                    return true;
                }
            }
        }

        false
    }

    /// Renderizar icono de estado de validación
    fn render_validation_icon(&self, ui: &mut egui::Ui, result: &crate::core::PathValidationResult) {
        match result {
            crate::core::PathValidationResult::Valid => {
                ui.colored_label(egui::Color32::from_rgb(0, 150, 0), SafeIcons::SUCCESS);
            }
            crate::core::PathValidationResult::Warning(msg) => {
                ui.colored_label(egui::Color32::from_rgb(255, 140, 0), SafeIcons::WARNING)
                    .on_hover_text(msg);
            }
            crate::core::PathValidationResult::Error(msg) => {
                ui.colored_label(egui::Color32::from_rgb(255, 80, 80), SafeIcons::ERROR)
                    .on_hover_text(msg);
            }
        }
    }

    /// Renderizar panel completo de validación
    fn render_validation_panel(&self, ui: &mut egui::Ui, validation: &crate::core::BackupPairValidation) {
        // Solo mostrar si hay errores o advertencias
        let errors = validation.get_error_messages();
        let warnings = validation.get_warning_messages();

        if errors.is_empty() && warnings.is_empty() {
            // Todo válido - mostrar mensaje de éxito
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(0, 150, 0), SafeIcons::SUCCESS);
                    ui.label("Configuración válida");
                });
            });
            return;
        }

        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 20.0);

            // Mostrar errores
            if !errors.is_empty() {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(255, 80, 80), SafeIcons::ERROR);
                    ui.strong("Errores que deben corregirse:");
                });

                for error in &errors {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "•");
                        ui.label(error);
                    });
                }

                if !warnings.is_empty() {
                    ui.add_space(8.0);
                }
            }

            // Mostrar advertencias
            if !warnings.is_empty() {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(255, 140, 0), SafeIcons::WARNING);
                    ui.strong("Advertencias:");
                });

                for warning in &warnings {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.colored_label(egui::Color32::from_rgb(255, 140, 0), "•");
                        ui.label(warning);
                    });
                }
            }
        });
    }

    // === FUNCIONES HELPER PARA ADVANCED MANAGEMENT ===

    /// Toggle enable/disable de un backup pair
    fn toggle_backup_pair_enabled(&mut self, index: usize, enabled: bool, action_callback: &mut dyn FnMut(UIAction)) {
        info!("🔄 UI: Toggling backup pair #{} to {}", index + 1, if enabled { "enabled" } else { "disabled" });
        action_callback(UIAction::ToggleBackupPairEnabled(index, enabled));
    }

    /// Obtener prioridad de un backup pair activo
    fn get_active_priority(&self, index: usize, _existing_pairs: &[crate::core::config::BackupPair]) -> usize {
        let mut active_count = 0;
        for (i, pair) in _existing_pairs.iter().enumerate() {
            if pair.enabled {
                active_count += 1;
                if i == index {
                    return active_count;
                }
            }
        }
        0
    }

    /// Contar backup pairs activos
    fn count_active_pairs(&self, _existing_pairs: &[crate::core::config::BackupPair]) -> usize {
        _existing_pairs.iter().filter(|pair| pair.enabled).count()
    }

    /// Obtener índice dentro de los backup pairs activos
    fn get_active_index(&self, index: usize, _existing_pairs: &[crate::core::config::BackupPair]) -> usize {
        let mut active_index = 0;
        for (i, pair) in _existing_pairs.iter().enumerate() {
            if i == index {
                return active_index;
            }
            if pair.enabled {
                active_index += 1;
            }
        }
        0
    }
}

// === ESTRUCTURAS DE DATOS PARA ADVANCED MANAGEMENT ===

/// Estado de drag & drop para reordenamiento
#[derive(Debug, Clone)]
pub struct DragState {
    pub dragged_index: usize,
    pub drag_start_pos: egui::Pos2,
    pub current_pos: egui::Pos2,
    pub dragged_id: String,
}

/// Tipos de operaciones bulk disponibles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BulkOperationType {
    Enable,
    Disable,
    Delete,
}

impl BulkOperationType {
    pub fn display_name(&self) -> &'static str {
        match self {
            BulkOperationType::Enable => "Habilitar",
            BulkOperationType::Disable => "Deshabilitar",
            BulkOperationType::Delete => "Eliminar",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BulkOperationType::Enable => SafeIcons::SUCCESS,
            BulkOperationType::Disable => "⏸",
            BulkOperationType::Delete => SafeIcons::DELETE,
        }
    }
}