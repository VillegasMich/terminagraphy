use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::content::Language;
use crate::game::Mode;

/// User settings stored in `config.toml`. Missing fields fall back to defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Language preselected in the menu (last one played).
    pub language: Language,
    /// Mode preselected in the menu (last one played).
    pub mode: Mode,
    /// Show WPM / accuracy while typing.
    pub show_live_stats: bool,
    /// Fetch paragraphs from tatoeba.org in the background. When `false`
    /// only the embedded corpus is used.
    pub online: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::default(),
            mode: Mode::default(),
            show_live_stats: true,
            online: true,
        }
    }
}

/// Loads settings. A missing file gives defaults; an invalid file gives
/// defaults plus a warning message instead of an error.
pub fn load(path: &Path) -> io::Result<(Settings, Option<String>)> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok((Settings::default(), None)),
        Err(e) => return Err(e),
    };
    match toml::from_str(&text) {
        Ok(settings) => Ok((settings, None)),
        Err(e) => Ok((
            Settings::default(),
            Some(format!(
                "Invalid {}, using defaults: {}",
                path.display(),
                e.message()
            )),
        )),
    }
}

/// Writes settings atomically (temp file + rename).
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    super::ensure_parent(path)?;
    let text = toml::to_string_pretty(settings).map_err(io::Error::other)?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cfg/config.toml");
        let settings = Settings {
            language: Language::Es,
            mode: Mode::Endurance,
            show_live_stats: false,
            online: false,
        };
        save(&path, &settings).unwrap();
        let (loaded, warning) = load(&path).unwrap();
        assert_eq!(loaded, settings);
        assert!(warning.is_none());
        assert!(!path.with_extension("toml.tmp").exists());
    }

    #[test]
    fn missing_fields_use_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "language = \"es\"\n").unwrap();
        let (loaded, _) = load(&path).unwrap();
        assert_eq!(loaded.language, Language::Es);
        assert_eq!(loaded.mode, Mode::default());
        assert!(loaded.show_live_stats);
        assert!(
            loaded.online,
            "configs written before `online` existed stay online"
        );
    }

    #[test]
    fn online_defaults_to_true_and_can_be_disabled() {
        assert!(Settings::default().online);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "online = false\n").unwrap();
        let (loaded, warning) = load(&path).unwrap();
        assert!(!loaded.online);
        assert!(warning.is_none());
    }

    #[test]
    fn invalid_file_warns_and_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "mode = \"forever\"").unwrap();
        let (loaded, warning) = load(&path).unwrap();
        assert_eq!(loaded, Settings::default());
        assert!(warning.is_some());
    }
}
