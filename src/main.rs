//! GraalHax TUI - GS2 Development Environment
//!
//! A terminal-based IDE for GS2 scripting with integrated compilation and Frida injection.

mod app;
mod ui;
mod config;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Run app
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Draw the UI
        terminal.draw(|f| ui::draw(f, app))?;

        // Block until next event (this prevents duplicate processing)
        if let Ok(event) = crossterm::event::read() {
            // Handle key events only - ignore mouse and other events
            if let crossterm::event::Event::Key(key) = event {
                if !app.handle_key(key).await? {
                    return Ok(());
                }
            }
        }
    }
}
