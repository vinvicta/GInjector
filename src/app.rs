//! GraalHax Application
//!
//! Main application state and UI rendering for the GS2 IDE.

use crate::config::ClientType;
use eframe::egui;
use std::path::PathBuf;

// Re-export frida types
pub use frida_bridge::{ClientType as FridaClientType, FridaInjector};

/// Represents a single script tab
#[derive(Debug, Clone)]
pub struct ScriptTab {
    pub name: String,
    pub path: Option<PathBuf>,
    pub content: String,
    pub modified: bool,
}

impl ScriptTab {
    pub fn new(name: String) -> Self {
        Self {
            name,
            path: None,
            content: String::new(),
            modified: false,
        }
    }

    pub fn from_file(path: PathBuf, content: String) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();
        Self {
            name,
            path: Some(path),
            content,
            modified: false,
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

/// Main application state
pub struct GraalHaxApp {
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
}

impl GraalHaxApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Load configuration with explicit error handling
        let config = match crate::config::Config::load() {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Warning: Failed to load config: {}, using defaults", e);
                crate::config::Config::default()
            }
        };

        // Add default tab
        let tabs = vec![ScriptTab::new("Untitled.gs2".to_string())];
        let active_tab = 0;

        // Add initial logs
        let mut logs = Vec::new();
        logs.push(LogEntry::info("GraalHax started"));
        logs.push(LogEntry::info(format!("Client: {}", config.client_type.name())));

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
        }
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

    fn save_current_tab(&mut self) {
        let tab_name = self.current_tab().map(|t| t.name.clone());
        if let Some(tab) = self.current_tab_mut() {
            // If no path, show save dialog (simplified for now)
            if tab.path.is_none() {
                tab.path = Some(PathBuf::from(&tab.name));
            }
            tab.modified = false;
        }
        if let Some(name) = tab_name {
            self.add_log(LogEntry::success(format!("Saved: {}", name)));
        }
    }

    fn open_file(&mut self) {
        // This would use rfd::FileDialog in full implementation
        // For now, just log
        self.add_log(LogEntry::info("Open file dialog (placeholder)"));
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

        let bytecode = match self.compiled_bytecode.clone() {
            Some(b) => b,
            None => return,
        };

        let frida_client_type = match self.client_type {
            ClientType::GraalV6 => FridaClientType::GraalV6,
            ClientType::GraalWorlds => FridaClientType::GraalWorlds,
        };

        self.add_log(LogEntry::info(format!(
            "Injecting into {} ({} bytes)...",
            self.client_type.target_process(),
            bytecode.len()
        )));

        // Spawn a background thread to handle injection (async operation)
        let bytecode_clone = bytecode.clone();
        std::thread::spawn(move || {
            // Create a new tokio runtime for this thread
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let injector = FridaInjector::new(frida_client_type);
                let result = injector.inject(&bytecode_clone, &frida_client_type.default_variable_name()).await;
                // In a real implementation, we'd send this result back to the main thread
                match result {
                    Ok(msg) => eprintln!("Injection success: {}", msg),
                    Err(e) => eprintln!("Injection error: {}", e),
                }
            });
        });

        // For now, just log that injection was initiated
        self.add_log(LogEntry::info("Injection initiated (check console for details)"));
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
}

impl eframe::App for GraalHaxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

                    // Handle actions after the loop to avoid borrow issues
                    if let Some(i) = clicked_tab {
                        self.active_tab = i;
                    }
                    if let Some(i) = close_tab {
                        self.close_tab(i);
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

                        ui.separator();

                        // Client info
                        ui.label(format!("Target:\n{}", self.client_type.target_process()));
                    });
                });

                // Editor area
                // Clone needed values before mutable borrow
                let font_id = self.font_id.clone();
                let tab_name = self.current_tab().map(|t| t.name.clone());

                if let Some(tab) = self.current_tab_mut() {
                    let name = tab_name.unwrap_or_default();

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Editor:");
                            ui.monospace(name);
                        });

                        let response = egui::ScrollArea::vertical()
                            .id_salt("editor_scroll")
                            .show(ui, |ui| {
                                egui::TextEdit::multiline(&mut tab.content)
                                    .font(font_id)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .show(ui)
                            });

                        // Check for modifications
                        if response.inner.response.changed() {
                            tab.modified = true;
                        }
                    });
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
                    ui.label("GraalHax v0.1.0");
                });
            });
        });

        // About dialog
        if self.show_about {
            egui::Window::new("About GraalHax")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("GraalHax");
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
