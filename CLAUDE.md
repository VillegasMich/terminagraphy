# CLAUDE.md

Terminal typing trainer (mecanografía) in Rust + ratatui. Player types
unrelated paragraphs centered on screen in timed sessions (30s / 2m / 5m),
English or Spanish, then sees metrics; history and a daily streak tracker are
saved locally.

## Docs — read before changing behavior

- `docs/GAME_DESIGN.md` — source of truth for rules, modes, metrics formulas, screens.
- `docs/ARCHITECTURE.md` — crates, module layout, data model, storage, testing.
- `docs/CONTENT.md` — paragraph file format and content rules.
- `docs/ROADMAP.md` — milestones; tick boxes when work lands.

If an implementation decision contradicts these docs, update the doc in the
same change (or ask first if it changes game rules).

## Commands

```sh
cargo build
cargo run                       # launches TUI (needs a real terminal)
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

Before considering work done: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`
and `cargo test` must pass.

CI (`.github/workflows/ci.yml`) runs the same fmt/clippy/test checks on
Linux/macOS/Windows plus an MSRV check, so respect `rust-version` in `Cargo.toml`.

After visible UI changes, refresh the README screenshots (`docs/screenshots/*.svg`,
rendered from `TestBackend` buffers by `src/ui/screenshots.rs`):

```sh
cargo test screenshots -- --ignored
```

The TUI cannot be meaningfully driven from a non-interactive shell. Verify
logic through unit tests and UI through `ratatui::backend::TestBackend`
render tests instead of launching the binary.

## Architecture rules

- Elm-style: `Event` → `update(&mut App, Event)` → `view(&App, &mut Frame)`.
  `view` never mutates state or does I/O.
- Game logic (`game/`, `content/`, `storage/tracker`) must not depend on
  ratatui or crossterm types; it is pure and unit-tested.
- Time is injected: logic takes `Duration`/timestamps as arguments. Only the
  event loop calls `Instant::now()`. Use `Instant` for session timing, local
  calendar date (`chrono::Local`) only for the daily tracker.
- Text comparison is per **grapheme cluster** (`unicode-segmentation`) after
  **NFC normalization**. Never index strings by byte or compare `char`-by-`char`
  for typing logic — Spanish accents depend on this.
- Use `unicode-width` for layout math, not `.len()`.
- All colors/styles live in `ui/theme.rs`.
- Terminal setup/teardown via `ratatui::init()` / `ratatui::restore()`; never
  leave the terminal in raw mode on error or panic.

## Storage rules

- History: `history.jsonl` in `directories::ProjectDirs` data dir, append-only,
  one `SessionRecord` per line with a `version` field.
- Daily tracker is derived from history, not stored separately.
- Config: `config.toml`, written atomically (temp file + rename).
- Bad/unknown lines are skipped with a warning; never panic on user data.
- Never touch the real user data dir in tests — use `tempfile` + `--data-dir`.

## Conventions

- Rust edition 2024, stable toolchain.
- Add dependencies with `cargo add` (latest versions); justify new crates
  against the table in `docs/ARCHITECTURE.md` and add them there.
- Errors: `color_eyre::Result` in app/main; small typed errors (`thiserror`)
  inside library-like modules if needed. No `unwrap()` outside tests except
  on true invariants (with a comment).
- Tests live next to code in `#[cfg(test)] mod tests`.
- Paragraph content: original or public-domain text only, follow `docs/CONTENT.md`.
- Commits: Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`).
