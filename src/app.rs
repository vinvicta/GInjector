//! Application state management

use crate::config::{Config, ClientType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use frida_bridge::FridaInjector;

/// Current mode of the editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    Visual,
}

/// Frida connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FridaStatus {
    Disconnected,
    Attached,
    Injecting,
    Error,
}

/// Compilation result
#[derive(Debug, Clone)]
pub struct CompilationResult {
    pub success: bool,
    pub bytecode: Vec<u8>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub timestamp: u64,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub message: String,
    pub level: LogLevel,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// Tab in the editor
#[derive(Debug, Clone)]
pub struct Tab {
    pub name: String,
    pub path: Option<PathBuf>,
    pub content: String,
    pub modified: bool,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub scroll_offset: usize,
}

impl Tab {
    pub fn new(name: String, content: String) -> Self {
        Self {
            name,
            path: None,
            content,
            modified: false,
            cursor_line: 0,
            cursor_column: 0,
            scroll_offset: 0,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string();
        Ok(Self {
            name,
            path: Some(path),
            content,
            modified: false,
            cursor_line: 0,
            cursor_column: 0,
            scroll_offset: 0,
        })
    }

    pub fn lines(&self) -> Vec<&str> {
        self.content.lines().collect()
    }
}

/// Main application state
pub struct App {
    /// Configuration
    pub config: Config,

    /// Editor mode
    pub mode: EditorMode,

    /// Open tabs
    pub tabs: Vec<Tab>,

    /// Current tab index
    pub current_tab: usize,

    /// Frida status
    pub frida_status: FridaStatus,

    /// Last compilation result
    pub compilation_result: Option<CompilationResult>,

    /// Log entries
    pub logs: Vec<LogEntry>,

    /// Status message
    pub status_message: String,

    /// Should quit
    pub should_quit: bool,

    /// Input buffer (for commands, search, etc.)
    pub input_buffer: String,

    /// Show input popup
    pub show_input: bool,
}

impl App {
    pub fn new() -> Self {
        let config = Config::load().unwrap_or_default();

        Self {
            config,
            mode: EditorMode::Normal,
            tabs: vec![Tab::new(
                ["untitled", ".", "gs2"].join(""),
                String::new(),
            )],
            current_tab: 0,
            frida_status: FridaStatus::Disconnected,
            compilation_result: None,
            logs: Vec::new(),
            status_message: "Welcome to GraalHax TUI".to_string(),
            should_quit: false,
            input_buffer: String::new(),
            show_input: false,
        }
    }

    /// Get current tab
    pub fn current_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.current_tab)
    }

    /// Get mutable current tab
    pub fn current_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.current_tab)
    }

    /// Add a log entry
    pub fn log(&mut self, message: String, level: LogLevel) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.logs.push(LogEntry {
            message,
            level,
            timestamp,
        });
        // Keep only last 1000 logs
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
    }

    /// Handle key event
    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        if self.show_input {
            return self.handle_input_key(key);
        }

        match self.mode {
            EditorMode::Normal => self.handle_normal_key(key).await,
            EditorMode::Insert => self.handle_insert_key(key).await,
            EditorMode::Visual => self.handle_visual_key(key).await,
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Enter => {
                // Process input
                self.show_input = false;
                self.input_buffer.clear();
            }
            KeyCode::Esc => {
                self.show_input = false;
                self.input_buffer.clear();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            _ => {}
        }
        Ok(true)
    }

    async fn handle_normal_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return Ok(false);
            }
            KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.inject_script().await?;
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.compile_script().await?;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_current_tab()?;
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_file()?;
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.new_tab();
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.close_tab();
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_client_type();
            }
            KeyCode::Char('i') | KeyCode::Enter => {
                self.mode = EditorMode::Insert;
                self.status_message = "INSERT MODE".to_string();
            }
            KeyCode::Char(':') => {
                self.show_input = true;
                self.input_buffer = ":".to_string();
            }
            // Arrow key navigation
            KeyCode::Up => {
                self.move_cursor(-1, 0);
            }
            KeyCode::Down => {
                self.move_cursor(1, 0);
            }
            KeyCode::Left => {
                self.move_cursor(0, -1);
            }
            KeyCode::Right => {
                self.move_cursor(0, 1);
            }
            KeyCode::Tab => {
                // Next tab
                if self.tabs.len() > 1 {
                    self.current_tab = (self.current_tab + 1) % self.tabs.len();
                }
            }
            KeyCode::BackTab => {
                // Previous tab
                if self.tabs.len() > 1 {
                    self.current_tab = if self.current_tab == 0 {
                        self.tabs.len() - 1
                    } else {
                        self.current_tab - 1
                    };
                }
            }
            _ => {}
        }
        Ok(true)
    }

    async fn handle_insert_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.status_message = "NORMAL MODE".to_string();
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Enter => {
                self.insert_newline();
            }
            KeyCode::Backspace => {
                self.backspace();
            }
            KeyCode::Delete => {
                self.delete();
            }
            KeyCode::Tab => {
                self.insert_tab();
            }
            // Arrow key navigation in insert mode
            KeyCode::Up => {
                self.move_cursor(-1, 0);
            }
            KeyCode::Down => {
                self.move_cursor(1, 0);
            }
            KeyCode::Left => {
                self.move_cursor(0, -1);
            }
            KeyCode::Right => {
                self.move_cursor(0, 1);
            }
            _ => {}
        }
        Ok(true)
    }

    async fn handle_visual_key(&mut self, _key: KeyEvent) -> Result<bool> {
        // For now, just exit visual mode
        self.mode = EditorMode::Normal;
        Ok(true)
    }

    fn insert_char(&mut self, c: char) {
        if let Some(tab) = self.current_tab_mut() {
            let lines: Vec<&str> = tab.content.lines().collect();
            let current_line = lines.get(tab.cursor_line).unwrap_or(&"");

            let mut new_line = String::with_capacity(current_line.len() + 1);
            if tab.cursor_column <= current_line.len() {
                new_line.push_str(&current_line[..tab.cursor_column]);
            }
            new_line.push(c);
            if tab.cursor_column < current_line.len() {
                new_line.push_str(&current_line[tab.cursor_column..]);
            }

            let mut new_content = String::new();
            for (i, line) in lines.iter().enumerate() {
                if i == tab.cursor_line {
                    new_content.push_str(&new_line);
                } else {
                    new_content.push_str(line);
                }
                if i < lines.len() - 1 {
                    new_content.push('\n');
                }
            }
            if lines.is_empty() {
                new_content.push(c);
            }

            tab.content = new_content;
            tab.cursor_column += 1;
            tab.modified = true;
        }
    }

    fn insert_newline(&mut self) {
        if let Some(tab) = self.current_tab_mut() {
            let lines: Vec<&str> = tab.content.lines().collect();
            let current_line = lines.get(tab.cursor_line).unwrap_or(&"");

            let before = if tab.cursor_column <= current_line.len() {
                &current_line[..tab.cursor_column]
            } else {
                current_line
            };
            let after = if tab.cursor_column < current_line.len() {
                &current_line[tab.cursor_column..]
            } else {
                ""
            };

            let mut new_content = String::new();
            for (i, line) in lines.iter().enumerate() {
                if i == tab.cursor_line {
                    new_content.push_str(before);
                    new_content.push('\n');
                    new_content.push_str(after);
                } else {
                    new_content.push_str(line);
                }
                if i < lines.len() - 1 {
                    new_content.push('\n');
                }
            }

            tab.content = new_content;
            tab.cursor_line += 1;
            tab.cursor_column = 0;
            tab.modified = true;
        }
    }

    fn backspace(&mut self) {
        if let Some(tab) = self.current_tab_mut() {
            let lines: Vec<&str> = tab.content.lines().collect();
            let current_line = lines.get(tab.cursor_line).unwrap_or(&"");

            if tab.cursor_column > 0 {
                let mut new_line = String::from(*current_line);
                new_line.remove(tab.cursor_column - 1);

                let mut new_content = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if i == tab.cursor_line {
                        new_content.push_str(&new_line);
                    } else {
                        new_content.push_str(line);
                    }
                    if i < lines.len() - 1 {
                        new_content.push('\n');
                    }
                }

                tab.content = new_content;
                tab.cursor_column -= 1;
                tab.modified = true;
            } else if tab.cursor_line > 0 {
                // Join with previous line
                let prev_line = lines.get(tab.cursor_line - 1).unwrap_or(&"");
                let new_column = prev_line.len();

                let mut new_content = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if i == tab.cursor_line - 1 {
                        new_content.push_str(prev_line);
                        new_content.push_str(current_line);
                    } else if i != tab.cursor_line {
                        new_content.push_str(line);
                    }
                    if i < lines.len() - 1 && i != tab.cursor_line - 1 {
                        new_content.push('\n');
                    }
                }

                tab.content = new_content;
                tab.cursor_line -= 1;
                tab.cursor_column = new_column;
                tab.modified = true;
            }
        }
    }

    fn delete(&mut self) {
        if let Some(tab) = self.current_tab_mut() {
            let lines: Vec<&str> = tab.content.lines().collect();
            let current_line = lines.get(tab.cursor_line).unwrap_or(&"");

            if tab.cursor_column < current_line.len() {
                let mut new_line = String::from(*current_line);
                new_line.remove(tab.cursor_column);

                let mut new_content = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if i == tab.cursor_line {
                        new_content.push_str(&new_line);
                    } else {
                        new_content.push_str(line);
                    }
                    if i < lines.len() - 1 {
                        new_content.push('\n');
                    }
                }

                tab.content = new_content;
                tab.modified = true;
            } else if tab.cursor_line < lines.len() - 1 {
                // Join with next line
                let next_line = lines.get(tab.cursor_line + 1).map_or("", |v| *v);

                let mut new_content = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if i == tab.cursor_line {
                        new_content.push_str(current_line);
                        new_content.push_str(next_line);
                    } else if i != tab.cursor_line + 1 {
                        new_content.push_str(line);
                    }
                    if i < lines.len() - 1 && i != tab.cursor_line && i != tab.cursor_line + 1 {
                        new_content.push('\n');
                    }
                }

                tab.content = new_content;
                tab.modified = true;
            }
        }
    }

    fn insert_tab(&mut self) {
        for _ in 0..self.config.editor.tab_width {
            self.insert_char(' ');
        }
    }

    fn move_cursor(&mut self, line_delta: i32, column_delta: i32) {
        if let Some(tab) = self.current_tab_mut() {
            let lines: Vec<&str> = tab.content.lines().collect();
            let max_line = lines.len().saturating_sub(1);

            let new_line = if line_delta >= 0 {
                (tab.cursor_line as i32 + line_delta).min(max_line as i32) as usize
            } else {
                (tab.cursor_line as i32 + line_delta).max(0) as usize
            };

            let current_line = lines.get(new_line).map(|s| s.len()).unwrap_or(0);
            let new_column = if column_delta >= 0 {
                (tab.cursor_column as i32 + column_delta).min(current_line as i32) as usize
            } else {
                (tab.cursor_column as i32 + column_delta).max(0) as usize
            };

            tab.cursor_line = new_line;
            tab.cursor_column = new_column;

            // Update scroll offset
            if tab.cursor_line >= tab.scroll_offset + 20 {
                tab.scroll_offset = tab.cursor_line.saturating_sub(15);
            } else if tab.cursor_line < tab.scroll_offset {
                tab.scroll_offset = tab.cursor_line;
            }
        }
    }

    fn new_tab(&mut self) {
        let num = self.tabs.len();
        let name = ["untitled", &num.to_string(), ".", "gs2"].join("");
        self.tabs.push(Tab::new(name, String::new()));
        self.current_tab = self.tabs.len() - 1;
    }

    fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.current_tab);
            if self.current_tab >= self.tabs.len() {
                self.current_tab = self.tabs.len() - 1;
            }
        }
    }

    fn toggle_client_type(&mut self) {
        self.config.client_type = match self.config.client_type {
            ClientType::GraalV6 => {
                ClientType::GraalWorlds
            }
            ClientType::GraalWorlds => {
                ClientType::GraalV6
            }
        };
        self.frida_status = FridaStatus::Disconnected;
        self.log(
            format!("Switched to {} client", self.config.client_type.name()),
            LogLevel::Info,
        );
        self.status_message = format!("Client: {}", self.config.client_type.name());
    }

    fn save_current_tab(&mut self) -> Result<()> {
        // Check if tab has a path first
        let has_path = self.current_tab().map(|t| t.path.is_some()).unwrap_or(false);
        let path_str = self.current_tab().and_then(|t| t.path.as_ref().map(|p| p.display().to_string()));

        if has_path {
            if let Some(tab) = self.current_tab_mut() {
                if let Some(path) = &tab.path {
                    std::fs::write(path, &tab.content)?;
                    tab.modified = false;
                }
            }
            if let Some(ps) = path_str {
                self.log(["Saved: ", &ps].join(""), LogLevel::Success);
            }
        } else {
            // Save as new file
            if let Some(tab) = self.current_tab() {
                if tab.path.is_none() {
                    self.show_input = true;
                    self.input_buffer = "save-as: ".to_string();
                }
            }
        }
        Ok(())
    }

    fn open_file(&mut self) -> Result<()> {
        // For now, open a default test file
        let test_path = ["gs2-parser/scripts/syntax-test", ".", "txt"].join("");
        if let Ok(tab) = Tab::from_file(PathBuf::from(&test_path)) {
            self.tabs.push(tab);
            self.current_tab = self.tabs.len() - 1;
            self.log(["Opened: ", &test_path].join(""), LogLevel::Info);
        }
        Ok(())
    }

    async fn compile_script(&mut self) -> Result<()> {
        // Clone content before borrowing for log
        let tab_content = if let Some(tab) = self.current_tab() {
            tab.content.clone()
        } else {
            self.log("No tab open".to_string(), LogLevel::Error);
            return Ok(());
        };

        self.log("Compiling script...".to_string(), LogLevel::Info);

        // Write to temp file
        let temp_dir = std::env::temp_dir();
        let temp_file = ["graalhax_temp", ".", "gs2"].join("");
        let temp_script = temp_dir.join(&temp_file);
        std::fs::write(&temp_script, &tab_content)?;

        // Run the compiler
        let compiler_path = self.config.gs2_compiler_path.clone();
        let bytecode_file = ["graalhax_temp", ".", "gs2bc"].join("");
        let output = tokio::process::Command::new(&compiler_path)
            .arg(&temp_script)
            .arg("-o")
            .arg(temp_dir.join(&bytecode_file))
            .output()
            .await?;

        if output.status.success() {
            // Read bytecode
            let bytecode_path = temp_dir.join(&bytecode_file);
            let bytecode = std::fs::read(&bytecode_path)?;
            let bytecode_len = bytecode.len();

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            self.compilation_result = Some(CompilationResult {
                success: true,
                bytecode,
                errors: Vec::new(),
                warnings: Vec::new(),
                timestamp,
            });

            self.log(
                ["Compilation successful: ", &bytecode_len.to_string(), " bytes"].join(""),
                LogLevel::Success,
            );
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            self.log(["Compilation failed: ", &error_msg].join(""), LogLevel::Error);
            self.compilation_result = Some(CompilationResult {
                success: false,
                bytecode: Vec::new(),
                errors: vec![error_msg.to_string()],
                warnings: Vec::new(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }

        // Clean up temp files
        let _ = std::fs::remove_file(temp_script);
        let _ = std::fs::remove_file(temp_dir.join(&bytecode_file));

        Ok(())
    }

    async fn inject_script(&mut self) -> Result<()> {
        // Compile first if needed
        if self.compilation_result.is_none() || !self.compilation_result.as_ref().unwrap().success {
            self.compile_script().await?;
        }

        let bytecode = if let Some(ref result) = self.compilation_result {
            if result.success {
                result.bytecode.clone()
            } else {
                self.log("Cannot inject: compilation failed".to_string(), LogLevel::Error);
                return Ok(());
            }
        } else {
            self.log("Cannot inject: no compilation result".to_string(), LogLevel::Error);
            return Ok(());
        };

        // Get client type
        let frida_client_type = match self.config.client_type {
            ClientType::GraalV6 => frida_bridge::ClientType::GraalV6,
            ClientType::GraalWorlds => frida_bridge::ClientType::GraalWorlds,
        };

        let injector = FridaInjector::new(frida_client_type);
        let target_process = self.config.target_process();
        let variable_name = self.config.variable_name();

        self.log(
            format!(
                "Injecting {} bytes to {} (var: {})",
                bytecode.len(),
                target_process,
                variable_name
            ),
            LogLevel::Info,
        );
        self.frida_status = FridaStatus::Injecting;

        // Perform injection
        match injector.inject(&bytecode, &variable_name).await {
            Ok(msg) => {
                self.log("Injection successful".to_string(), LogLevel::Success);
                for line in msg.lines() {
                    self.log(line.to_string(), LogLevel::Info);
                }
                self.frida_status = FridaStatus::Attached;
            }
            Err(e) => {
                self.log(
                    format!("Injection failed: {}", e),
                    LogLevel::Error,
                );
                self.frida_status = FridaStatus::Error;
            }
        }

        Ok(())
    }
}
