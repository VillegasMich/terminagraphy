# Architecture

## Stack

| Concern | Crate | Notes |
|---------|-------|-------|
| TUI rendering | `ratatui` | Required. Use `ratatui::init()` / `ratatui::restore()` for terminal setup + panic-safe teardown |
| Terminal backend / input | `crossterm` | Default ratatui backend; key events via `crossterm::event`. Keep the version equal to the one ratatui uses (check `Cargo.lock` for a single `crossterm` entry) |
| Error handling | `color-eyre` | Pretty errors + panic hook that restores the terminal (or `anyhow` if simpler) |
| Serialization | `serde`, `serde_json`, `toml` | History as JSON Lines, config as TOML |
| Data paths | `directories` | `ProjectDirs` for cross-platform data/config dirs |
| Dates | `chrono` (or `jiff`) | Local calendar date for daily tracker and streaks |
| Randomness | `rand` | Paragraph shuffling |
| Unicode | `unicode-segmentation`, `unicode-normalization` | Grapheme-based matching, NFC normalize targets + input (Spanish accents) |
| Width | `unicode-width` | Correct column math when wrapping/centering |
| CLI | `clap` (derive) | Optional flags: `--lang`, `--mode`, `--data-dir`, `--offline` |
| HTTP | `ureq` (no default features, `rustls`) | Fetches Tatoeba sentences from a background thread. Small, blocking (no async runtime), pure-Rust TLS (no OpenSSL); one plain thread is all the prefetch needs, so `reqwest`/`tokio` would be overkill |
| Tests | `tempfile` | Isolated data dirs. Rendered frames are checked via `ratatui::backend::TestBackend` (`insta` snapshots optional later) |

Add crates with `cargo add <name>` to get current versions; do not hand-pin
old versions.

### Storage choice

JSON Lines (`history.jsonl`, one session per line, append-only) is chosen over
SQLite for the MVP: zero native deps, human-readable, trivially append-safe,
and the expected data size (thousands of rows) is tiny. If querying gets
complex later, migrate to `rusqlite` behind the same `storage` trait.

## Pattern: The Elm Architecture (TEA)

```
           ┌──────────────┐
  input ──►│  Event        │  (Key, Tick, Resize)
           └──────┬───────┘
                  ▼
           ┌──────────────┐
           │ update(&mut  │  pure-ish state transition, returns Option<Action>
           │   App, Event)│
           └──────┬───────┘
                  ▼
           ┌──────────────┐
           │ view(&App,   │  ratatui draw, no mutation
           │   Frame)     │
           └──────────────┘
```

- `App` owns all state, including the current `Screen`.
- Event loop polls crossterm with a short timeout (50 ms) and emits
  `Tick` events so the timer and live WPM update without input.
- The loop captures the clock once per step as `app::Now { instant, local }`
  and passes it down; nothing below the loop reads the clock.
- Side effects (saving history, loading paragraphs) happen in `update` via
  explicit calls into `storage`, never in `view`.
- **Prefetch thread**: when online, `main` starts one detached thread
  (`tatoeba::spawn_prefetch`) before the loop. It refills each language's
  paragraph cache when it holds fewer than 30 paragraphs, up to 100, then
  waits on a bounded `mpsc::sync_channel(1)` for a wake-up (sent after each
  cache `take`) or a 10-minute re-check. It never blocks the UI: the cache
  file is guarded by a `Mutex` held only for one small file operation, never
  across network calls. The thread is never joined, so it cannot keep the
  process alive on quit.
- Sessions get their paragraphs through the `content::ParagraphSupply`
  trait. `App` injects either `storage::paragraph_cache::CachedSupply` (cache,
  then embedded fallback) or the embedded `ParagraphPicker`; `game/` itself
  does no I/O.

## Module layout (target)

```
src/
├── main.rs              # arg parsing, ratatui::init/restore, run loop
├── app.rs               # App struct, Screen enum, top-level update()
├── event.rs             # Event enum, polling + tick generation
├── game/
│   ├── mod.rs
│   ├── mode.rs          # Mode enum (Sprint30, Normal120, Endurance300) → Duration
│   ├── session.rs       # Session state machine: Ready → Running → Finished
│   ├── input.rs         # Typed buffer vs target, grapheme comparison, backspace
│   └── metrics.rs       # WPM, raw, accuracy, consistency from keystroke log
├── content/
│   ├── mod.rs
│   ├── language.rs      # Language enum (En, Es)
│   ├── paragraphs.rs    # Embedded corpus, shuffle-bag picker, Paragraph/Source, ParagraphSupply
│   └── sanitize.rs      # Content rules: allowed chars, sentence cleanup, paragraph assembly
├── storage/
│   ├── mod.rs           # data dir resolution
│   ├── history.rs       # SessionRecord, append/load jsonl
│   ├── tracker.rs       # daily tracker: days played, streaks, heatmap data
│   ├── paragraph_cache.rs # cached online paragraphs (jsonl), SharedCache, CachedSupply
│   └── config.rs        # Settings (default lang/mode, show live stats, online)
├── tatoeba.rs           # Tatoeba API client (ureq), response parsing, refill + prefetch thread
└── ui/
    ├── mod.rs           # view() dispatch per Screen
    ├── theme.rs         # colors/styles in one place
    ├── menu.rs
    ├── typing.rs        # centered paragraph rendering with per-grapheme styles
    ├── results.rs
    ├── stats.rs
    └── widgets/
        └── heatmap.rs   # calendar heatmap widget
assets/
└── paragraphs/
    ├── en.txt
    └── es.txt
```

## Key types (sketch)

```rust
pub enum Screen { Menu(MenuState), Typing(Session), Results(SessionResult), Stats(StatsState) }

pub enum Mode { Sprint, Normal, Endurance }           // 30s, 120s, 300s
pub enum Language { En, Es }

pub struct Session {
    pub mode: Mode,
    pub language: Language,
    pub state: SessionState,                          // Ready | Running { started: Instant } | Finished
    pub target: Vec<String>,                          // graphemes of current paragraph
    pub typed: Vec<String>,                           // graphemes typed so far
    pub keystrokes: Vec<Keystroke>,                   // full log for metrics
    pub paragraphs_done: u32,
    picker: ParagraphPicker,
}

pub struct Keystroke { pub at: Duration, pub kind: KeyKind }   // Correct | Wrong | Backspace

#[derive(Serialize, Deserialize)]
pub struct SessionRecord {
    pub version: u32,                                 // schema version, start at 1
    pub ended_at: chrono::DateTime<chrono::Local>,
    pub language: Language,
    pub mode: Mode,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub errors: u32,
    pub uncorrected_errors: u32,
    pub correct_chars: u32,
    pub total_chars: u32,
    pub paragraphs: u32,
}
```

Metrics come from two inputs, both terminal-free and testable:

- the **keystroke log** → accuracy, raw WPM, errors, per-second samples, consistency;
- the **final text totals** (`TextTotals`: correct chars and uncorrected errors
  left in each paragraph) → net WPM, so retyping a correct character after an
  accidental backspace does not inflate WPM.

## Timing

- Use `std::time::Instant` (monotonic) for session timing, never wall clock.
- Timer starts on first non-control keystroke.
- Session ends exactly at mode duration; keystrokes arriving after are ignored.
- Per-second WPM samples are bucketed from the keystroke log after the session.

## Data files

| File | Location | Format |
|------|----------|--------|
| `history.jsonl` | data dir | One `SessionRecord` per line, append-only |
| `config.toml` | config dir | User settings |
| `paragraph_cache.jsonl` | data dir | One `{version, language, text, source}` per line; paragraphs are removed once served to a session |

The daily tracker is **derived** from `history.jsonl` (distinct local dates of
`ended_at`) rather than stored separately — one source of truth, no drift.

Write rules:
- Append with `OpenOptions::append`, write whole line + `\n`, flush.
- Any full-file rewrite (config, paragraph cache after a `take`) goes to a
  temp file then `rename`.
- Every record has `version` for future migrations.
- `--data-dir` flag overrides location (used by tests and for portability).

## Testing strategy

- **Unit tests** (bulk of coverage): `game::input`, `game::metrics`,
  `storage::tracker` (streak edge cases: today/yesterday, gaps, month/year
  boundaries), paragraph picker (no repeats until exhausted).
- **Storage tests**: use `tempfile` dirs; verify corrupted lines are skipped.
- **Network**: tests never hit the network. Response parsing is a pure
  function tested against an invented JSON fixture; the refill logic takes the
  fetch function as a parameter and is tested with fakes.
- **UI tests**: render screens into `TestBackend` and assert / snapshot the
  buffer. Keep UI logic thin so few of these are needed.
- Inject time: metric/session functions take `Duration`/timestamps as params
  instead of calling `Instant::now()` internally, so tests are deterministic.
