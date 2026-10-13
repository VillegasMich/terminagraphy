//! Local persistence: session history (JSON Lines), settings (TOML), the
//! cache of fetched paragraphs (JSON Lines) and the daily tracker derived
//! from history.

pub mod config;
pub mod history;
pub mod paragraph_cache;
pub mod tracker;

use std::io;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

pub use config::Settings;
pub use history::SessionRecord;
pub use tracker::DailyTracker;

const HISTORY_FILE: &str = "history.jsonl";
const CONFIG_FILE: &str = "config.toml";
const PARAGRAPH_CACHE_FILE: &str = "paragraph_cache.jsonl";

#[derive(Debug, Clone)]
pub struct DataPaths {
    pub history: PathBuf,
    pub config: PathBuf,
    pub paragraph_cache: PathBuf,
}

impl DataPaths {
    /// Uses `data_dir` for every file when given, otherwise the OS data/config dirs.
    pub fn resolve(data_dir: Option<&Path>) -> io::Result<Self> {
        if let Some(dir) = data_dir {
            return Ok(Self {
                history: dir.join(HISTORY_FILE),
                config: dir.join(CONFIG_FILE),
                paragraph_cache: dir.join(PARAGRAPH_CACHE_FILE),
            });
        }
        let dirs = ProjectDirs::from("", "", "terminagraphy").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not determine a home directory; pass --data-dir",
            )
        })?;
        Ok(Self {
            history: dirs.data_dir().join(HISTORY_FILE),
            config: dirs.config_dir().join(CONFIG_FILE),
            paragraph_cache: dirs.data_dir().join(PARAGRAPH_CACHE_FILE),
        })
    }
}

fn ensure_parent(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => std::fs::create_dir_all(parent),
        _ => Ok(()),
    }
}
