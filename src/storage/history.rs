use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::content::Language;
use crate::game::{Metrics, Mode};

pub const SCHEMA_VERSION: u32 = 1;

/// One completed session, stored as one line of `history.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub version: u32,
    pub ended_at: DateTime<Local>,
    pub language: Language,
    pub mode: Mode,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub errors: u32,
    pub uncorrected_errors: u32,
    pub correct_chars: u32,
    pub total_chars: u32,
    pub paragraphs: u32,
}

impl SessionRecord {
    pub fn new(
        metrics: &Metrics,
        mode: Mode,
        language: Language,
        ended_at: DateTime<Local>,
    ) -> Self {
        Self {
            version: SCHEMA_VERSION,
            ended_at,
            language,
            mode,
            wpm: metrics.wpm,
            raw_wpm: metrics.raw_wpm,
            accuracy: metrics.accuracy,
            consistency: metrics.consistency,
            errors: metrics.errors,
            uncorrected_errors: metrics.uncorrected_errors,
            correct_chars: metrics.correct_chars,
            total_chars: metrics.keystrokes,
            paragraphs: metrics.paragraphs,
        }
    }

    pub fn is_category(&self, language: Language, mode: Mode) -> bool {
        self.language == language && self.mode == mode
    }
}

#[derive(Debug, Default)]
pub struct LoadReport {
    pub records: Vec<SessionRecord>,
    /// Lines that could not be parsed or have an unknown schema version.
    pub skipped: usize,
}

/// Loads all records. A missing file is an empty history; bad lines are skipped.
pub fn load(path: &Path) -> io::Result<LoadReport> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(LoadReport::default()),
        Err(e) => return Err(e),
    };
    let mut report = LoadReport::default();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<SessionRecord>(line) {
            Ok(record) if record.version <= SCHEMA_VERSION => report.records.push(record),
            _ => report.skipped += 1,
        }
    }
    Ok(report)
}

/// Appends one record as a single line.
pub fn append(path: &Path, record: &SessionRecord) -> io::Result<()> {
    super::ensure_parent(path)?;
    let mut line = serde_json::to_string(record).map_err(io::Error::other)?;
    line.push('\n');
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    file.flush()
}

pub fn personal_best(
    records: &[SessionRecord],
    language: Language,
    mode: Mode,
) -> Option<&SessionRecord> {
    records
        .iter()
        .filter(|r| r.is_category(language, mode))
        .max_by(|a, b| a.wpm.total_cmp(&b.wpm))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(wpm: f64, language: Language, mode: Mode) -> SessionRecord {
        SessionRecord {
            version: SCHEMA_VERSION,
            ended_at: Local::now(),
            language,
            mode,
            wpm,
            raw_wpm: wpm,
            accuracy: 100.0,
            consistency: 90.0,
            errors: 0,
            uncorrected_errors: 0,
            correct_chars: 100,
            total_chars: 100,
            paragraphs: 1,
        }
    }

    #[test]
    fn append_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/history.jsonl");
        let a = record(50.0, Language::En, Mode::Sprint);
        let b = record(60.0, Language::Es, Mode::Normal);
        append(&path, &a).unwrap();
        append(&path, &b).unwrap();
        let report = load(&path).unwrap();
        assert_eq!(report.records, vec![a, b]);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let report = load(&dir.path().join("nope.jsonl")).unwrap();
        assert!(report.records.is_empty());
    }

    #[test]
    fn corrupted_and_future_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");
        let good = record(50.0, Language::En, Mode::Sprint);
        let mut future = serde_json::to_value(&good).unwrap();
        future["version"] = 99.into();
        let text = format!(
            "{}\nnot json\n{{\"half\":\n{}\n",
            serde_json::to_string(&good).unwrap(),
            future
        );
        std::fs::write(&path, text).unwrap();
        let report = load(&path).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.skipped, 3);
    }

    #[test]
    fn personal_best_is_per_category() {
        let records = vec![
            record(50.0, Language::En, Mode::Sprint),
            record(70.0, Language::En, Mode::Sprint),
            record(90.0, Language::Es, Mode::Sprint),
        ];
        let pb = personal_best(&records, Language::En, Mode::Sprint).unwrap();
        assert_eq!(pb.wpm, 70.0);
        assert!(personal_best(&records, Language::En, Mode::Normal).is_none());
    }
}
