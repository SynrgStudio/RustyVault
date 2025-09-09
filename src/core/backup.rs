/// Módulo de backup - ejecución de robocopy y manejo de procesos
/// TODO: Implementar ejecución real de robocopy según PRD

use anyhow::Result;
use std::path::Path;
use tracing::{info, debug};

use crate::core::RobocopyConfig;

/// Resultado de una operación de backup
#[derive(Debug, Clone)]
pub enum BackupResult {
    Success { files_copied: u32, bytes_transferred: u64 },
    Warning(String),
    Failed,
}

/// Ejecutar backup usando robocopy con configuración especificada
pub fn execute_backup(
    source: &Path,
    destination: &Path,
    config: &RobocopyConfig,
) -> Result<BackupResult> {
    use std::process::{Command, Stdio};
    
    info!("🚀 Iniciando backup: {} -> {}", source.display(), destination.display());
    
    // Validar que la carpeta de origen existe
    if !source.exists() {
        tracing::error!("❌ Carpeta de origen no existe: {}", source.display());
        return Ok(BackupResult::Failed);
    }
    
    // Crear carpeta destino si no existe
    if let Err(e) = std::fs::create_dir_all(destination) {
        tracing::error!("❌ Error creando carpeta destino {}: {}", destination.display(), e);
        return Ok(BackupResult::Failed);
    }
    
    // Construir argumentos robocopy
    let args = config.build_args();
    debug!("🔧 Argumentos robocopy: {:?}", args);
    
    // Ejecutar robocopy con CREATE_NO_WINDOW (proceso oculto)
    info!("⚡ Ejecutando robocopy...");
    
    let mut command = Command::new("robocopy");
    command
        .arg(source.to_string_lossy().as_ref())
        .arg(destination.to_string_lossy().as_ref())
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // Solo en Windows: usar CREATE_NO_WINDOW
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = command.output();
    
    match output {
        Ok(result) => {
            let exit_code = result.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            
            info!("✅ Robocopy terminado con código: {}", exit_code);
            
            if !stdout.is_empty() {
                debug!("📄 Robocopy stdout: {}", stdout.trim());
            }
            
            if !stderr.is_empty() && exit_code >= 8 {
                tracing::warn!("⚠️ Robocopy stderr: {}", stderr.trim());
            }
            
            Ok(parse_robocopy_output(exit_code, &stdout))
        }
        Err(e) => {
            tracing::error!("❌ Error ejecutando robocopy: {}", e);
            Ok(BackupResult::Failed)
        }
    }
}

/// Parsear output completo de robocopy para extraer estadísticas reales
fn parse_robocopy_output(exit_code: i32, stdout: &str) -> BackupResult {
    // Parsear estadísticas del output de robocopy
    let (files_copied, bytes_transferred) = parse_robocopy_stats(stdout);
    
    match exit_code {
        0 => BackupResult::Success { files_copied, bytes_transferred }, // No files copied (no changes)
        1 => BackupResult::Success { files_copied, bytes_transferred }, // Files copied successfully
        2 => BackupResult::Warning("Extra files/dirs in destination".to_string()),
        3 => BackupResult::Warning("Files copied + extra files in dest".to_string()),
        4 => BackupResult::Warning("Some mismatched files/dirs".to_string()),
        5 => BackupResult::Warning("Files copied + some mismatched".to_string()),
        6 => BackupResult::Warning("Extra + mismatched files".to_string()),
        7 => BackupResult::Warning("Files copied + extra + mismatched".to_string()),
        _ => BackupResult::Failed, // Exit codes 8+ indicate errors
    }
}

/// Parsear estadísticas específicas del output de robocopy
/// Busca líneas como: " Archivos:         1         1         0         0         0         0"
/// Y: "    Bytes:    14.4 k    14.4 k         0         0         0         0"
/// Formato: Total, Copiado, Omitido, No coincidencia, ERROR, Extras
fn parse_robocopy_stats(stdout: &str) -> (u32, u64) {
    let mut files_copied = 0u32;
    let mut bytes_transferred = 0u64;
    
    debug!("🔍 Parseando output de robocopy...");
    
    for line in stdout.lines() {
        let line = line.trim();
        
        // Buscar línea de archivos en español: " Archivos:         2         1         1         0         0         0"
        if line.starts_with("Archivos:") && line.contains(char::is_numeric) {
            debug!("📄 Línea de archivos encontrada: {}", line);
            let parts: Vec<&str> = line.split_whitespace().collect();
            debug!("📄 Parts: {:?}", parts);
            if parts.len() >= 3 {
                // parts[0] = "Archivos:", parts[1] = Total, parts[2] = Copiado
                if let Ok(copied) = parts[2].parse::<u32>() {
                    files_copied = copied;
                    debug!("📄 Archivos copiados parseados: {}", files_copied);
                } else {
                    debug!("❌ Error parseando archivos copiados: '{}'", parts[2]);
                }
            }
        }
        
        // Buscar línea de bytes en español: "    Bytes:    28.9 k    14.4 k    14.4 k         0         0         0"
        if line.starts_with("Bytes:") {
            debug!("💾 Línea de bytes encontrada: {}", line);
            
            let after_bytes = &line[6..]; // Skip "Bytes:"
            let parts: Vec<&str> = after_bytes.split_whitespace().collect();
            debug!("💾 Parts: {:?}", parts);
            
            // Estructura: Total, Copiado, Omitido, ...
            // Queremos los bytes copiados (segunda columna)
            if parts.len() >= 4 {
                let copied_part = parts[2]; // Copiado (14.4)
                let copied_suffix = parts[3]; // k
                
                // Verificar si el suffix es válido
                if ["k", "m", "g", "t"].contains(&copied_suffix.to_lowercase().as_str()) {
                    let combined = format!("{}{}", copied_part, copied_suffix);
                    debug!("💾 Parseando bytes copiados: '{}'", combined);
                    if let Ok(size) = parse_robocopy_size_combined(&combined) {
                        bytes_transferred = size;
                        debug!("💾 Bytes transferidos (copiados) parseados: {}", bytes_transferred);
                    } else {
                        debug!("❌ Error parseando bytes copiados: '{}'", combined);
                    }
                } else {
                    // Fallback: intentar parsear solo el número
                    if let Ok(size) = copied_part.parse::<u64>() {
                        bytes_transferred = size;
                        debug!("💾 Bytes transferidos parseados (sin sufijo): {}", bytes_transferred);
                    } else {
                        debug!("❌ Error parseando bytes sin sufijo: '{}'", copied_part);
                    }
                }
            } else if parts.len() >= 2 {
                // Fallback para formato simple
                let first_part = parts[0];
                let second_part = parts[1];
                
                if ["k", "m", "g", "t"].contains(&second_part.to_lowercase().as_str()) {
                    let combined = format!("{}{}", first_part, second_part);
                    debug!("💾 Parseando bytes (fallback): '{}'", combined);
                    if let Ok(size) = parse_robocopy_size_combined(&combined) {
                        bytes_transferred = size;
                        debug!("💾 Bytes transferidos parseados: {}", bytes_transferred);
                    }
                } else {
                    // Intentar parsear solo el primer número
                    if let Ok(size) = first_part.parse::<u64>() {
                        bytes_transferred = size;
                        debug!("💾 Bytes transferidos parseados (número simple): {}", bytes_transferred);
                    }
                }
            }
        }
    }
    
    debug!("🎯 Resultado final del parsing: {} archivos, {} bytes", files_copied, bytes_transferred);
    (files_copied, bytes_transferred)
}

/// Parsear tamaño de robocopy en formato combinado como "14.4k"
fn parse_robocopy_size_combined(size_str: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let size_str = size_str.trim();
    
    // Si es solo un número
    if let Ok(size) = size_str.parse::<u64>() {
        return Ok(size);
    }
    
    // Si tiene sufijo (k, m, g)
    if size_str.len() > 1 {
        let (number_part, suffix) = size_str.split_at(size_str.len() - 1);
        let suffix = suffix.to_lowercase();
        
        if let Ok(number) = number_part.parse::<f64>() {
            let multiplier = match suffix.as_str() {
                "k" => 1024,
                "m" => 1024 * 1024,
                "g" => 1024 * 1024 * 1024,
                "t" => 1024_u64.pow(4),
                _ => return Err(format!("Unknown suffix: {}", suffix).into()),
            };
            
            let result = (number * multiplier as f64) as u64;
            return Ok(result);
        }
    }
    
    Err(format!("Unable to parse size: {}", size_str).into())
}

/// Parsear código de salida de robocopy según documentación oficial (LEGACY - mantener por compatibilidad)
/// https://docs.microsoft.com/en-us/windows-server/administration/windows-commands/robocopy
fn parse_robocopy_exit_code(exit_code: i32) -> BackupResult {
    match exit_code {
        0 => BackupResult::Success { files_copied: 0, bytes_transferred: 0 }, // No files copied (no changes)
        1 => BackupResult::Success { files_copied: 0, bytes_transferred: 0 }, // Files copied successfully
        2 => BackupResult::Warning("Extra files/dirs in destination".to_string()),
        3 => BackupResult::Warning("Files copied + extra files in dest".to_string()),
        4 => BackupResult::Warning("Some mismatched files/dirs".to_string()),
        5 => BackupResult::Warning("Files copied + some mismatched".to_string()),
        6 => BackupResult::Warning("Extra + mismatched files".to_string()),
        7 => BackupResult::Warning("Files copied + extra + mismatched".to_string()),
        _ => BackupResult::Failed, // Exit codes 8+ indicate errors
    }
} 