use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{key_hints, theme};
use crate::app::{App, MenuField, Now};
use crate::storage::history;

pub fn render(frame: &mut Frame, area: Rect, app: &App, now: &Now) {
    let [body, footer] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

    let s = &app.settings;
    let mut lines = vec![
        Line::styled("T E R M I N A G R A P H Y", theme::TITLE).centered(),
        Line::styled("typing trainer · mecanografía", theme::DIM).centered(),
        Line::default(),
        selector(
            "Language",
            s.language.name(),
            app.menu_field == MenuField::Language,
        ),
        selector(
            "Mode",
            &format!("{} · {}", s.mode.label(), s.mode.name()),
            app.menu_field == MenuField::Mode,
        ),
        Line::default(),
    ];

    lines.push(
        match history::personal_best(&app.history, s.language, s.mode) {
            Some(pb) => Line::from(vec![
                Span::styled("Best  ", theme::LABEL),
                Span::styled(format!("{:.0} WPM", pb.wpm), theme::VALUE),
                Span::styled(format!(" · {:.1}% acc", pb.accuracy), theme::DIM),
            ])
            .centered(),
            None => Line::styled("No sessions in this mode yet", theme::DIM).centered(),
        },
    );

    let today = now.local.date_naive();
    let streak = app.tracker.current_streak(today);
    lines.push(
        Line::from(vec![
            Span::styled("Streak  ", theme::LABEL),
            Span::styled(plural(streak as usize, "day"), theme::VALUE),
            Span::styled(
                format!(" · {} played", plural(app.tracker.total_days(), "day")),
                theme::DIM,
            ),
        ])
        .centered(),
    );

    if let Some(notice) = &app.notice {
        lines.push(Line::default());
        lines.push(Line::styled(notice.clone(), theme::NOTICE).centered());
    }

    let height = (lines.len() as u16).min(body.height);
    frame.render_widget(
        Paragraph::new(lines),
        body.centered_vertically(Constraint::Length(height)),
    );

    let live = if s.show_live_stats { "on" } else { "off" };
    let live_hint = format!("live stats {live}");
    frame.render_widget(
        key_hints(&[
            ("↑↓ ←→", "choose"),
            ("Enter", "start"),
            ("s", "stats"),
            ("t", &live_hint),
            ("q", "quit"),
        ]),
        footer,
    );
}

fn selector(label: &str, value: &str, focused: bool) -> Line<'static> {
    let (marker, value_style) = if focused {
        ("▸ ", theme::SELECTED)
    } else {
        ("  ", theme::VALUE)
    };
    Line::from(vec![
        Span::styled(marker, theme::KEY),
        Span::styled(format!("{label:<10}"), theme::LABEL),
        Span::styled("◀ ", theme::DIM),
        Span::styled(format!("{value:^16}"), value_style),
        Span::styled(" ▶", theme::DIM),
    ])
    .centered()
}

pub(super) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}
