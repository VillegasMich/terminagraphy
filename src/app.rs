use std::time::Instant;

use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::content::Language;
use crate::game::{Metrics, Mode, Session};
use crate::storage::paragraph_cache::{CachedSupply, SharedCache};
use crate::storage::{DailyTracker, DataPaths, SessionRecord, Settings, config, history};

/// Clock readings taken once per loop iteration by the event loop.
#[derive(Debug, Clone, Copy)]
pub struct Now {
    pub instant: Instant,
    pub local: DateTime<Local>,
}

impl Now {
    pub fn capture() -> Self {
        Self {
            instant: Instant::now(),
            local: Local::now(),
        }
    }
}

pub enum Screen {
    Menu,
    Typing(Box<Session>),
    Results(Box<ResultsState>),
    Stats,
}

pub struct ResultsState {
    pub record: SessionRecord,
    pub metrics: Metrics,
    /// Best WPM in this category before this session, if any.
    pub previous_best: Option<f64>,
    pub save_error: Option<String>,
    /// Some typed text came from Tatoeba and must be attributed.
    pub used_tatoeba: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuField {
    Language,
    Mode,
}

pub struct App {
    pub screen: Screen,
    pub settings: Settings,
    pub menu_field: MenuField,
    pub stats_language: Language,
    pub stats_mode: Mode,
    pub history: Vec<SessionRecord>,
    pub tracker: DailyTracker,
    pub notice: Option<String>,
    pub should_quit: bool,
    paths: DataPaths,
    /// Cache of online paragraphs; `None` when offline (embedded corpus only).
    paragraph_cache: Option<SharedCache>,
}

impl App {
    pub fn new(
        paths: DataPaths,
        settings: Settings,
        history: Vec<SessionRecord>,
        notice: Option<String>,
    ) -> Self {
        Self {
            screen: Screen::Menu,
            stats_language: settings.language,
            stats_mode: settings.mode,
            settings,
            menu_field: MenuField::Language,
            tracker: DailyTracker::from_records(&history),
            history,
            notice,
            should_quit: false,
            paths,
            paragraph_cache: None,
        }
    }

    /// Serves sessions from `cache` first, falling back to the embedded corpus.
    pub fn with_paragraph_cache(mut self, cache: SharedCache) -> Self {
        self.paragraph_cache = Some(cache);
        self
    }

    fn new_session(&self, mode: Mode, language: Language) -> Session {
        match &self.paragraph_cache {
            Some(cache) => Session::with_supply(
                mode,
                language,
                Box::new(CachedSupply::new(cache.clone(), language)),
            ),
            None => Session::new(mode, language),
        }
    }

    pub fn start_session(&mut self, language: Language, mode: Mode) {
        if self.settings.language != language || self.settings.mode != mode {
            self.settings.language = language;
            self.settings.mode = mode;
            self.save_settings();
        }
        self.screen = Screen::Typing(Box::new(self.new_session(mode, language)));
    }

    fn save_settings(&mut self) {
        if let Err(e) = config::save(&self.paths.config, &self.settings) {
            self.notice = Some(format!("Could not save settings: {e}"));
        }
    }

    fn open_stats(&mut self, language: Language, mode: Mode) {
        self.stats_language = language;
        self.stats_mode = mode;
        self.screen = Screen::Stats;
    }

    pub fn tick(&mut self, now: &Now) {
        if let Screen::Typing(session) = &mut self.screen
            && session.tick(now.instant)
        {
            self.finish_session(now);
        }
    }

    /// Moves a finished session to the results screen and persists it.
    fn finish_session(&mut self, now: &Now) {
        let Screen::Typing(session) = std::mem::replace(&mut self.screen, Screen::Menu) else {
            return;
        };
        let metrics = session.metrics();
        let record = SessionRecord::new(&metrics, session.mode, session.language, now.local);
        let previous_best =
            history::personal_best(&self.history, session.language, session.mode).map(|r| r.wpm);
        let save_error = history::append(&self.paths.history, &record)
            .err()
            .map(|e| format!("Could not save session: {e}"));
        self.tracker.add(record.ended_at.date_naive());
        self.history.push(record.clone());
        self.screen = Screen::Results(Box::new(ResultsState {
            record,
            metrics,
            previous_best,
            save_error,
            used_tatoeba: session.used_tatoeba(),
        }));
    }

    pub fn handle_key(&mut self, key: KeyEvent, now: &Now) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        match self.screen {
            Screen::Menu => self.menu_key(key),
            Screen::Typing(_) => self.typing_key(key, now),
            Screen::Results(_) => self.results_key(key),
            Screen::Stats => self.stats_key(key),
        }
    }

    fn menu_key(&mut self, key: KeyEvent) {
        let s = &mut self.settings;
        match key.code {
            KeyCode::Up | KeyCode::Down | KeyCode::Char('k' | 'j') | KeyCode::Tab => {
                self.menu_field = match self.menu_field {
                    MenuField::Language => MenuField::Mode,
                    MenuField::Mode => MenuField::Language,
                };
            }
            KeyCode::Left | KeyCode::Char('h') => match self.menu_field {
                MenuField::Language => s.language = s.language.prev(),
                MenuField::Mode => s.mode = s.mode.prev(),
            },
            KeyCode::Right | KeyCode::Char('l') => match self.menu_field {
                MenuField::Language => s.language = s.language.next(),
                MenuField::Mode => s.mode = s.mode.next(),
            },
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.notice = None;
                let (language, mode) = (s.language, s.mode);
                self.save_settings();
                self.start_session(language, mode);
            }
            KeyCode::Char('s') => {
                let (language, mode) = (s.language, s.mode);
                self.open_stats(language, mode);
            }
            KeyCode::Char('t') => {
                s.show_live_stats = !s.show_live_stats;
                self.save_settings();
            }
            KeyCode::Esc | KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }

    fn typing_key(&mut self, key: KeyEvent, now: &Now) {
        let Screen::Typing(session) = &mut self.screen else {
            return;
        };
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Esc => {
                self.screen = Screen::Menu;
                return;
            }
            KeyCode::Tab => {
                let (mode, language) = (session.mode, session.language);
                self.screen = Screen::Typing(Box::new(self.new_session(mode, language)));
                return;
            }
            KeyCode::Backspace if ctrl || alt => session.delete_word(now.instant),
            KeyCode::Backspace => session.backspace(now.instant),
            KeyCode::Char('w') if ctrl => session.delete_word(now.instant),
            KeyCode::Char('h') if ctrl => session.backspace(now.instant),
            // Ctrl+Alt is how AltGr arrives on some platforms (e.g. Windows).
            KeyCode::Char(c) if !ctrl || alt => session.type_char(c, now.instant),
            _ => {}
        }
        if session.is_finished() {
            self.finish_session(now);
        }
    }

    fn results_key(&mut self, key: KeyEvent) {
        let Screen::Results(results) = &self.screen else {
            return;
        };
        let (language, mode) = (results.record.language, results.record.mode);
        match key.code {
            KeyCode::Enter | KeyCode::Tab => self.start_session(language, mode),
            KeyCode::Char('s') => self.open_stats(language, mode),
            KeyCode::Esc | KeyCode::Char('m') => self.screen = Screen::Menu,
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }

    fn stats_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.stats_mode = self.stats_mode.prev(),
            KeyCode::Right | KeyCode::Char('l') => self.stats_mode = self.stats_mode.next(),
            KeyCode::Up | KeyCode::Down | KeyCode::Char('k' | 'j') => {
                self.stats_language = self.stats_language.next();
            }
            KeyCode::Tab => {
                self.stats_mode = self.stats_mode.next();
                if self.stats_mode == Mode::ALL[0] {
                    self.stats_language = self.stats_language.next();
                }
            }
            KeyCode::Esc | KeyCode::Char('m') => self.screen = Screen::Menu,
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn app(dir: &std::path::Path) -> App {
        let paths = DataPaths::resolve(Some(dir)).unwrap();
        App::new(paths, Settings::default(), Vec::new(), None)
    }

    fn press(app: &mut App, code: KeyCode, now: &Now) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), now);
    }

    #[test]
    fn full_session_is_saved_and_shown() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        let mut now = Now::capture();
        app.settings.mode = Mode::Sprint;
        press(&mut app, KeyCode::Enter, &now);
        let Screen::Typing(session) = &app.screen else {
            panic!("expected typing screen");
        };
        let first = session.buffer().target()[0].chars().next().unwrap();
        press(&mut app, KeyCode::Char(first), &now);

        now.instant += Mode::Sprint.duration();
        app.tick(&now);

        let Screen::Results(results) = &app.screen else {
            panic!("expected results screen");
        };
        assert_eq!(results.record.mode, Mode::Sprint);
        assert_eq!(results.metrics.correct_chars, 1);
        assert!(results.save_error.is_none());
        assert_eq!(app.history.len(), 1);
        assert_eq!(app.tracker.total_days(), 1);

        let saved = history::load(&dir.path().join("history.jsonl")).unwrap();
        assert_eq!(saved.records.len(), 1);
        let (settings, _) = config::load(&dir.path().join("config.toml")).unwrap();
        assert_eq!(settings.mode, Mode::Sprint);
    }

    #[test]
    fn escape_aborts_without_saving() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        let now = Now::capture();
        press(&mut app, KeyCode::Enter, &now);
        press(&mut app, KeyCode::Char('x'), &now);
        press(&mut app, KeyCode::Esc, &now);
        assert!(matches!(app.screen, Screen::Menu));
        let later = Now {
            instant: now.instant + Duration::from_secs(600),
            ..now
        };
        app.tick(&later);
        assert!(app.history.is_empty());
        assert!(!dir.path().join("history.jsonl").exists());
    }

    #[test]
    fn ctrl_c_quits_from_typing() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        let now = Now::capture();
        press(&mut app, KeyCode::Enter, &now);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &now,
        );
        assert!(app.should_quit);
    }
}
