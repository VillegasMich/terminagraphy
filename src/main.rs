mod app;
mod content;
mod event;
mod game;
mod storage;
mod ui;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use ratatui::DefaultTerminal;

use app::{App, Now};
use content::Language;
use game::Mode;
use storage::{DataPaths, config, history};

/// How long to wait for input before redrawing (timer refresh rate).
const TICK_RATE: Duration = Duration::from_millis(50);

/// Terminal typing trainer: type paragraphs against the clock and track your progress.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Paragraph language; skips the menu and starts a session.
    #[arg(long, value_enum)]
    lang: Option<Language>,
    /// Session length; skips the menu and starts a session.
    #[arg(long, value_enum)]
    mode: Option<Mode>,
    /// Directory for history and settings (default: OS data/config dirs).
    #[arg(long)]
    data_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let paths = DataPaths::resolve(args.data_dir.as_deref())?;
    let (settings, config_warning) = config::load(&paths.config)?;
    let report = history::load(&paths.history)?;

    let mut notices: Vec<String> = config_warning.into_iter().collect();
    if report.skipped > 0 {
        notices.push(format!(
            "Skipped {} unreadable line(s) in {}",
            report.skipped,
            paths.history.display()
        ));
    }
    let notice = (!notices.is_empty()).then(|| notices.join(" · "));

    let mut app = App::new(paths, settings.clone(), report.records, notice);
    if args.lang.is_some() || args.mode.is_some() {
        app.start_session(
            args.lang.unwrap_or(settings.language),
            args.mode.unwrap_or(settings.mode),
        );
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.should_quit {
        let now = Now::capture();
        terminal.draw(|frame| ui::view(app, frame, &now))?;
        if let event::Event::Key(key) = event::next(TICK_RATE)? {
            app.handle_key(key, &Now::capture());
        }
        app.tick(&Now::capture());
    }
    Ok(())
}
