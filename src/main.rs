pub mod app;
pub mod config;
pub mod history;
pub mod scanner;
pub mod tmux;
pub mod ui;

use app::App;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

#[derive(Parser)]
#[command(name = "qs", version, about = "Fast, interactive directory selector for tmux")]
struct Cli {
    /// Open selection in new tmux window
    #[arg(short, long)]
    tmux: bool,

    /// Start browsing from a specific path
    #[arg(short, long)]
    path: Option<String>,

    /// Show debug info
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.debug {
        eprintln!(
            "qs v{} — tmux={}, path={:?}",
            env!("CARGO_PKG_VERSION"),
            cli.tmux,
            cli.path
        );
    }

    // Load config and history
    let config = config::Config::load();
    let history_path = config.history_path();
    let history = history::History::load(&history_path);

    // Create app
    let mut app = App::new(config, history, cli.path, cli.tmux);

    // Set panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), LeaveAlternateScreen);
        let _ = execute!(
            io::stderr(),
            crossterm::cursor::Show
        );
        original_hook(info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend)?;

    // Event loop
    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Event::Key(key) = event::read()? {
            // Only handle Press events (avoid repeats on some terminals)
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Handle 'e' (edit config) specially — needs terminal restore
            if key.code == KeyCode::Char('e') && app.filter_input.is_empty() {
                let editor =
                    std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                let config_dir = dirs::config_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
                    .join("tmux-quickselect")
                    .join("config.toml");

                // Temporarily leave alternate screen
                disable_raw_mode()?;
                execute!(io::stderr(), LeaveAlternateScreen, crossterm::cursor::Show)?;

                let _ = std::process::Command::new(&editor)
                    .arg(&config_dir)
                    .status();

                // Re-enter alternate screen
                enable_raw_mode()?;
                execute!(
                    io::stderr(),
                    EnterAlternateScreen,
                    crossterm::cursor::Hide
                )?;
                terminal.clear()?;

                // Reload config and re-scan
                app.config = config::Config::load();
                app.scan();
                continue;
            }

            app.handle_key(key);
        }

        if app.should_quit || app.selected_project.is_some() {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        io::stderr(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    // Post-selection actions
    if let Some(project) = &app.selected_project {
        // Save history
        let mut history = app.history;
        history.record(&project.path);
        let _ = history.save(&history_path);

        if app.tmux_mode {
            tmux::open_in_tmux(
                &project.name,
                &project.path,
                app.config.command.as_deref(),
            )?;
        } else {
            // Print path to stdout for shell integration
            println!("{}", project.path);
        }
    }

    Ok(())
}
