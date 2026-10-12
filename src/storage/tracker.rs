use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};

use super::SessionRecord;

/// Days played, derived from history using each session's local end date.
#[derive(Debug, Clone, Default)]
pub struct DailyTracker {
    sessions_per_day: BTreeMap<NaiveDate, u32>,
}

impl DailyTracker {
    pub fn from_records(records: &[SessionRecord]) -> Self {
        Self::from_dates(records.iter().map(|r| r.ended_at.date_naive()))
    }

    pub fn from_dates(dates: impl IntoIterator<Item = NaiveDate>) -> Self {
        let mut tracker = Self::default();
        for date in dates {
            tracker.add(date);
        }
        tracker
    }

    pub fn add(&mut self, date: NaiveDate) {
        *self.sessions_per_day.entry(date).or_default() += 1;
    }

    pub fn sessions_on(&self, date: NaiveDate) -> u32 {
        self.sessions_per_day.get(&date).copied().unwrap_or(0)
    }

    pub fn total_days(&self) -> usize {
        self.sessions_per_day.len()
    }

    /// Consecutive days ending today, or ending yesterday if today has no
    /// session yet (the streak is only broken once a full day is missed).
    pub fn current_streak(&self, today: NaiveDate) -> u32 {
        let played = |d: NaiveDate| self.sessions_per_day.contains_key(&d);
        let mut day = if played(today) {
            today
        } else {
            match today.checked_sub_days(Days::new(1)) {
                Some(yesterday) if played(yesterday) => yesterday,
                _ => return 0,
            }
        };
        let mut streak = 0;
        loop {
            streak += 1;
            match day.checked_sub_days(Days::new(1)) {
                Some(prev) if played(prev) => day = prev,
                _ => return streak,
            }
        }
    }

    pub fn longest_streak(&self) -> u32 {
        let mut longest = 0;
        let mut run = 0;
        let mut prev: Option<NaiveDate> = None;
        for &day in self.sessions_per_day.keys() {
            run = match prev {
                Some(p) if p.succ_opt() == Some(day) => run + 1,
                _ => 1,
            };
            longest = longest.max(run);
            prev = Some(day);
        }
        longest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn empty_tracker() {
        let t = DailyTracker::default();
        assert_eq!(t.current_streak(d(2026, 1, 1)), 0);
        assert_eq!(t.longest_streak(), 0);
        assert_eq!(t.total_days(), 0);
    }

    #[test]
    fn counts_sessions_per_day() {
        let t = DailyTracker::from_dates([d(2026, 3, 1), d(2026, 3, 1), d(2026, 3, 2)]);
        assert_eq!(t.sessions_on(d(2026, 3, 1)), 2);
        assert_eq!(t.sessions_on(d(2026, 3, 3)), 0);
        assert_eq!(t.total_days(), 2);
    }

    #[test]
    fn streak_including_today() {
        let t = DailyTracker::from_dates([d(2026, 3, 1), d(2026, 3, 2), d(2026, 3, 3)]);
        assert_eq!(t.current_streak(d(2026, 3, 3)), 3);
    }

    #[test]
    fn streak_alive_if_played_yesterday() {
        let t = DailyTracker::from_dates([d(2026, 3, 1), d(2026, 3, 2)]);
        assert_eq!(t.current_streak(d(2026, 3, 3)), 2);
    }

    #[test]
    fn streak_broken_after_missed_day() {
        let t = DailyTracker::from_dates([d(2026, 3, 1), d(2026, 3, 2)]);
        assert_eq!(t.current_streak(d(2026, 3, 4)), 0);
    }

    #[test]
    fn streak_across_month_and_year_boundaries() {
        let t = DailyTracker::from_dates([
            d(2025, 12, 30),
            d(2025, 12, 31),
            d(2026, 1, 1),
            d(2026, 2, 28),
            d(2026, 3, 1),
        ]);
        assert_eq!(t.current_streak(d(2026, 3, 1)), 2);
        assert_eq!(t.longest_streak(), 3);
    }

    #[test]
    fn longest_streak_picks_max_run() {
        let t = DailyTracker::from_dates([
            d(2026, 1, 1),
            d(2026, 1, 3),
            d(2026, 1, 4),
            d(2026, 1, 5),
            d(2026, 1, 7),
        ]);
        assert_eq!(t.longest_streak(), 3);
    }
}
