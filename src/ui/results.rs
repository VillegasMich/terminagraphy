use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph};

use super::{key_hints, theme};
use crate::app::ResultsState;
use crate::game::Metrics;

pub fn render(frame: &mut Frame, area: Rect, results: &ResultsState) {
    let [headline, details, chart, footer] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area.inner(ratatui::layout::Margin::new(1, 0)));

    let m = &results.metrics;
    let r = &results.record;

    let mut head = vec![
        Line::from(vec![Span::styled(
            format!("{} · {}", r.language.name(), r.mode.label()),
            theme::DIM,
        )])
        .centered(),
        Line::from(vec![
            Span::styled("WPM ", theme::LABEL),
            Span::styled(format!("{:.0}", m.wpm), theme::BIG_VALUE),
            Span::styled("     ACC ", theme::LABEL),
            Span::styled(format!("{:.1}%", m.accuracy), theme::BIG_VALUE),
        ])
        .centered(),
        best_line(m.wpm, results.previous_best),
    ];
    if let Some(err) = &results.save_error {
        head.push(Line::styled(err.clone(), theme::ERROR).centered());
    }
    frame.render_widget(Paragraph::new(head), headline);

    let detail = |label: &str, value: String| {
        [
            Span::styled(format!("{label} "), theme::LABEL),
            Span::styled(value, theme::VALUE),
            Span::styled("   ", theme::DIM),
        ]
    };
    let row1: Vec<Span> = [
        detail("raw", format!("{:.0}", m.raw_wpm)),
        detail("consistency", format!("{:.0}%", m.consistency)),
        detail("paragraphs", m.paragraphs.to_string()),
    ]
    .concat();
    let row2: Vec<Span> = [
        detail(
            "errors",
            format!("{} ({} uncorrected)", m.errors, m.uncorrected_errors),
        ),
        detail("chars", format!("{}/{}", m.correct_chars, m.keystrokes)),
    ]
    .concat();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(row1).centered(),
            Line::from(row2).centered(),
        ]),
        details,
    );

    if chart.height >= 4 {
        frame.render_widget(ChartData::new(m).chart(), chart);
    }

    frame.render_widget(
        key_hints(&[
            ("Enter", "retry"),
            ("s", "stats"),
            ("Esc", "menu"),
            ("q", "quit"),
        ]),
        footer,
    );
}

fn best_line(wpm: f64, previous_best: Option<f64>) -> Line<'static> {
    match previous_best {
        None => Line::styled("First session in this mode!", theme::SUCCESS).centered(),
        Some(best) if wpm > best => Line::styled(
            format!("New personal best! +{:.1} WPM", wpm - best),
            theme::SUCCESS,
        )
        .centered(),
        Some(best) => Line::styled(
            format!("Personal best {best:.0} WPM ({:+.1})", wpm - best),
            theme::DIM,
        )
        .centered(),
    }
}

/// Chart points; kept in a struct so the chart can borrow them for one frame.
struct ChartData {
    wpm: Vec<(f64, f64)>,
    raw: Vec<(f64, f64)>,
    errors: Vec<(f64, f64)>,
    max_x: f64,
    max_y: f64,
}

impl ChartData {
    fn new(m: &Metrics) -> Self {
        let point = |second: u32, y: f64| (f64::from(second), y);
        let max_y = m
            .samples
            .iter()
            .map(|s| s.wpm.max(s.raw))
            .fold(10.0, f64::max);
        Self {
            wpm: m.samples.iter().map(|s| point(s.second, s.wpm)).collect(),
            raw: m.samples.iter().map(|s| point(s.second, s.raw)).collect(),
            errors: m
                .samples
                .iter()
                .filter(|s| s.errors > 0)
                .map(|s| point(s.second, s.raw))
                .collect(),
            max_x: m.samples.len().max(1) as f64,
            max_y: (max_y * 1.1).ceil(),
        }
    }

    fn chart(&self) -> Chart<'_> {
        let line = |name: &'static str, data, style| {
            Dataset::default()
                .name(name)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(style)
                .data(data)
        };
        let datasets = vec![
            line("raw", &self.raw, theme::CHART_RAW),
            line("wpm", &self.wpm, theme::CHART_WPM),
            Dataset::default()
                .name("errors")
                .marker(Marker::Dot)
                .graph_type(GraphType::Scatter)
                .style(theme::CHART_ERRORS)
                .data(&self.errors),
        ];
        Chart::new(datasets)
            .x_axis(
                Axis::default()
                    .style(theme::CHART_AXIS)
                    .bounds([1.0, self.max_x])
                    .labels(["1s".to_string(), format!("{:.0}s", self.max_x)]),
            )
            .y_axis(
                Axis::default()
                    .style(theme::CHART_AXIS)
                    .bounds([0.0, self.max_y])
                    .labels(["0".to_string(), format!("{:.0}", self.max_y)]),
            )
    }
}
