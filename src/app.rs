use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, error, warn};

use crate::core::AppConfig;
use crate::core::daemon::BackupDaemon;
use crate::system::tray::SystemTray;
use crate::ui::main_window::{MainWindow, UIAction};
use crate::ui::settings_window::{SettingsWindow, SettingsAction};

/// Estado de ejecución de un backup pair individual
#[derive(Debug, Clone)]
pub enum BackupStatus {
    Pending,    // No ejecutado aún
    Running,    // En ejecución 
    Success(BackupMetrics),    // Completado exitosamente con métricas
    Warning(String), // Completado con advertencias
    Error(String),   // Falló con error
}

/// Métricas de una ejecución de backup
#[derive(Debug, Clone)]
pub struct BackupMetrics {
    pub files_copied: u32,
    pub bytes_transferred: u64,
}

/// Estado y metadata de un backup pair
#[derive(Debug, Clone)]
pub struct BackupPairStatus {
    pub backup_pair_id: String,
    pub status: BackupStatus,
    pub last_execution: Option<u64>, // Unix timestamp
    pub execution_count: u32,
    pub success_count: u32,           // Contador de ejecuciones exitosas
    pub files_copied_last: Option<u32>, // Archivos copiados en última ejecución
    pub total_size_transferred: Option<u64>, // Bytes transferidos en última ejecución
}

impl BackupPairStatus {
    pub fn new(backup_pair_id: String) -> Self {
        Self {
            backup_pair_id,
            status: BackupStatus::Pending,
            last_execution: None,
            execution_count: 0,
            success_count: 0,
            files_copied_last: None,
            total_size_transferred: None,
        }
    }
    
    pub fn update_execution(&mut self, status: BackupStatus) {
        self.status = status.clone();
        self.execution_count += 1;
        
        // Incrementar success_count y actualizar métricas
        match status {
            BackupStatus::Success(metrics) => {
                self.success_count += 1;
                self.files_copied_last = Some(metrics.files_copied);
                self.total_size_transferred = Some(metrics.bytes_transferred);
            }
            BackupStatus::Warning(_) => {
                self.success_count += 1;
                // Mantener datos anteriores si existen
            }
            _ => {
                self.files_copied_last = Some(0);
            }
        }
        
        self.last_execution = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }
    
    /// Calcular porcentaje de éxito
    pub fn success_rate(&self) -> u32 {
        if self.execution_count == 0 {
            0
        } else {
            (self.success_count * 100) / self.execution_count
        }
    }
    
    /// Obtener timestamp formateado para UI
    pub fn format_last_execution(&self) -> String {
        if let Some(timestamp) = self.last_execution {
            if let Some(datetime) = std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(timestamp)) {
                if let Ok(local_time) = std::time::SystemTime::now().duration_since(datetime) {
                    let seconds_ago = local_time.as_secs();
                    
                    if seconds_ago < 60 {
                        format!("{}s", seconds_ago)
                    } else if seconds_ago < 3600 {
                        format!("{}m", seconds_ago / 60)
                    } else if seconds_ago < 86400 {
                        format!("{}h", seconds_ago / 3600)
                    } else {
                        format!("{}d", seconds_ago / 86400)
                    }
                } else {
                    "ahora".to_string()
                }
            } else {
                "error".to_string()
            }
        } else {
            "nunca".to_string()
        }
    }
}

/// Comandos que puede recibir el hilo de fondo
#[derive(Debug, Clone)]
pub enum BackgroundCommand {
    ShowWindow,
    HideWindow,
    StartDaemon,
    StopDaemon,
    RunBackupNow,
    UpdateConfig(AppConfig),
    
    // === BACKUP PAIR MANAGEMENT ===
    AddBackupPair { source: String, destination: String },
    UpdateBackupPair { index: usize, source: String, destination: String },
    RemoveBackupPair(usize),
    MoveBackupPairUp(usize),
    MoveBackupPairDown(usize),
    ToggleBackupPairEnabled(usize, bool),
    
    // === BACKUP STATUS TRACKING ===
    UpdateBackupStatus { backup_pair_id: String, status: BackupStatus },
    
    Exit,
}

/// Estado global de la aplicación (independiente de egui)
#[derive(Debug, Clone)]
pub struct AppState {
    pub window_visible: bool,
    pub daemon_running: bool,
    pub should_exit: bool,
    
    /// Estado de cada backup pair (key = backup_pair_id)
    pub backup_statuses: HashMap<String, BackupPairStatus>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            window_visible: true,
            daemon_running: false,
            should_exit: false,
            backup_statuses: HashMap::new(),
        }
    }
}

/// Hilo de fondo que maneja el estado de la aplicación
pub struct BackgroundManager {
    state: Arc<Mutex<AppState>>,
    command_receiver: Receiver<BackgroundCommand>,
    daemon: BackupDaemon,
    daemon_running: Arc<AtomicBool>,
    config: Arc<Mutex<AppConfig>>, // Config compartido con la UI
}

impl BackgroundManager {
    fn new(command_receiver: Receiver<BackgroundCommand>, config: Arc<Mutex<AppConfig>>) -> Self {
        let daemon = BackupDaemon::new(Arc::clone(&config));
        let daemon_running = daemon.get_running_flag();
        
        let mut manager = Self {
            state: Arc::new(Mutex::new(AppState::default())),
            command_receiver,
            daemon,
            daemon_running,
            config, // Guardar referencia al config compartido
        };
        
        // Inicializar estados de backup pairs
        manager.initialize_backup_statuses();
        
        manager
    }
    
    fn run(mut self, egui_ctx: egui::Context) {
        info!("Background manager iniciado");
        
        while let Ok(command) = self.command_receiver.recv() {
            match command {
                BackgroundCommand::ShowWindow => {
                    if let Ok(mut state) = self.state.lock() {
                        state.window_visible = true;
                        
                        // Secuencia de comandos para restaurar ventana
                        egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        egui_ctx.request_repaint();
                        
                        let ctx_1 = egui_ctx.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(50));
                            ctx_1.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                            ctx_1.request_repaint();
                        });
                        
                        let ctx_2 = egui_ctx.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            ctx_2.send_viewport_cmd(egui::ViewportCommand::Focus);
                            ctx_2.request_repaint();
                            
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            ctx_2.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                            ctx_2.send_viewport_cmd(egui::ViewportCommand::Focus);
                            ctx_2.request_repaint();
                        });
                    }
                }
                BackgroundCommand::HideWindow => {
                    if let Ok(mut state) = self.state.lock() {
                        state.window_visible = false;
                        egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        egui_ctx.request_repaint();
                    }
                }
                BackgroundCommand::StartDaemon => {
                    self.start_daemon();
                    if let Ok(mut state) = self.state.lock() {
                        state.daemon_running = true;
                    }
                }
                BackgroundCommand::StopDaemon => {
                    self.stop_daemon();
                    if let Ok(mut state) = self.state.lock() {
                        state.daemon_running = false;
                    }
                }
                BackgroundCommand::RunBackupNow => {
                    info!("🔄 Ejecutando backup manual desde UI");
                    self.run_manual_backup();
                }
                BackgroundCommand::UpdateConfig(new_config) => {
                    info!("⚙️ Actualizando configuración desde UI");
                    self.update_config(new_config);
                }
                
                // === BACKUP PAIR MANAGEMENT ===
                BackgroundCommand::AddBackupPair { source, destination } => {
                    info!("➕ Agregando backup pair: {} → {}", source, destination);
                    self.add_backup_pair(source, destination);
                }
                BackgroundCommand::UpdateBackupPair { index, source, destination } => {
                    info!("✏️ Actualizando backup pair #{}: {} → {}", index + 1, source, destination);
                    self.update_backup_pair(index, source, destination);
                }
                BackgroundCommand::RemoveBackupPair(index) => {
                    info!("🗑️ Eliminando backup pair #{}", index + 1);
                    self.remove_backup_pair(index);
                }
                BackgroundCommand::MoveBackupPairUp(index) => {
                    info!("⬆️ Moviendo backup pair #{} hacia arriba", index + 1);
                    self.move_backup_pair_up(index);
                }
                BackgroundCommand::MoveBackupPairDown(index) => {
                    info!("⬇️ Moviendo backup pair #{} hacia abajo", index + 1);
                    self.move_backup_pair_down(index);
                }
                BackgroundCommand::ToggleBackupPairEnabled(index, enabled) => {
                    info!("🔄 Toggling backup pair #{} to {}", index + 1, if enabled { "enabled" } else { "disabled" });
                    self.toggle_backup_pair_enabled(index, enabled);
                }
                
                BackgroundCommand::UpdateBackupStatus { backup_pair_id, status } => {
                    self.update_backup_status(backup_pair_id, status);
                }
                
                BackgroundCommand::Exit => {
                    info!("❌ Background: Exit requested");
                    if let Ok(mut state) = self.state.lock() {
                        state.should_exit = true;
                    }
                    
                    // Detener daemon antes de salir
                    if self.daemon_running.load(Ordering::Relaxed) {
                        self.stop_daemon();
                    }
                    
                    // Limpiar sender global para evitar más comandos
                    unsafe {
                        BACKGROUND_SENDER = None;
                    }
                    
                    info!("🚪 Cerrando aplicación completamente");
                    
                    // Múltiples comandos de cierre para asegurar que funcione
                    egui_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    
                    // Forzar exit de forma más agresiva
                    let ctx_clone = egui_ctx.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        std::process::exit(0); // Exit forzado si no funciona viewport close
                    });
                    
                    break;
                }
            }
        }
        
        info!("Background manager terminado");
    }
    
    fn start_daemon(&mut self) {
        if !self.daemon_running.load(Ordering::Relaxed) {
            if let Err(e) = self.daemon.start() {
                error!("❌ Error starting daemon: {}", e);
            }
        }
    }
    
    fn stop_daemon(&mut self) {
        if self.daemon_running.load(Ordering::Relaxed) {
            info!("🛑 Deteniendo daemon desde background manager");
            match self.daemon.stop() {
                Ok(_) => {
                    self.daemon_running.store(false, Ordering::Relaxed);
                    info!("✅ Daemon detenido exitosamente");
                }
                Err(e) => {
                    error!("❌ Error deteniendo daemon: {}", e);
                }
            }
        } else {
            info!("⚠️ Daemon ya está detenido");
        }
    }
    
    fn run_manual_backup(&self) {
        // Ejecutar backup inmediato usando la configuración actual
        let config = match self.daemon.get_config() {
            Ok(config) => config,
            Err(e) => {
                error!("❌ Error obteniendo configuración para backup manual: {}", e);
                return;
            }
        };
        
        // Clonar sender para usar en el thread de backup
        let sender = unsafe { BACKGROUND_SENDER.as_ref() }.cloned();
        
        // Ejecutar backup en thread separado para no bloquear background manager
        std::thread::spawn(move || {
            use crate::core::backup::execute_backup;
            
            let backup_pairs = &config.backup_pairs;
            
            if backup_pairs.is_empty() {
                warn!("⚠️ No hay backup pairs configurados");
                if let Err(e) = crate::system::notifications::show_backup_warning("No hay directorios configurados para backup") {
                    warn!("⚠️ Error mostrando notificación: {}", e);
                }
                return;
            }
            
            info!("🚀 Backup manual iniciado - {} pair(s) a procesar", backup_pairs.len());
            
            let mut total_success = 0;
            let mut total_warnings = 0;
            let mut total_failures = 0;
            
            // Ejecutar backups secuencialmente (daisy-chain)
            for (i, pair) in backup_pairs.iter().enumerate() {
                if !pair.enabled {
                    info!("⏭️ Backup pair #{} deshabilitado - omitiendo", i + 1);
                    continue;
                }
                
                info!("🔄 Procesando backup pair #{}: {} → {}", 
                     i + 1, pair.source.display(), pair.destination.display());
                
                // Marcar como "Running" antes de comenzar
                if let Some(ref sender) = sender {
                    if let Err(e) = sender.send(BackgroundCommand::UpdateBackupStatus {
                        backup_pair_id: pair.id.clone(),
                        status: BackupStatus::Running,
                    }) {
                        warn!("⚠️ Error enviando estado Running: {}", e);
                    }
                }
                
                match execute_backup(&pair.source, &pair.destination, &config.robocopy) {
                    Ok(result) => {
                        match result {
                            crate::core::backup::BackupResult::Success { files_copied, bytes_transferred } => {
                                info!("✅ Backup pair #{} completado exitosamente - {} archivos, {} bytes", i + 1, files_copied, bytes_transferred);
                                total_success += 1;
                                
                                // Actualizar estado a Success con métricas reales
                                if let Some(ref sender) = sender {
                                    if let Err(e) = sender.send(BackgroundCommand::UpdateBackupStatus {
                                        backup_pair_id: pair.id.clone(),
                                        status: BackupStatus::Success(BackupMetrics {
                                            files_copied,
                                            bytes_transferred,
                                        }),
                                    }) {
                                        warn!("⚠️ Error enviando estado Success: {}", e);
                                    }
                                }
                            }
                            crate::core::backup::BackupResult::Warning(msg) => {
                                warn!("⚠️ Backup pair #{} completado con advertencias: {}", i + 1, msg);
                                total_warnings += 1;
                                
                                // Actualizar estado a Warning
                                if let Some(ref sender) = sender {
                                    if let Err(e) = sender.send(BackgroundCommand::UpdateBackupStatus {
                                        backup_pair_id: pair.id.clone(),
                                        status: BackupStatus::Warning(msg.clone()),
                                    }) {
                                        warn!("⚠️ Error enviando estado Warning: {}", e);
                                    }
                                }
                            }
                            crate::core::backup::BackupResult::Failed => {
                                error!("❌ Backup pair #{} falló", i + 1);
                                total_failures += 1;
                                
                                // Actualizar estado a Error
                                if let Some(ref sender) = sender {
                                    if let Err(e) = sender.send(BackgroundCommand::UpdateBackupStatus {
                                        backup_pair_id: pair.id.clone(),
                                        status: BackupStatus::Error("Backup falló".to_string()),
                                    }) {
                                        warn!("⚠️ Error enviando estado Error: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ Error crítico en backup pair #{}: {}", i + 1, e);
                        total_failures += 1;
                        
                        // Actualizar estado a Error con mensaje específico
                        if let Some(ref sender) = sender {
                            if let Err(send_err) = sender.send(BackgroundCommand::UpdateBackupStatus {
                                backup_pair_id: pair.id.clone(),
                                status: BackupStatus::Error(format!("Error crítico: {}", e)),
                            }) {
                                warn!("⚠️ Error enviando estado Error: {}", send_err);
                            }
                        }
                    }
                }
            }
            
            // Notificación final consolidada
            if total_failures > 0 {
                let msg = format!("{} exitosos, {} con advertencias, {} fallidos", 
                                 total_success, total_warnings, total_failures);
                if let Err(e) = crate::system::notifications::show_backup_failed(&msg) {
                    warn!("⚠️ Error mostrando notificación: {}", e);
                }
            } else if total_warnings > 0 {
                let msg = format!("{} exitosos, {} con advertencias", total_success, total_warnings);
                if let Err(e) = crate::system::notifications::show_backup_warning(&msg) {
                    warn!("⚠️ Error mostrando notificación: {}", e);
                }
                         } else {
                 info!("🎉 Todos los {} backups completados exitosamente", total_success);
                 if let Err(e) = crate::system::notifications::show_backup_success(Some(total_success as u32), None) {
                     warn!("⚠️ Error mostrando notificación: {}", e);
                 }
             }
            
            info!("🏁 Backup manual finalizado: {} éxito, {} advertencias, {} fallos", 
                 total_success, total_warnings, total_failures);
        });
    }
    
    fn update_config(&mut self, new_config: AppConfig) {
        // Actualizar configuración compartida PRIMERO
        if let Ok(mut config) = self.config.lock() {
            *config = new_config.clone();
        }
        
        // Guardar la nueva configuración a disco
        if let Err(e) = new_config.save() {
            error!("❌ Error guardando configuración: {}", e);
            return;
        }
        
        info!("💾 Configuración guardada exitosamente");
        
        // Reinicializar estados de backup pairs
        self.initialize_backup_statuses();
        
        // Si el daemon está corriendo, reiniciarlo con la nueva configuración
        let was_running = self.daemon_running.load(Ordering::Relaxed);
        
        if was_running {
            info!("🔄 Reiniciando daemon con nueva configuración");
            self.stop_daemon();
            
            // Actualizar la configuración del daemon con config compartido
            self.daemon = BackupDaemon::new(Arc::clone(&self.config));
            
            // Reiniciar el daemon
            self.start_daemon();
            
            info!("✅ Daemon reiniciado con nueva configuración");
        } else {
            // Solo actualizar la configuración del daemon
            self.daemon = BackupDaemon::new(Arc::clone(&self.config));
            info!("✅ Configuración del daemon actualizada");
        }
    }
    
    // === BACKUP PAIR MANAGEMENT METHODS ===
    
    fn add_backup_pair(&mut self, source: String, destination: String) {
        use crate::core::config::BackupPair;
        
        // Crear nuevo backup pair
        let new_pair = BackupPair::new(source, destination);
        
        // Actualizar config compartido
        if let Ok(mut config) = self.config.lock() {
            config.backup_pairs.push(new_pair);
            
            // Guardar a disco
            if let Err(e) = config.save() {
                error!("❌ Error guardando backup pair: {}", e);
                return;
            }
            
            // Actualizar daemon con config actualizado
            self.daemon = BackupDaemon::new(Arc::clone(&self.config));
            info!("✅ Backup pair agregado exitosamente");
        } else {
            error!("❌ Error accediendo configuración compartida para agregar backup pair");
            return;
        }
        
        // Reinicializar estados DESPUÉS de liberar lock
        self.initialize_backup_statuses();
    }
    
    fn update_backup_pair(&mut self, index: usize, source: String, destination: String) {
        use crate::core::config::BackupPair;
        
        // Actualizar config compartido
        if let Ok(mut config) = self.config.lock() {
            if index < config.backup_pairs.len() {
                // Crear nuevo backup pair actualizado
                let updated_pair = BackupPair::new(source, destination);
                config.backup_pairs[index] = updated_pair;
                
                // Guardar a disco
                if let Err(e) = config.save() {
                    error!("❌ Error guardando tras actualizar backup pair: {}", e);
                    return;
                }
                
                // Actualizar daemon con config actualizado
                self.daemon = BackupDaemon::new(Arc::clone(&self.config));
                info!("✅ Backup pair #{} actualizado exitosamente", index + 1);
            } else {
                error!("❌ Índice de backup pair inválido para actualizar: {}", index);
            }
        } else {
            error!("❌ Error accediendo configuración compartida para actualizar backup pair");
        }
    }
    
    fn remove_backup_pair(&mut self, index: usize) {
        // Actualizar config compartido
        let removed_pair = if let Ok(mut config) = self.config.lock() {
            if index < config.backup_pairs.len() {
                let removed_pair = config.backup_pairs.remove(index);
                
                // Guardar a disco
                if let Err(e) = config.save() {
                    error!("❌ Error guardando tras eliminar backup pair: {}", e);
                    return;
                }
                
                // Actualizar daemon con config actualizado
                self.daemon = BackupDaemon::new(Arc::clone(&self.config));
                
                Some(removed_pair)
            } else {
                warn!("⚠️ Índice inválido para eliminar backup pair: {}", index);
                None
            }
        } else {
            error!("❌ Error accediendo configuración compartida para eliminar backup pair");
            return;
        };
        
        // Reinicializar estados DESPUÉS de liberar lock
        self.initialize_backup_statuses();
        
        if let Some(removed_pair) = removed_pair {
            info!("✅ Backup pair eliminado: {} → {}", 
                 removed_pair.source.display(), 
                 removed_pair.destination.display());
        }
    }
    
    fn move_backup_pair_up(&mut self, index: usize) {
        // Actualizar config compartido
        if let Ok(mut config) = self.config.lock() {
            if index > 0 && index < config.backup_pairs.len() {
                // Intercambiar posiciones
                config.backup_pairs.swap(index, index - 1);
                
                // Guardar a disco
                if let Err(e) = config.save() {
                    error!("❌ Error guardando tras mover backup pair: {}", e);
                    return;
                }
                
                // Actualizar daemon con config actualizado
                self.daemon = BackupDaemon::new(Arc::clone(&self.config));
                info!("✅ Backup pair movido hacia arriba: #{} → #{}", index + 1, index);
            } else {
                warn!("⚠️ No se puede mover backup pair hacia arriba: índice {}", index);
            }
        } else {
            error!("❌ Error accediendo configuración compartida para mover backup pair");
        }
    }
    
    fn move_backup_pair_down(&mut self, index: usize) {
        // Actualizar config compartido
        if let Ok(mut config) = self.config.lock() {
            if index < config.backup_pairs.len().saturating_sub(1) {
                // Intercambiar posiciones
                config.backup_pairs.swap(index, index + 1);
                
                // Guardar a disco
                if let Err(e) = config.save() {
                    error!("❌ Error guardando tras mover backup pair: {}", e);
                    return;
                }
                
                // Actualizar daemon con config actualizado
                self.daemon = BackupDaemon::new(Arc::clone(&self.config));
                info!("✅ Backup pair movido hacia abajo: #{} → #{}", index + 1, index + 2);
            } else {
                warn!("⚠️ No se puede mover backup pair hacia abajo: índice {}", index);
            }
        } else {
            error!("❌ Error accediendo configuración compartida para mover backup pair");
        }
    }

    fn toggle_backup_pair_enabled(&mut self, index: usize, enabled: bool) {
        // Actualizar config compartido
        if let Ok(mut config) = self.config.lock() {
            if index < config.backup_pairs.len() {
                // Actualizar estado enabled
                config.backup_pairs[index].enabled = enabled;

                // Guardar a disco
                if let Err(e) = config.save() {
                    error!("❌ Error guardando tras toggle backup pair: {}", e);
                    return;
                }

                // Actualizar daemon con config actualizado
                self.daemon = BackupDaemon::new(Arc::clone(&self.config));

                let action = if enabled { "habilitado" } else { "deshabilitado" };
                info!("✅ Backup pair #{} {} exitosamente", index + 1, action);
            } else {
                error!("❌ Índice de backup pair inválido para toggle: {}", index);
            }
        } else {
            error!("❌ Error accediendo configuración compartida para toggle backup pair");
        }
    }

    /// Actualizar estado de un backup pair específico
    fn update_backup_status(&mut self, backup_pair_id: String, status: BackupStatus) {
        if let Ok(mut state) = self.state.lock() {
            // Obtener o crear entrada para este backup pair
            let backup_status = state.backup_statuses
                .entry(backup_pair_id.clone())
                .or_insert_with(|| BackupPairStatus::new(backup_pair_id.clone()));
                
            // Actualizar estado y timestamp
            backup_status.update_execution(status.clone());
            
            info!("📊 Estado actualizado para backup pair {}: {:?}", backup_pair_id, status);
        } else {
            error!("❌ Error actualizando estado de backup pair");
        }
    }
    
    /// Inicializar estados para todos los backup pairs configurados
    fn initialize_backup_statuses(&mut self) {
        if let (Ok(config), Ok(mut state)) = (self.config.lock(), self.state.lock()) {
            // Inicializar estado para cada backup pair si no existe
            for pair in &config.backup_pairs {
                if !state.backup_statuses.contains_key(&pair.id) {
                    state.backup_statuses.insert(
                        pair.id.clone(),
                        BackupPairStatus::new(pair.id.clone())
                    );
                }
            }
            
            // Limpiar estados de backup pairs que ya no existen
            let existing_ids: std::collections::HashSet<_> = config.backup_pairs
                .iter()
                .map(|p| p.id.clone())
                .collect();
                
            state.backup_statuses.retain(|id, _| existing_ids.contains(id));
            
            info!("✅ Estados de backup inicializados para {} pairs", config.backup_pairs.len());
        }
    }
}

/// Canal global para comandos al hilo de fondo
static mut BACKGROUND_SENDER: Option<Sender<BackgroundCommand>> = None;

/// Enviar comando al hilo de fondo
pub fn send_background_command(command: BackgroundCommand) {
    unsafe {
        if let Some(sender) = &BACKGROUND_SENDER {
            if let Err(e) = sender.send(command.clone()) {
                if !matches!(command, BackgroundCommand::Exit) {
                    error!("❌ Error sending background command {:?}: {}", command, e);
                }
            }
        } else if !matches!(command, BackgroundCommand::Exit) {
            error!("❌ Background sender not available");
        }
    }
}

/// Estado principal de la aplicación RustyVault
/// Ahora solo maneja UI, el estado real está en background thread
pub struct BackupApp {
    /// Configuración de la aplicación
    config: Arc<Mutex<AppConfig>>,
    
    /// System tray integration
    system_tray: Option<SystemTray>,
    
    /// UI state management
    ui_state: MainWindow,
    
    /// Settings window
    settings_window: Option<SettingsWindow>,
    
    /// Auto-start daemon flag (desde CLI)
    auto_start_daemon: bool,
    
    /// Referencia al estado del background thread
    background_state: Arc<Mutex<AppState>>,
}

impl BackupApp {
    /// Constructor principal - llamado desde main.rs
    pub fn new(_cc: &eframe::CreationContext<'_>, auto_start_daemon: bool) -> Self {
        info!("🏗️ Inicializando BackupApp con arquitectura de background thread...");
        
        // Cargar configuración
        let config = match AppConfig::load() {
            Ok(cfg) => {
                info!("✅ Configuración cargada exitosamente");
                cfg
            }
            Err(e) => {
                error!("❌ Error cargando configuración: {}", e);
                warn!("🔄 Usando configuración por defecto");
                AppConfig::default()
            }
        };
        
        // Estado compartido thread-safe
        let config_shared = Arc::new(Mutex::new(config));
        
        // Crear canal para comunicación con background thread
        let (command_sender, command_receiver) = mpsc::channel::<BackgroundCommand>();
        
        // Guardar sender globalmente para que tray pueda usarlo
        unsafe {
            BACKGROUND_SENDER = Some(command_sender.clone());
        }
        
        // Crear background manager
        let background_manager = BackgroundManager::new(command_receiver, Arc::clone(&config_shared));
        let background_state = Arc::clone(&background_manager.state);
        
        // Iniciar background thread
        let egui_ctx = _cc.egui_ctx.clone();
        thread::spawn(move || {
            background_manager.run(egui_ctx);
        });
        
        // Inicializar system tray SIGNALS
        let system_tray = match SystemTray::new(_cc.egui_ctx.clone()) {
            Ok(tray) => {
                info!("System tray inicializado");
                Some(tray)
            }
            Err(e) => {
                error!("❌ Error inicializando system tray: {}", e);
                warn!("⚠️ Continuando sin system tray");
                None
            }
        };
        
        // Inicializar UI state
        let ui_state = MainWindow::new();
        
        info!("BackupApp inicializado");
        
        Self {
            config: config_shared,
            system_tray,
            ui_state,
            settings_window: None,
            auto_start_daemon,
            background_state,
        }
    }
    
    /// Handle auto-start daemon
    fn handle_auto_start(&mut self) {
        if self.auto_start_daemon {
            info!("🚀 Auto-starting daemon from CLI flag");
            send_background_command(BackgroundCommand::StartDaemon);
        }
    }
    
    /// Handle settings window actions
    fn handle_settings_action(&mut self, action: SettingsAction, _ctx: &egui::Context) {
        match action {
            SettingsAction::StartDaemon => {
                send_background_command(BackgroundCommand::StartDaemon);
                info!("🚀 Daemon start requested from settings");
            }
            SettingsAction::StopDaemon => {
                send_background_command(BackgroundCommand::StopDaemon);
                info!("⏹ Daemon stop requested from settings");
            }
            SettingsAction::UpdateInterval(interval) => {
                // Update the configuration
                if let Ok(mut config) = self.config.lock() {
                    config.check_interval_seconds = interval;
                    if let Err(e) = config.save() {
                        error!("❌ Error saving config: {}", e);
                    }
                }
                send_background_command(BackgroundCommand::UpdateConfig(self.extract_config_from_ui().unwrap_or_default()));
                info!("⏰ Interval updated to {} seconds", interval);
            }
            SettingsAction::UpdateRobocopyConfig(robocopy_config) => {
                if let Ok(mut config) = self.config.lock() {
                    config.robocopy = robocopy_config;
                    if let Err(e) = config.save() {
                        error!("❌ Error saving robocopy config: {}", e);
                    }
                }
                info!("🔧 Robocopy configuration updated");
            }
            SettingsAction::UpdateAutoStart(enabled) => {
                info!("🚀 Auto-start setting: {}", enabled);
                // TODO: Implement Windows startup registry modification
            }
            SettingsAction::UpdateNotificationEnabled(enabled) => {
                info!("🔔 Notifications enabled: {}", enabled);
                // TODO: Store in config
            }
            SettingsAction::UpdateTheme(theme) => {
                info!("🎨 Theme updated: {:?}", theme);
                // TODO: Implement theme switching
            }
            SettingsAction::ExportConfig => {
                info!("📤 Export config requested");
                // TODO: Implement file dialog for export
            }
            SettingsAction::ImportConfig(config_path) => {
                info!("📥 Import config from: {}", config_path);
                // TODO: Implement config import
            }
            SettingsAction::CloseSettings => {
                self.settings_window = None;
                info!("⚙️ Settings window closed");
            }
            SettingsAction::ApplyAndSave => {
                info!("💾 Apply and save settings");
                // This is handled by individual setting actions
            }
        }
    }

    fn handle_ui_action(&mut self, action: UIAction, _ctx: &egui::Context) {
        match action {
            UIAction::MinimizeToTray => {
                send_background_command(BackgroundCommand::HideWindow);
                
                // Mostrar notificación
                if let Some(ref tray) = self.system_tray {
                    if let Err(e) = tray.minimize_to_tray() {
                        error!("❌ Error en tray notification: {}", e);
                    }
                }
            }
            UIAction::StartDaemon => {
                send_background_command(BackgroundCommand::StartDaemon);
            }
            UIAction::StopDaemon => {
                send_background_command(BackgroundCommand::StopDaemon);
            }
            UIAction::Exit => {
                send_background_command(BackgroundCommand::Exit);
            }
            UIAction::OpenSettings => {
                if self.settings_window.is_none() {
                    let mut settings_window = SettingsWindow::new();
                    if let Ok(config) = self.config.lock() {
                        settings_window.initialize_from_config(&config);
                    }
                    self.settings_window = Some(settings_window);
                    info!("⚙️ Settings window opened");
                }
            }
            UIAction::RunBackupNow => {
                send_background_command(BackgroundCommand::RunBackupNow);
            }
            UIAction::ConfigChanged => {
                // Extraer configuración actual de la UI y enviar al background
                if let Ok(updated_config) = self.extract_config_from_ui() {
                    send_background_command(BackgroundCommand::UpdateConfig(updated_config));
                } else {
                    error!("❌ Error extrayendo configuración de la UI");
                }
            }
            
            // === NEW CARDS ACTIONS ===
            UIAction::AddBackupPair { source, destination } => {
                send_background_command(BackgroundCommand::AddBackupPair { source, destination });
            }
            UIAction::UpdateBackupPair { index, source, destination } => {
                send_background_command(BackgroundCommand::UpdateBackupPair { index, source, destination });
            }
            UIAction::RemoveBackupPair(index) => {
                send_background_command(BackgroundCommand::RemoveBackupPair(index));
            }
            UIAction::EditBackupPair(index) => {
                info!("✏️ Iniciando edición de backup pair #{}", index + 1);
                
                // Obtener backup pair del config compartido
                if let Ok(config) = self.config.lock() {
                    if let Some(pair) = config.backup_pairs.get(index) {
                        // Poblar modal con datos existentes
                        self.ui_state.temp_source_buffer = pair.source.display().to_string();
                        self.ui_state.temp_destination_buffer = pair.destination.display().to_string();
                        self.ui_state.editing_pair_index = Some(index);
                        self.ui_state.show_add_modal = true;
                        
                        info!("✏️ Modal de edición abierto para: {} → {}", 
                             pair.source.display(), pair.destination.display());
                    } else {
                        error!("❌ Índice de backup pair inválido: {}", index);
                    }
                } else {
                    error!("❌ Error accediendo configuración para editar backup pair");
                }
            }
            UIAction::MoveBackupPairUp(index) => {
                send_background_command(BackgroundCommand::MoveBackupPairUp(index));
            }
            UIAction::MoveBackupPairDown(index) => {
                send_background_command(BackgroundCommand::MoveBackupPairDown(index));
            }
            UIAction::ToggleBackupPairEnabled(index, enabled) => {
                send_background_command(BackgroundCommand::ToggleBackupPairEnabled(index, enabled));
            }
        }
    }
    
    /// Extraer configuración actual de la UI para sync de settings globales
    /// Los backup_pairs se manejan por separado con commands específicos
    fn extract_config_from_ui(&self) -> Result<AppConfig, Box<dyn std::error::Error>> {
        // Parsear el intervalo
        let check_interval_seconds = self.ui_state.interval_buffer.parse::<u64>()
            .map_err(|e| format!("Invalid interval: {}", e))?;
        
        // Obtener configuración actual para preservar backup_pairs
        let mut config = if let Ok(current_config) = self.config.lock() {
            current_config.clone()
        } else {
            return Err("Error accediendo configuración actual".into());
        };
        
        // Actualizar solo los campos que la UI global maneja
        config.check_interval_seconds = check_interval_seconds;
        config.start_with_windows = self.ui_state.temp_start_with_windows;
        config.robocopy = self.ui_state.temp_robocopy_config.clone();
        
        Ok(config)
    }
}

impl eframe::App for BackupApp {
    /// Update loop principal de egui - ahora solo maneja UI
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle auto-start en primer frame
        if self.auto_start_daemon {
            self.handle_auto_start();
            self.auto_start_daemon = false; // Solo una vez
        }
        
        // Leer estado actual del background thread
        let current_state = if let Ok(state) = self.background_state.lock() {
            state.clone()
        } else {
            AppState::default()
        };
        
        // Verificar si debemos salir
        if current_state.should_exit {
            info!("🔚 Exit requested by background thread");
            return;
        }
        
        // Recolectar acciones de UI
        let mut ui_actions = Vec::new();
        
        // Renderizar UI principal - pasar estado desde background
        self.ui_state.show(
            ctx,
            &self.config,
            &Arc::new(AtomicBool::new(current_state.daemon_running)), // Convertir bool a AtomicBool para compatibilidad
            &self.background_state,
            &mut |action| ui_actions.push(action)
        );
        
        // Renderizar Settings Window si está abierta
        if let Some(ref mut settings_window) = self.settings_window {
            let daemon_running = Arc::new(AtomicBool::new(current_state.daemon_running));
            
            let (keep_open, settings_actions) = settings_window.render(
                ctx,
                &self.config,
                &daemon_running,
            );
            
            if !keep_open {
                self.settings_window = None;
            }
            
            // Process settings actions
            for action in settings_actions {
                self.handle_settings_action(action, ctx);
            }
        }
        
        // Procesar acciones después del render
        for action in ui_actions {
            self.handle_ui_action(action, ctx);
        }
    }
    
    /// Manejo de cierre de ventana
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("🔚 App exit requested");
        send_background_command(BackgroundCommand::Exit);
        
        // No guardamos config aquí porque el auto-save en tiempo real ya lo hace
        // y self.config puede tener valores desactualizados
    }
} 