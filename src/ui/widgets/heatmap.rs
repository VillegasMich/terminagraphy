use chrono::{Datelike, Days, NaiveDate};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::storage::DailyTracker;
use crate::ui::theme;

const LABEL_WIDTH: u16 = 4;
const CELL_WIDTH: u16 = 2;
const MAX_WEEKS: u16 = 53;
const DAY_LABELS: [&str; 7] = ["Mon", "", "Wed", "", "Fri", "", "Sun"];

/// GitHub-style calendar: one column per week (Monday first), the last
/// column is the current week. Needs 8 rows to show month labels, 7 without.
pub struct Heatmap<'a> {
    tracker: &'a DailyTracker,
    today: NaiveDate,
}

impl<'a> Heatmap<'a> {
    pub fn new(tracker: &'a DailyTracker, today: NaiveDate) -> Self {
        Self { tracker, today }
    }

    fn level(sessions: u32) -> usize {
        match sessions {
            0 => 0,
            1 => 1,
            2 => 2,
            3..=4 => 3,
            _ => 4,
        }
    }
}

impl Widget for Heatmap<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 7 || area.width <= LABEL_WIDTH + CELL_WIDTH {
            return;
        }
        let weeks = ((area.width - LABEL_WIDTH) / CELL_WIDTH).min(MAX_WEEKS);
        let with_months = area.height >= 8;
        let grid_top = area.y + u16::from(with_months);

        let this_monday = self
            .today
            .checked_sub_days(Days::new(
                self.today.weekday().num_days_from_monday().into(),
            ))
            .unwrap_or(self.today);
        let Some(first_monday) = this_monday.checked_sub_days(Days::new(u64::from(weeks - 1) * 7))
        else {
            return;
        };

        for (row, label) in DAY_LABELS.iter().enumerate() {
            buf.set_string(area.x, grid_top + row as u16, label, theme::DIM);
        }

        let mut next_free_x = area.x + LABEL_WIDTH;
        let mut prev_month = None;
        for week in 0..weeks {
            let x = area.x + LABEL_WIDTH + week * CELL_WIDTH;
            let Some(monday) = first_monday.checked_add_days(Days::new(u64::from(week) * 7)) else {
                break;
            };

            if with_months && prev_month != Some(monday.month()) {
                let label = monday.format("%b").to_string();
                if x >= next_free_x && x + label.len() as u16 <= area.right() {
                    buf.set_string(x, area.y, &label, theme::DIM);
                    next_free_x = x + label.len() as u16 + 1;
                }
                prev_month = Some(monday.month());
            }

            for day in 0..7u16 {
                let Some(date) = monday.checked_add_days(Days::new(day.into())) else {
                    continue;
                };
                if date > self.today {
                    continue;
                }
                let level = Self::level(self.tracker.sessions_on(date));
                let style = Style::new().fg(theme::HEATMAP[level]);
                buf.set_string(x, grid_top + day, "■", style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(tracker: &DailyTracker, today: NaiveDate, width: u16) -> Buffer {
        let area = Rect::new(0, 0, width, 8);
        let mut buf = Buffer::empty(area);
        Heatmap::new(tracker, today).render(area, &mut buf);
        buf
    }

    #[test]
    fn last_column_is_current_week() {
        // 2026-10-14 is a Wednesday.
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let tracker = DailyTracker::from_dates([today, today, today]);
        let buf = render(&tracker, today, 4 + 2 * 3);
        let last_x = 4 + 2 * 2;
        // Wednesday row is index 2, grid starts at row 1 (month labels).
        let cell = &buf[(last_x, 3)];
        assert_eq!(cell.symbol(), "■");
        assert_eq!(cell.fg, theme::HEATMAP[3]);
        // Thursday is in the future: not drawn.
        assert_eq!(buf[(last_x, 4)].symbol(), " ");
        assert_eq!(buf[(0, 1)].symbol(), "M");
    }

    #[test]
    fn too_small_renders_nothing() {
        let tracker = DailyTracker::default();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let buf = render(&tracker, today, 5);
        assert!(buf.content.iter().all(|c| c.symbol() == " "));
    }
}
