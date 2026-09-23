//! The CLI: `args` → `commands` → `report`; the exit code; a panic caught once. In: the arguments and the
//! environment. Out: exit 0 ran · 1 failed · 2 declined · 3 needs a human · 4 the adapter failed. Only a reflex's
//! result and the `--json` line reach stdout; everything else goes to stderr.

// Errors are data with their own types, as in the core: `Diagnostic` and `Exit` are the surface's errors by design.
#![allow(clippy::result_large_err)]

mod adapter;
mod args;
mod commands;
mod hosts;
mod report;

use commands::Exit;
use hosts::{Environment, terminal};

/// The crate version: part of every plan digest, and of the line a bug prints.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    std::panic::set_hook(Box::new(|panic| {
        let payload = panic.payload();
        let message = payload.downcast_ref::<&str>().map_or_else(
            || {
                payload
                    .downcast_ref::<String>()
                    .map_or("no message", String::as_str)
            },
            |message| *message,
        );
        let at = panic
            .location()
            .map_or_else(String::new, |at| format!(" ({}:{})", at.file(), at.line()));
        terminal::note(
            &format!(
                "  evoke {VERSION} hit a bug: {}{at}  →  https://github.com/evoke-build/evoke/issues",
                report::plain(message)
            )
            .into(),
        );
    }));
    let code = std::panic::catch_unwind(run).map_or(1, |exit| exit.code());
    std::process::exit(code);
}

fn run() -> Exit {
    let environment = Environment::of_process();
    terminal::configure(&environment);
    match args::parse(std::env::args_os().skip(1), terminal::stdin_is_pipe()) {
        Ok(command) => commands::dispatch(&command, &environment),
        Err(problem) => {
            terminal::note(&report::diagnostic(&problem, "", None));
            Exit::Human(problem)
        }
    }
}
