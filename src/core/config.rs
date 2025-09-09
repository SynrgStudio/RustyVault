use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{info, warn, debug};

/// Pair de directorio origen → destino para backup
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupPair {
    pub id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub enabled: bool,
    #[serde(default)]
    pub priority: usize,  // Para ordenamiento manual
}

impl BackupPair {
    /// Crear nuevo backup pair con valores por defecto
    pub fn new(source: impl Into<PathBuf>, destination: impl Into<PathBuf>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.into(),
            destination: destination.into(),
            enabled: true,  // Por defecto habilitado
            priority: 0,    // Se asignará automáticamente
        }
    }

    /// Crear backup pair con ID específico (para compatibilidad)
    pub fn with_id(id: String, source: PathBuf, destination: PathBuf) -> Self {
        Self {
            id,
            source,
            destination,
            enabled: true,
            priority: 0,
        }
    }

    /// Verificar si el backup pair está activo (habilitado)
    pub fn is_active(&self) -> bool {
        self.enabled
    }

    /// Obtener nombre corto para display
    pub fn display_name(&self) -> String {
        format!("{} → {}",
            self.source.file_name().unwrap_or_default().to_string_lossy(),
            self.destination.file_name().unwrap_or_default().to_string_lossy()
        )
    }
}

/// Configuración principal de la aplicación - Simple JSON junto al ejecutable
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    // NEW: Lista de backup pairs
    pub backup_pairs: Vec<BackupPair>,
    
    // OLD: Para migración automática (deprecated)
    #[serde(default)]
    pub source_folder: String,
    #[serde(default)]  
    pub destination_folder: String,
    
    // Configuración global
    pub check_interval_seconds: u64,
    pub start_with_windows: bool,
    pub robocopy: RobocopyConfig,
}

/// Configuración específica de Robocopy con tooltips explicativos
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RobocopyConfig {
    /// /MIR - Modo Espejo: Crea copia EXACTA del origen en destino
    /// ⚠️ ATENCIÓN: Elimina archivos del destino que no existan en origen
    pub mirror_mode: bool,
    
    /// /MT:X - Hilos simultáneos para copiar archivos (1-128)
    /// 💡 Más hilos = Más velocidad. Recomendado: 8 para HDD, 16+ para SSD
    pub multithreading: u8,
    
    /// /FFT - Compatibilidad con discos FAT32/exFAT  
    /// 🔧 Soluciona problemas con USBs y NAS antiguos. Recomendado: activar
    pub fat_file_timing: bool,
    
    /// /R:X - Reintentos por archivo fallido (0-1000000)
    /// 🔄 Por defecto robocopy reintenta 1,000,000 veces (!). Recomendado: 3-5
    pub retry_count: u8,
    
    /// /W:X - Segundos entre reintentos (0-300)
    /// ⏱️ Por defecto robocopy espera 30 segundos (!). Recomendado: 2-5
    pub retry_wait: u8,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            backup_pairs: vec![], // Lista vacía por defecto
            source_folder: String::new(), // Deprecated: solo para migración
            destination_folder: String::new(), // Deprecated: solo para migración  
            check_interval_seconds: 3600, // 1 hora por defecto
            start_with_windows: false,
            robocopy: RobocopyConfig::default(),
        }
    }
}

impl Default for RobocopyConfig {
    fn default() -> Self {
        Self {
            mirror_mode: true,        // Modo espejo por defecto
            multithreading: 8,        // 8 hilos - buen balance
            fat_file_timing: true,    // Compatibilidad activada
            retry_count: 3,           // 3 reintentos razonables
            retry_wait: 2,            // 2 segundos entre reintentos
        }
    }
}

impl AppConfig {
    /// Cargar configuración desde config.json en carpeta del ejecutable
    /// Si no existe, crear con valores por defecto
    /// Auto-migra formato legacy (single backup) a nuevo formato (multiple backups)
    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;
        
        if config_path.exists() {
            debug!("📁 Cargando configuración desde: {}", config_path.display());
            
            let config_content = fs::read_to_string(&config_path)
                .with_context(|| format!("Error leyendo config.json: {}", config_path.display()))?;
            
            let mut config: AppConfig = serde_json::from_str(&config_content)
                .with_context(|| "Error parseando config.json - JSON inválido")?;
            
            // 🔄 AUTO-MIGRACIÓN: legacy single backup → multiple backups
            if config.backup_pairs.is_empty() && !config.source_folder.is_empty() && !config.destination_folder.is_empty() {
                info!("🔄 Migrando configuración legacy a formato múltiple backups");
                
                config.backup_pairs.push(BackupPair::new(
                    config.source_folder.clone(),
                    config.destination_folder.clone(),
                ));
                
                // Limpiar campos legacy
                config.source_folder.clear();
                config.destination_folder.clear();
                
                // Auto-guardar formato migrado
                config.save().context("Error guardando configuración migrada")?;
                info!("✅ Migración automática completada");
            }
            
            info!("✅ Configuración cargada correctamente");
            debug!("🔧 Backup pairs: {}", config.backup_pairs.len());
            debug!("🔧 Interval: {}s", config.check_interval_seconds);
            
            Ok(config)
        } else {
            warn!("⚠️ config.json no encontrado, creando configuración por defecto");
            let mut default_config = Self::default();
            
            // Agregar un backup pair por defecto con carpetas sensatas
            default_config.backup_pairs.push(BackupPair::new(
                get_default_source_folder(),
                get_default_destination_folder(),
            ));
            
            default_config.save()?;
            info!("✅ Configuración por defecto creada en: {}", config_path.display());
            Ok(default_config)
        }
    }
    
    /// Guardar configuración a config.json
    /// Auto-save en cada cambio según PRD
    pub fn save(&self) -> Result<()> {
        let config_path = get_config_path()?;
        
        let config_json = serde_json::to_string_pretty(self)
            .context("Error serializando configuración a JSON")?;
        
        fs::write(&config_path, config_json)
            .with_context(|| format!("Error escribiendo config.json: {}", config_path.display()))?;
        
        debug!("💾 Configuración guardada en: {}", config_path.display());
        Ok(())
    }
    
    /// Validar que todas las rutas de backup pairs sean válidas
    pub fn validate_paths(&self) -> Result<()> {
        if self.backup_pairs.is_empty() {
            return Err(anyhow::anyhow!("❌ No hay backup pairs configurados"));
        }
        
        for (i, pair) in self.backup_pairs.iter().enumerate() {
            if !pair.enabled {
                continue; // Skip disabled pairs
            }
            
            // Validar source folder existe
            if !pair.source.exists() {
            return Err(anyhow::anyhow!(
                    "❌ Backup #{}: Carpeta origen no existe: {}", 
                    i + 1, pair.source.display()
            ));
        }
        
            if !pair.source.is_dir() {
                return Err(anyhow::anyhow!(
                    "❌ Backup #{}: Ruta origen no es una carpeta: {}", 
                    i + 1, pair.source.display()
                ));
            }
            
            // Destination se auto-crea, solo validar que sea una ruta válida
            if let Some(parent) = pair.destination.parent() {
                if !parent.exists() {
                    return Err(anyhow::anyhow!(
                        "❌ Backup #{}: Carpeta padre del destino no existe: {}", 
                        i + 1, parent.display()
                ));
                }
            }
        }
        
        info!("✅ Validación de {} backup pairs exitosa", self.backup_pairs.len());
        Ok(())
    }
}

impl RobocopyConfig {
    /// Construir argumentos de robocopy según configuración
    pub fn build_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        
        // Parámetros configurables
        if self.mirror_mode {
            args.push("/MIR".to_string());
        }
        
        args.push(format!("/MT:{}", self.multithreading));
        
        if self.fat_file_timing {
            args.push("/FFT".to_string());
        }
        
        args.push(format!("/R:{}", self.retry_count));
        args.push(format!("/W:{}", self.retry_wait));
        
        // Parámetros adicionales para mejor funcionamiento
        args.push("/NP".to_string());    // No mostrar progreso (% copiado)
        args.push("/NDL".to_string());   // No mostrar lista de directorios
        args.push("/TEE".to_string());   // Output a console y log file
        
        debug!("🔧 Argumentos robocopy generados: {:?}", args);
        args
    }
    
    /// Obtener preview del comando completo para mostrar en UI
    pub fn preview_command(&self, source: &str, dest: &str) -> String {
        let args = self.build_args();
        format!("robocopy \"{}\" \"{}\" {}", source, dest, args.join(" "))
    }
}

/// Obtener ruta del archivo config.json (carpeta del ejecutable)
fn get_config_path() -> Result<PathBuf> {
    // Prioridad: carpeta del ejecutable
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return Ok(exe_dir.join("config.json"));
        }
    }
    
    // Fallback a directorio actual
    Ok(PathBuf::from("config.json"))
}

/// Carpeta por defecto para source (Documents del usuario)
fn get_default_source_folder() -> String {
    if let Some(docs_dir) = dirs::document_dir() {
        docs_dir.to_string_lossy().to_string()
    } else {
        "C:\\Users\\%USERNAME%\\Documents".to_string()
    }
}

/// Carpeta por defecto para destination (Backup en drive secundario si existe)
fn get_default_destination_folder() -> String {
    // Intentar encontrar un drive secundario para backup
    for drive in ['D', 'E', 'F'] {
        let backup_path = format!("{}:\\Backup", drive);
        if Path::new(&format!("{}:\\", drive)).exists() {
            return backup_path;
        }
    }
    
    // Fallback a C:\Backup
    "C:\\Backup".to_string()
} 