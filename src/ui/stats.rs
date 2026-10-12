use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph};

use super::menu::plural;
use super::widgets::Heatmap;
use super::{format_duration, key_hints, theme};
use crate::app::{App, Now};
use crate::storage::{SessionRecord, history};

/// Sessions shown in the trend chart.
const TREND_SESSIONS: usize = 30;
/// Sessions averaged for "recent average".
const RECENT_SESSIONS: usize = 10;

pub fn render(frame: &mut Frame, area: Rect, app: &App, now: &Now) {
    // Fixed rows first so the heatmap still fits at the minimum terminal size;
    // the trend chart takes whatever is left.
    let [header, summary, chart, tracker, heatmap, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(8),
        Constraint::Length(1),
    ])
    .areas(area.inner(Margin::new(1, 0)));

    let (language, mode) = (app.stats_language, app.stats_mode);
    frame.render_widget(
        Line::from(vec![
            Span::styled("◀ ", theme::DIM),
            Span::styled(language.name(), theme::SELECTED),
            Span::styled(" ▶   ◀ ", theme::DIM),
            Span::styled(
                format!("{} · {}", mode.label(), mode.name()),
                theme::SELECTED,
            ),
            Span::styled(" ▶", theme::DIM),
        ])
        .centered(),
        header,
    );

    let records: Vec<&SessionRecord> = app
        .history
        .iter()
        .filter(|r| r.is_category(language, mode))
        .collect();
    frame.render_widget(
        summary_line(&app.history, &records, language, mode),
        summary,
    );

    let chart = if chart.height >= 6 {
        chart.inner(Margin::new(0, 1))
    } else {
        chart
    };
    if records.is_empty() {
        frame.render_widget(
            Line::styled("No sessions in this mode yet", theme::DIM).centered(),
            chart,
        );
    } else if chart.height >= 4 {
        let trend = Trend::new(&records);
        frame.render_widget(trend.chart(), chart);
    }

    let today = now.local.date_naive();
    let t = &app.tracker;
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Streak ", theme::LABEL),
                Span::styled(
                    plural(t.current_streak(today) as usize, "day"),
                    theme::VALUE,
                ),
                Span::styled("   Longest ", theme::LABEL),
                Span::styled(plural(t.longest_streak() as usize, "day"), theme::VALUE),
                Span::styled("   Days played ", theme::LABEL),
                Span::styled(t.total_days().to_string(), theme::VALUE),
            ])
            .centered(),
        ]),
        tracker,
    );

    let grid_width = heatmap.width.min(4 + 2 * 53);
    frame.render_widget(
        Heatmap::new(t, today),
        heatmap.centered_horizontally(Constraint::Length(grid_width)),
    );

    frame.render_widget(
        key_hints(&[
            ("←→", "mode"),
            ("↑↓", "language"),
            ("Esc", "menu"),
            ("q", "quit"),
        ]),
        footer,
    );
}

fn summary_line(
    all: &[SessionRecord],
    records: &[&SessionRecord],
    language: crate::content::Language,
    mode: crate::game::Mode,
) -> Paragraph<'static> {
    let best = history::personal_best(all, language, mode);
    let recent: Vec<f64> = records
        .iter()
        .rev()
        .take(RECENT_SESSIONS)
        .map(|r| r.wpm)
        .collect();
    let avg = if recent.is_empty() {
        0.0
    } else {
        recent.iter().sum::<f64>() / recent.len() as f64
    };
    let total_secs = records.len() as u64 * mode.duration().as_secs();
    let stat = |label: &str, value: String| {
        [
            Span::styled(format!("{label} "), theme::LABEL),
            Span::styled(value, theme::VALUE),
            Span::styled("   ", theme::DIM),
        ]
    };
    let spans = [
        stat(
            "Best",
            best.map_or("-".into(), |b| format!("{:.0} WPM", b.wpm)),
        ),
        stat(&format!("Avg last {RECENT_SESSIONS}"), format!("{avg:.0}")),
        stat("Sessions", records.len().to_string()),
        stat("Time", format_duration(total_secs)),
    ]
    .concat();
    Paragraph::new(Line::from(spans).centered())
}

struct Trend {
    wpm: Vec<(f64, f64)>,
    accuracy_scaled: Vec<(f64, f64)>,
    max_y: f64,
}

impl Trend {
    fn new(records: &[&SessionRecord]) -> Self {
        let start = records.len().saturating_sub(TREND_SESSIONS);
        let recent = &records[start..];
        let max_y = (recent.iter().map(|r| r.wpm).fold(10.0, f64::max) * 1.1).ceil();
        let x = |i: usize| (i + 1) as f64;
        Self {
            wpm: recent
                .iter()
                .enumerate()
                .map(|(i, r)| (x(i), r.wpm))
                .collect(),
            // Accuracy plotted on the WPM scale so both share one axis.
            accuracy_scaled: recent
                .iter()
                .enumerate()
                .map(|(i, r)| (x(i), r.accuracy / 100.0 * max_y))
                .collect(),
            max_y,
        }
    }

    fn chart(&self) -> Chart<'_> {
        let max_x = self.wpm.len().max(2) as f64;
        let datasets = vec![
            Dataset::default()
                .name("accuracy")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(theme::CHART_RAW)
                .data(&self.accuracy_scaled),
            Dataset::default()
                .name("wpm")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(theme::CHART_WPM)
                .data(&self.wpm),
        ];
        Chart::new(datasets)
            .x_axis(
                Axis::default()
                    .style(theme::CHART_AXIS)
                    .bounds([1.0, max_x])
                    .labels(["oldest", "latest"]),
            )
            .y_axis(
                Axis::default()
                    .style(theme::CHART_AXIS)
                    .bounds([0.0, self.max_y])
                    .labels(["0".to_string(), format!("{:.0}", self.max_y)]),
            )
    }
}
