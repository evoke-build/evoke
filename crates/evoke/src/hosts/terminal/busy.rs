//! A spinner while something is in flight — the request to the adapter, a fetch from a remote. It shows on stderr
//! only where the terminal moves, and only once the wait has lasted a moment, so a cached or replayed answer never
//! flickers; it is cleared the instant the wait ends. In: what is happening. Out: nothing that outlives the wait.

use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::Look;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
/// How long a wait lasts before the spinner shows.
const PATIENCE: Duration = Duration::from_millis(120);
/// The time between two frames.
const TICK: Duration = Duration::from_millis(80);

/// The wait, spinning while it is held; dropping it ends the spinner and clears its line.
pub struct Busy(Option<Spinning>);

struct Spinning {
    /// Dropped to stop: the thread wakes at once.
    stop: Sender<()>,
    thread: JoinHandle<()>,
}

impl Busy {
    pub(super) fn start(what: String, look: Look) -> Self {
        if !look.animated {
            return Self(None);
        }
        let (stop, stopped) = mpsc::channel();
        let thread = thread::spawn(move || spin(&what, look.styled, &stopped));
        Self(Some(Spinning { stop, thread }))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        if let Some(Spinning { stop, thread }) = self.0.take() {
            drop(stop);
            let _ = thread.join();
        }
    }
}

/// Nothing for a moment, then a frame every tick until stopped; the line cleared when one was drawn.
fn spin(what: &str, styled: bool, stopped: &Receiver<()>) {
    if stopped.recv_timeout(PATIENCE) != Err(RecvTimeoutError::Timeout) {
        return;
    }
    for glyph in FRAMES.iter().cycle() {
        let line = if styled {
            format!("\r\x1b[2K  \x1b[36m{glyph}\x1b[0m \x1b[2m{what}\x1b[0m")
        } else {
            format!("\r\x1b[2K  {glyph} {what}")
        };
        show(&line);
        if stopped.recv_timeout(TICK) != Err(RecvTimeoutError::Timeout) {
            break;
        }
    }
    show("\r\x1b[2K");
}

fn show(text: &str) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(text.as_bytes());
    let _ = stderr.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terminal_that_cannot_move_gets_no_spinner() {
        let busy = Busy::start("deciding".to_owned(), Look::default());
        assert!(busy.0.is_none());
        drop(busy);
    }

    #[test]
    fn a_wait_shorter_than_the_patience_draws_nothing_and_ends_at_once() {
        let look = Look {
            styled: true,
            animated: true,
            editing: true,
        };
        let started = std::time::Instant::now();
        let busy = Busy::start("deciding".to_owned(), look);
        assert!(busy.0.is_some());
        drop(busy);
        assert!(started.elapsed() < PATIENCE);
    }
}
