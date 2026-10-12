use std::sync::OnceLock;

use rand::Rng;
use rand::seq::SliceRandom;
use unicode_normalization::UnicodeNormalization;

use super::Language;

const EN: &str = include_str!("../../assets/paragraphs/en.txt");
const ES: &str = include_str!("../../assets/paragraphs/es.txt");

/// Parses a corpus file: paragraphs separated by blank lines, `#` lines are
/// comments, inner line breaks become single spaces, output is NFC.
pub fn parse(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut flush = |current: &mut Vec<&str>| {
        if !current.is_empty() {
            let joined = current.join(" ");
            let collapsed = joined.split_whitespace().collect::<Vec<_>>().join(" ");
            out.push(collapsed.nfc().collect());
            current.clear();
        }
    };
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut current);
        } else {
            current.push(trimmed);
        }
    }
    flush(&mut current);
    out
}

/// All paragraphs for a language, parsed once and cached.
pub fn corpus(language: Language) -> &'static [String] {
    static EN_CORPUS: OnceLock<Vec<String>> = OnceLock::new();
    static ES_CORPUS: OnceLock<Vec<String>> = OnceLock::new();
    match language {
        Language::En => EN_CORPUS.get_or_init(|| parse(EN)),
        Language::Es => ES_CORPUS.get_or_init(|| parse(ES)),
    }
}

/// Shuffle bag over a corpus: every paragraph is handed out once before any
/// repeats, and a reshuffle never starts with the paragraph just shown.
#[derive(Debug, Clone)]
pub struct ParagraphPicker {
    items: &'static [String],
    order: Vec<usize>,
    pos: usize,
}

impl ParagraphPicker {
    pub fn new(language: Language) -> Self {
        Self::from_items(corpus(language))
    }

    pub fn from_items(items: &'static [String]) -> Self {
        assert!(!items.is_empty(), "paragraph corpus must not be empty");
        Self {
            items,
            order: Vec::new(),
            pos: 0,
        }
    }

    pub fn next_paragraph(&mut self) -> &'static str {
        self.next_with(&mut rand::rng())
    }

    pub fn next_with<R: Rng + ?Sized>(&mut self, rng: &mut R) -> &'static str {
        if self.pos >= self.order.len() {
            let last = self.order.last().copied();
            self.order = (0..self.items.len()).collect();
            self.order.shuffle(rng);
            if self.order.len() > 1 && self.order.first().copied() == last {
                let end = self.order.len() - 1;
                self.order.swap(0, end);
            }
            self.pos = 0;
        }
        let idx = self.order[self.pos];
        self.pos += 1;
        &self.items[idx]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use unicode_normalization::is_nfc;

    use super::*;

    /// Paragraph length range from docs/CONTENT.md.
    const MIN_LEN: usize = 150;
    const MAX_LEN: usize = 400;

    #[test]
    fn parse_joins_lines_and_skips_comments() {
        let src = "# comment\nfirst line\n  second   line \n\n\nnext one\n";
        assert_eq!(parse(src), vec!["first line second line", "next one"]);
    }

    #[test]
    fn picker_does_not_repeat_until_exhausted() {
        let items = corpus(Language::En);
        let mut picker = ParagraphPicker::from_items(items);
        let mut rng = StdRng::seed_from_u64(7);
        let seen: HashSet<_> = (0..items.len())
            .map(|_| picker.next_with(&mut rng))
            .collect();
        assert_eq!(seen.len(), items.len());
    }

    #[test]
    fn picker_never_repeats_across_reshuffle() {
        let mut picker = ParagraphPicker::from_items(corpus(Language::Es));
        let mut rng = StdRng::seed_from_u64(42);
        let mut prev = picker.next_with(&mut rng);
        for _ in 0..500 {
            let next = picker.next_with(&mut rng);
            assert_ne!(prev, next);
            prev = next;
        }
    }

    fn allowed(c: char, language: Language) -> bool {
        c.is_ascii_alphanumeric()
            || " .,;:!?'\"-()".contains(c)
            || (language == Language::Es && "áéíóúüñÁÉÍÓÚÜÑ¿¡".contains(c))
    }

    #[test]
    fn corpus_follows_content_guidelines() {
        for language in Language::ALL {
            let paragraphs = corpus(language);
            assert!(paragraphs.len() >= 20, "{language:?} corpus too small");
            for p in paragraphs {
                let len = p.chars().count();
                assert!(
                    (MIN_LEN..=MAX_LEN).contains(&len),
                    "{language:?} paragraph has {len} chars: {p}"
                );
                assert!(is_nfc(p), "{language:?} paragraph not NFC: {p}");
                if let Some(bad) = p.chars().find(|c| !allowed(*c, language)) {
                    panic!("{language:?} paragraph has disallowed char {bad:?}: {p}");
                }
            }
            let unique: HashSet<_> = paragraphs.iter().collect();
            assert_eq!(
                unique.len(),
                paragraphs.len(),
                "{language:?} has duplicates"
            );
        }
    }
}
