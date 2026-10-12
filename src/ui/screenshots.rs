//! Regenerates the README screenshots from real render buffers:
//!
//! ```sh
//! cargo test screenshots -- --ignored
//! ```
//!
//! Each screen is drawn into a `TestBackend` and exported as an SVG to
//! `docs/screenshots/`. Typing and history are simulated with a seeded RNG.

use std::fmt::Write as _;
use std::time::Duration;

use chrono::{Days, Local};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

use super::view;
use crate::app::{App, Now, Screen};
use crate::content::Language;
use crate::game::Mode;
use crate::storage::history::SCHEMA_VERSION;
use crate::storage::{DataPaths, SessionRecord, Settings};

const WIDTH: u16 = 90;
const HEIGHT: u16 = 26;
const CELL_W: f64 = 9.0;
const CELL_H: f64 = 18.0;
const FONT_SIZE: f64 = 15.0;
const PAD: f64 = 18.0;
const BACKGROUND: &str = "#1e1e2e";
const FOREGROUND: &str = "#cdd6f4";

#[test]
#[ignore = "writes files; run explicitly to refresh README screenshots"]
fn screenshots() {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/screenshots");
    std::fs::create_dir_all(&out_dir).unwrap();
    let data_dir = tempfile::tempdir().unwrap();
    let mut rng = StdRng::seed_from_u64(2026);
    let mut now = Now::capture();

    let settings = Settings {
        language: Language::Es,
        mode: Mode::Sprint,
        show_live_stats: true,
    };
    let paths = DataPaths::resolve(Some(data_dir.path())).unwrap();
    let mut app = App::new(paths, settings, fake_history(&mut rng, &now), None);
    save(&out_dir, "menu", &app, &now);

    // Mid-session: type part of a paragraph with a couple of uncorrected mistakes.
    app.handle_key(key(KeyCode::Enter), &now);
    let Screen::Typing(session) = &app.screen else {
        panic!("expected typing screen")
    };
    let text: String = session.buffer().target()[..95].concat();
    for (i, c) in text.chars().enumerate() {
        now.instant += keystroke_delay(&mut rng);
        let typed = if i == 31 || i == 77 { 'x' } else { c };
        app.handle_key(key(KeyCode::Char(typed)), &now);
    }
    save(&out_dir, "typing", &app, &now);

    // Keep typing until time runs out to get a realistic results chart.
    while let Screen::Typing(session) = &app.screen {
        let buffer = session.buffer();
        let target = buffer.target()[buffer.cursor()].clone();
        now.instant += keystroke_delay(&mut rng);
        if rng.random_bool(0.03) {
            app.handle_key(key(KeyCode::Char('#')), &now);
            now.instant += keystroke_delay(&mut rng);
            app.handle_key(key(KeyCode::Backspace), &now);
        } else {
            for c in target.chars() {
                app.handle_key(key(KeyCode::Char(c)), &now);
            }
        }
        app.tick(&now);
    }
    save(&out_dir, "results", &app, &now);

    app.handle_key(key(KeyCode::Char('s')), &now);
    save(&out_dir, "stats", &app, &now);
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Roughly 70 WPM with human-looking jitter.
fn keystroke_delay(rng: &mut StdRng) -> Duration {
    Duration::from_millis(rng.random_range(110..230))
}

/// About four months of sessions with gaps, slowly improving.
fn fake_history(rng: &mut StdRng, now: &Now) -> Vec<SessionRecord> {
    let today = now.local.date_naive();
    let mut records = Vec::new();
    for days_ago in (1..=130u64).rev() {
        if rng.random_bool(0.35) && days_ago > 6 {
            continue;
        }
        let date = today.checked_sub_days(Days::new(days_ago)).unwrap();
        let progress = (130 - days_ago) as f64 / 130.0;
        for _ in 0..rng.random_range(1..=5) {
            let language = if rng.random_bool(0.6) {
                Language::Es
            } else {
                Language::En
            };
            let mode = Mode::ALL[rng.random_range(0..Mode::ALL.len())];
            let wpm = 42.0 + 24.0 * progress + rng.random_range(-6.0..6.0);
            let ended_at = date
                .and_hms_opt(rng.random_range(7..23), rng.random_range(0..60), 0)
                .unwrap()
                .and_local_timezone(Local)
                .earliest()
                .unwrap();
            records.push(SessionRecord {
                version: SCHEMA_VERSION,
                ended_at,
                language,
                mode,
                wpm,
                raw_wpm: wpm + rng.random_range(1.0..6.0),
                accuracy: rng.random_range(91.0..99.5),
                consistency: rng.random_range(60.0..85.0),
                errors: rng.random_range(2..20),
                uncorrected_errors: rng.random_range(0..4),
                correct_chars: (wpm * 5.0 * mode.duration().as_secs_f64() / 60.0) as u32,
                total_chars: (wpm * 5.4 * mode.duration().as_secs_f64() / 60.0) as u32,
                paragraphs: rng.random_range(0..6),
            });
        }
    }
    records
}

fn save(dir: &std::path::Path, name: &str, app: &App, now: &Now) {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal.draw(|f| view(app, f, now)).unwrap();
    let svg = to_svg(terminal.backend().buffer());
    std::fs::write(dir.join(format!("{name}.svg")), svg).unwrap();
}

fn color(c: Color) -> Option<&'static str> {
    Some(match c {
        Color::Reset => return None,
        Color::Black => "#11111b",
        Color::Red => "#f38ba8",
        Color::Green => "#a6e3a1",
        Color::Yellow => "#fab387",
        Color::Gray => "#9399b2",
        Color::DarkGray => "#6c7086",
        Color::White => "#cdd6f4",
        Color::Indexed(237) => "#313244",
        Color::Indexed(22) => "#2e4f3a",
        Color::Indexed(28) => "#40794f",
        Color::Indexed(34) => "#6fb27a",
        Color::Indexed(46) => "#a6e3a1",
        other => panic!("add {other:?} to the screenshot palette"),
    })
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Renders the buffer as SVG. Backgrounds are rects; text is emitted per
/// run of non-space cells with the same style, each placed at its own column
/// and stretched to its exact cell width, so alignment holds even when glyphs
/// come from fallback fonts and no renderer whitespace handling is involved.
fn to_svg(buf: &Buffer) -> String {
    let width = f64::from(buf.area.width) * CELL_W + 2.0 * PAD;
    let height = f64::from(buf.area.height) * CELL_H + 2.0 * PAD;
    let mut svg = String::new();
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    )
    .unwrap();
    writeln!(
        svg,
        r#"<rect width="100%" height="100%" rx="10" fill="{BACKGROUND}"/>"#
    )
    .unwrap();
    writeln!(
        svg,
        r#"<g font-family="JetBrains Mono, SF Mono, Menlo, Consolas, DejaVu Sans Mono, monospace" font-size="{FONT_SIZE}">"#
    )
    .unwrap();

    let cell_x = |x: u16| PAD + f64::from(x) * CELL_W;
    for y in 0..buf.area.height {
        let py = PAD + f64::from(y) * CELL_H;
        let blank = |x: u16| buf[(x, y)].symbol().trim().is_empty();
        let style = |x: u16| {
            let c = &buf[(x, y)];
            (c.fg, c.bg, c.modifier)
        };

        let mut x = 0;
        while x < buf.area.width {
            let start = x;
            let bg = buf[(x, y)].bg;
            while x < buf.area.width && buf[(x, y)].bg == bg {
                x += 1;
            }
            if let Some(fill) = color(bg) {
                let w = f64::from(x - start) * CELL_W;
                writeln!(
                    svg,
                    r#"<rect x="{}" y="{py}" width="{w}" height="{CELL_H}" fill="{fill}"/>"#,
                    cell_x(start)
                )
                .unwrap();
            }
        }

        let mut x = 0;
        while x < buf.area.width {
            if blank(x) {
                x += 1;
                continue;
            }
            let start = x;
            let run_style = style(x);
            let mut text = String::new();
            while x < buf.area.width && !blank(x) && style(x) == run_style {
                text.push_str(buf[(x, y)].symbol());
                x += 1;
            }
            let (fg, _, modifier) = run_style;
            let fill = color(fg).unwrap_or(FOREGROUND);
            let mut attrs = String::new();
            if modifier.contains(Modifier::BOLD) {
                attrs.push_str(r#" font-weight="bold""#);
            }
            if modifier.contains(Modifier::UNDERLINED) {
                attrs.push_str(r#" text-decoration="underline""#);
            }
            writeln!(
                svg,
                r#"<text x="{}" y="{}" fill="{fill}" textLength="{}" lengthAdjust="spacingAndGlyphs"{attrs}>{}</text>"#,
                cell_x(start),
                py + CELL_H * 0.75,
                f64::from(x - start) * CELL_W,
                escape(&text)
            )
            .unwrap();
        }
    }
    svg.push_str("</g>\n</svg>\n");
    svg
}
