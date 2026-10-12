use serde::{Deserialize, Serialize};

/// Language of the paragraphs the player types.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Es,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::En, Language::Es];

    pub fn name(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Es => "Español",
        }
    }

    pub fn next(self) -> Self {
        cycle(&Self::ALL, self, 1)
    }

    pub fn prev(self) -> Self {
        cycle(&Self::ALL, self, Self::ALL.len() - 1)
    }
}

/// Returns the item `step` positions after `current` in `all`, wrapping around.
pub(crate) fn cycle<T: Copy + PartialEq>(all: &[T], current: T, step: usize) -> T {
    let idx = all.iter().position(|x| *x == current).unwrap_or(0);
    all[(idx + step) % all.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_both_ways() {
        assert_eq!(Language::En.next(), Language::Es);
        assert_eq!(Language::Es.next(), Language::En);
        assert_eq!(Language::En.prev(), Language::Es);
    }

    #[test]
    fn serializes_as_code() {
        assert_eq!(serde_json::to_string(&Language::Es).unwrap(), "\"es\"");
    }
}
