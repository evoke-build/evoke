//! The terminal's width, asked of stdout: what a layout that must fit on a line reads — the help, laid out in two
//! columns only where they fit. In: nothing. Out: the columns, or none when stdout is not a terminal or the
//! terminal does not say.

// TIOCGWINSZ is an ioctl: the kernel fills a struct of its own, and `std` has no door to it.
#![expect(unsafe_code)]

use std::io::{self, IsTerminal};

/// The columns stdout has, when it is a terminal that says.
#[must_use]
pub fn columns() -> Option<usize> {
    if !io::stdout().is_terminal() {
        return None;
    }
    let mut size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: TIOCGWINSZ writes one `winsize` through the pointer and nothing else; `size` is one, and stdout is
    // open for as long as the process is.
    let asked = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &raw mut size) };
    (asked == 0 && size.ws_col > 0).then(|| usize::from(size.ws_col))
}
