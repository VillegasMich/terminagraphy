//! Content rules from `docs/CONTENT.md`, shared by the embedded corpus check
//! and the cleanup of sentences fetched at runtime.

use unicode_normalization::UnicodeNormalization;

use super::Language;

/// Shortest paragraph allowed, in characters.
pub const MIN_LEN: usize = 150;
/// Longest paragraph allowed, in characters.
pub const MAX_LEN: usize = 400;

/// Whether `c` may appear in a paragraph of `language`.
pub fn is_allowed(c: char, language: Language) -> bool {
    c.is_ascii_alphanumeric()
        || " .,;:!?'\"-()".contains(c)
        || (language == Language::Es && "áéíóúüñÁÉÍÓÚÜÑ¿¡".contains(c))
}

/// Cleans one sentence: NFC, typographic quotes/dashes/ellipsis replaced by
/// their typable ASCII forms, whitespace collapsed. Returns `None` if the
/// result is empty, too long to fit a paragraph, or still has characters
/// outside the allowed set.
pub fn sanitize_sentence(raw: &str, language: Language) -> Option<String> {
    let mut replaced = String::with_capacity(raw.len());
    for c in raw.nfc() {
        match c {
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{2032}' => replaced.push('\''),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '«' | '»' => replaced.push('"'),
            '\u{2010}' | '\u{2011}' | '\u{2013}' | '\u{2014}' => replaced.push('-'),
            '…' => replaced.push_str("..."),
            c => replaced.push(c),
        }
    }
    let sentence = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let len = sentence.chars().count();
    let valid = len > 0 && len <= MAX_LEN && sentence.chars().all(|c| is_allowed(c, language));
    valid.then_some(sentence)
}

/// Joins sentences, in order, into paragraphs of `MIN_LEN..=MAX_LEN`
/// characters separated by single spaces. A sentence that would push a
/// too-short paragraph past `MAX_LEN` is skipped; whatever is left at the end
/// without reaching `MIN_LEN` is dropped.
pub fn assemble(sentences: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();
    let mut current_len = 0;
    for sentence in sentences {
        let len = sentence.chars().count();
        if len == 0 || len > MAX_LEN {
            continue;
        }
        if current_len == 0 {
            current = sentence;
            current_len = len;
        } else if current_len + 1 + len <= MAX_LEN {
            current.push(' ');
            current.push_str(&sentence);
            current_len += 1 + len;
        } else if current_len >= MIN_LEN {
            paragraphs.push(std::mem::replace(&mut current, sentence));
            current_len = len;
        }
    }
    if current_len >= MIN_LEN {
        paragraphs.push(current);
    }
    paragraphs
}

#[cfg(test)]
mod tests {
    use unicode_normalization::is_nfc;

    use super::*;

    #[test]
    fn sanitize_replaces_typographic_characters() {
        let raw = "\u{201C}It\u{2019}s late\u{201D} \u{2014} she said\u{2026}";
        assert_eq!(
            sanitize_sentence(raw, Language::En).as_deref(),
            Some("\"It's late\" - she said...")
        );
        assert_eq!(
            sanitize_sentence("«Hola», dijo – adiós.", Language::Es).as_deref(),
            Some("\"Hola\", dijo - adiós.")
        );
    }

    #[test]
    fn sanitize_collapses_whitespace_and_trims() {
        let raw = "  The\tcat \u{A0} sat.\n";
        assert_eq!(
            sanitize_sentence(raw, Language::En).as_deref(),
            Some("The cat sat.")
        );
    }

    #[test]
    fn sanitize_normalizes_to_nfc() {
        let decomposed = "¿Que\u{301} an\u{303}o es?";
        let clean = sanitize_sentence(decomposed, Language::Es).unwrap();
        assert_eq!(clean, "¿Qué año es?");
        assert!(is_nfc(&clean));
    }

    #[test]
    fn sanitize_rejects_disallowed_characters() {
        for raw in ["I ❤ tea.", "Price: 5€.", "a/b", "Tab\u{0}"] {
            assert_eq!(sanitize_sentence(raw, Language::En), None, "{raw}");
        }
        // Spanish letters belong to Spanish only.
        assert_eq!(sanitize_sentence("Mañana.", Language::En), None);
        assert!(sanitize_sentence("Mañana.", Language::Es).is_some());
    }

    #[test]
    fn sanitize_rejects_empty_and_overlong() {
        assert_eq!(sanitize_sentence("   ", Language::En), None);
        let long = "a".repeat(MAX_LEN + 1);
        assert_eq!(sanitize_sentence(&long, Language::En), None);
    }

    fn sentences(lens: &[usize]) -> Vec<String> {
        lens.iter().map(|&n| "x".repeat(n - 1) + ".").collect()
    }

    fn assert_in_bounds(paragraphs: &[String]) {
        for p in paragraphs {
            let len = p.chars().count();
            assert!((MIN_LEN..=MAX_LEN).contains(&len), "{len}: {p}");
            assert!(!p.contains("  "));
        }
    }

    #[test]
    fn assemble_joins_with_single_spaces() {
        let out = assemble(sentences(&[80, 80, 80]));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].chars().count(), 80 * 3 + 2);
        assert_in_bounds(&out);
    }

    #[test]
    fn assemble_never_exceeds_max_and_drops_short_leftover() {
        let out = assemble(sentences(&[200, 199, 150, 100, 60]));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].chars().count(), 400);
        assert_eq!(out[1].chars().count(), 150 + 1 + 100 + 1 + 60);
        assert_in_bounds(&out);

        assert_eq!(assemble(sentences(&[100, 40])), Vec::<String>::new());
    }

    #[test]
    fn assemble_skips_sentences_that_cannot_fit() {
        // 100 is too short to flush, 350 would overflow it: skip 350, keep growing.
        let out = assemble(sentences(&[100, 350, 60]));
        assert_eq!(
            out,
            vec![format!("{} {}", "x".repeat(99) + ".", "x".repeat(59) + ".")]
        );
        assert_eq!(assemble(sentences(&[401])), Vec::<String>::new());
    }

    #[test]
    fn assemble_many_sentences_stays_in_bounds() {
        let lens: Vec<usize> = (0..500).map(|i| 5 + (i * 37) % 120).collect();
        let out = assemble(sentences(&lens));
        assert!(out.len() > 20);
        assert_in_bounds(&out);
    }
}
