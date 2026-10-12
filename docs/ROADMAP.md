# Roadmap

Each milestone should end in a runnable, committed state.

## M0 — Skeleton
- [ ] Add deps: `ratatui`, `crossterm`, `color-eyre`.
- [ ] `main.rs` with `ratatui::init()` / `restore()`, event loop, tick.
- [ ] Empty `App` with `Screen` enum; `q`/`Esc` quits cleanly.

## M1 — Core typing (English, 30 s)
- [ ] `content`: embedded `en.txt` with ~10 paragraphs, shuffle-bag picker.
- [ ] `game::input`: grapheme comparison, backspace, completion detection.
- [ ] `game::session`: Ready → Running (first key) → Finished (timer).
- [ ] `ui::typing`: centered, wrapped paragraph with correct/wrong/pending styles + cursor.
- [ ] Next paragraph loads on completion.
- [ ] Unit tests for input + session transitions.

## M2 — Metrics & results
- [ ] Keystroke log; `game::metrics` (WPM, raw, accuracy, consistency, errors).
- [ ] Results screen with numbers + WPM-over-time chart.
- [ ] Live WPM / accuracy bar during typing.
- [ ] Metric unit tests with fixed keystroke logs.

## M3 — Modes & languages
- [ ] Menu screen: select language (en/es) and mode (30s/2m/5m).
- [ ] `es.txt` corpus; NFC normalization; accent handling verified.
- [ ] `Tab` restart, `Esc` abort.
- [ ] CLI flags `--lang`, `--mode` to skip the menu.

## M4 — Persistence
- [ ] `storage::history` append/load JSONL in `ProjectDirs` data dir.
- [ ] Personal best per category; "New PB!" on results.
- [ ] `config.toml` for default language/mode and live-stats toggle.
- [ ] `--data-dir` override; tests with `tempfile`.

## M5 — Stats & daily tracker
- [ ] `storage::tracker`: days played, current/longest streak (local dates).
- [ ] Stats screen: per-category PB, averages, trend chart.
- [ ] Calendar heatmap widget.
- [ ] Streak edge-case tests.

## M6 — Polish / 1.0
- [ ] ≥100 paragraphs per language + corpus validation test.
- [ ] Small-terminal handling, resize handling.
- [ ] Theme module, color fallback for limited terminals.
- [ ] README screenshots/GIF, release build, `cargo install` instructions.

## Ideas (post-1.0)
- Lenient-accent option for Spanish.
- Localized UI strings (menus in Spanish), not just content.
- Word-count modes (25/50/100 words), custom text import.
- Per-key error heatmap (which letters you miss most).
- Export history to CSV.
- More languages.
