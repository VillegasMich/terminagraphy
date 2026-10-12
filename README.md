# terminagraphy

A terminal typing trainer (mecanografía) written in Rust with
[ratatui](https://ratatui.rs). Type unrelated paragraphs as fast and accurately
as you can against the clock, review your metrics, and track your progress and
daily streak over time.

> Status: early development. See [docs/ROADMAP.md](docs/ROADMAP.md).

## Features (planned)

- Centered paragraph typing with live correct/incorrect highlighting.
- Timed modes: **30 seconds**, **2 minutes**, **5 minutes**.
- Endless stream of unrelated paragraphs within a session.
- **English** and **Spanish** paragraph sets (full accent support: á é í ó ú ü ñ ¿ ¡).
- End-of-session metrics: WPM, raw WPM, accuracy, consistency, errors.
- Local history with personal bests per language + mode.
- Daily tracker: streaks and a calendar heatmap of days played.

## Install & run

Requires a recent stable Rust toolchain (edition 2024).

```sh
cargo run --release
# or install locally
cargo install --path .
terminagraphy
```

Planned flags:

```sh
terminagraphy --lang es --mode 2m   # jump straight into a session
terminagraphy --data-dir ./mydata   # custom storage location
```

## Controls

| Key | Action |
|-----|--------|
| type | Type the paragraph; timer starts on first key |
| `Backspace` | Delete previous character |
| `Tab` | Restart the current mode |
| `Esc` | Abort session / go back |
| `q` | Quit (from menus) |

## Data

Stored locally only, nothing is sent anywhere.

| OS | History |
|----|---------|
| Linux | `~/.local/share/terminagraphy/history.jsonl` |
| macOS | `~/Library/Application Support/terminagraphy/history.jsonl` |
| Windows | `%APPDATA%\terminagraphy\data\history.jsonl` |

## Documentation

- [Game design](docs/GAME_DESIGN.md) — rules, modes, metrics, screens.
- [Architecture](docs/ARCHITECTURE.md) — crates, modules, data model, testing.
- [Content](docs/CONTENT.md) — paragraph file format and writing guidelines.
- [Roadmap](docs/ROADMAP.md) — milestones.
