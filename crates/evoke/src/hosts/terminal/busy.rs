//! A spinner while something is in flight — the request to the adapter, a fetch from a remote, a batch of cases.
//! It shows on stderr only where the terminal moves, and only once the wait has lasted a moment, so a cached or
//! replayed answer never flickers; over a batch it counts what is done; it is cleared the instant the wait ends,
//! and by the interrupt handler when Ctrl-C lands on it. In: what is happening, and the size of a batch. Out:
//! nothing that outlives the wait.

use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::Look;
use crate::hosts::interrupt;

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
    /// How many of a batch are done, shared with the thread that draws.
    done: Arc<AtomicUsize>,
}

impl Busy {
    pub(super) fn start(what: String, total: Option<usize>, look: Look) -> Self {
        if !look.animated {
            return Self(None);
        }
        interrupt::spinning(true);
        let (stop, stopped) = mpsc::channel();
        let done = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&done);
        let thread = thread::spawn(move || spin(&what, total, &counted, look.styled.err, &stopped));
        Self(Some(Spinning { stop, thread, done }))
    }

    /// One more of the batch done.
    pub fn tick(&self) {
        if let Some(spinning) = &self.0 {
            spinning.done.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        if let Some(Spinning { stop, thread, .. }) = self.0.take() {
            drop(stop);
            let _ = thread.join();
            interrupt::spinning(false);
        }
    }
}

/// Nothing for a moment, then a frame every tick until stopped; the line cleared when one was drawn.
fn spin(
    what: &str,
    total: Option<usize>,
    done: &AtomicUsize,
    styled: bool,
    stopped: &Receiver<()>,
) {
    if stopped.recv_timeout(PATIENCE) != Err(RecvTimeoutError::Timeout) {
        return;
    }
    for glyph in FRAMES.iter().cycle() {
        let label = label(what, total, done.load(Ordering::Relaxed));
        let line = if styled {
            format!("\r\x1b[2K  \x1b[36m{glyph}\x1b[0m \x1b[2m{label}\x1b[0m")
        } else {
            format!("\r\x1b[2K  {glyph} {label}")
        };
        show(&line);
        if stopped.recv_timeout(TICK) != Err(RecvTimeoutError::Timeout) {
            break;
        }
    }
    show("\r\x1b[2K");
}

/// `deciding`, or over a batch `testing 12 of 48`.
fn label(what: &str, total: Option<usize>, done: usize) -> String {
    match total {
        Some(total) => format!("{what} {done} of {total}"),
        None => what.to_owned(),
    }
}

fn show(text: &str) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(text.as_bytes());
    let _ = stderr.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hosts::terminal::Styled;

    #[test]
    fn a_terminal_that_cannot_move_gets_no_spinner() {
        let busy = Busy::start("deciding".to_owned(), None, Look::default());
        assert!(busy.0.is_none());
        busy.tick();
        drop(busy);
    }

    #[test]
    fn a_wait_shorter_than_the_patience_draws_nothing_and_ends_at_once() {
        let look = Look {
            styled: Styled {
                out: true,
                err: true,
            },
            animated: true,
            editing: true,
        };
        let started = std::time::Instant::now();
        let busy = Busy::start("deciding".to_owned(), None, look);
        assert!(busy.0.is_some());
        drop(busy);
        assert!(started.elapsed() < PATIENCE);
    }

    #[test]
    fn a_batch_is_counted_as_it_is_ticked() {
        let look = Look {
            styled: Styled::default(),
            animated: true,
            editing: false,
        };
        let busy = Busy::start("testing".to_owned(), Some(3), look);
        busy.tick();
        busy.tick();
        let done = busy
            .0
            .as_ref()
            .map(|spinning| spinning.done.load(Ordering::Relaxed));
        assert_eq!(done, Some(2));
        assert_eq!(label("testing", Some(3), 2), "testing 2 of 3");
        assert_eq!(label("deciding", None, 7), "deciding");
    }
}
