# Paragraph Content

## Format

One file per language under `assets/paragraphs/`:

- `assets/paragraphs/en.txt`
- `assets/paragraphs/es.txt`

Paragraphs are separated by **one blank line**. Lines inside a paragraph are
joined with a single space when loaded, so hard wraps in the file are fine.
Lines starting with `#` are comments and ignored.

```text
# en.txt
The lighthouse keeper climbed the stairs every evening, counting each step
out of habit rather than need.

Fresh bread was cooling on the windowsill when the first drops of rain
began to fall on the empty street.
```

Files are embedded in the binary with `include_str!`, so the game works with
no external files. The loader:

1. Splits on blank lines, joins lines, trims, collapses repeated whitespace.
2. Normalizes to Unicode NFC.
3. Rejects (debug assert / test failure) paragraphs outside the length range.

## Guidelines

- **Length**: 150–400 characters each (≈ 30–70 words). Short enough that a
  30 s session can finish one, long enough to build rhythm.
- **Unrelated**: each paragraph stands alone — different topic, no continuing
  story. The picker shuffles, so order in the file does not matter.
- **Typable characters only**: letters, digits, standard punctuation
  `. , ; : ! ? ' " - ( )`. No smart quotes (`“ ” ‘ ’`), em dashes (`—`),
  ellipsis glyph (`…`), tabs, or emoji. Use straight `'` and `"` and `-`.
- **Spanish**: use correct accents and `ñ`, opening `¿` `¡`. These are part of
  the practice.
- **Single spaces** between sentences.
- **Original or public-domain text only.** Do not paste copyrighted text
  (books, articles, song lyrics). Writing original sentences is preferred.
- Aim for **at least 100 paragraphs per language** before 1.0 so long
  sessions rarely repeat across days.

## Picker behavior

`ParagraphPicker` is a shuffle bag: shuffle all indices, hand them out in
order, reshuffle when exhausted. Guarantees no repeat within a session until
the whole corpus has been used.

A test must verify every paragraph in every language file passes the
guidelines above (length, allowed character set, NFC).
