//! Paragraphs assembled from text fetched online, waiting to be typed
//! (`paragraph_cache.jsonl`, one `CachedParagraph` per line). The prefetch
//! thread appends and sessions consume, so every access goes through the lock
//! in `SharedCache`.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use crate::content::{Language, Paragraph, ParagraphPicker, ParagraphSupply, Source};

pub const SCHEMA_VERSION: u32 = 1;

/// One cached paragraph, stored as one line of `paragraph_cache.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedParagraph {
    pub version: u32,
    pub language: Language,
    pub text: String,
    pub source: Source,
}

impl CachedParagraph {
    pub fn new(language: Language, text: String, source: Source) -> Self {
        Self {
            version: SCHEMA_VERSION,
            language,
            text,
            source,
        }
    }
}

#[derive(Debug, Default)]
pub struct LoadReport {
    pub paragraphs: Vec<CachedParagraph>,
    /// Lines that could not be parsed or have an unknown schema version.
    pub skipped: usize,
}

/// Loads all cached paragraphs. A missing file is an empty cache; bad lines are skipped.
pub fn load(path: &Path) -> io::Result<LoadReport> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(LoadReport::default()),
        Err(e) => return Err(e),
    };
    let mut report = LoadReport::default();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<CachedParagraph>(line) {
            Ok(p) if p.version <= SCHEMA_VERSION => report.paragraphs.push(p),
            _ => report.skipped += 1,
        }
    }
    Ok(report)
}

fn to_lines(paragraphs: &[CachedParagraph]) -> io::Result<String> {
    let mut out = String::new();
    for p in paragraphs {
        out.push_str(&serde_json::to_string(p).map_err(io::Error::other)?);
        out.push('\n');
    }
    Ok(out)
}

/// Appends paragraphs, one line each, in a single write.
pub fn append(path: &Path, paragraphs: &[CachedParagraph]) -> io::Result<()> {
    if paragraphs.is_empty() {
        return Ok(());
    }
    super::ensure_parent(path)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(to_lines(paragraphs)?.as_bytes())?;
    file.flush()
}

/// Replaces the whole file atomically (temp file + rename).
fn rewrite(path: &Path, paragraphs: &[CachedParagraph]) -> io::Result<()> {
    super::ensure_parent(path)?;
    let tmp = path.with_extension("jsonl.tmp");
    std::fs::write(&tmp, to_lines(paragraphs)?)?;
    std::fs::rename(&tmp, path)
}

/// Removes and returns the oldest paragraph for `language`. Unreadable lines
/// are dropped by the rewrite.
pub fn take(path: &Path, language: Language) -> io::Result<Option<CachedParagraph>> {
    let mut paragraphs = load(path)?.paragraphs;
    let Some(idx) = paragraphs.iter().position(|p| p.language == language) else {
        return Ok(None);
    };
    let taken = paragraphs.remove(idx);
    rewrite(path, &paragraphs)?;
    Ok(Some(taken))
}

/// Cached paragraphs available for `language`.
pub fn count(path: &Path, language: Language) -> io::Result<usize> {
    Ok(load(path)?
        .paragraphs
        .iter()
        .filter(|p| p.language == language)
        .count())
}

/// Thread-safe handle to the cache file. The lock is only held for the
/// duration of one small file operation, never across network calls.
#[derive(Debug, Clone)]
pub struct SharedCache {
    path: Arc<Mutex<PathBuf>>,
    /// Pinged after each `take` so the prefetch thread can top the cache up.
    on_take: Option<SyncSender<()>>,
}

impl SharedCache {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path: Arc::new(Mutex::new(path)),
            on_take: None,
        }
    }

    pub fn notify_on_take(mut self, tx: SyncSender<()>) -> Self {
        self.on_take = Some(tx);
        self
    }

    fn lock(&self) -> MutexGuard<'_, PathBuf> {
        // A panic elsewhere can't leave the file half-written (writes are
        // append or rename), so a poisoned lock is still safe to use.
        self.path.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn count(&self, language: Language) -> io::Result<usize> {
        count(&self.lock(), language)
    }

    pub fn append(&self, paragraphs: &[CachedParagraph]) -> io::Result<()> {
        append(&self.lock(), paragraphs)
    }

    pub fn take(&self, language: Language) -> io::Result<Option<CachedParagraph>> {
        let taken = take(&self.lock(), language);
        if let Some(tx) = &self.on_take {
            // Full means a wake-up is already pending; disconnected means no
            // prefetch thread. Either way there is nothing to do.
            let _ = tx.try_send(());
        }
        taken
    }
}

/// Session supply: cached paragraphs first, the embedded corpus only once the
/// cache has nothing left for the language (or can't be read).
#[derive(Debug)]
pub struct CachedSupply {
    cache: SharedCache,
    language: Language,
    fallback: ParagraphPicker,
}

impl CachedSupply {
    pub fn new(cache: SharedCache, language: Language) -> Self {
        Self {
            cache,
            language,
            fallback: ParagraphPicker::new(language),
        }
    }
}

impl ParagraphSupply for CachedSupply {
    fn next_paragraph(&mut self) -> Paragraph {
        match self.cache.take(self.language) {
            Ok(Some(cached)) => Paragraph {
                text: cached.text,
                source: cached.source,
            },
            // Errors fall back silently: the game must keep working offline
            // or with an unwritable data dir.
            Ok(None) | Err(_) => self.fallback.next_paragraph(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn para(language: Language, text: &str) -> CachedParagraph {
        CachedParagraph::new(language, text.to_owned(), Source::Tatoeba)
    }

    #[test]
    fn append_then_take_is_fifo_per_language() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/paragraph_cache.jsonl");
        append(
            &path,
            &[para(Language::En, "one"), para(Language::Es, "uno")],
        )
        .unwrap();
        append(&path, &[para(Language::En, "two")]).unwrap();
        assert_eq!(count(&path, Language::En).unwrap(), 2);
        assert_eq!(count(&path, Language::Es).unwrap(), 1);

        assert_eq!(take(&path, Language::En).unwrap().unwrap().text, "one");
        assert_eq!(take(&path, Language::En).unwrap().unwrap().text, "two");
        assert_eq!(take(&path, Language::En).unwrap(), None);
        assert_eq!(count(&path, Language::Es).unwrap(), 1);
        assert!(!path.with_extension("jsonl.tmp").exists());
    }

    #[test]
    fn missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paragraph_cache.jsonl");
        assert_eq!(count(&path, Language::En).unwrap(), 0);
        assert_eq!(take(&path, Language::En).unwrap(), None);
        assert!(!path.exists());
    }

    #[test]
    fn bad_and_future_lines_are_skipped_and_dropped_on_rewrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paragraph_cache.jsonl");
        let good = para(Language::En, "good");
        let mut future = serde_json::to_value(&good).unwrap();
        future["version"] = 99.into();
        let text = format!(
            "not json\n{}\n{{\"half\":\n{future}\n{}\n",
            serde_json::to_string(&good).unwrap(),
            serde_json::to_string(&para(Language::En, "next")).unwrap(),
        );
        std::fs::write(&path, text).unwrap();

        let report = load(&path).unwrap();
        assert_eq!(report.paragraphs.len(), 2);
        assert_eq!(report.skipped, 3);

        assert_eq!(take(&path, Language::En).unwrap().unwrap().text, "good");
        let report = load(&path).unwrap();
        assert_eq!(report.paragraphs, vec![para(Language::En, "next")]);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn supply_uses_cache_then_falls_back_to_embedded() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SharedCache::new(dir.path().join("paragraph_cache.jsonl"));
        cache
            .append(&[para(Language::Es, "cached"), para(Language::En, "other")])
            .unwrap();
        let mut supply = CachedSupply::new(cache.clone(), Language::Es);

        let first = supply.next_paragraph();
        assert_eq!(first.text, "cached");
        assert_eq!(first.source, Source::Tatoeba);
        let second = supply.next_paragraph();
        assert_eq!(second.source, Source::Embedded);
        assert!(crate::content::paragraphs::corpus(Language::Es).contains(&second.text));
        assert_eq!(cache.count(Language::En).unwrap(), 1);
    }

    #[test]
    fn supply_with_empty_cache_is_embedded() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SharedCache::new(dir.path().join("paragraph_cache.jsonl"));
        let mut supply = CachedSupply::new(cache, Language::En);
        assert_eq!(supply.next_paragraph().source, Source::Embedded);
    }

    #[test]
    fn take_wakes_prefetch_without_blocking() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        let cache = SharedCache::new(dir.path().join("paragraph_cache.jsonl")).notify_on_take(tx);
        cache.take(Language::En).unwrap();
        cache.take(Language::En).unwrap();
        assert_eq!(rx.try_recv(), Ok(()));
        assert!(rx.try_recv().is_err());
    }
}
