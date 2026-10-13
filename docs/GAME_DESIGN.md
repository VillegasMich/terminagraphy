# Game Design

Terminagraphy is a terminal typing trainer (mecanografía). The player types
paragraphs shown in the center of the screen as fast and as accurately as
possible within a fixed time limit. At the end of each session the game shows
metrics, saves them locally, and updates a daily play tracker.

## Core loop

```
Main menu ──► pick language + mode ──► Typing session ──► Results ──► (retry | menu | stats)
                                         ▲          │
                                         └─ next paragraph when current one is done
```

1. Player selects a **language** (English / Español) and a **mode** (30s, 2m, 5m).
2. A random paragraph appears centered on screen (see [Paragraph source](#paragraph-source)). The timer is shown but does
   **not** start until the first keystroke.
3. The player types. Each character is checked against the target as it is typed.
4. When the paragraph is fully typed, the next paragraph (unrelated, randomly
   chosen, never repeated within the same session) replaces it immediately.
5. When the timer hits zero, input stops and the Results screen is shown.
6. The session is appended to local history and the daily tracker is updated.

## Game modes

| Mode   | Duration | Purpose                          |
|--------|----------|----------------------------------|
| Sprint | 30 s     | Burst speed                      |
| Normal | 2 min    | Balanced speed / accuracy        |
| Endurance | 5 min | Sustained rhythm and consistency |

Modes are a closed enum in code, but adding a new duration should require only
adding a variant and its `Duration`.

Languages:

- **English** (`en`)
- **Spanish** (`es`) — paragraphs contain accented characters (á é í ó ú ü ñ ¿ ¡).
  These must be typed exactly; no accent-insensitive matching in the default
  rules (a possible future "lenient accents" setting is listed in the roadmap).

## Paragraph source

- **Online (default)**: paragraphs are built from random Tatoeba sentences
  that a background thread downloads ahead of time into a local cache.
  Starting a session never waits for the network.
- **Offline**: with `--offline`, `online = false` in `config.toml`, no
  network, or an empty cache for the chosen language, the built-in corpus is
  used, exactly as before. The game never hangs or fails because of the
  network.
- A session uses cached paragraphs while they last and only then switches to
  the built-in corpus.

See `docs/CONTENT.md` for the cleaning and assembly rules.

Language + mode together form a **category**. Personal bests and charts are
tracked per category (e.g. `es / 2m`).

## Typing rules

- Matching is per Unicode grapheme cluster, not per byte or `char`.
- Correct characters render in the "correct" color, wrong ones in the "error"
  color, untyped text dimmed. A cursor marks the next position.
- A wrong keystroke **does not block** progress — the cursor advances and the
  character is marked wrong.
- **Backspace** is allowed within the current paragraph to fix mistakes.
  - Fixing a mistake does not erase it from the error count: every wrong
    keystroke counts toward *accuracy* even if later corrected.
  - Uncorrected errors at the moment the paragraph is completed (or time ends)
    are tracked separately.
- `Ctrl+Backspace` / `Ctrl+W` deletes the previous word (optional, nice to have).
- `Esc` aborts the session (not saved). `Tab` restarts the same mode instantly.
- The paragraph is complete when the number of typed graphemes equals the
  target length. Errors may remain; they are counted.

## Layout

```
┌──────────────────────────── terminagraphy ────────────────────────────┐
│  es · 2m                                                 ⏱ 1:37       │
│                                                                       │
│        El viento del norte bajaba por la ladera y movía las           │
│        ramas de los pinos con un rumor constante, como si el          │
│        bosque entero estuviera respirando█                            │
│                                                                       │
│                    WPM 64   ·   ACC 96.2%   ·   ¶ 3                   │
│  Esc quit · Tab restart                                               │
└───────────────────────────────────────────────────────────────────────┘
```

- Paragraph is horizontally and vertically centered, wrapped to a max width
  (~70 columns) so lines stay readable on wide terminals.
- Live stats bar below the paragraph (WPM, accuracy, paragraphs completed).
  A setting may hide live stats for distraction-free mode.
- Must render correctly down to 60×15. Below that, show a "terminal too small"
  message instead of a broken layout.

## Metrics

All computed from the keystroke log of the session (see ARCHITECTURE.md).

| Metric | Definition |
|--------|-----------|
| **WPM** (net) | `(correct_chars / 5) / minutes` — standard "word = 5 chars" convention, spaces included |
| **Raw WPM** | `(all_typed_chars / 5) / minutes`, ignoring correctness |
| **Accuracy** | `correct_keystrokes / total_keystrokes × 100` (backspaces not counted as keystrokes) |
| **Errors** | Total wrong keystrokes (corrected + uncorrected) |
| **Uncorrected errors** | Wrong chars still present when paragraph completed / time ran out |
| **Consistency** | `100 − coefficient of variation` of per-second WPM samples, clamped to `0..=100` |
| **Paragraphs completed** | Count of fully typed paragraphs |
| **Characters** | correct / incorrect / total |

`minutes` is the configured mode duration (the session always runs to the end).

## Results screen

- Big numbers: WPM and accuracy.
- Secondary: raw WPM, consistency, errors, characters, paragraphs.
- A WPM-over-time line chart (per-second samples) with errors marked.
- Comparison with personal best for this category ("+4 WPM vs best", "New PB!").
- If any paragraph shown in the session came from Tatoeba, a dim attribution
  line above the key hints: `Text: Tatoeba (tatoeba.org) · CC BY 2.0 FR`.
  This is the only place attribution is shown.
- Actions: `Enter`/`Tab` retry same mode, `s` stats, `m`/`Esc` menu.

## Stats screen

- Per category: personal best, average of last 10, total sessions, total time.
- WPM trend chart over the last N sessions.
- **Daily tracker**:
  - Current streak (consecutive local days with ≥1 completed session).
  - Longest streak.
  - Total days played.
  - A GitHub-style calendar heatmap of the last ~12–20 weeks, cell intensity
    by number of sessions (or minutes typed) that day.

A "day" uses the **local** calendar date of the session end time. A streak
stays alive today if the player played yesterday (it is not broken until a full
day is missed).

## Persistence

- Only **completed** sessions are saved (aborted ones are discarded).
- Data lives in the OS data directory (e.g. `~/.local/share/terminagraphy/` on
  Linux). Settings in the OS config directory.
- Storage must survive crashes: append-only history, atomic writes for any
  rewritten file.
- Corrupted / unknown lines are skipped with a warning, never crash the game.

## Out of scope (for now)

- Online leaderboards / accounts (the only network use is fetching paragraphs).
- Multiplayer.
- Word-count modes, custom text import, code-typing mode (see roadmap ideas).
