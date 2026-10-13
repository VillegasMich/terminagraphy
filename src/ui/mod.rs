//! Rendering. `view` reads the `App` and draws; it never mutates state or does I/O.

mod menu;
mod results;
#[cfg(test)]
mod screenshots;
mod stats;
pub mod theme;
mod typing;
pub mod widgets;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::app::{App, Now, Screen};

pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 15;

pub fn view(app: &App, frame: &mut Frame, now: &Now) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(frame, area);
        return;
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER)
        .title(Line::styled(" terminagraphy ", theme::TITLE).centered());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match &app.screen {
        Screen::Menu => menu::render(frame, inner, app, now),
        Screen::Typing(session) => typing::render(frame, inner, session, &app.settings, now),
        Screen::Results(results) => results::render(frame, inner, results),
        Screen::Stats => stats::render(frame, inner, app, now),
    }
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::styled("Terminal too small", theme::ERROR),
        Line::styled(
            format!(
                "{}x{} — need at least {MIN_WIDTH}x{MIN_HEIGHT}",
                area.width, area.height
            ),
            theme::DIM,
        ),
    ];
    let height = (text.len() as u16).min(area.height);
    let rect = area.centered_vertically(Constraint::Length(height));
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        rect,
    );
}

/// Footer line of `key description` pairs.
fn key_hints(hints: &[(&str, &str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", theme::DIM));
        }
        spans.push(Span::styled(key.to_string(), theme::KEY));
        spans.push(Span::styled(format!(" {desc}"), theme::DIM));
    }
    Line::from(spans).centered()
}

fn format_duration(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::app::ResultsState;
    use crate::content::Language;
    use crate::game::Mode;
    use crate::game::metrics::{TextTotals, compute};
    use crate::storage::{DataPaths, SessionRecord, Settings};

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn draw(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let now = Now::capture();
        terminal.draw(|f| view(app, f, &now)).unwrap();
        buffer_text(&terminal)
    }

    fn app() -> App {
        let dir = std::env::temp_dir().join("terminagraphy-ui-tests-unused");
        let paths = DataPaths::resolve(Some(&dir)).unwrap();
        App::new(paths, Settings::default(), Vec::new(), None)
    }

    #[test]
    fn menu_renders() {
        let text = draw(&app(), 80, 24);
        assert!(text.contains("terminagraphy"));
        assert!(text.contains("English"));
        assert!(text.contains("2m"));
    }

    fn results_app(used_tatoeba: bool) -> App {
        let mut app = app();
        let metrics = compute(&[], Mode::Sprint.duration(), TextTotals::default());
        let record = SessionRecord::new(&metrics, Mode::Sprint, Language::Es, chrono::Local::now());
        app.screen = Screen::Results(Box::new(ResultsState {
            record,
            metrics,
            previous_best: None,
            save_error: None,
            used_tatoeba,
        }));
        app
    }

    #[test]
    fn results_credit_tatoeba_only_when_used() {
        let with = draw(&results_app(true), 80, 24);
        assert!(with.contains(results::TATOEBA_ATTRIBUTION), "{with}");
        let without = draw(&results_app(false), 80, 24);
        assert!(!without.contains("Tatoeba"), "{without}");
        let min = draw(&results_app(true), MIN_WIDTH, MIN_HEIGHT);
        assert!(min.contains(results::TATOEBA_ATTRIBUTION), "{min}");
    }

    #[test]
    fn small_terminal_shows_message() {
        let text = draw(&app(), 40, 10);
        assert!(text.contains("Terminal too small"));
    }

    #[test]
    fn every_screen_renders_at_minimum_size() {
        let mut app = app();
        let _ = draw(&app, MIN_WIDTH, MIN_HEIGHT);
        app.start_session(app.settings.language, app.settings.mode);
        let text = draw(&app, MIN_WIDTH, MIN_HEIGHT);
        assert!(text.contains("2:00"));
        app.screen = Screen::Stats;
        let text = draw(&app, MIN_WIDTH, MIN_HEIGHT);
        assert!(text.contains("Streak"));
        assert!(text.contains("Mon"), "heatmap missing at minimum size");
    }
}
