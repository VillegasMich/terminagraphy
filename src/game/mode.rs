use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::content::language::cycle;

/// Timed session length.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, clap::ValueEnum,
)]
pub enum Mode {
    #[serde(rename = "30s")]
    #[value(name = "30s")]
    Sprint,
    #[default]
    #[serde(rename = "2m")]
    #[value(name = "2m")]
    Normal,
    #[serde(rename = "5m")]
    #[value(name = "5m")]
    Endurance,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Sprint, Mode::Normal, Mode::Endurance];

    pub fn duration(self) -> Duration {
        Duration::from_secs(match self {
            Mode::Sprint => 30,
            Mode::Normal => 120,
            Mode::Endurance => 300,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Sprint => "30s",
            Mode::Normal => "2m",
            Mode::Endurance => "5m",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Sprint => "Sprint",
            Mode::Normal => "Normal",
            Mode::Endurance => "Endurance",
        }
    }

    pub fn next(self) -> Self {
        cycle(&Self::ALL, self, 1)
    }

    pub fn prev(self) -> Self {
        cycle(&Self::ALL, self, Self::ALL.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(Mode::Sprint.duration().as_secs(), 30);
        assert_eq!(Mode::Normal.duration().as_secs(), 120);
        assert_eq!(Mode::Endurance.duration().as_secs(), 300);
    }

    #[test]
    fn serializes_as_label() {
        for mode in Mode::ALL {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{}\"", mode.label()));
        }
    }
}
