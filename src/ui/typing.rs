use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use super::{format_duration, key_hints, theme};
use crate::app::Now;
use crate::game::Session;
use crate::game::input::{CharStatus, TypingBuffer};
use crate::game::session::SessionState;
use crate::storage::Settings;

/// Max paragraph width in columns so lines stay readable on wide terminals.
const MAX_TEXT_WIDTH: u16 = 70;

pub fn render(frame: &mut Frame, area: Rect, session: &Session, settings: &Settings, now: &Now) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area.inner(ratatui::layout::Margin::new(1, 0)));

    render_header(frame, header, session, now);

    let text_width = MAX_TEXT_WIDTH.min(body.width.saturating_sub(4)).max(10);
    let buffer = session.buffer();
    let lines = wrap(buffer.target(), text_width as usize);
    let paragraph: Vec<Line> = lines
        .iter()
        .map(|r| styled_line(buffer, r.clone()))
        .collect();

    let block_height = paragraph.len() as u16 + 2;
    let [text_area, _, stats_area] = Layout::vertical([
        Constraint::Length(paragraph.len() as u16),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(body.centered_vertically(Constraint::Length(block_height)));

    // +1 column so a cursor on a trailing space at the end of a line is visible.
    let text_area = text_area.centered_horizontally(Constraint::Length(text_width + 1));
    frame.render_widget(Paragraph::new(paragraph), text_area);
    frame.render_widget(stats_line(session, settings, now), stats_area);

    frame.render_widget(
        key_hints(&[
            ("Esc", "menu"),
            ("Tab", "restart"),
            ("Ctrl+W", "delete word"),
        ]),
        footer,
    );
}

fn render_header(frame: &mut Frame, area: Rect, session: &Session, now: &Now) {
    let left = Line::from(vec![
        Span::styled(session.language.name(), theme::LABEL),
        Span::styled(" · ", theme::DIM),
        Span::styled(session.mode.label(), theme::LABEL),
    ]);
    // Round up so the timer shows 0:30 at start and 0:00 only when time is up.
    let remaining = session.remaining(now.instant).as_millis().div_ceil(1000) as u64;
    let timer_style = match session.state() {
        SessionState::Ready => theme::DIM,
        _ => theme::BIG_VALUE,
    };
    let right = Line::styled(format_duration(remaining), timer_style).right_aligned();
    frame.render_widget(left, area);
    frame.render_widget(right, area);
}

fn stats_line(session: &Session, settings: &Settings, now: &Now) -> Line<'static> {
    if session.state() == SessionState::Ready {
        return Line::styled(
            "start typing — the timer begins on your first key",
            theme::DIM,
        )
        .centered();
    }
    let mut spans = Vec::new();
    if settings.show_live_stats {
        let (wpm, acc) = session.live_stats(now.instant);
        spans.extend([
            Span::styled("WPM ", theme::LABEL),
            Span::styled(format!("{wpm:.0}"), theme::VALUE),
            Span::styled("   ACC ", theme::LABEL),
            Span::styled(format!("{acc:.1}%"), theme::VALUE),
            Span::styled("   ", theme::DIM),
        ]);
    }
    spans.extend([
        Span::styled("¶ ", theme::LABEL),
        Span::styled(session.paragraphs_done().to_string(), theme::VALUE),
    ]);
    Line::from(spans).centered()
}

fn styled_line(buffer: &TypingBuffer, range: Range<usize>) -> Line<'static> {
    let target = buffer.target();
    let cursor = buffer.cursor();
    let spans: Vec<Span> = range
        .map(|i| {
            let g = target[i].clone();
            let style = match buffer.status(i) {
                _ if i == cursor => theme::CURSOR,
                CharStatus::Pending => theme::PENDING,
                CharStatus::Correct => theme::CORRECT,
                CharStatus::Wrong if g == " " => theme::WRONG_SPACE,
                CharStatus::Wrong => theme::WRONG,
            };
            Span::styled(g, style)
        })
        .collect();
    Line::from(spans)
}

/// Word-wraps graphemes into lines of at most `width` columns (a trailing
/// space may hang past the edge). Words longer than `width` are split.
pub fn wrap(graphemes: &[String], width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let col = |r: Range<usize>| -> usize { graphemes[r].iter().map(|g| g.width()).sum() };
    let mut lines = Vec::new();
    let mut start = 0;
    let mut line_width = 0;
    let mut i = 0;
    while i < graphemes.len() {
        let mut word_end = i;
        while word_end < graphemes.len() && graphemes[word_end] != " " {
            word_end += 1;
        }
        let mut next = word_end;
        while next < graphemes.len() && graphemes[next] == " " {
            next += 1;
        }
        let word_width = col(i..word_end);
        if line_width > 0 && line_width + word_width > width {
            lines.push(start..i);
            start = i;
            line_width = 0;
        }
        if word_width > width {
            // Hard-split a word that cannot fit on any line.
            for (j, g) in graphemes.iter().enumerate().take(word_end).skip(i) {
                let w = g.width();
                if line_width > 0 && line_width + w > width {
                    lines.push(start..j);
                    start = j;
                    line_width = 0;
                }
                line_width += w;
            }
            line_width += col(word_end..next);
        } else {
            line_width += col(i..next);
        }
        i = next;
    }
    if start < graphemes.len() || lines.is_empty() {
        lines.push(start..graphemes.len());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::input::graphemes;

    fn wrapped(text: &str, width: usize) -> Vec<String> {
        let g = graphemes(text);
        wrap(&g, width).into_iter().map(|r| g[r].concat()).collect()
    }

    #[test]
    fn wraps_on_word_boundaries() {
        assert_eq!(
            wrapped("the quick brown fox", 10),
            vec!["the quick ", "brown fox"]
        );
    }

    #[test]
    fn keeps_every_grapheme() {
        let text = "¿Qué tal? El año pasado fue muy bueno, ¡qué alegría!";
        for width in 1..30 {
            assert_eq!(wrapped(text, width).concat(), text, "width {width}");
        }
    }

    #[test]
    fn splits_long_words() {
        assert_eq!(wrapped("abcdefgh ij", 4), vec!["abcd", "efgh ", "ij"]);
    }

    #[test]
    fn lines_fit_width_ignoring_trailing_space() {
        let text = "one two three four five six seven eight nine ten";
        for line in wrapped(text, 12) {
            assert!(line.trim_end().width() <= 12, "{line:?}");
        }
    }
}
