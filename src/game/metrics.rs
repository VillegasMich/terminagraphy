use std::time::Duration;

/// Characters per "word" in WPM calculations.
const CHARS_PER_WORD: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Correct,
    Wrong,
    Backspace,
}

/// One entry of the session keystroke log; `at` is time since the session started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keystroke {
    pub at: Duration,
    pub kind: KeyKind,
}

/// Per-second sample used for the results chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// End of the one-second bucket (1, 2, 3, ...).
    pub second: u32,
    /// Cumulative net WPM up to this second.
    pub wpm: f64,
    /// Raw WPM within this second only.
    pub raw: f64,
    /// Wrong keystrokes within this second.
    pub errors: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub consistency: f64,
    /// Every wrong keystroke, corrected or not.
    pub errors: u32,
    /// Wrong characters still present when a paragraph completed or time ran out.
    pub uncorrected_errors: u32,
    /// Correct characters in the final text of every paragraph.
    pub correct_chars: u32,
    /// Correct + wrong keystrokes (backspaces excluded).
    pub keystrokes: u32,
    pub paragraphs: u32,
    pub samples: Vec<Sample>,
}

/// Final state of the typed text, as opposed to the keystroke history.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextTotals {
    pub correct_chars: u32,
    pub uncorrected_errors: u32,
    pub paragraphs: u32,
}

pub fn wpm(chars: u32, elapsed: Duration) -> f64 {
    let minutes = elapsed.as_secs_f64() / 60.0;
    if minutes <= 0.0 {
        return 0.0;
    }
    chars as f64 / CHARS_PER_WORD / minutes
}

/// Accuracy in percent; 100 when nothing was typed.
pub fn accuracy(correct: u32, wrong: u32) -> f64 {
    let total = correct + wrong;
    if total == 0 {
        return 100.0;
    }
    correct as f64 / total as f64 * 100.0
}

/// `100 - coefficient of variation` of the samples, clamped to 0..=100.
pub fn consistency(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    if mean <= 0.0 {
        return 0.0;
    }
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (100.0 - variance.sqrt() / mean * 100.0).clamp(0.0, 100.0)
}

pub fn compute(log: &[Keystroke], duration: Duration, totals: TextTotals) -> Metrics {
    let count = |kind: KeyKind| log.iter().filter(|k| k.kind == kind).count() as u32;
    let correct_keys = count(KeyKind::Correct);
    let wrong_keys = count(KeyKind::Wrong);

    let seconds = duration.as_secs_f64().ceil().max(1.0) as u32;
    let mut correct_per_sec = vec![0u32; seconds as usize];
    let mut typed_per_sec = vec![0u32; seconds as usize];
    let mut errors_per_sec = vec![0u32; seconds as usize];
    for k in log {
        let bucket = (k.at.as_secs() as usize).min(seconds as usize - 1);
        match k.kind {
            KeyKind::Correct => {
                correct_per_sec[bucket] += 1;
                typed_per_sec[bucket] += 1;
            }
            KeyKind::Wrong => {
                errors_per_sec[bucket] += 1;
                typed_per_sec[bucket] += 1;
            }
            KeyKind::Backspace => {}
        }
    }

    let mut cumulative = 0;
    let samples: Vec<Sample> = (0..seconds as usize)
        .map(|i| {
            cumulative += correct_per_sec[i];
            let second = i as u32 + 1;
            Sample {
                second,
                wpm: wpm(cumulative, Duration::from_secs(second as u64)),
                raw: wpm(typed_per_sec[i], Duration::from_secs(1)),
                errors: errors_per_sec[i],
            }
        })
        .collect();
    let raw_samples: Vec<f64> = samples.iter().map(|s| s.raw).collect();

    Metrics {
        wpm: wpm(totals.correct_chars, duration),
        raw_wpm: wpm(correct_keys + wrong_keys, duration),
        accuracy: accuracy(correct_keys, wrong_keys),
        consistency: consistency(&raw_samples),
        errors: wrong_keys,
        uncorrected_errors: totals.uncorrected_errors,
        correct_chars: totals.correct_chars,
        keystrokes: correct_keys + wrong_keys,
        paragraphs: totals.paragraphs,
        samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(ms: u64, kind: KeyKind) -> Keystroke {
        Keystroke {
            at: Duration::from_millis(ms),
            kind,
        }
    }

    #[test]
    fn wpm_uses_five_chars_per_word() {
        assert_eq!(wpm(300, Duration::from_secs(60)), 60.0);
        assert_eq!(wpm(150, Duration::from_secs(30)), 60.0);
        assert_eq!(wpm(10, Duration::ZERO), 0.0);
    }

    #[test]
    fn accuracy_counts_corrected_errors() {
        assert_eq!(accuracy(9, 1), 90.0);
        assert_eq!(accuracy(0, 0), 100.0);
    }

    #[test]
    fn consistency_perfectly_even_is_100() {
        assert_eq!(consistency(&[60.0, 60.0, 60.0]), 100.0);
        assert_eq!(consistency(&[0.0, 0.0]), 0.0);
        let c = consistency(&[40.0, 80.0]);
        assert!((c - 66.666).abs() < 0.01, "{c}");
    }

    #[test]
    fn compute_from_log() {
        let log = vec![
            key(100, KeyKind::Correct),
            key(200, KeyKind::Wrong),
            key(300, KeyKind::Backspace),
            key(400, KeyKind::Correct),
            key(1500, KeyKind::Correct),
        ];
        let totals = TextTotals {
            correct_chars: 3,
            uncorrected_errors: 0,
            paragraphs: 0,
        };
        let m = compute(&log, Duration::from_secs(2), totals);
        assert_eq!(m.errors, 1);
        assert_eq!(m.keystrokes, 4);
        assert_eq!(m.accuracy, 75.0);
        assert_eq!(m.wpm, wpm(3, Duration::from_secs(2)));
        assert_eq!(m.raw_wpm, wpm(4, Duration::from_secs(2)));
        assert_eq!(m.samples.len(), 2);
        assert_eq!(m.samples[0].errors, 1);
        assert_eq!(m.samples[0].raw, wpm(3, Duration::from_secs(1)));
        assert_eq!(m.samples[1].wpm, wpm(3, Duration::from_secs(2)));
    }

    #[test]
    fn keystroke_at_exact_end_goes_to_last_bucket() {
        let log = vec![key(2000, KeyKind::Correct)];
        let m = compute(&log, Duration::from_secs(2), TextTotals::default());
        assert_eq!(m.samples.len(), 2);
        assert_eq!(m.samples[1].raw, 12.0);
    }
}
