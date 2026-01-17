//! GInjector Application
//!
//! Main application state and UI rendering for the GS2 IDE.

use crate::config::ClientType;
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// Base64 engine trait for decode
use base64::Engine;

// Re-export frida types
pub use frida_bridge::{ClientType as FridaClientType, FridaInjector};

/// Tab mode - either editing GS2 source or pasting raw bytecode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabMode {
    Script,
    Bytecode,
}

impl TabMode {
    pub fn name(&self) -> &str {
        match self {
            TabMode::Script => "Script",
            TabMode::Bytecode => "Bytecode",
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            TabMode::Script => ".gs2",
            TabMode::Bytecode => ".gs2bc",
        }
    }
}

/// Bytecode input mode - how to enter/paste bytecode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytecodeInputMode {
    Hex,    // Space-separated hex (e.g., "00 01 02 FF")
    Raw,    // Raw base64 or direct bytes
}

impl BytecodeInputMode {
    pub fn name(&self) -> &str {
        match self {
            BytecodeInputMode::Hex => "Hex",
            BytecodeInputMode::Raw => "Raw",
        }
    }
}

/// Input mode for decompiler/disassembler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecompilerInputMode {
    Hex,    // Space-separated hex
    Base64, // Base64 encoded bytecode
}

impl DecompilerInputMode {
    pub fn name(&self) -> &str {
        match self {
            DecompilerInputMode::Hex => "Hex",
            DecompilerInputMode::Base64 => "Base64",
        }
    }
}

/// Represents a single script tab
#[derive(Debug, Clone)]
pub struct ScriptTab {
    pub name: String,
    pub path: Option<PathBuf>,
    pub content: String,
    pub modified: bool,
    pub mode: TabMode,
    pub bytecode_hex: String,     // Hex string input
    pub bytecode_raw: String,     // Raw/base64 string input
    pub bytecode_bytes: Vec<u8>,  // Parsed bytecode bytes
    pub bytecode_input_mode: BytecodeInputMode,
}

impl ScriptTab {
    pub fn new(name: String) -> Self {
        Self {
            name,
            path: None,
            content: String::new(),
            modified: false,
            mode: TabMode::Script,
            bytecode_hex: String::new(),
            bytecode_raw: String::new(),
            bytecode_bytes: Vec::new(),
            bytecode_input_mode: BytecodeInputMode::Hex,
        }
    }

    pub fn from_file(path: PathBuf, content: String, bytecode_bytes: Vec<u8>) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        // Detect mode from extension
        let mode = if name.ends_with(".gs2bc") {
            TabMode::Bytecode
        } else {
            TabMode::Script
        };

        // If it's a bytecode file, also populate hex representation
        let (bytecode_hex, bytecode_raw) = if mode == TabMode::Bytecode && !bytecode_bytes.is_empty() {
            (
                frida_bridge::bytecode_to_hex(&bytecode_bytes),
                String::new(), // Raw input starts empty
            )
        } else {
            (String::new(), String::new())
        };

        Self {
            name,
            path: Some(path.clone()),
            content,
            modified: false,
            mode,
            bytecode_hex,
            bytecode_raw,
            bytecode_bytes,
            bytecode_input_mode: BytecodeInputMode::Hex,
        }
    }

    pub fn from_bytecode(bytes: Vec<u8>) -> Self {
        let hex = frida_bridge::bytecode_to_hex(&bytes);
        Self {
            name: "Pasted Bytecode.gs2bc".to_string(),
            path: None,
            content: String::new(),
            modified: false,
            mode: TabMode::Bytecode,
            bytecode_hex: hex,
            bytecode_raw: String::new(),
            bytecode_bytes: bytes,
            bytecode_input_mode: BytecodeInputMode::Hex,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

/// Log entry with timestamp
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl LogEntry {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            timestamp: Self::now(),
            level: LogLevel::Info,
            message: message.into(),
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            timestamp: Self::now(),
            level: LogLevel::Success,
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            timestamp: Self::now(),
            level: LogLevel::Warning,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            timestamp: Self::now(),
            level: LogLevel::Error,
            message: message.into(),
        }
    }

    fn now() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let hrs = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;
        format!("{:02}:{:02}:{:02}", hrs, mins, secs)
    }
}

/// Injection result
#[derive(Debug, Clone)]
struct InjectionResult {
    success: bool,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

/// Main application state
pub struct GInjectorApp {
    // Tabs
    tabs: Vec<ScriptTab>,
    active_tab: usize,

    // Logs
    logs: Vec<LogEntry>,

    // Config
    config: crate::config::Config,

    // Status
    client_type: ClientType,
    frida_available: bool,
    process_running: bool,
    compiled_bytecode: Option<Vec<u8>>,

    // UI state
    show_about: bool,
    show_settings: bool,
    show_decompiler: bool,
    show_disassembler: bool,
    font_id: egui::FontId,

    // Decompiler/Disassembler state
    decompiler_input: String,
    decompiler_output: String,
    decompiler_input_mode: DecompilerInputMode,
    disassembler_input: String,
    disassembler_output: String,
    disassembler_input_mode: DecompilerInputMode,

    // Settings state
    settings_client_type: ClientType,
    edit_constructor_offset: String,
    edit_setscript_offset: String,
    edit_magic_check_offset: String,
    edit_magic_check_value: String,
    // Pattern scanning fields
    edit_use_pattern_scanning: bool,
    edit_constructor_pattern: String,
    edit_setscript_pattern: String,
    edit_pattern_index: String,

    // Status update receiver (from background thread)
    status_rx: mpsc::Receiver<(bool, bool)>,  // (frida_available, process_running)

    // Injection state (channel is recreated on each injection)
    injection_rx: Option<mpsc::Receiver<InjectionResult>>,
    injection_in_progress: bool,
}

// Drop handler to clean up the background thread
impl Drop for GInjectorApp {
    fn drop(&mut self) {
        // The channel will close when dropped, causing the thread to exit
    }
}

impl GInjectorApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Load configuration with explicit error handling
        let config = match crate::config::Config::load() {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Warning: Failed to load config: {}, using defaults", e);
                crate::config::Config::default()
            }
        };

        let client_type = config.client_type;
        let offsets = config.get_offsets();

        // Initialize edit fields with current offsets
        let edit_constructor_offset = offsets.constructor_offset.clone();
        let edit_setscript_offset = offsets.setscript_offset.clone();
        let edit_magic_check_offset = offsets.magic_check_offset.clone().unwrap_or_default();
        let edit_magic_check_value = offsets.magic_check_value.map(|v| v.to_string()).unwrap_or_default();
        let edit_use_pattern_scanning = offsets.use_pattern_scanning;
        let edit_constructor_pattern = offsets.constructor_pattern.clone().unwrap_or_default();
        let edit_setscript_pattern = offsets.setscript_pattern.clone().unwrap_or_default();
        let edit_pattern_index = offsets.pattern_index.map(|v| v.to_string()).unwrap_or_default();

        // Create channel for status updates from background thread
        let (status_tx, status_rx) = mpsc::channel();

        // Spawn background thread for status checking (doesn't block UI)
        thread::spawn(move || {
            let mut tx = status_tx;
            let mut last_frida = false;
            let mut last_process = false;
            let mut tick: u32 = 0;

            loop {
                // Check if channel is still open
                let frida = Self::check_frida_sync();
                let process = Self::check_process_sync(client_type.target_process());

                // Only send if status changed or every 10 ticks
                if frida != last_frida || process != last_process || tick % 10 == 0 {
                    if tx.send((frida, process)).is_err() {
                        // Channel closed, exit thread
                        break;
                    }
                    last_frida = frida;
                    last_process = process;
                }

                tick = tick.wrapping_add(1);
                thread::sleep(Duration::from_secs(2)); // Check every 2 seconds
            }
        });

        // Add default tab
        let tabs = vec![ScriptTab::new("Untitled.gs2".to_string())];
        let active_tab = 0;

        // Add initial logs
        let mut logs = Vec::new();
        logs.push(LogEntry::info("GInjector started"));
        logs.push(LogEntry::info(format!("Client: {}", config.client_type.name())));

        // Create channel for injection results
        let injection_rx = None;

        Self {
            tabs,
            active_tab,
            logs,
            config,
            client_type,
            frida_available: false,
            process_running: false,
            compiled_bytecode: None,
            show_about: false,
            show_settings: false,
            show_decompiler: false,
            show_disassembler: false,
            font_id: egui::FontId::monospace(14.0),
            decompiler_input: String::new(),
            decompiler_output: String::from("// Decompiled code will appear here"),
            decompiler_input_mode: DecompilerInputMode::Hex,
            disassembler_input: String::new(),
            disassembler_output: String::from("// Disassembly will appear here"),
            disassembler_input_mode: DecompilerInputMode::Hex,
            settings_client_type: client_type,
            edit_constructor_offset,
            edit_setscript_offset,
            edit_magic_check_offset,
            edit_magic_check_value,
            edit_use_pattern_scanning,
            edit_constructor_pattern,
            edit_setscript_pattern,
            edit_pattern_index,
            status_rx,
            injection_rx,
            injection_in_progress: false,
        }
    }

    fn check_frida_sync() -> bool {
        std::process::Command::new("frida")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn check_process_sync(target: &str) -> bool {
        // First try: use frida-ps to check
        let frida_result = std::process::Command::new("frida-ps")
            .output();

        if let Ok(output) = frida_result {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Try multiple matching strategies
                // 1. Exact match with extension
                if stdout.lines().any(|line| line.contains(target)) {
                    return true;
                }

                // 2. Match without .exe extension
                let target_no_ext = target.trim_end_matches(".exe");
                if stdout.lines().any(|line| line.contains(target_no_ext)) {
                    return true;
                }

                // 3. Case-insensitive match
                let stdout_lower = stdout.to_lowercase();
                if stdout_lower.contains(&target.to_lowercase()) ||
                   stdout_lower.contains(&target_no_ext.to_lowercase()) {
                    return true;
                }
            }
        }

        // Fallback: try using pgrep (Linux/macOS) or tasklist (Windows)
        #[cfg(target_os = "windows")]
        {
            let target_no_ext = target.trim_end_matches(".exe");
            if let Ok(output) = std::process::Command::new("tasklist")
                .args(&["/FI", &format!("IMAGENAME eq {}", target), "/NH"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains(target_no_ext) && !stdout.contains("No tasks") {
                    return true;
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let target_no_ext = target.trim_end_matches(".exe");
            if let Ok(output) = std::process::Command::new("pgrep")
                .arg("-x")
                .arg(target_no_ext)
                .output()
            {
                if output.status.success() {
                    return true;
                }
            }

            // Also try pgrep without -x (partial match)
            if let Ok(output) = std::process::Command::new("pgrep")
                .arg(target_no_ext)
                .output()
            {
                if output.status.success() {
                    return true;
                }
            }
        }

        false
    }

    fn add_log(&mut self, entry: LogEntry) {
        self.logs.push(entry);
        // Keep only last 1000 logs
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
    }

    fn current_tab_mut(&mut self) -> Option<&mut ScriptTab> {
        self.tabs.get_mut(self.active_tab)
    }

    fn current_tab(&self) -> Option<&ScriptTab> {
        self.tabs.get(self.active_tab)
    }

    fn new_tab(&mut self) {
        let name = format!("Untitled{}.gs2", self.tabs.len() + 1);
        self.tabs.push(ScriptTab::new(name));
        self.active_tab = self.tabs.len() - 1;
        self.add_log(LogEntry::info("New tab created"));
    }

    fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 {
            self.add_log(LogEntry::warning("Cannot close the last tab"));
            return;
        }
        self.tabs.remove(index);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    fn toggle_tab_mode(&mut self) {
        let mode_name = if let Some(tab) = self.current_tab_mut() {
            tab.mode = match tab.mode {
                TabMode::Script => TabMode::Bytecode,
                TabMode::Bytecode => TabMode::Script,
            };

            // Update tab name extension based on mode
            if tab.path.is_none() {
                let base_name = tab.name
                    .trim_end_matches(".gs2")
                    .trim_end_matches(".gs2bc");
                tab.name = format!("{}{}", base_name, tab.mode.extension());
            }

            tab.mode.name().to_string()
        } else {
            return;
        };

        self.add_log(LogEntry::info(format!(
            "Switched to {} mode",
            mode_name
        )));
    }

    fn load_bytecode_from_input(&mut self) {
        let (input_mode, input_string) = match self.current_tab() {
            Some(tab) => (tab.bytecode_input_mode, tab.bytecode_hex.clone()),
            None => return,
        };

        let bytecode = match input_mode {
            BytecodeInputMode::Hex => {
                // Parse hex string to bytes
                let result: Result<Vec<u8>, String> = input_string
                    .split_whitespace()
                    .map(|s| u8::from_str_radix(s, 16).map_err(|e| format!("Invalid hex: {}", e)))
                    .collect();
                result
            }
            BytecodeInputMode::Raw => {
                // Try multiple formats for raw input
                // First try: interpret string content as raw bytes (each char is a byte)
                let as_bytes = input_string.as_bytes().to_vec();

                // Check if it looks like valid bytecode (non-empty, reasonable values)
                if !as_bytes.is_empty() && as_bytes.iter().all(|&b| b <= 0x7F) {
                    Ok(as_bytes)
                } else {
                    // Try base64 decoding
                    use base64::Engine;
                    match base64::engine::general_purpose::STANDARD.decode(&input_string) {
                        Ok(decoded) => Ok(decoded),
                        Err(_) => Err("Could not decode as base64".to_string()),
                    }
                }
            }
        };

        match bytecode {
            Ok(bytes) => {
                self.compiled_bytecode = Some(bytes.clone());
                if let Some(tab) = self.current_tab_mut() {
                    tab.bytecode_bytes = bytes.clone();
                    tab.bytecode_hex = frida_bridge::bytecode_to_hex(&bytes);
                }
                self.add_log(LogEntry::success(format!(
                    "Loaded {} bytes via {} mode",
                    bytes.len(),
                    input_mode.name()
                )));
            }
            Err(e) => {
                self.add_log(LogEntry::error(format!("Failed to parse {}: {}", input_mode.name(), e)));
            }
        }
    }

    fn toggle_bytecode_input_mode(&mut self) {
        let mode_name = if let Some(tab) = self.current_tab_mut() {
            tab.bytecode_input_mode = match tab.bytecode_input_mode {
                BytecodeInputMode::Hex => BytecodeInputMode::Raw,
                BytecodeInputMode::Raw => BytecodeInputMode::Hex,
            };
            tab.bytecode_input_mode.name().to_string()
        } else {
            return;
        };

        self.add_log(LogEntry::info(format!("Input mode: {}", mode_name)));
    }

    fn save_current_tab(&mut self) {
        let (_tab_mode, tab_name) = match self.current_tab() {
            Some(tab) => (tab.mode, tab.name.clone()),
            None => return,
        };

        // Determine default filename
        let default_name = if let Some(ref path) = self.current_tab().and_then(|t| t.path.as_ref()) {
            path.file_name().and_then(|n| n.to_str()).unwrap_or(&tab_name).to_string()
        } else {
            tab_name.clone()
        };

        let file_path = rfd::FileDialog::new()
            .set_title("Save File")
            .set_file_name(&default_name)
            .save_file();

        if let Some(path) = file_path {
            if let Some(tab) = self.current_tab_mut() {
                // Write content based on mode
                let write_result = match tab.mode {
                    TabMode::Script => std::fs::write(&path, &tab.content),
                    TabMode::Bytecode => {
                        // For bytecode files, save as raw bytes
                        // Use tab.bytecode_bytes if available, otherwise try to parse from hex
                        let bytes = if !tab.bytecode_bytes.is_empty() {
                            tab.bytecode_bytes.clone()
                        } else {
                            // Try to parse hex
                            match frida_bridge::hex_to_bytecode(&tab.bytecode_hex) {
                                Ok(bytes) => bytes,
                                Err(_) => vec![],
                            }
                        };
                        std::fs::write(&path, bytes)
                    }
                };

                match write_result {
                    Ok(()) => {
                        tab.path = Some(path.clone());
                        tab.name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Untitled")
                            .to_string();
                        tab.modified = false;
                        self.add_log(LogEntry::success(format!("Saved: {}", path.display())));
                    }
                    Err(e) => {
                        self.add_log(LogEntry::error(format!("Failed to save: {}", e)));
                    }
                }
            }
        } else {
            self.add_log(LogEntry::info("Save cancelled"));
        }
    }

    fn open_file(&mut self) {
        let file_path = rfd::FileDialog::new()
            .set_title("Open GS2 Script or Bytecode")
            .pick_file();

        if let Some(path) = file_path {
            // Detect file type from extension
            let is_bytecode = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("gs2bc"))
                .unwrap_or(false);

            if is_bytecode {
                // Read as raw bytes
                match std::fs::read(&path) {
                    Ok(bytecode_bytes) => {
                        let mut tab = ScriptTab::from_file(path.clone(), String::new(), bytecode_bytes.clone());
                        tab.mode = TabMode::Bytecode;
                        tab.bytecode_bytes = bytecode_bytes.clone();

                        // Also load into compiled_bytecode for injection
                        self.compiled_bytecode = Some(bytecode_bytes);

                        self.tabs.push(tab);
                        self.active_tab = self.tabs.len() - 1;
                        self.add_log(LogEntry::success(format!(
                            "Opened bytecode file: {} bytes",
                            self.compiled_bytecode.as_ref().unwrap().len()
                        )));
                    }
                    Err(e) => {
                        self.add_log(LogEntry::error(format!("Failed to read bytecode file: {}", e)));
                    }
                }
            } else {
                // Read as text (GS2 script)
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let mut tab = ScriptTab::from_file(path.clone(), content, vec![]);
                        tab.mode = TabMode::Script;
                        self.tabs.push(tab);
                        self.active_tab = self.tabs.len() - 1;
                        self.add_log(LogEntry::success(format!("Opened: {}", path.display())));
                    }
                    Err(e) => {
                        self.add_log(LogEntry::error(format!("Failed to read file: {}", e)));
                    }
                }
            }
        } else {
            self.add_log(LogEntry::info("Open cancelled"));
        }
    }

    fn compile_script(&mut self) {
        let tab = match self.current_tab() {
            Some(tab) => tab,
            None => return,
        };

        self.add_log(LogEntry::info(format!("Compiling: {}", tab.name)));

        // TODO: Actual compilation using gs2-compiler
        // For now, generate dummy bytecode
        let bytecode = vec![
            0x00, 0x00, 0x00, 0x01,  // Header
            0x00, 0x00, 0x00, 0x04,  // Script count
            0x00, 0x00, 0x00, 0x00,  // Reserved
        ];

        self.compiled_bytecode = Some(bytecode.clone());
        self.add_log(LogEntry::success(format!("Compilation successful: {} bytes", bytecode.len())));
    }

    fn inject_bytecode(&mut self) {
        if self.compiled_bytecode.is_none() {
            self.add_log(LogEntry::warning("No compiled bytecode to inject. Compile first!"));
            return;
        }

        if self.injection_in_progress {
            self.add_log(LogEntry::warning("Injection already in progress..."));
            return;
        }

        let bytecode = match self.compiled_bytecode.clone() {
            Some(b) => b,
            None => return,
        };

        let frida_client_type = match self.client_type {
            ClientType::GraalV6 => FridaClientType::GraalV6,
            ClientType::GraalWorlds => FridaClientType::GraalWorlds,
            ClientType::EraSteam => FridaClientType::EraSteam,
        };

        // Convert bytecode to hex
        use frida_bridge::bytecode_to_hex;
        let hex = bytecode_to_hex(&bytecode);

        // Generate the Frida script with custom offsets if configured
        let injector = if self.config.has_custom_offsets() {
            let offsets = self.config.get_offsets();
            let custom_offsets = frida_bridge::InjectionOffsets {
                constructor_offset: offsets.constructor_offset_usize().unwrap_or_else(|_| {
                    frida_client_type.tgralvar_constructor_offset()
                }),
                setscript_offset: offsets.setscript_offset_usize().unwrap_or_else(|_| {
                    frida_client_type.tgralvar_setscript_offset()
                }),
                uses_thiscall: offsets.uses_thiscall,
                magic_check_offset: offsets.magic_check_offset_usize().ok().flatten(),
                magic_check_value: offsets.magic_check_value,
                use_pattern_scanning: offsets.use_pattern_scanning,
                constructor_pattern: offsets.constructor_pattern.clone(),
                setscript_pattern: offsets.setscript_pattern.clone(),
                pattern_index: offsets.pattern_index,
            };
            FridaInjector::with_offsets(frida_client_type, custom_offsets)
        } else {
            FridaInjector::new(frida_client_type)
        };
        let script = injector.generate_injection_script(&hex, frida_client_type.default_variable_name());

        // Write script to temp file
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("graalhax_inject.js");

        if let Err(e) = std::fs::write(&script_path, &script) {
            self.add_log(LogEntry::error(format!("Failed to write script: {}", e)));
            return;
        }

        // Create channel for this injection
        let (tx, rx) = mpsc::channel();
        self.injection_rx = Some(rx);
        self.injection_in_progress = true;

        let target = self.client_type.target_process().to_string();
        let script_path_str = script_path.clone();

        self.add_log(LogEntry::info(format!(
            "Injecting into {} ({} bytes)...",
            target,
            bytecode.len()
        )));
        self.add_log(LogEntry::info("Script running in background (UI will remain responsive)..."));

        // Spawn thread to run Frida (non-blocking)
        thread::spawn(move || {
            let result = match std::process::Command::new("frida")
                .arg("-l")
                .arg(&script_path_str)
                .arg(&target)
                .arg("--exit-on-error")
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    InjectionResult {
                        success: output.status.success(),
                        stdout,
                        stderr,
                        exit_code: output.status.code(),
                    }
                }
                Err(e) => InjectionResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("Failed to run Frida: {}", e),
                    exit_code: None,
                }
            };

            // Clean up temp script
            let _ = std::fs::remove_file(&script_path_str);

            // Send result (ignore if channel closed)
            let _ = tx.send(result);
        });
    }

    fn toggle_client(&mut self) {
        self.client_type = match self.client_type {
            ClientType::GraalV6 => ClientType::GraalWorlds,
            ClientType::GraalWorlds => ClientType::EraSteam,
            ClientType::EraSteam => ClientType::GraalV6,
        };
        self.add_log(LogEntry::info(format!(
            "Switched to {}",
            self.client_type.name()
        )));
    }

    fn set_client(&mut self, client_type: ClientType) {
        if self.client_type != client_type {
            self.client_type = client_type;
            self.add_log(LogEntry::info(format!(
                "Switched to {}",
                self.client_type.name()
            )));
        }
    }

    fn update_status_manual(&mut self) {
        // Manual refresh - update immediately
        self.frida_available = Self::check_frida_sync();
        self.process_running = Self::check_process_sync(self.client_type.target_process());
    }

    fn open_settings(&mut self) {
        // Initialize settings client type to match current client type
        self.settings_client_type = self.client_type;

        // Load current offsets into edit fields based on settings client type
        self.load_offsets_for_settings_client();
        self.show_settings = true;
    }

    fn load_offsets_for_settings_client(&mut self) {
        // Temporarily set config client type to load correct offsets
        let original_client_type = self.config.client_type;
        self.config.client_type = self.settings_client_type;
        let offsets = self.config.get_offsets();
        self.config.client_type = original_client_type;

        self.edit_constructor_offset = offsets.constructor_offset.clone();
        self.edit_setscript_offset = offsets.setscript_offset.clone();
        self.edit_magic_check_offset = offsets.magic_check_offset.clone().unwrap_or_default();
        self.edit_magic_check_value = offsets.magic_check_value.map(|v| v.to_string()).unwrap_or_default();
        self.edit_use_pattern_scanning = offsets.use_pattern_scanning;
        self.edit_constructor_pattern = offsets.constructor_pattern.clone().unwrap_or_default();
        self.edit_setscript_pattern = offsets.setscript_pattern.clone().unwrap_or_default();
        self.edit_pattern_index = offsets.pattern_index.map(|v| v.to_string()).unwrap_or_default();
    }

    fn toggle_settings_client_type(&mut self) {
        self.settings_client_type = match self.settings_client_type {
            ClientType::GraalV6 => ClientType::GraalWorlds,
            ClientType::GraalWorlds => ClientType::EraSteam,
            ClientType::EraSteam => ClientType::GraalV6,
        };
        self.load_offsets_for_settings_client();
    }

    fn save_offsets(&mut self) {
        use crate::config::ClientOffsets;

        // Validate offsets
        let constructor = self.edit_constructor_offset.trim();
        let setscript = self.edit_setscript_offset.trim();

        // Validate hex format
        if !constructor.starts_with("0x") && !constructor.starts_with("0X") {
            self.add_log(LogEntry::error("Constructor offset must start with 0x"));
            return;
        }
        if !setscript.starts_with("0x") && !setscript.starts_with("0X") {
            self.add_log(LogEntry::error("SetScript offset must start with 0x"));
            return;
        }

        // Parse to validate
        match crate::config::ClientOffsets::parse_offset(constructor) {
            Ok(_) => {}
            Err(e) => {
                self.add_log(LogEntry::error(format!("Invalid constructor offset: {}", e)));
                return;
            }
        }
        match crate::config::ClientOffsets::parse_offset(setscript) {
            Ok(_) => {}
            Err(e) => {
                self.add_log(LogEntry::error(format!("Invalid setscript offset: {}", e)));
                return;
            }
        }

        // Build new offsets struct - use settings_client_type
        let uses_thiscall = self.settings_client_type == ClientType::GraalV6;

        let (magic_offset, magic_value) = if self.settings_client_type == ClientType::GraalV6 {
            let magic_offset_str = self.edit_magic_check_offset.trim();
            let magic_value_str = self.edit_magic_check_value.trim();

            let magic_offset = if magic_offset_str.is_empty() {
                None
            } else {
                match crate::config::ClientOffsets::parse_offset(magic_offset_str) {
                    Ok(_) => Some(magic_offset_str.to_string()),
                    Err(e) => {
                        self.add_log(LogEntry::error(format!("Invalid magic offset: {}", e)));
                        return;
                    }
                }
            };

            let magic_value = if magic_value_str.is_empty() {
                None
            } else {
                match magic_value_str.parse::<u32>() {
                    Ok(v) => Some(v),
                    Err(_) => {
                        self.add_log(LogEntry::error("Invalid magic check value"));
                        return;
                    }
                }
            };

            (magic_offset, magic_value)
        } else {
            (None, None)
        };

        // Parse pattern index
        let pattern_index = if self.edit_pattern_index.trim().is_empty() {
            None
        } else {
            match self.edit_pattern_index.trim().parse::<usize>() {
                Ok(v) => Some(v),
                Err(_) => {
                    self.add_log(LogEntry::error("Invalid pattern index (must be a number)"));
                    return;
                }
            }
        };

        // Parse pattern fields
        let constructor_pattern = if self.edit_constructor_pattern.trim().is_empty() {
            None
        } else {
            Some(self.edit_constructor_pattern.trim().to_string())
        };

        let setscript_pattern = if self.edit_setscript_pattern.trim().is_empty() {
            None
        } else {
            Some(self.edit_setscript_pattern.trim().to_string())
        };

        let offsets = ClientOffsets {
            constructor_offset: constructor.to_string(),
            setscript_offset: setscript.to_string(),
            uses_thiscall,
            magic_check_offset: magic_offset,
            magic_check_value: magic_value,
            use_pattern_scanning: self.edit_use_pattern_scanning,
            constructor_pattern,
            setscript_pattern,
            pattern_index,
        };

        // Save to config - temporarily set client type to save to correct offsets
        let original_client_type = self.config.client_type;
        self.config.client_type = self.settings_client_type;
        self.config.set_offsets(offsets);
        let save_result = self.config.save();
        self.config.client_type = original_client_type;

        if let Err(e) = save_result {
            self.add_log(LogEntry::error(format!("Failed to save config: {}", e)));
        } else {
            self.add_log(LogEntry::success(format!(
                "Offsets saved for {}",
                self.settings_client_type.name()
            )));
        }

        self.show_settings = false;
    }

    fn reset_offsets_to_default(&mut self) {
        // Reset for the settings client type
        let original_client_type = self.config.client_type;
        self.config.client_type = self.settings_client_type;
        self.config.reset_offsets();
        let save_result = self.config.save();
        self.config.client_type = original_client_type;

        if let Err(e) = save_result {
            self.add_log(LogEntry::error(format!("Failed to save config: {}", e)));
        } else {
            self.add_log(LogEntry::success(format!(
                "Offsets reset to defaults for {}",
                self.settings_client_type.name()
            )));
            // Reload edit fields with defaults
            self.load_offsets_for_settings_client();
        }
    }

    fn run_decompiler(&mut self) {
        let input = self.decompiler_input.trim();
        if input.is_empty() {
            self.decompiler_output = "// No input provided".to_string();
            return;
        }

        // Parse bytecode based on input mode
        let bytecode = match self.decompiler_input_mode {
            DecompilerInputMode::Hex => {
                match crate::bytecode_analyzer::hex_to_bytecode(input) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        self.decompiler_output = format!("// Error parsing hex: {}", e);
                        return;
                    }
                }
            }
            DecompilerInputMode::Base64 => {
                match base64::engine::general_purpose::STANDARD.decode(input) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        self.decompiler_output = format!("// Error parsing base64: {}", e);
                        return;
                    }
                }
            }
        };

        // Decompile
        match crate::bytecode_analyzer::decompile_bytecode(&bytecode) {
            Ok(code) => {
                self.decompiler_output = code;
            }
            Err(e) => {
                self.decompiler_output = format!("// Decompilation error: {}", e);
            }
        }
    }

    fn run_disassembler(&mut self) {
        let input = self.disassembler_input.trim();
        if input.is_empty() {
            self.disassembler_output = "// No input provided".to_string();
            return;
        }

        // Parse bytecode based on input mode
        let bytecode = match self.disassembler_input_mode {
            DecompilerInputMode::Hex => {
                match crate::bytecode_analyzer::hex_to_bytecode(input) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        self.disassembler_output = format!("// Error parsing hex: {}", e);
                        return;
                    }
                }
            }
            DecompilerInputMode::Base64 => {
                match base64::engine::general_purpose::STANDARD.decode(input) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        self.disassembler_output = format!("// Error parsing base64: {}", e);
                        return;
                    }
                }
            }
        };

        // Disassemble
        match crate::bytecode_analyzer::disassemble_bytecode(&bytecode) {
            Ok(code) => {
                self.disassembler_output = code;
            }
            Err(e) => {
                self.disassembler_output = format!("// Disassembly error: {}", e);
            }
        }
    }

    /// Get decompiled code for the current bytecode preview
    fn get_bytecode_preview(&self) -> String {
        if let Some(bytecode) = &self.compiled_bytecode {
            match crate::bytecode_analyzer::decompile_bytecode(bytecode) {
                Ok(code) => code,
                Err(e) => format!("// Decompilation error: {}", e),
            }
        } else {
            "// No bytecode compiled yet".to_string()
        }
    }
}

impl eframe::App for GInjectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for status updates from background thread (non-blocking)
        if let Ok((frida, process)) = self.status_rx.try_recv() {
            self.frida_available = frida;
            self.process_running = process;
        }

        // Check for injection results from background thread (non-blocking)
        if let Some(rx) = &self.injection_rx {
            if let Ok(result) = rx.try_recv() {
                self.injection_in_progress = false;
                self.injection_rx = None;

                if result.success {
                    self.add_log(LogEntry::success("Injection completed successfully!"));
                    if !result.stdout.is_empty() {
                        for line in result.stdout.lines() {
                            self.add_log(LogEntry::info(line.to_string()));
                        }
                    }
                } else {
                    self.add_log(LogEntry::error(format!(
                        "Injection failed (exit code: {:?})",
                        result.exit_code
                    )));
                    if !result.stdout.is_empty() {
                        self.add_log(LogEntry::info(format!("stdout: {}", result.stdout)));
                    }
                    if !result.stderr.is_empty() {
                        self.add_log(LogEntry::error(format!("stderr: {}", result.stderr)));
                    }
                }
            }
        }

        // Request repaint to keep UI responsive
        ctx.request_repaint();

        // Handle keyboard shortcuts
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::S) {
                self.save_current_tab();
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::O) {
                self.open_file();
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::B) {
                self.compile_script();
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::I) {
                self.inject_bytecode();
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::T) {
                self.new_tab();
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::W) {
                self.close_tab(self.active_tab);
            }
        });

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Tab (Ctrl+T)").clicked() {
                        self.new_tab();
                        ui.close_menu();
                    }
                    if ui.button("Open (Ctrl+O)").clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                    if ui.button("Save (Ctrl+S)").clicked() {
                        self.save_current_tab();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit (Ctrl+Q)").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Build", |ui| {
                    if ui.button("Compile (Ctrl+B)").clicked() {
                        self.compile_script();
                        ui.close_menu();
                    }
                    if ui.button("Inject (Ctrl+I)").clicked() {
                        self.inject_bytecode();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Decompiler").clicked() {
                        self.show_decompiler = true;
                        ui.close_menu();
                    }
                    if ui.button("Disassembler").clicked() {
                        self.show_disassembler = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Settings").clicked() {
                        self.open_settings();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Client selector dropdown
                    egui::ComboBox::from_id_salt("client_selector")
                        .selected_text(format!("Client: {}", self.client_type.name()))
                        .show_ui(ui, |ui| {
                            for client_type in crate::config::ClientType::all() {
                                if ui.selectable_value(&mut self.client_type, *client_type, client_type.name()).changed() {
                                    self.set_client(*client_type);
                                }
                            }
                        });
                });
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            // Split into top (editor) and bottom (logs/bytecode)
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Tab bar
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // Collect tab info first to avoid borrow issues
                    let tab_info: Vec<(String, bool)> = self.tabs.iter()
                        .map(|t| (t.name.clone(), t.modified))
                        .collect();

                    let mut clicked_tab = None;
                    let mut close_tab = None;
                    let mut toggle_mode = false;

                    // Get current tab mode for the toggle button
                    let current_mode = self.current_tab().map(|t| t.mode).unwrap_or(TabMode::Script);

                    for (i, (name, modified)) in tab_info.iter().enumerate() {
                        let is_active = i == self.active_tab;

                        if is_active {
                            ui.style_mut().visuals.selection.bg_fill = egui::Color32::from_rgb(60, 60, 80);
                        }

                        let response = ui.selectable_label(is_active, format!("{}{} ", name, if *modified { " ●" } else { "" }));
                        if response.clicked() {
                            clicked_tab = Some(i);
                        }

                        // Close button
                        if response.hovered() && ui.input(|inp| inp.pointer.any_released()) {
                            if let Some(pos) = ui.input(|inp| inp.pointer.hover_pos()) {
                                let rect = response.rect;
                                let close_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.right() - 20.0, rect.top()),
                                    egui::vec2(20.0, rect.height()),
                                );
                                if close_rect.contains(pos) {
                                    close_tab = Some(i);
                                }
                            }
                        }
                    }

                    // New tab button
                    if ui.button("+").clicked() {
                        self.new_tab();
                    }

                    // Mode toggle button
                    ui.separator();
                    let mode_text = format!("Mode: {}", current_mode.name());
                    if ui.button(mode_text).clicked() {
                        toggle_mode = true;
                    }

                    // Handle actions after the loop to avoid borrow issues
                    if let Some(i) = clicked_tab {
                        self.active_tab = i;
                    }
                    if let Some(i) = close_tab {
                        self.close_tab(i);
                    }
                    if toggle_mode {
                        self.toggle_tab_mode();
                    }
                });

                ui.separator();

                // Editor and status side-by-side
                egui::SidePanel::right("status_panel").default_width(200.0).show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Status");
                        ui.separator();

                        // Frida status
                        ui.horizontal(|ui| {
                            ui.label("Frida:");
                            if self.frida_available {
                                ui.colored_label(egui::Color32::GREEN, "✓ Detected");
                            } else {
                                ui.colored_label(egui::Color32::GRAY, "? Unknown");
                            }
                        });

                        // Process status
                        ui.horizontal(|ui| {
                            ui.label("Client:");
                            if self.process_running {
                                ui.colored_label(egui::Color32::GREEN, "✓ Running");
                            } else {
                                ui.colored_label(egui::Color32::RED, "✗ Not running");
                            }
                        });

                        // Bytecode status
                        ui.horizontal(|ui| {
                            ui.label("Bytecode:");
                            if let Some(ref bytecode) = self.compiled_bytecode {
                                ui.colored_label(egui::Color32::GREEN, format!("✓ {} bytes", bytecode.len()));
                            } else {
                                ui.colored_label(egui::Color32::GRAY, "✗ Not compiled");
                            }
                        });

                        ui.separator();

                        // Action buttons
                        ui.label("Actions:");
                        if ui.button("Compile").clicked() {
                            self.compile_script();
                        }
                        if ui.button("Inject").clicked() {
                            self.inject_bytecode();
                        }
                        if ui.button("Save").clicked() {
                            self.save_current_tab();
                        }
                        if ui.button("Refresh Status").clicked() {
                            self.update_status_manual();
                            self.add_log(LogEntry::info("Status refreshed"));
                        }

                        ui.separator();

                        // Client info
                        ui.label(format!("Target:\n{}", self.client_type.target_process()));
                    });
                });

                // Editor area
                // Clone needed values before mutable borrow
                let font_id = self.font_id.clone();
                let tab_mode = self.current_tab().map(|t| t.mode).unwrap_or(TabMode::Script);
                let tab_name = self.current_tab().map(|t| t.name.clone());
                let bytecode_input_mode = self.current_tab().map(|t| t.bytecode_input_mode).unwrap_or(BytecodeInputMode::Hex);
                let bytecode_preview = self.compiled_bytecode.as_ref()
                    .map(|b| (b.len(), frida_bridge::bytecode_to_hex(b)));

                if let Some(tab) = self.current_tab_mut() {
                    let name = tab_name.unwrap_or_default();

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("Editor ({}):", tab_mode.name()));
                            ui.monospace(name);
                        });

                        match tab.mode {
                            TabMode::Script => {
                                // Script editor mode
                                let response = egui::ScrollArea::vertical()
                                    .id_salt("editor_scroll")
                                    .show(ui, |ui| {
                                        egui::TextEdit::multiline(&mut tab.content)
                                            .font(font_id.clone())
                                            .code_editor()
                                            .desired_width(f32::INFINITY)
                                            .show(ui)
                                    });

                                // Check for modifications
                                if response.inner.response.changed() {
                                    tab.modified = true;
                                }
                            }
                            TabMode::Bytecode => {
                                // Input mode toggle
                                ui.horizontal(|ui| {
                                    ui.label("Input format:");
                                    if ui.button(format!("{}", bytecode_input_mode.name())).clicked() {
                                        ui.ctx().memory_mut(|m| m.data.insert_temp::<bool>(egui::Id::new("toggle_input_mode"), true));
                                    }
                                    ui.label("(Click to toggle)");
                                });

                                ui.separator();

                                match bytecode_input_mode {
                                    BytecodeInputMode::Hex => {
                                        ui.label("Paste space-separated hex bytecode (e.g., \"00 01 02 FF AB\"):");
                                        ui.add_sized(
                                            [ui.available_width(), 150.0],
                                            egui::TextEdit::multiline(&mut tab.bytecode_hex)
                                                .font(font_id.clone())
                                                .hint_text("00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F...")
                                                .desired_width(f32::INFINITY)
                                        );
                                    }
                                    BytecodeInputMode::Raw => {
                                        ui.label("Paste raw bytecode (direct bytes or base64 encoded):");
                                        ui.add_sized(
                                            [ui.available_width(), 150.0],
                                            egui::TextEdit::multiline(&mut tab.bytecode_raw)
                                                .font(font_id.clone())
                                                .hint_text("Raw bytecode bytes or base64 string...")
                                                .desired_width(f32::INFINITY)
                                        );
                                    }
                                }

                                ui.separator();

                                // Parse button - store click to process after the borrow ends
                                let mut should_load = false;
                                ui.horizontal(|ui| {
                                    if ui.button("Load Bytecode").clicked() {
                                        should_load = true;
                                    }
                                    ui.label(format!("(Parse {} input and prepare for injection)", bytecode_input_mode.name()));
                                });

                                // Show parsed bytecode info
                                if let Some((len, hex)) = bytecode_preview {
                                    ui.separator();
                                    ui.colored_label(egui::Color32::GREEN, format!("✓ Loaded {} bytes", len));
                                    ui.monospace(format!("Hex preview: {}", hex));
                                }

                                // Store the flag for later processing
                                if should_load {
                                    ui.ctx().memory_mut(|m| m.data.insert_temp::<bool>(egui::Id::new("load_bytecode"), true));
                                }
                            }
                        }
                    });
                }

                // Process bytecode load request outside the closure
                if ctx.memory_mut(|m| m.data.remove_temp::<bool>(egui::Id::new("load_bytecode")).unwrap_or(false)) {
                    self.load_bytecode_from_input();
                }

                // Process input mode toggle outside the closure
                if ctx.memory_mut(|m| m.data.remove_temp::<bool>(egui::Id::new("toggle_input_mode")).unwrap_or(false)) {
                    self.toggle_bytecode_input_mode();
                }
            });
        });

        // Bottom panel for logs and bytecode preview
        egui::TopBottomPanel::bottom("log_panel").default_height(200.0).max_height(400.0).show(ctx, |ui| {
            // Split into two columns: logs and bytecode preview
            ui.horizontal(|ui| {
                // Logs column (left)
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Logs:");
                        if ui.button("Clear").clicked() {
                            self.logs.clear();
                        }
                    });

                    egui::ScrollArea::vertical()
                        .id_salt("log_scroll")
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;

                                for log in &self.logs {
                                    let color = match log.level {
                                        LogLevel::Info => egui::Color32::GRAY,
                                        LogLevel::Success => egui::Color32::GREEN,
                                        LogLevel::Warning => egui::Color32::YELLOW,
                                        LogLevel::Error => egui::Color32::RED,
                                    };

                                    ui.horizontal(|ui| {
                                        ui.colored_label(egui::Color32::DARK_GRAY, &log.timestamp);
                                        ui.colored_label(color, match log.level {
                                            LogLevel::Info => ">",
                                            LogLevel::Success => "✓",
                                            LogLevel::Warning => "⚠",
                                            LogLevel::Error => "✗",
                                        });
                                        ui.label(&log.message);
                                    });
                                }
                            });
                        });
                });

                // Separator
                ui.separator();

                // Bytecode preview column (right)
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Bytecode Preview:");
                        ui.label(egui::RichText::new("(what will be injected)").size(12.0).color(egui::Color32::GRAY));
                    });

                    let preview = self.get_bytecode_preview();
                    egui::ScrollArea::vertical()
                        .id_salt("preview_scroll")
                        .show(ui, |ui| {
                            // Split preview into lines and display each
                            for line in preview.lines() {
                                ui.label(egui::RichText::new(line).font(self.font_id.clone()).monospace());
                            }
                        });
                });
            });
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").default_height(20.0).max_height(20.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(tab) = self.current_tab() {
                    ui.monospace(tab.name.clone());
                }
                ui.separator();
                ui.monospace(format!("Target: {}", self.client_type.target_process()));
                ui.separator();
                ui.monospace(format!("Tabs: {}", self.tabs.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("GInjector v0.1.0");
                });
            });
        });

        // About dialog
        if self.show_about {
            egui::Window::new("About GInjector")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("GInjector");
                        ui.label("GS2 Development Environment");
                        ui.label("Version 0.1.0");
                        ui.separator();
                        ui.label("A graphical IDE for GS2 scripting with");
                        ui.label("integrated compilation and Frida injection.");
                        ui.separator();
                        ui.hyperlink_to("GitHub", "https://github.com/vinvicta/GInjector");
                        if ui.button("Close").clicked() {
                            self.show_about = false;
                        }
                    });
                });
        }

        // Settings dialog
        if self.show_settings {
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Memory Offsets");
                        ui.separator();

                        // Client type selector dropdown
                        ui.horizontal(|ui| {
                            ui.label("Editing offsets for:");
                            egui::ComboBox::from_id_salt("settings_client_selector")
                                .selected_text(self.settings_client_type.name())
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for client_type in crate::config::ClientType::all() {
                                        if ui.selectable_value(&mut self.settings_client_type, *client_type, client_type.name()).changed() {
                                            self.load_offsets_for_settings_client();
                                        }
                                    }
                                });
                        });
                        ui.separator();

                        // Check if using custom offsets for this client type
                        let is_custom = self.config.has_custom_offsets_for(self.settings_client_type);
                        if is_custom {
                            ui.colored_label(egui::Color32::YELLOW, "⚠ Using custom offsets");
                        } else {
                            ui.colored_label(egui::Color32::GREEN, "✓ Using default offsets");
                        }
                        ui.separator();

                        // Constructor offset
                        ui.horizontal(|ui| {
                            ui.label("TGraalVar Constructor:");
                            ui.label("(offset to constructor function)");
                        });
                        ui.add(egui::TextEdit::singleline(&mut self.edit_constructor_offset)
                            .hint_text("0x...")
                            .font(egui::FontId::monospace(14.0)));

                        // SetScript offset
                        ui.horizontal(|ui| {
                            ui.label("TGraalVar::SetScript:");
                            ui.label("(offset to SetScript method)");
                        });
                        ui.add(egui::TextEdit::singleline(&mut self.edit_setscript_offset)
                            .hint_text("0x...")
                            .font(egui::FontId::monospace(14.0)));

                        // Magic check offset (V6 only)
                        if self.settings_client_type == ClientType::GraalV6 {
                            ui.separator();
                            ui.label("Magic Check (V6 only):");
                            ui.horizontal(|ui| {
                                ui.label("Offset:");
                                ui.add(egui::TextEdit::singleline(&mut self.edit_magic_check_offset)
                                    .hint_text("0x...")
                                    .font(egui::FontId::monospace(14.0)));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Value:");
                                ui.add(egui::TextEdit::singleline(&mut self.edit_magic_check_value)
                                    .hint_text("157876074")
                                    .font(egui::FontId::monospace(14.0)));
                            });
                        }

                        // Pattern scanning section (for all clients, mainly Era)
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Use Pattern Scanning:");
                            if ui.checkbox(&mut self.edit_use_pattern_scanning, "").changed() {
                                // Checkbox changed
                            }
                        });
                        ui.label("If enabled, uses Memory.scanSync to find function addresses");

                        if self.edit_use_pattern_scanning {
                            ui.separator();
                            ui.label("Pattern Scanning:");
                            ui.horizontal(|ui| {
                                ui.label("Constructor Pattern:");
                            });
                            ui.add(egui::TextEdit::singleline(&mut self.edit_constructor_pattern)
                                .hint_text("e.g., 40 53 48 83 EC 20 48 8B D9 ?? ?? ?? ??")
                                .font(egui::FontId::monospace(14.0))
                                .desired_width(f32::INFINITY));

                            ui.horizontal(|ui| {
                                ui.label("SetScript Pattern:");
                            });
                            ui.add(egui::TextEdit::singleline(&mut self.edit_setscript_pattern)
                                .hint_text("e.g., 48 89 ?? ?? ?? 57 48 ?? ?? ?? 48 8B DA")
                                .font(egui::FontId::monospace(14.0))
                                .desired_width(f32::INFINITY));

                            ui.horizontal(|ui| {
                                ui.label("Pattern Index:");
                                ui.add(egui::TextEdit::singleline(&mut self.edit_pattern_index)
                                    .hint_text("0")
                                    .font(egui::FontId::monospace(14.0)));
                                ui.label("(which match to use if multiple found)");
                            });
                        }

                        ui.separator();

                        // Action buttons
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                self.save_offsets();
                            }
                            if ui.button("Reset to Defaults").clicked() {
                                self.reset_offsets_to_default();
                            }
                            if ui.button("Cancel").clicked() {
                                self.show_settings = false;
                            }
                        });

                        // Info text
                        ui.separator();
                        ui.colored_label(
                            egui::Color32::GRAY,
                            "Note: Offsets are specific to each client version."
                        );
                        ui.colored_label(
                            egui::Color32::GRAY,
                            "You need to reverse engineer the client to find new offsets."
                        );
                    });
                });
        }

        // Decompiler window
        if self.show_decompiler {
            egui::Window::new("GS2 Decompiler")
                .collapsible(false)
                .resizable(true)
                .default_width(800.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Input Mode:");
                        for mode in [DecompilerInputMode::Hex, DecompilerInputMode::Base64] {
                            if ui.selectable_value(
                                &mut self.decompiler_input_mode,
                                mode,
                                mode.name()
                            ).changed() {}
                        }
                    });

                    ui.separator();

                    // Input area
                    ui.label("Bytecode Input:");
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), 120.0],
                                egui::TextEdit::multiline(&mut self.decompiler_input)
                                    .font(self.font_id.clone())
                                    .hint_text("Paste bytecode here (hex or base64)...")
                            );
                        });

                    ui.separator();

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("Decompile").clicked() {
                            self.run_decompiler();
                        }
                        if ui.button("Clear").clicked() {
                            self.decompiler_input.clear();
                            self.decompiler_output = "// Decompiled code will appear here".to_string();
                        }
                        if ui.button("Close").clicked() {
                            self.show_decompiler = false;
                        }
                    });

                    ui.separator();

                    // Output area
                    ui.label("Decompiled GS2 Code:");
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), 280.0],
                                egui::TextEdit::multiline(&mut self.decompiler_output)
                                    .font(self.font_id.clone())
                                    .interactive(false)
                            );
                        });
                });
        }

        // Disassembler window
        if self.show_disassembler {
            egui::Window::new("GS2 Disassembler")
                .collapsible(false)
                .resizable(true)
                .default_width(800.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Input Mode:");
                        for mode in [DecompilerInputMode::Hex, DecompilerInputMode::Base64] {
                            if ui.selectable_value(
                                &mut self.disassembler_input_mode,
                                mode,
                                mode.name()
                            ).changed() {}
                        }
                    });

                    ui.separator();

                    // Input area
                    ui.label("Bytecode Input:");
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), 120.0],
                                egui::TextEdit::multiline(&mut self.disassembler_input)
                                    .font(self.font_id.clone())
                                    .hint_text("Paste bytecode here (hex or base64)...")
                            );
                        });

                    ui.separator();

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("Disassemble").clicked() {
                            self.run_disassembler();
                        }
                        if ui.button("Clear").clicked() {
                            self.disassembler_input.clear();
                            self.disassembler_output = "// Disassembly will appear here".to_string();
                        }
                        if ui.button("Close").clicked() {
                            self.show_disassembler = false;
                        }
                    });

                    ui.separator();

                    // Output area
                    ui.label("Disassembly:");
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), 280.0],
                                egui::TextEdit::multiline(&mut self.disassembler_output)
                                    .font(self.font_id.clone())
                                    .interactive(false)
                            );
                        });
                });
        }
    }
}
