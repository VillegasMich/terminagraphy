//! Paragraph corpus, content rules and languages. Pure logic, no terminal types.

pub mod language;
pub mod paragraphs;
pub mod sanitize;

pub use language::Language;
pub use paragraphs::{Paragraph, ParagraphPicker, ParagraphSupply, Source};
