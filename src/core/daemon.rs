/// Módulo de daemon - lógica del backup automático con intervalos
/// Implementación real con threads para consistencia con el resto de la app

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tracing::{info, debug, error, warn};

use crate::core::AppConfig;
use crate::core::backup::{execute_backup, BackupResult};

/// Estructura del daemon de backup automático
pub struct BackupDaemon {
    /// Configuración de la aplicación
    config: Arc<Mutex<AppConfig>>,
    /// Flag para controlar si el daemon debe seguir corriendo
    running: Arc<AtomicBool>,
    /// Handle del thread del daemon
    handle: Option<std::thread::JoinHandle<()>>,
}

impl BackupDaemon {
    /// Crear nueva instancia del daemon
    pub fn new(config: Arc<Mutex<AppConfig>>) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }
    
    /// Iniciar el daemon de backup automático
    pub fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            info!("⚠️ Daemon ya está corriendo");
            return Ok(());
        }
        
        info!("🚀 Iniciando daemon de backup automático...");
        self.running.store(true, Ordering::Relaxed);
        
        // Clonar datos para el thread
        let config_clone = Arc::clone(&self.config);
        let running_clone = Arc::clone(&self.running);
        
        // Spawear daemon task en thread separado
        let handle = std::thread::spawn(move || {
            daemon_task(config_clone, running_clone);
        });
        
        self.handle = Some(handle);
        
        // Mostrar notificación de daemon iniciado
        if let Ok(config) = self.config.lock() {
            if let Err(e) = crate::system::notifications::show_daemon_started(config.check_interval_seconds) {
                warn!("⚠️ Error mostrando notificación daemon: {}", e);
            }
        }
        
        info!("✅ Daemon iniciado exitosamente");
        Ok(())
    }
    
    /// Detener el daemon
    pub fn stop(&mut self) -> Result<()> {
        if !self.running.load(Ordering::Relaxed) {
            info!("⚠️ Daemon no está corriendo");
            return Ok(());
        }
        
        info!("🛑 Deteniendo daemon de backup...");
        
        // Señalizar al daemon que pare
        self.running.store(false, Ordering::Relaxed);
        
        // Esperar a que termine el thread
        if let Some(handle) = self.handle.take() {
            match handle.join() {
                Ok(_) => info!("✅ Daemon thread terminado correctamente"),
                Err(_) => error!("❌ Error terminando daemon thread"),
            }
        }
        
        // Mostrar notificación de daemon detenido
        if let Err(e) = crate::system::notifications::show_daemon_stopped() {
            warn!("⚠️ Error mostrando notificación daemon: {}", e);
        }
        
        info!("✅ Daemon detenido exitosamente");
        Ok(())
    }
    
    /// Verificar si el daemon está corriendo
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
    
    /// Obtener handle del flag running para compartir con la UI
    pub fn get_running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }
    
    /// Obtener copia de la configuración actual
    pub fn get_config(&self) -> Result<AppConfig> {
        self.config.lock()
            .map_err(|e| anyhow::anyhow!("Error accediendo configuración: {}", e))
            .map(|config| config.clone())
    }
}

/// Task principal del daemon - se ejecuta en background
fn daemon_task(config: Arc<Mutex<AppConfig>>, running: Arc<AtomicBool>) {
    info!("🤖 Daemon task iniciado - comenzando loop automático");
    
    let mut iteration = 0;
    
    while running.load(Ordering::Relaxed) {
        iteration += 1;
        debug!("🔄 Daemon iteration #{}", iteration);
        
        // Obtener configuración actual
        let (backup_pairs, robocopy_config, interval) = match config.lock() {
            Ok(cfg) => {
                (
                    cfg.backup_pairs.clone(),
                    cfg.robocopy.clone(),
                    cfg.check_interval_seconds,
                )
            }
            Err(e) => {
                error!("❌ Error accediendo configuración en daemon: {}", e);
                // Sleep un poco y continuar
                std::thread::sleep(Duration::from_secs(60));
                continue;
            }
        };
        
        // Validar configuración antes de ejecutar
        if backup_pairs.is_empty() {
            warn!("⚠️ No hay backup pairs configurados - omitiendo backup automático");
        } else {
            // Ejecutar backup automático secuencial
            info!("🚀 Ejecutando backup automático #{} - {} pair(s) a procesar", iteration, backup_pairs.len());
            
            let mut total_success = 0;
            let mut total_warnings = 0;
            let mut total_failures = 0;
            
            // Ejecutar cada backup pair secuencialmente
            for (i, pair) in backup_pairs.iter().enumerate() {
                if !pair.enabled {
                    info!("⏭️ Backup pair #{} deshabilitado - omitiendo", i + 1);
                    continue;
                }
                
                info!("🔄 Procesando backup pair #{}: {} → {}", 
                     i + 1, pair.source.display(), pair.destination.display());
                
                match execute_backup(&pair.source, &pair.destination, &robocopy_config) {
                    Ok(result) => {
                        match result {
                            BackupResult::Success { files_copied, bytes_transferred } => {
                                info!("✅ Backup automático pair #{} completado exitosamente - {} archivos, {} bytes", i + 1, files_copied, bytes_transferred);
                                total_success += 1;
                            }
                            BackupResult::Warning(msg) => {
                                warn!("⚠️ Backup automático pair #{} con advertencias: {}", i + 1, msg);
                                total_warnings += 1;
                            }
                            BackupResult::Failed => {
                                error!("❌ Backup automático pair #{} falló", i + 1);
                                total_failures += 1;
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ Error crítico en backup automático pair #{}: {}", i + 1, e);
                        total_failures += 1;
                    }
                }
            }
            
            // Notificación consolidada para daemon automático
            if total_failures > 0 {
                let msg = format!("Daemon #{}: {} exitosos, {} advertencias, {} fallidos", 
                                 iteration, total_success, total_warnings, total_failures);
                if let Err(e) = crate::system::notifications::show_backup_failed(&msg) {
                    warn!("⚠️ Error mostrando notificación: {}", e);
                }
            } else if total_warnings > 0 {
                let msg = format!("Daemon #{}: {} exitosos, {} advertencias", 
                                 iteration, total_success, total_warnings);
                if let Err(e) = crate::system::notifications::show_backup_warning(&msg) {
                    warn!("⚠️ Error mostrando notificación: {}", e);
                }
            } else {
                info!("✅ Daemon #{}: {} backups completados exitosamente", iteration, total_success);
                if let Err(e) = crate::system::notifications::show_backup_success(Some(total_success as u32), Some("automático")) {
                    warn!("⚠️ Error mostrando notificación: {}", e);
                }
            }
            
            info!("🏁 Backup automático #{} finalizado: {} éxito, {} advertencias, {} fallos", 
                 iteration, total_success, total_warnings, total_failures);
        }
        
        // Sleep hasta el próximo backup (o hasta que se detenga)
        info!("😴 Próximo backup automático en {} segundos", interval);
        
        // Sleep en chunks para poder responder rápido al stop
        let sleep_chunks = interval.max(1); // Evitar división por 0
        let chunk_size = if sleep_chunks > 60 { 60 } else { 1 }; // Chunks de máximo 60 segundos
        let chunks = sleep_chunks / chunk_size;
        
        for chunk in 0..chunks {
            if !running.load(Ordering::Relaxed) {
                info!("🛑 Daemon stop signal received during sleep - exiting");
                return;
            }
            
            debug!("😴 Sleep chunk {}/{} ({}s)", chunk + 1, chunks, chunk_size);
            std::thread::sleep(Duration::from_secs(chunk_size));
        }
        
        // Sleep del resto si no es exactamente divisible
        let remainder = sleep_chunks % chunk_size;
        if remainder > 0 && running.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_secs(remainder));
        }
    }
    
    info!("🏁 Daemon task terminado - loop finalizado");
} 