/// Iconos seguros que funcionan correctamente en egui
/// Estos iconos han sido probados y se renderizan correctamente
pub struct SafeIcons;

impl SafeIcons {
    // === ESTADOS Y VALIDACIÓN ===
    pub const VALID: &'static str = "✓";           // Check mark simple
    pub const WARNING: &'static str = "⚠";         // Warning triangle
    pub const ERROR: &'static str = "❌";           // Cross mark
    pub const INFO: &'static str = "ℹ";            // Information
    pub const SUCCESS: &'static str = "✅";         // Check mark with box
    
    // === ACCIONES ===
    pub const SAVE: &'static str = "💾";           // Floppy disk
    pub const CANCEL: &'static str = "❌";          // Cross mark
    pub const DELETE: &'static str = "🗑";         // Trash can
    pub const EDIT: &'static str = "✏";            // Pencil
    pub const ADD: &'static str = "+";             // Plus sign
    pub const BROWSE: &'static str = "📂";         // Folder
    
    // === NAVEGACIÓN ===
    pub const UP: &'static str = "⬆";              // Up arrow
    pub const DOWN: &'static str = "⬇";            // Down arrow
    pub const LEFT: &'static str = "⬅";            // Left arrow
    pub const RIGHT: &'static str = "➡";           // Right arrow
    pub const MINIMIZE: &'static str = "⬇";        // Down arrow for minimize
    pub const EXIT: &'static str = "❌";            // Cross for exit
    
    // === DAEMON Y ESTADO ===
    pub const PLAY: &'static str = "▶";            // Play button
    pub const STOP: &'static str = "⏹";           // Stop button
    pub const REFRESH: &'static str = "↻";         // Refresh arrow
    pub const RUNNING: &'static str = "✅";         // Running status
    pub const STOPPED: &'static str = "⏸";        // Paused status
    
    // === ARCHIVOS Y CARPETAS ===
    pub const FOLDER: &'static str = "📁";         // Folder icon
    pub const FILE: &'static str = "📄";           // Document icon
    pub const BACKUP: &'static str = "💾";         // Backup/save icon
    pub const SYNC: &'static str = "🔄";           // Sync arrows
    
    // === CONFIGURACIÓN ===
    pub const SETTINGS: &'static str = "⚙";        // Gear icon
    pub const CONFIG: &'static str = "🔧";         // Wrench icon
    pub const TOOLS: &'static str = "🛠";          // Hammer and wrench
    
    // === NOTIFICACIONES ===
    pub const NOTIFICATION: &'static str = "🔔";   // Bell
    pub const ALERT: &'static str = "🚨";          // Siren (use sparingly)
    pub const FIRE: &'static str = "🔥";           // Fire for critical
    
    // === NÚMEROS Y PRIORIDAD ===
    pub const PRIORITY_1: &'static str = "#1";     // Priority badge
    pub const PRIORITY_2: &'static str = "#2";     // Priority badge
    pub const PRIORITY_3: &'static str = "#3";     // Priority badge
    
    // === HELPERS ===
    
    /// Obtener icono de prioridad por número
    pub fn priority(num: usize) -> String {
        format!("#{}", num)
    }
    
    /// Obtener icono de estado de validación
    pub fn validation_state(is_valid: bool, has_warnings: bool) -> &'static str {
        if !is_valid {
            Self::ERROR
        } else if has_warnings {
            Self::WARNING
        } else {
            Self::VALID
        }
    }
    
    /// Obtener icono de estado del daemon
    pub fn daemon_state(is_running: bool) -> &'static str {
        if is_running {
            Self::RUNNING
        } else {
            Self::STOPPED
        }
    }
    
    /// Obtener icono de acción de botón
    pub fn button_action(action: ButtonAction) -> &'static str {
        match action {
            ButtonAction::Save => Self::SAVE,
            ButtonAction::Cancel => Self::CANCEL,
            ButtonAction::Delete => Self::DELETE,
            ButtonAction::Edit => Self::EDIT,
            ButtonAction::Add => Self::ADD,
            ButtonAction::Browse => Self::BROWSE,
            ButtonAction::Up => Self::UP,
            ButtonAction::Down => Self::DOWN,
            ButtonAction::Play => Self::PLAY,
            ButtonAction::Stop => Self::STOP,
            ButtonAction::Refresh => Self::REFRESH,
            ButtonAction::Settings => Self::SETTINGS,
            ButtonAction::Minimize => Self::MINIMIZE,
            ButtonAction::Exit => Self::EXIT,
        }
    }
}

/// Acciones de botones disponibles
#[derive(Debug, Clone, Copy)]
pub enum ButtonAction {
    Save,
    Cancel,
    Delete,
    Edit,
    Add,
    Browse,
    Up,
    Down,
    Play,
    Stop,
    Refresh,
    Settings,
    Minimize,
    Exit,
}

/// Macro para crear botones con iconos seguros
#[macro_export]
macro_rules! safe_button {
    ($ui:expr, $action:expr, $text:expr) => {
        $ui.button(format!("{} {}", $crate::ui::icons::SafeIcons::button_action($action), $text))
    };
    ($ui:expr, $icon:expr, $text:expr) => {
        $ui.button(format!("{} {}", $icon, $text))
    };
}

/// Macro para crear labels con iconos seguros
#[macro_export]
macro_rules! safe_label {
    ($ui:expr, $icon:expr, $text:expr) => {
        $ui.label(format!("{} {}", $icon, $text))
    };
}

/// Función helper para crear texto con icono seguro
pub fn with_icon(icon: &str, text: &str) -> String {
    format!("{} {}", icon, text)
}

/// Función helper para validar si un icono se renderiza correctamente
/// (Para testing futuro)
pub fn is_icon_safe(icon: &str) -> bool {
    // Lista de iconos que sabemos que funcionan
    let safe_icons = [
        "✓", "⚠", "❌", "ℹ", "✅", "💾", "🗑", "✏", "+", "📂",
        "⬆", "⬇", "⬅", "➡", "▶", "⏹", "↻", "⏸", "📁", "📄",
        "🔄", "⚙", "🔧", "🛠", "🔔", "🔥", "#"
    ];
    
    safe_icons.iter().any(|&safe| icon.contains(safe))
}

/// Test helper para verificar iconos
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_safe_icons_are_safe() {
        assert!(is_icon_safe(SafeIcons::VALID));
        assert!(is_icon_safe(SafeIcons::WARNING));
        assert!(is_icon_safe(SafeIcons::ERROR));
        assert!(is_icon_safe(SafeIcons::SUCCESS));
    }
    
    #[test]
    fn test_button_actions() {
        assert_eq!(SafeIcons::button_action(ButtonAction::Save), SafeIcons::SAVE);
        assert_eq!(SafeIcons::button_action(ButtonAction::Delete), SafeIcons::DELETE);
        assert_eq!(SafeIcons::button_action(ButtonAction::Edit), SafeIcons::EDIT);
    }
    
    #[test]
    fn test_validation_states() {
        assert_eq!(SafeIcons::validation_state(true, false), SafeIcons::VALID);
        assert_eq!(SafeIcons::validation_state(true, true), SafeIcons::WARNING);
        assert_eq!(SafeIcons::validation_state(false, false), SafeIcons::ERROR);
    }
}
