# Paragraph Content

Paragraphs come from two sources:

1. **Tatoeba (online, default)** — random sentences fetched at runtime from
   the [Tatoeba](https://tatoeba.org) API, cleaned and joined into paragraphs,
   and kept in a local cache (`paragraph_cache.jsonl`). See
   [Online paragraphs](#online-paragraphs-tatoeba).
2. **Embedded corpus (offline fallback)** — the hand-written files in
   `assets/paragraphs/`, compiled into the binary. Used only when the cache for
   the language is empty, when `--offline` is passed, or when `online = false`
   in `config.toml`. The embedded corpus is no longer being grown.

## Embedded corpus format

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
- **Committed text: original or public-domain only.** Do not paste
  copyrighted text (books, articles, song lyrics) into the repo. Writing
  original sentences is preferred. Text fetched from Tatoeba is never
  committed (not as corpus, fixtures or screenshots); test fixtures use
  invented sentences in Tatoeba's response shape.

The character set and length range live in `content::sanitize`
(`is_allowed`, `MIN_LEN`, `MAX_LEN`) and are shared by the corpus test and
the online sanitizer.

## Online paragraphs (Tatoeba)

Runtime-fetched text from Tatoeba is allowed. Tatoeba sentences are licensed
**CC BY 2.0 FR**, so whenever a session showed Tatoeba text the results screen
credits it: `Text: Tatoeba (tatoeba.org) · CC BY 2.0 FR`.

Fetching (`src/tatoeba.rs`):

- `GET https://api.tatoeba.org/unstable/sentences?lang=<eng|spa>&sort=random&limit=100`
  with a `terminagraphy/<version> (+repo url)` User-Agent and a 10 s timeout.
- Entries are kept only if `is_unapproved` is not `true`, `lang` matches the
  request and `license` starts with `CC BY`. Others (including `CC0` and
  missing licenses) are skipped; unknown fields are ignored.
- Any failure (offline, timeout, bad JSON, unwritable cache) is ignored and the
  game keeps using whatever is cached, then the embedded corpus.

Sanitizing each sentence (`content::sanitize::sanitize_sentence`):

1. Normalize to NFC.
2. Replace typographic characters: `‘ ’ ‚ ′` → `'`, `“ ” „ « »` → `"`,
   `‐ ‑ – —` → `-`, `…` → `...`.
3. Collapse whitespace to single spaces and trim.
4. Reject the sentence if it is empty, longer than 400 characters, or still
   contains a character outside the allowed set above (Spanish letters only
   for Spanish).

Assembling paragraphs (`content::sanitize::assemble`): sentences, in the
random order they arrive, are joined with single spaces until the next one
would exceed 400 characters. A paragraph of at least 150 characters is then
closed and the sentence starts the next one; a shorter one skips that
sentence and keeps growing. A leftover under 150 characters is dropped.
Every cached paragraph therefore meets the length, character-set and NFC
rules.

## Picker behavior

Each session gets a `ParagraphSupply`:

- **Online**: `CachedSupply` takes the oldest cached paragraph for the
  language and removes it from the cache (each is served once). Only when the
  cache has none left for that language does it fall back to the embedded
  picker below. Cached and embedded text are never mixed while the cache has
  paragraphs.
- **Offline / fallback**: `ParagraphPicker` is a shuffle bag over the
  embedded corpus: shuffle all indices, hand them out in order, reshuffle
  when exhausted. Guarantees no repeat within a session until the whole
  corpus has been used.

A test must verify every paragraph in every language file passes the
guidelines above (length, allowed character set, NFC). Sanitizer and
assembly tests cover the same rules for online paragraphs.
