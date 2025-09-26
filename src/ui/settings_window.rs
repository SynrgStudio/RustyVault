#![allow(dead_code)]
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;
use crate::core::{AppConfig, RobocopyConfig};

/// Actions that the settings window can trigger
#[derive(Debug, Clone)]
pub enum SettingsAction {
    // Daemon Control
    StartDaemon,
    StopDaemon,
    
    // Configuration Changes
    UpdateInterval(u64),
    UpdateRobocopyConfig(RobocopyConfig),
    UpdateAutoStart(bool),
    UpdateNotificationEnabled(bool),
    UpdateTheme(AppTheme),
    
    // Import/Export
    ExportConfig,
    ImportConfig(String),
    
    // Window Control
    CloseSettings,
    ApplyAndSave,
}

/// Available UI themes
#[derive(Debug, Clone, PartialEq)]
pub enum AppTheme {
    Auto,
    Light,
    Dark,
}

impl Default for AppTheme {
    fn default() -> Self {
        AppTheme::Auto
    }
}

/// Dedicated settings window with tabbed interface
pub struct SettingsWindow {
    /// Current active tab
    active_tab: SettingsTab,
    
    /// Temporary buffers for editing
    temp_interval_buffer: String,
    temp_robocopy_threads: String,
    temp_robocopy_retries: String,
    temp_robocopy_wait: String,
    
    /// UI state
    show_advanced_robocopy: bool,
    
    /// Configuration backup (for Cancel functionality)
    original_config: Option<AppConfig>,
    
    /// Whether changes have been made
    has_unsaved_changes: bool,
}

/// Available tabs in settings window
#[derive(Debug, Clone, PartialEq)]
enum SettingsTab {
    Daemon,
    Robocopy,
    Interface,
    General,
}

impl Default for SettingsWindow {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::Daemon,
            temp_interval_buffer: String::new(),
            temp_robocopy_threads: "8".to_string(),
            temp_robocopy_retries: "3".to_string(),
            temp_robocopy_wait: "2".to_string(),
            show_advanced_robocopy: false,
            original_config: None,
            has_unsaved_changes: false,
        }
    }
}

impl SettingsWindow {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Initialize settings window with current configuration
    pub fn initialize_from_config(&mut self, config: &AppConfig) {
        self.temp_interval_buffer = config.check_interval_seconds.to_string();
        
        self.temp_robocopy_threads = config.robocopy.multithreading.to_string();
        self.temp_robocopy_retries = config.robocopy.retry_count.to_string();
        self.temp_robocopy_wait = config.robocopy.retry_wait.to_string();
        
        self.original_config = Some(config.clone());
        self.has_unsaved_changes = false;
    }
    
    /// Main render function for settings window
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        config: &Arc<Mutex<AppConfig>>,
        daemon_running: &Arc<AtomicBool>,
    ) -> (bool, Vec<SettingsAction>) {
        let mut keep_open = true;
        let mut actions = Vec::new();
        
        egui::Window::new("⚙ Settings")
            .default_size([600.0, 500.0])
            .min_size([500.0, 400.0])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                // Header with unsaved changes indicator
                if self.has_unsaved_changes {
                    ui.horizontal(|ui| {
                        ui.label("⚠️ You have unsaved changes");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("💾 Apply & Save").clicked() {
                                actions.push(SettingsAction::ApplyAndSave);
                                self.has_unsaved_changes = false;
                            }
                        });
                    });
                    ui.separator();
                }
                
                // Tab navigation
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Daemon, "🔥 Daemon");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Robocopy, "🔧 Robocopy");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Interface, "🎨 Interface");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::General, "⚙ General");
                });
                
                ui.separator();
                
                // Tab content
                match self.active_tab {
                    SettingsTab::Daemon => self.render_daemon_tab(ui, config, daemon_running, &mut actions),
                    SettingsTab::Robocopy => self.render_robocopy_tab(ui, config, &mut actions),
                    SettingsTab::Interface => self.render_interface_tab(ui, config, &mut actions),
                    SettingsTab::General => self.render_general_tab(ui, config, &mut actions),
                }
                
                ui.separator();
                
                // Footer buttons
                ui.horizontal(|ui| {
                    if ui.button("❌ Cancel").clicked() {
                        // TODO: Restore original config
                        keep_open = false;
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✅ OK").clicked() {
                            if self.has_unsaved_changes {
                                actions.push(SettingsAction::ApplyAndSave);
                            }
                            keep_open = false;
                        }
                        
                        if ui.button("📤 Export Config").clicked() {
                            actions.push(SettingsAction::ExportConfig);
                        }
                        
                        if ui.button("📥 Import Config").clicked() {
                            // TODO: File dialog for import
                            info!("Import config clicked - TODO: implement file dialog");
                        }
                    });
                });
            });
        
        (keep_open, actions)
    }
    
    /// Render daemon control tab
    fn render_daemon_tab(
        &mut self,
        ui: &mut egui::Ui,
        _config: &Arc<Mutex<AppConfig>>,
        daemon_running: &Arc<AtomicBool>,
        actions: &mut Vec<SettingsAction>,
    ) {
        ui.heading("⚙ Daemon Control");
        ui.add_space(10.0);
        
        // Current daemon status
        let is_running = daemon_running.load(Ordering::Relaxed);
        ui.horizontal(|ui| {
            ui.label("Status:");
            if is_running {
                ui.colored_label(egui::Color32::GREEN, "▶ Running");
            } else {
                ui.colored_label(egui::Color32::GRAY, "⏸ Stopped");
            }
        });
        
        ui.add_space(10.0);
        
        // Daemon controls
        ui.horizontal(|ui| {
            if is_running {
                if ui.button("⏹ Stop Daemon").clicked() {
                    actions.push(SettingsAction::StopDaemon);
                }
            } else {
                if ui.button("▶ Start Daemon").clicked() {
                    actions.push(SettingsAction::StartDaemon);
                }
            }
        });
        
        ui.add_space(20.0);
        
        // Interval configuration
        ui.horizontal(|ui| {
            ui.label("Check Interval:");
            if ui.text_edit_singleline(&mut self.temp_interval_buffer).changed() {
                self.has_unsaved_changes = true;
            }
            ui.label("seconds");
        });
        
        // Quick interval presets
        ui.horizontal(|ui| {
            ui.label("Quick set:");
            if ui.small_button("1 min").clicked() {
                self.temp_interval_buffer = "60".to_string();
                self.has_unsaved_changes = true;
            }
            if ui.small_button("5 min").clicked() {
                self.temp_interval_buffer = "300".to_string();
                self.has_unsaved_changes = true;
            }
            if ui.small_button("1 hour").clicked() {
                self.temp_interval_buffer = "3600".to_string();
                self.has_unsaved_changes = true;
            }
            if ui.small_button("6 hours").clicked() {
                self.temp_interval_buffer = "21600".to_string();
                self.has_unsaved_changes = true;
            }
        });
        
        ui.add_space(20.0);
        
        // Auto-start options
        ui.checkbox(&mut true, "Auto-start daemon when application starts")
            .on_hover_text("Automatically start the backup daemon when RustyVault launches");
            
        ui.checkbox(&mut false, "Start with Windows")
            .on_hover_text("Add RustyVault to Windows startup programs");
    }
    
    /// Render robocopy configuration tab
    fn render_robocopy_tab(
        &mut self,
        ui: &mut egui::Ui,
        _config: &Arc<Mutex<AppConfig>>,
        _actions: &mut Vec<SettingsAction>,
    ) {
        ui.heading("🔧 Robocopy Configuration");
        ui.add_space(10.0);
        
        // Multi-threading
        ui.horizontal(|ui| {
            ui.label("Threads:");
            if ui.text_edit_singleline(&mut self.temp_robocopy_threads).changed() {
                self.has_unsaved_changes = true;
            }
            ui.label("(1-128, recommended: 8)")
                .on_hover_text("Number of parallel threads for file copying. More threads = faster but more CPU usage.");
        });
        
        // Retry settings
        ui.horizontal(|ui| {
            ui.label("Retries:");
            if ui.text_edit_singleline(&mut self.temp_robocopy_retries).changed() {
                self.has_unsaved_changes = true;
            }
            ui.label("attempts");
        });
        
        ui.horizontal(|ui| {
            ui.label("Wait time:");
            if ui.text_edit_singleline(&mut self.temp_robocopy_wait).changed() {
                self.has_unsaved_changes = true;
            }
            ui.label("seconds between retries");
        });
        
        ui.add_space(10.0);
        
        // Standard options
        ui.label("Standard Options:");
        ui.checkbox(&mut true, "Mirror mode (/MIR)")
            .on_hover_text("Mirrors source to destination (deletes extra files in destination)");
            
        ui.checkbox(&mut true, "Use file timestamps (/FFT)")
            .on_hover_text("Assume FAT file times (2-second granularity)");
            
        ui.checkbox(&mut false, "Copy subdirectories including empty ones (/E)")
            .on_hover_text("Copies subdirectories, including empty ones");
        
        ui.add_space(10.0);
        
        // Advanced options toggle
        ui.checkbox(&mut self.show_advanced_robocopy, "Show advanced options");
        
        if self.show_advanced_robocopy {
            ui.separator();
            ui.label("Advanced Options:");
            
            ui.checkbox(&mut false, "Verbose output (/V)")
                .on_hover_text("Show detailed information about copied files");
                
            ui.checkbox(&mut true, "No progress indicator (/NP)")
                .on_hover_text("Don't display percentage progress for individual files");
                
            ui.checkbox(&mut false, "Log output (/LOG)")
                .on_hover_text("Write status output to log file");
        }
    }
    
    /// Render interface/UI tab
    fn render_interface_tab(
        &mut self,
        ui: &mut egui::Ui,
        _config: &Arc<Mutex<AppConfig>>,
        _actions: &mut Vec<SettingsAction>,
    ) {
        ui.heading("🎨 Interface Settings");
        ui.add_space(10.0);
        
        // Theme selection
        ui.horizontal(|ui| {
            ui.label("Theme:");
            egui::ComboBox::from_label("")
                .selected_text("Auto (System)")
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut AppTheme::Auto, AppTheme::Auto, "Auto (System)");
                    ui.selectable_value(&mut AppTheme::Light, AppTheme::Light, "Light");
                    ui.selectable_value(&mut AppTheme::Dark, AppTheme::Dark, "Dark");
                });
        });
        
        ui.add_space(10.0);
        
        // Notification settings
        ui.label("Notifications:");
        ui.checkbox(&mut true, "Show backup completion notifications");
        ui.checkbox(&mut true, "Show error notifications");
        ui.checkbox(&mut false, "Show daemon start/stop notifications");
        
        ui.add_space(10.0);
        
        // Window behavior
        ui.label("Window Behavior:");
        ui.checkbox(&mut true, "Minimize to system tray");
        ui.checkbox(&mut false, "Start minimized");
        ui.checkbox(&mut true, "Close to tray (don't exit)");
    }
    
    /// Render general/misc settings tab
    fn render_general_tab(
        &mut self,
        ui: &mut egui::Ui,
        _config: &Arc<Mutex<AppConfig>>,
        _actions: &mut Vec<SettingsAction>,
    ) {
        ui.heading("⚙ General Settings");
        ui.add_space(10.0);
        
        // Logging
        ui.label("Logging:");
        ui.horizontal(|ui| {
            ui.label("Log level:");
            egui::ComboBox::from_label("")
                .selected_text("Info")
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut "debug", "debug", "Debug");
                    ui.selectable_value(&mut "info", "info", "Info");
                    ui.selectable_value(&mut "warn", "warn", "Warning");
                    ui.selectable_value(&mut "error", "error", "Error");
                });
        });
        
        ui.checkbox(&mut true, "Enable file logging");
        ui.checkbox(&mut false, "Enable console logging");
        
        ui.add_space(10.0);
        
        // Performance
        ui.label("Performance:");
        ui.checkbox(&mut true, "Enable multi-threaded backup processing");
        ui.checkbox(&mut false, "Priority boost for backup operations");
        
        ui.add_space(10.0);
        
        // Safety
        ui.label("Safety & Confirmations:");
        ui.checkbox(&mut true, "Confirm before deleting backup pairs");
        ui.checkbox(&mut true, "Warn about potentially dangerous paths");
        ui.checkbox(&mut false, "Enable backup verification");
        
        ui.add_space(20.0);
        
        // About section
        ui.separator();
        ui.label("About RustyVault:");
        ui.label("Version: 2.0");
        ui.label("Developer: Damian Naone");
        ui.label("Built with: Rust + egui + robocopy");
    }
}
