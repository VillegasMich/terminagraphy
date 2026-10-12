# Roadmap

Each milestone should end in a runnable, committed state.

## M0 — Skeleton
- [x] Add deps: `ratatui`, `crossterm`, `color-eyre`.
- [x] `main.rs` with `ratatui::init()` / `restore()`, event loop, tick.
- [x] Empty `App` with `Screen` enum; `q`/`Esc` quits cleanly.

## M1 — Core typing (English, 30 s)
- [x] `content`: embedded `en.txt` with ~10 paragraphs, shuffle-bag picker.
- [x] `game::input`: grapheme comparison, backspace, completion detection.
- [x] `game::session`: Ready → Running (first key) → Finished (timer).
- [x] `ui::typing`: centered, wrapped paragraph with correct/wrong/pending styles + cursor.
- [x] Next paragraph loads on completion.
- [x] Unit tests for input + session transitions.

## M2 — Metrics & results
- [x] Keystroke log; `game::metrics` (WPM, raw, accuracy, consistency, errors).
- [x] Results screen with numbers + WPM-over-time chart.
- [x] Live WPM / accuracy bar during typing.
- [x] Metric unit tests with fixed keystroke logs.

## M3 — Modes & languages
- [x] Menu screen: select language (en/es) and mode (30s/2m/5m).
- [x] `es.txt` corpus; NFC normalization; accent handling verified.
- [x] `Tab` restart, `Esc` abort.
- [x] CLI flags `--lang`, `--mode` to skip the menu.

## M4 — Persistence
- [x] `storage::history` append/load JSONL in `ProjectDirs` data dir.
- [x] Personal best per category; "New PB!" on results.
- [x] `config.toml` for default language/mode and live-stats toggle.
- [x] `--data-dir` override; tests with `tempfile`.

## M5 — Stats & daily tracker
- [x] `storage::tracker`: days played, current/longest streak (local dates).
- [x] Stats screen: per-category PB, averages, trend chart.
- [x] Calendar heatmap widget.
- [x] Streak edge-case tests.

## M6 — Polish / 1.0
- [x] Corpus validation test.
- [ ] ≥100 paragraphs per language (currently 30 each).
- [x] Small-terminal handling, resize handling.
- [x] Theme module (`ui/theme.rs`).
- [ ] Color fallback for limited terminals (heatmap uses 256-color palette).
- [ ] README screenshots/GIF, release build, `cargo install` instructions.

## Ideas (post-1.0)
- Lenient-accent option for Spanish.
- Localized UI strings (menus in Spanish), not just content.
- Word-count modes (25/50/100 words), custom text import.
- Per-key error heatmap (which letters you miss most).
- Export history to CSV.
- More languages.
