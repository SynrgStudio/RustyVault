use std::path::{Path, PathBuf};
use std::fs;
use crate::core::config::BackupPair;

/// Resultado de validación de una ruta
#[derive(Debug, Clone, PartialEq)]
pub enum PathValidationResult {
    Valid,
    Warning(String),
    Error(String),
}

/// Resultado completo de validación de un backup pair
#[derive(Debug, Clone)]
pub struct BackupPairValidation {
    pub source_result: PathValidationResult,
    pub destination_result: PathValidationResult,
    pub cross_validation_result: PathValidationResult,
}

impl BackupPairValidation {
    /// Verifica si la validación es completamente exitosa
    pub fn is_valid(&self) -> bool {
        matches!(self.source_result, PathValidationResult::Valid) &&
        matches!(self.destination_result, PathValidationResult::Valid) &&
        matches!(self.cross_validation_result, PathValidationResult::Valid)
    }
    
    /// Verifica si hay errores críticos que impiden guardar
    pub fn has_errors(&self) -> bool {
        matches!(self.source_result, PathValidationResult::Error(_)) ||
        matches!(self.destination_result, PathValidationResult::Error(_)) ||
        matches!(self.cross_validation_result, PathValidationResult::Error(_))
    }
    
    /// Obtiene todos los mensajes de error
    pub fn get_error_messages(&self) -> Vec<String> {
        let mut errors = Vec::new();
        
        if let PathValidationResult::Error(msg) = &self.source_result {
            errors.push(format!("Origen: {}", msg));
        }
        if let PathValidationResult::Error(msg) = &self.destination_result {
            errors.push(format!("Destino: {}", msg));
        }
        if let PathValidationResult::Error(msg) = &self.cross_validation_result {
            errors.push(msg.clone());
        }
        
        errors
    }
    
    /// Obtiene todos los mensajes de advertencia
    pub fn get_warning_messages(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        
        if let PathValidationResult::Warning(msg) = &self.source_result {
            warnings.push(format!("Origen: {}", msg));
        }
        if let PathValidationResult::Warning(msg) = &self.destination_result {
            warnings.push(format!("Destino: {}", msg));
        }
        if let PathValidationResult::Warning(msg) = &self.cross_validation_result {
            warnings.push(msg.clone());
        }
        
        warnings
    }
}

/// Validador avanzado de rutas para backup pairs
pub struct PathValidator;

impl PathValidator {
    /// Validar un backup pair completo
    pub fn validate_backup_pair(
        source: &str, 
        destination: &str, 
        existing_pairs: &[BackupPair],
        editing_index: Option<usize>
    ) -> BackupPairValidation {
        let source_path = PathBuf::from(source);
        let dest_path = PathBuf::from(destination);
        
        BackupPairValidation {
            source_result: Self::validate_source_path(&source_path),
            destination_result: Self::validate_destination_path(&dest_path),
            cross_validation_result: Self::validate_cross_dependencies(
                &source_path, 
                &dest_path, 
                existing_pairs, 
                editing_index
            ),
        }
    }
    
    /// Validar ruta de origen
    fn validate_source_path(path: &Path) -> PathValidationResult {
        // 1. Verificar que no esté vacía
        if path.as_os_str().is_empty() {
            return PathValidationResult::Error("La ruta de origen no puede estar vacía".to_string());
        }
        
        // 2. Verificar caracteres válidos
        if let Err(msg) = Self::validate_path_characters(path) {
            return PathValidationResult::Error(msg);
        }
        
        // 3. Verificar si es ruta de red
        if Self::is_network_path(path) {
            if let Err(msg) = Self::validate_network_path(path) {
                return PathValidationResult::Error(msg);
            }
            return PathValidationResult::Warning("Ruta de red detectada - verificar conectividad".to_string());
        }
        
        // 4. Verificar existencia
        if !path.exists() {
            return PathValidationResult::Error("La ruta de origen no existe".to_string());
        }
        
        // 5. Verificar que sea directorio
        if !path.is_dir() {
            return PathValidationResult::Error("La ruta de origen debe ser un directorio".to_string());
        }
        
        // 6. Verificar permisos de lectura
        if let Err(msg) = Self::validate_read_permissions(path) {
            return PathValidationResult::Error(msg);
        }
        
        // 7. Verificar si es ruta crítica del sistema (solo para rutas realmente peligrosas)
        if Self::is_critical_system_path(path) {
            return PathValidationResult::Warning("Directorio del sistema - verificar que sea intencional".to_string());
        }
        
        PathValidationResult::Valid
    }
    
    /// Validar ruta de destino
    fn validate_destination_path(path: &Path) -> PathValidationResult {
        // 1. Verificar que no esté vacía
        if path.as_os_str().is_empty() {
            return PathValidationResult::Error("La ruta de destino no puede estar vacía".to_string());
        }
        
        // 2. Verificar caracteres válidos
        if let Err(msg) = Self::validate_path_characters(path) {
            return PathValidationResult::Error(msg);
        }
        
        // 3. Verificar si es ruta de red
        if Self::is_network_path(path) {
            if let Err(msg) = Self::validate_network_path(path) {
                return PathValidationResult::Error(msg);
            }
            return PathValidationResult::Warning("Ruta de red detectada - verificar conectividad".to_string());
        }
        
        // 4. Si existe, verificar que sea directorio
        if path.exists() && !path.is_dir() {
            return PathValidationResult::Error("La ruta de destino existe pero no es un directorio".to_string());
        }
        
        // 5. Verificar que el directorio padre exista y sea accesible
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return PathValidationResult::Error("El directorio padre del destino no existe".to_string());
            }
            
            // Verificar permisos de escritura en el padre
            if let Err(msg) = Self::validate_write_permissions(parent) {
                return PathValidationResult::Error(msg);
            }
        }
        
        // 6. Si el destino existe, verificar permisos de escritura
        if path.exists() {
            if let Err(msg) = Self::validate_write_permissions(path) {
                return PathValidationResult::Error(msg);
            }
        }
        
        PathValidationResult::Valid
    }
    
    /// Validar dependencias cruzadas y duplicados
    fn validate_cross_dependencies(
        source: &Path, 
        destination: &Path, 
        existing_pairs: &[BackupPair],
        editing_index: Option<usize>
    ) -> PathValidationResult {
        // 1. Verificar que source y destination no sean iguales
        if source == destination {
            return PathValidationResult::Error("La ruta de origen y destino no pueden ser iguales".to_string());
        }
        
        // 2. Verificar dependencias circulares (solo si es realmente problemático)
        if Self::is_problematic_circular_dependency(source, destination) {
            return PathValidationResult::Error("Dependencia circular detectada: el origen está dentro del destino o viceversa".to_string());
        }
        
        // 3. Verificar duplicados
        for (i, existing_pair) in existing_pairs.iter().enumerate() {
            // Skip si estamos editando este mismo pair
            if let Some(edit_idx) = editing_index {
                if i == edit_idx {
                    continue;
                }
            }
            
            // Verificar duplicado exacto
            if existing_pair.source == source && existing_pair.destination == destination {
                return PathValidationResult::Error("Ya existe un backup con estas mismas rutas".to_string());
            }
            
            // Verificar source duplicado
            if existing_pair.source == source {
                return PathValidationResult::Warning(format!(
                    "El directorio origen ya está siendo respaldado en: {}", 
                    existing_pair.destination.display()
                ));
            }
            
            // Verificar destination duplicado
            if existing_pair.destination == destination {
                return PathValidationResult::Warning(format!(
                    "El directorio destino ya está siendo usado por: {}", 
                    existing_pair.source.display()
                ));
            }
        }
        
        PathValidationResult::Valid
    }
    
    /// Verificar si hay dependencia circular problemática (no solo directorios hermanos)
    fn is_problematic_circular_dependency(source: &Path, destination: &Path) -> bool {
        // Solo verificar si las rutas existen para evitar falsos positivos
        if !source.exists() || !destination.exists() {
            return false;
        }

        // Obtener rutas canónicas
        let source_canonical = match source.canonicalize() {
            Ok(path) => path,
            Err(_) => return false,
        };

        let dest_canonical = match destination.canonicalize() {
            Ok(path) => path,
            Err(_) => return false,
        };

        // Verificar si source está dentro de destination (problemático)
        if source_canonical.starts_with(&dest_canonical) {
            return true;
        }

        // Verificar si destination está dentro de source (problemático)
        if dest_canonical.starts_with(&source_canonical) {
            return true;
        }

        // Si son directorios hermanos en el mismo proyecto, está bien
        if let (Some(source_parent), Some(dest_parent)) = (source_canonical.parent(), dest_canonical.parent()) {
            if source_parent == dest_parent {
                return false; // Directorios hermanos son OK
            }
        }

        false
    }
    
    /// Verificar caracteres válidos en la ruta (excluyendo caracteres válidos de Windows)
    fn validate_path_characters(path: &Path) -> Result<(), String> {
        let path_str = path.to_string_lossy();

        // Caracteres realmente prohibidos en Windows (excluyendo : que es válido para drives)
        let invalid_chars = ['<', '>', '"', '|', '?', '*'];

        for ch in invalid_chars {
            if path_str.contains(ch) {
                return Err(format!("Carácter inválido '{}' en la ruta", ch));
            }
        }

        // Verificar que : solo aparezca en posición válida (drive letter)
        let colon_positions: Vec<usize> = path_str.match_indices(':').map(|(i, _)| i).collect();
        for &pos in &colon_positions {
            // : es válido solo después de una letra de drive (posición 1) o en rutas UNC
            if pos == 1 && path_str.chars().nth(0).map_or(false, |c| c.is_ascii_alphabetic()) {
                continue; // C:, D:, etc. son válidos
            }
            if path_str.starts_with("\\\\") {
                continue; // Rutas UNC pueden tener : en otros lugares
            }
            // Si llegamos aquí, es un : en posición inválida
            return Err("Carácter ':' en posición inválida".to_string());
        }

        Ok(())
    }
    
    /// Verificar si es ruta de red (UNC path)
    fn is_network_path(path: &Path) -> bool {
        path.to_string_lossy().starts_with("\\\\")
    }
    
    /// Validar ruta de red
    fn validate_network_path(path: &Path) -> Result<(), String> {
        let path_str = path.to_string_lossy();
        
        // Verificar formato UNC básico
        if !path_str.starts_with("\\\\") {
            return Err("Formato de ruta de red inválido".to_string());
        }
        
        // Verificar que tenga al menos servidor y share
        let parts: Vec<&str> = path_str.trim_start_matches("\\\\").split('\\').collect();
        if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err("Ruta de red debe tener formato \\\\servidor\\recurso".to_string());
        }
        
        Ok(())
    }
    
    /// Verificar permisos de lectura
    fn validate_read_permissions(path: &Path) -> Result<(), String> {
        match fs::read_dir(path) {
            Ok(_) => Ok(()),
            Err(_) => Err("Sin permisos de lectura en el directorio".to_string()),
        }
    }
    
    /// Verificar permisos de escritura
    fn validate_write_permissions(path: &Path) -> Result<(), String> {
        // Intentar crear un archivo temporal para verificar permisos
        let test_file = path.join(".rusty_vault_write_test");
        
        match fs::write(&test_file, b"test") {
            Ok(_) => {
                // Limpiar archivo de prueba
                let _ = fs::remove_file(&test_file);
                Ok(())
            }
            Err(_) => Err("Sin permisos de escritura en el directorio".to_string()),
        }
    }
    
    /// Verificar si es ruta crítica del sistema (solo rutas realmente peligrosas)
    fn is_critical_system_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        // Solo rutas realmente críticas del sistema
        let critical_paths = [
            "c:\\windows\\system32",
            "c:\\windows\\syswow64",
            "c:\\program files\\windows",
            "c:\\programdata\\microsoft\\windows",
            "c:\\system volume information",
            "c:\\$recycle.bin",
            "c:\\recovery",
            "c:\\boot",
            "c:\\efi",
        ];

        for critical_path in &critical_paths {
            if path_str.starts_with(critical_path) {
                return true;
            }
        }

        false
    }
}
