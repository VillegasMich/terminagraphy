//! Online paragraphs: random sentences from the Tatoeba API, cleaned and
//! joined into paragraphs, stored in the paragraph cache by a background
//! thread. Every failure is ignored; the game falls back to the embedded
//! corpus.

use std::fmt;
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::thread;
use std::time::Duration;

use serde::Deserialize;

use crate::content::sanitize::{assemble, sanitize_sentence};
use crate::content::{Language, Source};
use crate::storage::paragraph_cache::{CachedParagraph, SharedCache};

const API_URL: &str = "https://api.tatoeba.org/unstable/sentences";
const USER_AGENT: &str = concat!(
    "terminagraphy/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/VillegasMich/terminagraphy)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Sentences requested per call (the API maximum).
const BATCH_SIZE: u32 = 100;
/// Pause between consecutive requests, to be gentle with a volunteer-run API.
const REQUEST_PAUSE: Duration = Duration::from_secs(1);
/// Paragraphs per language the cache is filled up to.
pub const CACHE_TARGET: usize = 100;
/// A language is refilled once its cache drops below this many paragraphs.
pub const REFILL_BELOW: usize = 30;
/// Upper bound on requests per language in one refill pass, in case most
/// sentences are rejected by the sanitizer.
const MAX_REQUESTS_PER_PASS: usize = 20;
/// How often the cache is re-checked when no session wakes the thread.
const IDLE_RECHECK: Duration = Duration::from_secs(10 * 60);

fn api_code(language: Language) -> &'static str {
    match language {
        Language::En => "eng",
        Language::Es => "spa",
    }
}

#[derive(Debug)]
pub enum FetchError {
    Http(Box<ureq::Error>),
    Json(serde_json::Error),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Http(e) => write!(f, "tatoeba request failed: {e}"),
            FetchError::Json(e) => write!(f, "invalid tatoeba response: {e}"),
        }
    }
}

impl std::error::Error for FetchError {}

#[derive(Deserialize)]
struct Response {
    data: Vec<serde_json::Value>,
}

/// The fields we need from one sentence; everything else is ignored.
#[derive(Deserialize)]
struct Sentence {
    text: String,
    lang: Option<String>,
    license: Option<String>,
    #[serde(default)]
    is_unapproved: bool,
}

/// Extracts usable sentence texts from an API response body. Entries that are
/// malformed, unapproved, in another language or not under a CC BY license
/// are skipped.
pub fn parse_sentences(body: &str, language: Language) -> Result<Vec<String>, serde_json::Error> {
    let response: Response = serde_json::from_str(body)?;
    Ok(response
        .data
        .into_iter()
        .filter_map(|value| serde_json::from_value::<Sentence>(value).ok())
        .filter(|s| {
            !s.is_unapproved
                && s.lang.as_deref() == Some(api_code(language))
                && s.license.as_deref().is_some_and(|l| l.starts_with("CC BY"))
        })
        .map(|s| s.text)
        .collect())
}

/// Turns raw sentences into cache records that follow the content rules.
pub fn build_paragraphs(sentences: &[String], language: Language) -> Vec<CachedParagraph> {
    let clean = sentences
        .iter()
        .filter_map(|s| sanitize_sentence(s, language));
    assemble(clean)
        .into_iter()
        .map(|text| CachedParagraph::new(language, text, Source::Tatoeba))
        .collect()
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into()
}

/// One blocking request for a batch of random sentences.
fn fetch_sentences(agent: &ureq::Agent, language: Language) -> Result<Vec<String>, FetchError> {
    let body = agent
        .get(API_URL)
        .query("lang", api_code(language))
        .query("sort", "random")
        .query("limit", BATCH_SIZE.to_string())
        .call()
        .and_then(|mut response| response.body_mut().read_to_string())
        .map_err(|e| FetchError::Http(Box::new(e)))?;
    parse_sentences(&body, language).map_err(FetchError::Json)
}

/// Tops up every language whose cache is below `REFILL_BELOW`, up to
/// `CACHE_TARGET`. Stops a language at the first failed fetch or cache error.
/// Returns how many paragraphs were added per language.
pub fn refill<F>(cache: &SharedCache, mut fetch: F) -> Vec<(Language, usize)>
where
    F: FnMut(Language) -> Result<Vec<String>, FetchError>,
{
    let mut added = Vec::new();
    for language in Language::ALL {
        let Ok(mut have) = cache.count(language) else {
            continue;
        };
        if have >= REFILL_BELOW {
            continue;
        }
        let start = have;
        for _ in 0..MAX_REQUESTS_PER_PASS {
            let Ok(sentences) = fetch(language) else {
                break;
            };
            let mut batch = build_paragraphs(&sentences, language);
            batch.truncate(CACHE_TARGET - have);
            if cache.append(&batch).is_err() {
                break;
            }
            have += batch.len();
            if have >= CACHE_TARGET {
                break;
            }
        }
        added.push((language, have - start));
    }
    added
}

/// Starts the detached prefetch thread and returns the sender that wakes it
/// for an early re-check (see `SharedCache::notify_on_take`). The thread only
/// blocks on the network and its wake channel; it never holds the cache lock
/// while waiting, and it doesn't keep the process alive: it is never joined,
/// so it ends with the process (or when every sender is dropped).
pub fn spawn_prefetch(cache: SharedCache) -> std::io::Result<SyncSender<()>> {
    let (tx, rx) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("tatoeba-prefetch".into())
        .spawn(move || {
            let agent = agent();
            loop {
                refill(&cache, |language| {
                    let result = fetch_sentences(&agent, language);
                    thread::sleep(REQUEST_PAUSE);
                    result
                });
                match rx.recv_timeout(IDLE_RECHECK) {
                    Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        })?;
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use unicode_normalization::is_nfc;

    use super::*;
    use crate::content::sanitize::{MAX_LEN, MIN_LEN, is_allowed};

    /// Shaped like a real `/unstable/sentences` response, with invented text.
    const FIXTURE: &str = r#"{
      "data": [
        {"id": 1, "text": "The kettle whistled while we argued about maps.", "lang": "eng",
         "script": null, "license": "CC BY 2.0 FR", "owner": "someone", "is_unapproved": false,
         "translations": [], "new_field": {"nested": true}},
        {"id": 2, "text": "Unapproved sentence.", "lang": "eng", "license": "CC BY 2.0 FR",
         "owner": "someone", "is_unapproved": true},
        {"id": 3, "text": "No license here.", "lang": "eng", "license": null,
         "owner": "someone", "is_unapproved": false},
        {"id": 4, "text": "Missing license field.", "lang": "eng", "owner": "someone"},
        {"id": 5, "text": "Public domain sentence.", "lang": "eng", "license": "CC0 1.0",
         "owner": "someone", "is_unapproved": false},
        {"id": 6, "text": "Wrong language.", "lang": "fra", "license": "CC BY 2.0 FR",
         "owner": "someone", "is_unapproved": false},
        {"id": 7, "text": null, "lang": "eng", "license": "CC BY 2.0 FR"},
        {"id": 8, "text": "Her “quick” answer… surprised nobody.", "lang": "eng",
         "license": "CC BY 2.0 FR", "owner": "someone"}
      ],
      "paging": {"total": 2, "has_next": true, "next": "https://example.invalid/next"}
    }"#;

    #[test]
    fn parse_keeps_only_approved_cc_by_entries() {
        let sentences = parse_sentences(FIXTURE, Language::En).unwrap();
        assert_eq!(
            sentences,
            vec![
                "The kettle whistled while we argued about maps.",
                "Her \u{201c}quick\u{201d} answer\u{2026} surprised nobody.",
            ]
        );
    }

    #[test]
    fn parse_filters_by_requested_language() {
        assert_eq!(
            parse_sentences(FIXTURE, Language::Es).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse_sentences("<html>busy</html>", Language::En).is_err());
        assert!(parse_sentences("{\"paging\": {}}", Language::En).is_err());
    }

    fn spanish_sentences() -> Vec<String> {
        [
            "¿Dónde dejaste las llaves del coche?",
            "Mañana lloverá en la sierra según la radio.",
            "El pan de la esquina sale caliente a las siete.",
            "Nadie sabía por qué el reloj iba adelantado.",
            "«Ya voy», respondió desde la cocina.",
            "Mi abuela guarda botones en una lata vieja.",
            "Ese cuadro tiene un marco demasiado grande \u{1F600}.",
            "¡Qué frío hace esta noche junto al río!",
        ]
        .iter()
        .map(|s| s.to_string())
        .cycle()
        .take(200)
        .collect()
    }

    #[test]
    fn built_paragraphs_follow_content_rules() {
        let paragraphs = build_paragraphs(&spanish_sentences(), Language::Es);
        assert!(paragraphs.len() >= 5);
        for p in &paragraphs {
            assert_eq!(p.language, Language::Es);
            assert_eq!(p.source, Source::Tatoeba);
            let len = p.text.chars().count();
            assert!((MIN_LEN..=MAX_LEN).contains(&len), "{len}: {}", p.text);
            assert!(is_nfc(&p.text));
            assert!(p.text.chars().all(|c| is_allowed(c, Language::Es)));
            assert!(
                !p.text.contains('\u{1F600}'),
                "emoji sentence must be dropped"
            );
        }
    }

    fn cache() -> (tempfile::TempDir, SharedCache) {
        let dir = tempfile::tempdir().unwrap();
        let cache = SharedCache::new(dir.path().join("paragraph_cache.jsonl"));
        (dir, cache)
    }

    #[test]
    fn refill_fills_low_languages_up_to_target() {
        let (_dir, cache) = cache();
        let mut calls = 0;
        let added = refill(&cache, |language| {
            calls += 1;
            Ok(match language {
                Language::Es => spanish_sentences(),
                Language::En => vec!["x".repeat(199) + "."; 40],
            })
        });
        assert_eq!(
            added,
            vec![(Language::En, CACHE_TARGET), (Language::Es, CACHE_TARGET)]
        );
        assert_eq!(cache.count(Language::En).unwrap(), CACHE_TARGET);
        assert_eq!(cache.count(Language::Es).unwrap(), CACHE_TARGET);

        // Full caches need no requests.
        let calls_before = calls;
        assert_eq!(refill(&cache, |_| unreachable!()), vec![]);
        assert_eq!(calls, calls_before);
    }

    #[test]
    fn refill_stops_on_fetch_error() {
        let (_dir, cache) = cache();
        let mut calls = 0;
        let added = refill(&cache, |_| {
            calls += 1;
            Err(FetchError::Json(
                serde_json::from_str::<()>("x").unwrap_err(),
            ))
        });
        assert_eq!(added, vec![(Language::En, 0), (Language::Es, 0)]);
        assert_eq!(calls, 2, "one failed attempt per language, no retry loop");
    }

    #[test]
    fn refill_gives_up_when_everything_is_rejected() {
        let (_dir, cache) = cache();
        let mut calls = 0;
        refill(&cache, |_| {
            calls += 1;
            Ok(vec!["\u{1F600}".into()])
        });
        assert_eq!(calls, 2 * MAX_REQUESTS_PER_PASS);
    }
}
