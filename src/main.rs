mod app;
mod audio;
mod pitch;
mod tuning;
mod ui;

use std::io;
use std::time::Duration;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

#[derive(Parser)]
#[command(name = "tuner", about = "A terminal instrument tuner")]
struct Cli {
    /// Reference pitch for A4 in Hz (default: 440)
    #[arg(long, default_value_t = 440.0)]
    reference: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli.reference);

    // Start audio capture
    if let Err(e) = app.start_audio() {
        // We'll show the error in the TUI rather than crashing
        eprintln!("Warning: Could not start audio: {}", e);
    }

    // Main loop (~30fps)
    while app.running {
        // Process any pending audio samples
        app.process_audio();

        // Draw UI
        terminal.draw(|frame| ui::draw(frame, &app))?;

        // Handle input with timeout for ~30fps
        if event::poll(Duration::from_millis(33))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.running = false,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.running = false;
                    }
                    KeyCode::Tab => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            app.active_selector = app.active_selector.prev();
                        } else {
                            app.active_selector = app.active_selector.next();
                        }
                    }
                    KeyCode::BackTab => {
                        app.active_selector = app.active_selector.prev();
                    }
                    KeyCode::Left => app.handle_left(),
                    KeyCode::Right => app.handle_right(),
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
