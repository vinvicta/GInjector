//! GInjector Application
//!
//! Main application state and UI rendering for the GS2 IDE.

use crate::config::ClientType;
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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

    // Status
    client_type: ClientType,
    frida_available: bool,
    process_running: bool,
    compiled_bytecode: Option<Vec<u8>>,

    // UI state
    show_about: bool,
    font_id: egui::FontId,

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

        // Create channel for status updates from background thread
        let (status_tx, status_rx) = mpsc::channel();

        // Spawn background thread for status checking (doesn't block UI)
        let client_type = config.client_type;
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
            client_type: config.client_type,
            frida_available: false,
            process_running: false,
            compiled_bytecode: None,
            show_about: false,
            font_id: egui::FontId::monospace(14.0),
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
        };

        // Convert bytecode to hex
        use frida_bridge::bytecode_to_hex;
        let hex = bytecode_to_hex(&bytecode);

        // Generate the Frida script
        let injector = FridaInjector::new(frida_client_type);
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
            ClientType::GraalWorlds => ClientType::GraalV6,
        };
        self.add_log(LogEntry::info(format!(
            "Switched to {}",
            self.client_type.name()
        )));
    }

    fn update_status_manual(&mut self) {
        // Manual refresh - update immediately
        self.frida_available = Self::check_frida_sync();
        self.process_running = Self::check_process_sync(self.client_type.target_process());
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
                    if ui.button("Toggle Client").clicked() {
                        self.toggle_client();
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
                    if ui.button(format!("Client: {}", self.client_type.name())).clicked() {
                        self.toggle_client();
                    }
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

        // Bottom panel for logs
        egui::TopBottomPanel::bottom("log_panel").default_height(150.0).max_height(150.0).show(ctx, |ui| {
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
                        ui.hyperlink_to("GitHub", "https://github.com/yourusername/graalhax");
                        if ui.button("Close").clicked() {
                            self.show_about = false;
                        }
                    });
                });
        }
    }
}
