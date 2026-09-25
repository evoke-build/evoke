//! Ctrl-C: one handler, installed once. Armed — while a body runs, or a weave's steps take their turns — it notes
//! the interrupt and returns, and the host acts on it: every body's group ended, every line logged, then `end`,
//! the process ending as an unhandled Ctrl-C would have ended it. Unarmed, the default action ends the process at
//! once, as before. Either way the spinner's line is cleared first, and, armed on a terminal, the echoed `^C`
//! gets its line end, so what the host prints after it starts on a line of its own. In: `arm`, `pause`, the
//! spinner's state. Out: `interrupted`, `end`.

// The handler does only what a signal handler may: read and write an atomic, `write`, `signal`, `raise`.
#![expect(unsafe_code)]

use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

static ARMED: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static SPINNING: AtomicBool = AtomicBool::new(false);
static ON_TERMINAL: AtomicBool = AtomicBool::new(false);
static INSTALLED: Once = Once::new();
const CLEAR: &[u8] = b"\r\x1b[2K";
const LINE_END: &[u8] = b"\n";

/// The handler installed, once; `terminal` says whether stderr is one, where the echoed `^C` wants its line end.
pub fn install(terminal: bool) {
    ON_TERMINAL.store(terminal, Ordering::Relaxed);
    INSTALLED.call_once(|| {
        let handler: extern "C" fn(libc::c_int) = on_interrupt;
        // SAFETY: a handler that touches atomics, writes static bytes and re-raises is safe to install once.
        unsafe {
            libc::signal(libc::SIGINT, handler as libc::sighandler_t);
        }
    });
}

extern "C" fn on_interrupt(signal: libc::c_int) {
    let armed = ARMED.load(Ordering::Relaxed);
    if SPINNING.load(Ordering::Relaxed) {
        // SAFETY: write is async-signal-safe, and the bytes are a static string.
        let _ = unsafe { libc::write(2, CLEAR.as_ptr().cast(), CLEAR.len()) };
    } else if armed && ON_TERMINAL.load(Ordering::Relaxed) {
        // SAFETY: as above.
        let _ = unsafe { libc::write(2, LINE_END.as_ptr().cast(), LINE_END.len()) };
    }
    if armed {
        INTERRUPTED.store(true, Ordering::Relaxed);
        return;
    }
    // SAFETY: the default action restored and the signal raised again ends the process as an unhandled Ctrl-C
    // would, with the same status.
    unsafe {
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}

/// Whether Ctrl-C was pressed while armed: what every wait polls, and what the host acts on at the end.
#[must_use]
pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}

/// The interrupt acted on — every body ended, every line logged — the process ends as an unhandled Ctrl-C would
/// have ended it, with the same status, so the shell sees an interrupted process and a loop breaks as it would.
pub fn end() -> ! {
    // SAFETY: the default action restored, the signal raised on this thread; nothing runs after but the exit.
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_DFL);
        libc::raise(libc::SIGINT);
    }
    // A blocked signal leaves the process here: the shell's own code for it, then.
    std::process::exit(130)
}

/// Armed while held: an interrupt is noted for the host to act on, never the end of the process. Nested, the
/// state before stands again once the inner one is dropped.
pub struct Armed(bool);

/// Arms the handler while the returned value is held.
#[must_use]
pub fn arm() -> Armed {
    Armed(ARMED.swap(true, Ordering::Relaxed))
}

/// Disarms it while the returned value is held — around a call the host cannot cut short, the adapter's — so
/// Ctrl-C ends the process at once there, as it always did, nothing being under way.
#[must_use]
pub fn pause() -> Armed {
    Armed(ARMED.swap(false, Ordering::Relaxed))
}

impl Drop for Armed {
    fn drop(&mut self) {
        ARMED.store(self.0, Ordering::Relaxed);
    }
}

/// Whether a spinner is on the line right now, so the handler clears it.
pub(super) fn spinning(on: bool) {
    SPINNING.store(on, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arming_nests_and_restores_what_stood_before() {
        assert!(!ARMED.load(Ordering::Relaxed));
        {
            let _outer = arm();
            assert!(ARMED.load(Ordering::Relaxed));
            {
                let _inner = pause();
                assert!(!ARMED.load(Ordering::Relaxed));
            }
            assert!(ARMED.load(Ordering::Relaxed));
            {
                let _again = arm();
                assert!(ARMED.load(Ordering::Relaxed));
            }
            assert!(ARMED.load(Ordering::Relaxed));
        }
        assert!(!ARMED.load(Ordering::Relaxed));
        assert!(!interrupted());
    }
}
