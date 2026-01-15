//! UI rendering and widgets

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, EditorMode, FridaStatus, LogLevel};
use crate::config::ClientType;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Menu bar
            Constraint::Length(7),  // Dashboard
            Constraint::Min(0),     // Main content (editor + logs)
            Constraint::Length(3),  // Bytecode preview
            Constraint::Length(1),  // Status bar
        ])
        .split(f.size());

    draw_menu_bar(f, app, chunks[0]);
    draw_dashboard(f, app, chunks[1]);
    draw_main_content(f, app, chunks[2]);
    draw_bytecode_preview(f, app, chunks[3]);
    draw_status_bar(f, app, chunks[4]);
}

fn draw_menu_bar(f: &mut Frame, _app: &App, area: Rect) {
    let title = Span::styled(
        " GraalHax TUI - GS2 Development Environment ",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    );

    let menu_items = vec![
        " [File] ",
        " [Edit] ",
        " [Build] ",
        " [Tools] ",
        " [Help] ",
    ];

    let menu: Vec<Span> = menu_items
        .iter()
        .map(|&s| Span::styled(s, Style::default().fg(Color::Gray)))
        .collect();

    let mut line = vec![title];
    line.extend(menu);

    let paragraph = Paragraph::new(Line::from(line))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(area);

    // Client type
    let client_color = match app.config.client_type {
        ClientType::GraalV6 => Color::Cyan,
        ClientType::GraalWorlds => Color::Magenta,
    };
    let client_text = app.config.client_type.name();

    let client_widget = Paragraph::new(client_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Target Client ")
        )
        .style(Style::default().fg(client_color))
        .alignment(Alignment::Center);

    f.render_widget(client_widget, chunks[0]);

    // Frida status
    let frida_color = match app.frida_status {
        FridaStatus::Disconnected => Color::Gray,
        FridaStatus::Attached => Color::Green,
        FridaStatus::Injecting => Color::Yellow,
        FridaStatus::Error => Color::Red,
    };
    let frida_text = format!("{:?}", app.frida_status);

    let frida_widget = Paragraph::new(frida_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Frida Status ")
        )
        .style(Style::default().fg(frida_color))
        .alignment(Alignment::Center);

    f.render_widget(frida_widget, chunks[1]);

    // Compilation status
    let comp_status = if let Some(ref result) = app.compilation_result {
        if result.success {
            ("Success", Color::Green)
        } else {
            ("Failed", Color::Red)
        }
    } else {
        ("Not Compiled", Color::Gray)
    };

    let comp_widget = Paragraph::new(comp_status.0)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Script Status ")
        )
        .style(Style::default().fg(comp_status.1))
        .alignment(Alignment::Center);

    f.render_widget(comp_widget, chunks[2]);

    // Bytecode size
    let bytecode_size = if let Some(ref result) = app.compilation_result {
        format!("{} bytes", result.bytecode.len())
    } else {
        "N/A".to_string()
    };

    let size_widget = Paragraph::new(bytecode_size)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Bytecode Size ")
        )
        .alignment(Alignment::Center);

    f.render_widget(size_widget, chunks[3]);

    // Keybindings hint
    let hints = "^T=Client";

    let hints_widget = Paragraph::new(hints)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Toggle ")
        )
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

    f.render_widget(hints_widget, chunks[4]);
}

fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    draw_editor(f, app, chunks[0]);
    draw_log_viewer(f, app, chunks[1]);
}

fn draw_editor(f: &mut Frame, app: &App, area: Rect) {
    if let Some(tab) = app.current_tab() {
        let lines: Vec<Line> = tab
            .lines()
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let is_current_line = i == tab.cursor_line;
                let line_num = if app.config.editor.line_numbers {
                    format!("{:>4} ", i + 1)
                } else {
                    String::new()
                };

                let mut spans = vec![
                    Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        *line,
                        if is_current_line {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::Gray)
                        },
                    ),
                ];

                // Show cursor indicator in insert mode
                if is_current_line && app.mode == EditorMode::Insert {
                    if tab.cursor_column < line.len() {
                        // Split line at cursor
                        let before = &line[..tab.cursor_column];
                        let after = &line[tab.cursor_column..];
                        let line_num_2 = if app.config.editor.line_numbers {
                            format!("{:>4} ", i + 1)
                        } else {
                            String::new()
                        };
                        spans = vec![
                            Span::styled(line_num_2, Style::default().fg(Color::DarkGray)),
                            Span::styled(before, Style::default().fg(Color::White)),
                            Span::styled(
                                if after.chars().next().is_some() {
                                    after.chars().next().unwrap().to_string()
                                } else {
                                    " ".to_string()
                                },
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::REVERSED),
                            ),
                            Span::styled(
                                &after[after.chars().next().map_or(0, |c| c.len_utf8())..],
                                Style::default().fg(Color::Gray),
                            ),
                        ];
                    }
                }

                Line::from(spans)
            })
            .collect();

        let tab_title = if tab.modified {
            format!("* {} ", tab.name)
        } else {
            format!(" {} ", tab.name)
        };

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(tab_title)
            )
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No tabs open")
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(paragraph, area);
    }
}

fn draw_log_viewer(f: &mut Frame, app: &App, area: Rect) {
    let log_lines: Vec<Line> = app
        .logs
        .iter()
        .rev()
        .take(50)
        .rev()
        .map(|log| {
            let timestamp = format!("[{:02}:{:02}:{:02}] ", {
                let secs = log.timestamp % 86400;
                (secs / 3600) as u8
            }, {
                let secs = log.timestamp % 3600;
                (secs / 60) as u8
            }, {
                (log.timestamp % 60) as u8
            });

            let color = match log.level {
                LogLevel::Info => Color::Blue,
                LogLevel::Warning => Color::Yellow,
                LogLevel::Error => Color::Red,
                LogLevel::Success => Color::Green,
            };

            Line::from(vec![
                Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
                Span::styled(&log.message, Style::default().fg(color)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(log_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Logs ")
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_bytecode_preview(f: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(ref result) = app.compilation_result {
        if result.success {
            let hex: String = result
                .bytecode
                .iter()
                .take(100) // Show first 100 bytes
                .map(|b| format!("{:02X} ", b))
                .collect();

            if result.bytecode.len() > 100 {
                format!("{}... ({} bytes total)", hex, result.bytecode.len())
            } else {
                hex
            }
        } else {
            "Compilation failed - no bytecode generated".to_string()
        }
    } else {
        "Compile a script to see bytecode preview".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Bytecode Preview (hex) ")
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.mode {
        EditorMode::Normal => "NORMAL",
        EditorMode::Insert => "INSERT",
        EditorMode::Visual => "VISUAL",
    };

    let position = if let Some(tab) = app.current_tab() {
        format!("Ln {}, Col {}", tab.cursor_line + 1, tab.cursor_column + 1)
    } else {
        "".to_string()
    };

    let tab_info = if let Some(tab) = app.current_tab() {
        format!("{}{}", if tab.modified { "*" } else { "" }, tab.name)
    } else {
        "".to_string()
    };

    let target = format!("Target: {}", app.config.target_process());

    let status = format!(
        " {} | {} | {} | {} | {} ",
        mode_str,
        position,
        tab_info,
        target,
        app.status_message
    );

    let paragraph = Paragraph::new(status)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    f.render_widget(paragraph, area);
}
