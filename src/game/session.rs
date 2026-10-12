use std::time::{Duration, Instant};

use super::input::{InputOutcome, TypingBuffer};
use super::metrics::{self, KeyKind, Keystroke, Metrics, TextTotals};
use super::mode::Mode;
use crate::content::{Language, ParagraphPicker};

/// Minimum elapsed time used for live WPM so the first keystrokes don't spike it.
const LIVE_WPM_MIN_ELAPSED: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Paragraph shown, timer not started yet.
    Ready,
    Running,
    Finished,
}

/// One timed typing session. Every time-dependent method takes `now` so the
/// caller (the event loop) is the only place reading the clock.
#[derive(Debug, Clone)]
pub struct Session {
    pub mode: Mode,
    pub language: Language,
    started: Option<Instant>,
    finished: bool,
    picker: ParagraphPicker,
    buffer: TypingBuffer,
    log: Vec<Keystroke>,
    /// Totals of paragraphs already completed (the current one is in `buffer`).
    committed: TextTotals,
}

impl Session {
    pub fn new(mode: Mode, language: Language) -> Self {
        Self::with_picker(mode, language, ParagraphPicker::new(language))
    }

    pub fn with_picker(mode: Mode, language: Language, mut picker: ParagraphPicker) -> Self {
        let buffer = TypingBuffer::new(picker.next_paragraph());
        Self {
            mode,
            language,
            started: None,
            finished: false,
            picker,
            buffer,
            log: Vec::new(),
            committed: TextTotals::default(),
        }
    }

    pub fn state(&self) -> SessionState {
        match (self.started, self.finished) {
            (_, true) => SessionState::Finished,
            (None, false) => SessionState::Ready,
            (Some(_), false) => SessionState::Running,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn buffer(&self) -> &TypingBuffer {
        &self.buffer
    }

    pub fn paragraphs_done(&self) -> u32 {
        self.committed.paragraphs
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        match self.started {
            None => Duration::ZERO,
            Some(start) => now
                .saturating_duration_since(start)
                .min(self.mode.duration()),
        }
    }

    pub fn remaining(&self, now: Instant) -> Duration {
        self.mode.duration() - self.elapsed(now)
    }

    /// Ends the session if its time is up. Returns `true` if it is finished.
    pub fn tick(&mut self, now: Instant) -> bool {
        if !self.finished
            && let Some(start) = self.started
            && now.saturating_duration_since(start) >= self.mode.duration()
        {
            self.finished = true;
        }
        self.finished
    }

    /// Starts the timer on first input; returns elapsed time, or `None` if input
    /// must be ignored because the session is over.
    fn accept_input(&mut self, now: Instant) -> Option<Duration> {
        if self.tick(now) {
            return None;
        }
        let start = *self.started.get_or_insert(now);
        Some(now.saturating_duration_since(start))
    }

    pub fn type_char(&mut self, c: char, now: Instant) {
        let Some(at) = self.accept_input(now) else {
            return;
        };
        match self.buffer.push_char(c) {
            InputOutcome::Typed(correct) => {
                let kind = if correct {
                    KeyKind::Correct
                } else {
                    KeyKind::Wrong
                };
                self.log.push(Keystroke { at, kind });
            }
            InputOutcome::Amended(correct) => {
                if let Some(last) = self
                    .log
                    .iter_mut()
                    .rev()
                    .find(|k| k.kind != KeyKind::Backspace)
                {
                    last.kind = if correct {
                        KeyKind::Correct
                    } else {
                        KeyKind::Wrong
                    };
                }
            }
            InputOutcome::Ignored => {}
        }
        if self.buffer.is_complete() {
            self.commit_paragraph();
        }
    }

    pub fn backspace(&mut self, now: Instant) {
        if self.started.is_none() {
            return;
        }
        let Some(at) = self.accept_input(now) else {
            return;
        };
        if self.buffer.backspace() {
            self.log.push(Keystroke {
                at,
                kind: KeyKind::Backspace,
            });
        }
    }

    pub fn delete_word(&mut self, now: Instant) {
        if self.started.is_none() {
            return;
        }
        let Some(at) = self.accept_input(now) else {
            return;
        };
        if self.buffer.delete_word() {
            self.log.push(Keystroke {
                at,
                kind: KeyKind::Backspace,
            });
        }
    }

    fn commit_paragraph(&mut self) {
        self.committed.correct_chars += self.buffer.correct_count();
        self.committed.uncorrected_errors += self.buffer.wrong_count();
        self.committed.paragraphs += 1;
        self.buffer = TypingBuffer::new(self.picker.next_paragraph());
    }

    fn totals(&self) -> TextTotals {
        TextTotals {
            correct_chars: self.committed.correct_chars + self.buffer.correct_count(),
            uncorrected_errors: self.committed.uncorrected_errors + self.buffer.wrong_count(),
            paragraphs: self.committed.paragraphs,
        }
    }

    /// Live (net WPM, accuracy) for the stats bar.
    pub fn live_stats(&self, now: Instant) -> (f64, f64) {
        let elapsed = self.elapsed(now).max(LIVE_WPM_MIN_ELAPSED);
        let correct = self
            .log
            .iter()
            .filter(|k| k.kind == KeyKind::Correct)
            .count() as u32;
        let wrong = self.log.iter().filter(|k| k.kind == KeyKind::Wrong).count() as u32;
        (
            metrics::wpm(self.totals().correct_chars, elapsed),
            metrics::accuracy(correct, wrong),
        )
    }

    /// Final metrics over the full mode duration.
    pub fn metrics(&self) -> Metrics {
        metrics::compute(&self.log, self.mode.duration(), self.totals())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new(Mode::Sprint, Language::En)
    }

    fn type_target(s: &mut Session, n: usize, now: Instant) {
        let text: String = s.buffer().target()[s.buffer().cursor()..][..n].concat();
        for c in text.chars() {
            s.type_char(c, now);
        }
    }

    #[test]
    fn timer_starts_on_first_key() {
        let t0 = Instant::now();
        let mut s = session();
        assert_eq!(s.state(), SessionState::Ready);
        assert!(!s.tick(t0 + Duration::from_secs(100)));
        assert_eq!(s.remaining(t0), Mode::Sprint.duration());

        let start = t0 + Duration::from_secs(5);
        type_target(&mut s, 1, start);
        assert_eq!(s.state(), SessionState::Running);
        assert_eq!(s.remaining(start + Duration::from_secs(10)).as_secs(), 20);
    }

    #[test]
    fn backspace_before_start_does_not_start_timer() {
        let mut s = session();
        s.backspace(Instant::now());
        assert_eq!(s.state(), SessionState::Ready);
    }

    #[test]
    fn finishes_at_duration_and_ignores_late_input() {
        let t0 = Instant::now();
        let mut s = session();
        type_target(&mut s, 3, t0);
        let end = t0 + Mode::Sprint.duration();
        assert!(!s.tick(end - Duration::from_millis(1)));
        assert!(s.tick(end));
        assert_eq!(s.state(), SessionState::Finished);
        let cursor = s.buffer().cursor();
        s.type_char('x', end + Duration::from_secs(1));
        assert_eq!(s.buffer().cursor(), cursor);
        assert_eq!(s.metrics().keystrokes, 3);
    }

    #[test]
    fn completing_paragraph_loads_next() {
        let t0 = Instant::now();
        let mut s = session();
        let first = s.buffer().target().to_vec();
        type_target(&mut s, first.len(), t0);
        assert_eq!(s.paragraphs_done(), 1);
        assert_eq!(s.buffer().cursor(), 0);
        assert_ne!(s.buffer().target(), first.as_slice());
        let m = s.metrics();
        assert_eq!(m.correct_chars, first.len() as u32);
        assert_eq!(m.uncorrected_errors, 0);
        assert_eq!(m.accuracy, 100.0);
    }

    #[test]
    fn corrected_errors_hurt_accuracy_not_wpm() {
        let t0 = Instant::now();
        let mut s = session();
        let target = s.buffer().target()[0].clone();
        let wrong = if target == "#" { '%' } else { '#' };
        s.type_char(wrong, t0);
        s.backspace(t0);
        type_target(&mut s, 1, t0);
        let m = s.metrics();
        assert_eq!(m.errors, 1);
        assert_eq!(m.uncorrected_errors, 0);
        assert_eq!(m.correct_chars, 1);
        assert_eq!(m.accuracy, 50.0);
    }

    #[test]
    fn uncorrected_errors_counted() {
        let t0 = Instant::now();
        let mut s = session();
        s.type_char('#', t0);
        assert_eq!(s.metrics().uncorrected_errors, 1);
    }
}
