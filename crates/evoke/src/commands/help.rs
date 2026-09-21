//! `evoke --help` and `evoke --version`: every command, or the version — the output the invocation asked for, so
//! stdout, plain, exit 0. In: the crate's version. Out: `Exit::Ran`. No session opens and nothing is read.

use super::Exit;
use crate::hosts::terminal;
use crate::report;

pub fn run() -> Exit {
    terminal::result(&report::help(crate::VERSION));
    Exit::Ran
}

pub fn version() -> Exit {
    terminal::result(&format!("evoke {}", crate::VERSION));
    Exit::Ran
}
