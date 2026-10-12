use std::io;
use std::time::Duration;

use crossterm::event::{self, Event as TermEvent, KeyEvent, KeyEventKind};

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Key(KeyEvent),
    /// No input within the poll timeout; used to advance timers.
    Tick,
}

/// Waits up to `timeout` for input. Resize and other events are folded into
/// `Tick` since every loop iteration redraws anyway.
pub fn next(timeout: Duration) -> io::Result<Event> {
    if event::poll(timeout)?
        && let TermEvent::Key(key) = event::read()?
        && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
    {
        return Ok(Event::Key(key));
    }
    Ok(Event::Tick)
}
