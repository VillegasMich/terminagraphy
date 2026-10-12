use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use unicode_segmentation::UnicodeSegmentation;

/// Splits text into NFC-normalized grapheme clusters.
pub fn graphemes(text: &str) -> Vec<String> {
    let normalized: String = text.nfc().collect();
    normalized.graphemes(true).map(String::from).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharStatus {
    Correct,
    Wrong,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputOutcome {
    /// A new grapheme was typed; `true` if it matched the target.
    Typed(bool),
    /// A combining mark was merged into the previous grapheme (e.g. `a` + `´`
    /// sent as two key events); the previous keystroke's correctness changed to this.
    Amended(bool),
    /// Nothing happened (paragraph already complete, or stray combining mark).
    Ignored,
}

/// What the player has typed for one paragraph, compared grapheme by grapheme.
#[derive(Debug, Clone)]
pub struct TypingBuffer {
    target: Vec<String>,
    typed: Vec<String>,
}

impl TypingBuffer {
    pub fn new(text: &str) -> Self {
        Self {
            target: graphemes(text),
            typed: Vec::new(),
        }
    }

    pub fn target(&self) -> &[String] {
        &self.target
    }

    pub fn cursor(&self) -> usize {
        self.typed.len()
    }

    pub fn is_complete(&self) -> bool {
        self.typed.len() >= self.target.len()
    }

    pub fn push_char(&mut self, c: char) -> InputOutcome {
        if is_combining_mark(c) {
            let Some(last) = self.typed.last_mut() else {
                return InputOutcome::Ignored;
            };
            let mut combined = std::mem::take(last);
            combined.push(c);
            *last = combined.nfc().collect();
            let idx = self.typed.len() - 1;
            return InputOutcome::Amended(self.typed[idx] == self.target[idx]);
        }
        if self.is_complete() {
            return InputOutcome::Ignored;
        }
        let idx = self.typed.len();
        let typed: String = c.to_string();
        let correct = typed == self.target[idx];
        self.typed.push(typed);
        InputOutcome::Typed(correct)
    }

    /// Removes the last typed grapheme. Returns `false` if nothing was typed.
    pub fn backspace(&mut self) -> bool {
        self.typed.pop().is_some()
    }

    /// Removes trailing spaces, then the previous word. Returns `false` if nothing was typed.
    pub fn delete_word(&mut self) -> bool {
        if self.typed.is_empty() {
            return false;
        }
        while self.typed.last().is_some_and(|g| g == " ") {
            self.typed.pop();
        }
        while self.typed.last().is_some_and(|g| g != " ") {
            self.typed.pop();
        }
        true
    }

    pub fn status(&self, idx: usize) -> CharStatus {
        match self.typed.get(idx) {
            None => CharStatus::Pending,
            Some(g) if *g == self.target[idx] => CharStatus::Correct,
            Some(_) => CharStatus::Wrong,
        }
    }

    pub fn correct_count(&self) -> u32 {
        (0..self.typed.len())
            .filter(|i| self.status(*i) == CharStatus::Correct)
            .count() as u32
    }

    pub fn wrong_count(&self) -> u32 {
        self.typed.len() as u32 - self.correct_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_str(buf: &mut TypingBuffer, s: &str) {
        for c in s.chars() {
            buf.push_char(c);
        }
    }

    #[test]
    fn marks_correct_and_wrong() {
        let mut buf = TypingBuffer::new("abc");
        assert_eq!(buf.push_char('a'), InputOutcome::Typed(true));
        assert_eq!(buf.push_char('x'), InputOutcome::Typed(false));
        assert_eq!(buf.status(0), CharStatus::Correct);
        assert_eq!(buf.status(1), CharStatus::Wrong);
        assert_eq!(buf.status(2), CharStatus::Pending);
        assert_eq!(buf.correct_count(), 1);
        assert_eq!(buf.wrong_count(), 1);
    }

    #[test]
    fn completes_even_with_errors() {
        let mut buf = TypingBuffer::new("ab");
        type_str(&mut buf, "xb");
        assert!(buf.is_complete());
        assert_eq!(buf.push_char('z'), InputOutcome::Ignored);
    }

    #[test]
    fn backspace_fixes_mistake() {
        let mut buf = TypingBuffer::new("ab");
        type_str(&mut buf, "x");
        assert!(buf.backspace());
        type_str(&mut buf, "a");
        assert_eq!(buf.status(0), CharStatus::Correct);
        assert!(buf.backspace());
        assert!(!buf.backspace());
    }

    #[test]
    fn delete_word_removes_previous_word() {
        let mut buf = TypingBuffer::new("hello big world");
        type_str(&mut buf, "hello big ");
        assert!(buf.delete_word());
        assert_eq!(buf.cursor(), 6);
        assert!(buf.delete_word());
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn precomposed_spanish_accents_match() {
        let mut buf = TypingBuffer::new("¿Qué año?");
        type_str(&mut buf, "¿Qué año?");
        assert!(buf.is_complete());
        assert_eq!(buf.wrong_count(), 0);
    }

    #[test]
    fn decomposed_target_is_normalized() {
        // "é" written as 'e' + U+0301 in the source text.
        let buf = TypingBuffer::new("cafe\u{301}");
        assert_eq!(buf.target().len(), 4);
        assert_eq!(buf.target()[3], "é");
    }

    #[test]
    fn combining_mark_amends_previous_grapheme() {
        let mut buf = TypingBuffer::new("é");
        assert_eq!(buf.push_char('e'), InputOutcome::Typed(false));
        assert_eq!(buf.push_char('\u{301}'), InputOutcome::Amended(true));
        assert_eq!(buf.status(0), CharStatus::Correct);
        assert!(buf.is_complete());
    }

    #[test]
    fn accent_mismatch_is_wrong() {
        let mut buf = TypingBuffer::new("é");
        assert_eq!(buf.push_char('e'), InputOutcome::Typed(false));
    }
}
