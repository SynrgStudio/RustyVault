# 📋 **RustyVault - Product Requirements Document (PRD)**
## **Rust Implementation Specification**

---

## 📖 **1. PRODUCT OVERVIEW**

### **Product Information**
- **Product Name**: RustyVault
- **Target Platform**: Windows 10/11
- **Technology Stack**: Rust + egui/tauri
- **Migration From**: Python + Tkinter + pystray
- **Version**: 2.0 (Rust Implementation)

### **Product Description**
Una aplicación GUI **minimalista** para Windows que automatiza backups usando Robocopy. Interfaz súper simple con daemon en segundo plano, system tray integration, y tooltips explicativos para cada opción. Migrada a Rust para mayor estabilidad, performance y resolución de problemas de threading.

### **Key Benefits of Rust Migration**
- ✅ **Zero Dependencies**: Single executable, no runtime required
- ✅ **Native Threading**: Sin problemas de GIL o conflictos subprocess
- ✅ **Memory Efficiency**: ~10MB RAM vs ~100MB Python
- ✅ **Windows Integration**: Acceso directo a APIs sin wrappers
- ✅ **Startup Speed**: ~0.5s vs ~2-3s Python
- ✅ **Process Control**: CREATE_NO_WINDOW funciona perfectamente

---

## 🎯 **2. FUNCTIONAL REQUIREMENTS**

### **2.1 Core Backup System**

#### **2.1.1 Automated Backup Daemon**
```rust
// Rust Implementation Concept
use tokio::time::{sleep, Duration};

pub struct BackupDaemon {
    interval: Duration,
    source: PathBuf,
    destination: PathBuf,
    robocopy_config: RobocopyConfig,
    is_running: Arc<AtomicBool>,
}
```

**Requirements**:
- Daemon ejecutándose en async task separado
- Intervalos configurables: 1 segundo a 99,999,999 segundos
- Botones preset: 1h (3600s), 2h (7200s), 5h (18000s)
- Control Start/Stop con estado persistente
- Robocopy ejecutado sin ventana de consola (CREATE_NO_WINDOW)
- Thread-safe sin conflictos con system tray

#### **2.1.2 Manual Backup Execution**
**Requirements**:
- Botón "Run Backup Now"
- Ejecución inmediata en task separado
- UI responsiva durante backup
- Feedback visual y logging completo
- No interferencia con daemon activo

#### **2.1.3 Robocopy Integration**
```rust
use std::process::Command;

pub struct RobocopyConfig {
    pub mirror_mode: bool,           // /MIR
    pub multithreading: u8,          // /MT:X (1-128)
    pub fat_file_timing: bool,       // /FFT
    pub retry_count: u8,             // /R:X (0-1000000)
    pub retry_wait: u8,              // /W:X (0-300)
    pub additional_flags: Vec<String>, // Custom flags
}
```

**Current Default Parameters**:
- `/MIR`: Mirror mode (creates exact copy)
- `/MT:8`: Uses 8 threads for faster copying  
- `/FFT`: Uses FAT file timing (more compatible)
- `/R:3`: Retries 3 times on failed copies
- `/W:2`: Waits 2 seconds between retries

---

### **2.2 User Interface Requirements**

#### **2.2.1 Main Window Layout - SIMPLE UI**
```rust
// egui layout minimalista
impl eframe::App for BackupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Control básico del daemon
            // Folder Paths (Source & Destination)
            // Robocopy Settings con tooltips explicativos
            // Window controls (Minimize to Tray, Settings, Exit)
        });
    }
}
```

**Layout Simplificado**:
- **Daemon Control**: Start with Windows + Check interval + Start/Stop buttons
- **Folder Paths**: Source y Destination con Browse buttons
- **Robocopy Settings**: Configuración básica con tooltips MUY explicativos
- **Window Actions**: Minimize to Tray, Settings, Exit

#### **2.2.2 Folder Paths Section**
**Fields**:
- Source Folder: Entry + Browse button
- Destination Folder: Entry + Browse button  
- Log Folder: Entry + Browse button

**Validation**:
- Source must exist
- Destination auto-created if not exists
- Log folder auto-created if not exists
- Real-time validation feedback

#### **2.2.3 NEW: Robocopy Configuration Section - CON TOOLTIPS EXPLICATIVOS**
```rust
pub struct RobocopyUI {
    mirror_mode: bool,
    multithreading: u8,
    fat_file_timing: bool,
    retry_count: u8,
    retry_wait: u8,
}
```

**Layout Simplificado**:
```
┌─ Robocopy Settings ──────────────────────────────────┐
│ ☑ Mirror Mode [?]     ☑ FAT Timing [?]             │
│ Threads: [8] [?]      Retries: [3] [?]              │
│ Wait: [2] seconds [?]                                │
└──────────────────────────────────────────────────────┘
```

**Tooltips MUY Explicativos** (aparecen al hacer hover en [?]):
- **Mirror Mode [?]**: "/MIR - Modo Espejo: Crea copia EXACTA del origen.\n⚠️ ATENCIÓN: Elimina archivos del destino que no existan en origen.\nÚtil para: Backup completo idéntico"
- **Threads [?]**: "/MT:X - Hilos simultáneos para copiar archivos.\n💡 Más hilos = Más velocidad.\nRecomendado: 8 para HDD, 16+ para SSD"
- **FAT Timing [?]**: "/FFT - Compatibilidad con discos FAT32/exFAT.\n🔧 Soluciona problemas con USBs y NAS antiguos.\nRecomendado: Activar siempre"
- **Retries [?]**: "/R:X - Reintentos por archivo fallido.\n🔄 Por defecto robocopy reintenta 1,000,000 veces (!)\nRecomendado: 3-5 reintentos"
- **Wait [?]**: "/W:X - Segundos entre reintentos.\n⏱️ Por defecto robocopy espera 30 segundos (!)\nRecomendado: 2-5 segundos"

#### **2.2.4 Tooltip System - EDUCATIVO**
```rust
// Sistema de tooltips con egui
if ui.button("Mirror Mode").on_hover_text(MIRROR_MODE_TOOLTIP).clicked() {
    config.mirror_mode = !config.mirror_mode;
}

const MIRROR_MODE_TOOLTIP: &str = r#"/MIR - Modo Espejo: Crea una copia EXACTA del origen en destino.
⚠️ ATENCIÓN: Elimina archivos del destino que no existan en origen.
Útil para: Backup completo idéntico
Cuidado con: Puede borrar archivos si cambias la carpeta origen"#;
```

**Objetivo**: Cada parámetro de robocopy debe estar MUY bien explicado para usuarios no técnicos.
- **Iconos**: ⚠️ para warnings, 💡 para tips, 🔧 para técnico, 🔄 para repetición, ⏱️ para tiempo
- **Formato**: Explicación simple + Warning/Tip + Recomendación
- **Largo**: Máximo 4 líneas por tooltip para no abrumar

#### **2.2.5 Options and Control Section**
**Fields**:
- Check Interval: Entry + preset buttons (1h, 2h, 5h)
- Start with Windows: Checkbox
- Control Buttons: Start Daemon, Stop Daemon, Run Backup Now

#### **2.2.5 Activity Log Section**
**Features**:
- Scrollable text area
- Color-coded log levels (Info: Blue, Warning: Orange, Error: Red)
- Auto-scroll to latest
- Timestamp on all entries
- Thread-safe logging from daemon

---

### **2.3 Configuration Management**

#### **2.3.1 Configuration Structure**
```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub source_folder: PathBuf,
    pub destination_folder: PathBuf,
    pub log_folder: PathBuf,
    pub check_interval_seconds: u64,
    pub start_with_windows: bool,
    pub robocopy_config: RobocopyConfig, // NEW
}
```

#### **2.3.2 Configuration Persistence - SIMPLE**
**Requirements**:
- JSON format: `config.json` EN CARPETA DEL EJECUTABLE
- Auto-save on any change
- Load on startup with defaults fallback  
- Portable - archivo junto al .exe
- Migration from Python config format

**Estructura JSON Simple**:
```json
{
  "source_folder": "C:\\Users\\Documents",
  "destination_folder": "D:\\Backup\\Documents", 
  "check_interval_seconds": 3600,
  "start_with_windows": true,
  "robocopy": {
    "mirror_mode": true,
    "multithreading": 8,
    "fat_file_timing": true,
    "retry_count": 3,
    "retry_wait": 2
  }
}
```

#### **2.3.3 Windows Registry Integration**
```rust
use winreg::enums::*;
use winreg::RegKey;

pub fn set_windows_startup(enabled: bool, exe_path: &Path) -> Result<(), Box<dyn Error>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = hkcu.create_subkey(path)?;
    
    if enabled {
        let command = format!("\"{}\" --start-daemon", exe_path.display());
        key.set_value("RobocopyBackupTool", &command)?;
    } else {
        let _ = key.delete_value("RobocopyBackupTool");
    }
    Ok(())
}
```

---

### **2.4 System Integration**

#### **2.4.1 System Tray Integration**
```rust
use tray_icon::{TrayIcon, TrayIconBuilder, menu::Menu};

pub struct SystemTray {
    _icon: TrayIcon,
    app_handle: AppHandle,
}
```

**Requirements**:
- Native Windows system tray icon
- Context menu: "Open", "Stop Daemon", "Exit"
- Show/hide window on double-click
- Icon file: embedded in executable
- No threading conflicts with main app

#### **2.4.2 Window Management - OPCIÓN A (Botón Explícito)**
**Behaviors**:
- **[Minimize to Tray]** button → Explicit minimize to system tray
- **[X] Close** button → Real exit/close application  
- Restore from tray → Show window and remove tray icon
- Proper window state management
- Window icon from embedded resource

**UI Controls**:
- **[Minimize to Tray]**: Usuario consciente de la acción
- **[Settings]**: Acceso a configuración avanzada (futuro)
- **[Exit]**: Cierre real de la aplicación

#### **2.4.3 Command Line Interface**
**Arguments**:
- `--start-daemon`: Auto-start daemon on launch
- `--help`: Show help information
- No GUI mode for headless operation (future)

---

### **2.5 Logging System**

#### **2.5.1 Multi-target Logging**
```rust
use tracing::{info, warn, error, debug};
use tracing_subscriber;

pub struct Logger {
    file_appender: tracing_appender::rolling::RollingFileAppender,
    ui_sender: mpsc::UnboundedSender<LogMessage>,
}
```

**Targets**:
- **UI Widget**: All levels (Info, Warning, Error, Debug)
- **File**: Error level only (space saving)
- **Console**: All levels (development)

#### **2.5.2 Log File Management**
**Requirements**:
- File: `daemon_backup_ui.log` in log folder
- Rolling logs to prevent excessive size
- Timestamp format: `2025-01-07 13:45:23 - LEVEL - Message`
- Error-only file logging for production

---

### **2.6 Process Management**

#### **2.6.1 Robocopy Execution**
```rust
use std::process::{Command, Stdio};
use winapi::um::winbase::CREATE_NO_WINDOW;

pub async fn execute_robocopy(config: &RobocopyConfig, source: &Path, dest: &Path) -> Result<bool, BackupError> {
    let mut cmd = Command::new("robocopy");
    cmd.arg(source)
       .arg(dest)
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
       .creation_flags(CREATE_NO_WINDOW); // Works perfectly in Rust
       
    // Add configurable flags
    if config.mirror_mode {
        cmd.arg("/MIR");
    }
    cmd.arg(format!("/MT:{}", config.multithreading));
    // ... more flags
    
    let output = cmd.output().await?;
    Ok(is_robocopy_success(output.status.code()))
}
```

**Requirements**:
- Hidden execution (CREATE_NO_WINDOW)
- Capture stdout/stderr
- Proper Robocopy exit code handling (0-7 = success, 8+ = error)  
- Timeout handling for long operations
- No window flashing or user interruption

#### **2.6.2 Return Code Handling**
**Robocopy Exit Codes**:
- 0: No files copied, no errors
- 1: Files copied successfully  
- 2: Extra files in destination
- 3: Files copied + extra files
- 4-7: Various success with warnings
- 8+: Critical errors

---

## 🏗️ **3. TECHNICAL ARCHITECTURE**

### **3.1 Rust Dependencies**
```toml
[dependencies]
# GUI Framework
egui = "0.24"
eframe = "0.24"

# Async Runtime
tokio = { version = "1.0", features = ["full"] }

# System Integration  
winapi = { version = "0.3", features = ["processthreadsapi", "winbase"] }
winreg = "0.52"
tray-icon = "0.14"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-appender = "0.2"

# File System
dirs = "5.0"
```

### **3.2 Application Structure**
```rust
src/
├── main.rs              // Entry point, CLI handling
├── app.rs               // Main application state
├── ui/
│   ├── mod.rs
│   ├── main_window.rs   // Main GUI
│   ├── robocopy_config.rs // NEW: Robocopy config UI
│   └── components.rs    // Reusable UI components
├── core/
│   ├── mod.rs
│   ├── daemon.rs        // Backup daemon logic
│   ├── backup.rs        // Robocopy execution
│   └── config.rs        // Configuration management
├── system/
│   ├── mod.rs  
│   ├── tray.rs          // System tray integration
│   ├── registry.rs      // Windows registry
│   └── process.rs       // Process management
└── logging/
    ├── mod.rs
    └── logger.rs        // Multi-target logging
```

### **3.3 State Management**
```rust
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub daemon_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub is_daemon_running: Arc<AtomicBool>,
    pub log_sender: mpsc::UnboundedSender<LogMessage>,
}
```

---

## 🚀 **4. IMPLEMENTATION PLAN**

### **Phase 1: Core Framework (Week 1)**
- [ ] Project setup with Cargo
- [ ] Basic egui window and layout
- [ ] Configuration management
- [ ] Logging system setup

### **Phase 2: Backup Engine (Week 2)**  
- [ ] Robocopy execution with configurable parameters
- [ ] Daemon implementation with async tasks
- [ ] Process management and error handling

### **Phase 3: UI Implementation (Week 3)**
- [ ] Folder paths section
- [ ] NEW: Robocopy configuration section  
- [ ] Options and control section
- [ ] Activity log with real-time updates

### **Phase 4: System Integration (Week 4)**
- [ ] System tray implementation
- [ ] Windows registry integration
- [ ] Window management and state persistence
- [ ] Testing and optimization

---

## 🧪 **5. TESTING REQUIREMENTS**

### **5.1 Unit Tests**
- Robocopy parameter generation
- Configuration serialization/deserialization  
- Return code interpretation
- Path validation logic

### **5.2 Integration Tests**
- Daemon start/stop cycles
- Configuration persistence
- System tray integration
- Windows startup registration

### **5.3 User Acceptance Tests**
- Manual backup execution
- Automated backup scheduling
- UI responsiveness during operations
- System tray behavior
- Configuration changes persistence

---

## 📦 **6. DEPLOYMENT SPECIFICATION**

### **6.1 Build Configuration**
```toml
[profile.release]
opt-level = "s"          # Optimize for size
lto = true               # Link-time optimization
codegen-units = 1        # Better optimization
panic = "abort"          # Smaller binary
strip = true             # Remove debug symbols
```

### **6.2 Executable Packaging**
- Single executable (5-8MB estimated)
- Embedded icon resources
- No external dependencies
- Windows code signing (future)
- Installer package (future)

### **6.3 Migration Strategy**
- Configuration import from Python version
- Side-by-side installation capability
- User data preservation
- Graceful Python version detection and migration

---

## 🔍 **7. SUCCESS CRITERIA**

### **7.1 Performance Metrics**
- **Startup Time**: < 1 second
- **Memory Usage**: < 15MB steady state
- **CPU Usage**: < 1% when idle, < 5% during backup
- **Binary Size**: < 10MB

### **7.2 Reliability Metrics** 
- **Threading Stability**: No GIL issues, clean async/await
- **Process Isolation**: Robocopy execution without UI blocking
- **System Integration**: Stable system tray, no resource leaks
- **Configuration**: 100% persistence accuracy

### **7.3 Feature Completeness**
- ✅ All Python functionality replicated
- ✅ NEW: Configurable Robocopy parameters
- ✅ Enhanced Windows integration
- ✅ Improved error handling and logging
- ✅ Native performance and stability

---

## 📋 **8. RISK MITIGATION**

### **8.1 Technical Risks**
- **GUI Framework Learning Curve**: egui documentation and examples
- **Windows API Integration**: winapi crate and documentation
- **Async/Threading Complexity**: Tokio best practices

### **8.2 Migration Risks**
- **Feature Parity**: Comprehensive testing against Python version
- **User Data**: Safe configuration migration path
- **Deployment**: Thorough testing on various Windows versions

---

## 📚 **9. REFERENCE IMPLEMENTATION**

### **9.1 Python Feature Mapping**
| Python Component | Rust Equivalent | Status | Notes |
|------------------|-----------------|--------|-------|
| tkinter GUI | egui | ✅ | Better performance |
| pystray | tray-icon | ✅ | Native Windows API |
| threading | tokio async | ✅ | No GIL limitations |
| subprocess | std::process | ✅ | CREATE_NO_WINDOW works |
| json config | serde_json | ✅ | Type-safe serialization |
| winreg | winreg crate | ✅ | Direct Windows API |
| logging | tracing | ✅ | Structured logging |

### **9.2 New Features in Rust Version**
- 🆕 **Tooltips Educativos** - Explicaciones MUY claras de cada parámetro
- 🆕 **Interfaz Minimalista** - Solo controles esenciales, súper simple
- 🆕 **Botón Explícito "Minimize to Tray"** - Usuario consciente de la acción
- 🆕 **Configurable Robocopy Parameters** - Con tooltips explicativos
- 🆕 **Config JSON Simple** - En carpeta del ejecutable
- 🆕 **Native Performance and Stability** - Sin problemas de threading
- 🆕 **Zero External Dependencies** - Single executable

---

**Document Version**: 1.0  
**Last Updated**: January 2025  
**Next Review**: After Phase 1 completion 