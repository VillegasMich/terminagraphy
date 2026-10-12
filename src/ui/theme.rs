//! Every color and style used by the UI.

use ratatui::style::{Color, Modifier, Style};

pub const ACCENT: Color = Color::Yellow;
pub const GOOD: Color = Color::Green;
pub const BAD: Color = Color::Red;

pub const BORDER: Style = Style::new().fg(Color::DarkGray);
pub const TITLE: Style = Style::new().fg(ACCENT).add_modifier(Modifier::BOLD);
pub const DIM: Style = Style::new().fg(Color::DarkGray);
pub const LABEL: Style = Style::new().fg(Color::Gray);
pub const VALUE: Style = Style::new().add_modifier(Modifier::BOLD);
pub const BIG_VALUE: Style = Style::new().fg(ACCENT).add_modifier(Modifier::BOLD);
pub const KEY: Style = Style::new().fg(ACCENT);
pub const SELECTED: Style = Style::new().fg(ACCENT).add_modifier(Modifier::BOLD);
pub const NOTICE: Style = Style::new().fg(Color::Yellow);
pub const ERROR: Style = Style::new().fg(BAD);
pub const SUCCESS: Style = Style::new().fg(GOOD).add_modifier(Modifier::BOLD);

// Typing area.
pub const PENDING: Style = Style::new().fg(Color::DarkGray);
pub const CORRECT: Style = Style::new().fg(Color::White);
pub const WRONG: Style = Style::new().fg(BAD).add_modifier(Modifier::UNDERLINED);
/// Wrong input where the target is a space, so the mistake stays visible.
pub const WRONG_SPACE: Style = Style::new().bg(BAD);
pub const CURSOR: Style = Style::new().fg(Color::Black).bg(Color::White);

// Charts.
pub const CHART_WPM: Style = Style::new().fg(ACCENT);
pub const CHART_RAW: Style = Style::new().fg(Color::Gray);
pub const CHART_ERRORS: Style = Style::new().fg(BAD);
pub const CHART_AXIS: Style = Style::new().fg(Color::DarkGray);

/// Heatmap cell colors from "no sessions" to "many sessions".
pub const HEATMAP: [Color; 5] = [
    Color::Indexed(237),
    Color::Indexed(22),
    Color::Indexed(28),
    Color::Indexed(34),
    Color::Indexed(46),
];
