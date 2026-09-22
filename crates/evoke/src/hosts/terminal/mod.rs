//! What reaches the terminal: results on stdout, everything else on stderr — coloured and weighted where stderr
//! is a terminal that shows it, plain everywhere else; whether stdin is a pipe, and its lines; prompts on
//! `/dev/tty`, never stdin; the REPL's lines, edited and remembered; a spinner while a request or a fetch is in
//! flight; the width of the terminal stdout is, for a layout that must fit. In: `Text` and prompts. Out: nothing,
//! the lines a pipe holds, or what was typed at a prompt.

mod busy;
mod repl;
mod size;
mod text;

use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::OnceLock;

pub use busy::Busy;
pub use repl::Repl;
pub use size::columns;
pub use text::{Role, Text};

use super::{Environment, Failure, failed};

/// What the terminal shows beyond plain text, decided once. Colour and weight need stderr to be a terminal,
/// `NO_COLOR` unset or empty, and `TERM` set to something other than `dumb`; the spinner needs the terminal and
/// the `TERM`; line editing on `/dev/tty` needs the `TERM`. A pipe, a log, `TERM=dumb` and the transcripts see
/// plain text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Look {
    pub styled: bool,
    pub animated: bool,
    pub editing: bool,
}

impl Look {
    #[must_use]
    pub fn of(stderr_is_terminal: bool, environment: &Environment) -> Self {
        let capable = environment
            .get("TERM")
            .is_some_and(|term| !term.is_empty() && term != "dumb");
        let colour = environment.get("NO_COLOR").is_none_or(str::is_empty);
        Self {
            styled: stderr_is_terminal && capable && colour,
            animated: stderr_is_terminal && capable,
            editing: capable,
        }
    }
}

static LOOK: OnceLock<Look> = OnceLock::new();

/// Decides the look from stderr and the environment, once at the start; until then everything is plain.
pub fn configure(environment: &Environment) {
    let _ = LOOK.set(Look::of(io::stderr().is_terminal(), environment));
}

fn look() -> Look {
    LOOK.get().copied().unwrap_or_default()
}

/// A reflex's result, the `--json` line, or what `--help` and `--version` asked for: stdout, flushed at once,
/// never styled.
pub fn result(line: &str) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

/// Everything else: stderr, styled where the terminal shows it.
pub fn note(text: &Text) {
    let shown = if look().styled {
        text.styled()
    } else {
        text.to_string()
    };
    let _ = writeln!(io::stderr().lock(), "{shown}");
}

/// A spinner on stderr while the returned value is held: `what` is happening.
#[must_use]
pub fn busy(what: impl Into<String>) -> Busy {
    Busy::start(what.into(), look())
}

/// The REPL's reader over the terminal, its history in the file; none when there is no terminal.
#[must_use]
pub fn repl(history: PathBuf) -> Option<Repl> {
    Repl::open(history, look())
}

/// Whether stdin is not a terminal, so its lines are input.
#[must_use]
pub fn stdin_is_pipe() -> bool {
    !io::stdin().is_terminal()
}

/// One input per line of stdin.
pub fn stdin_lines() -> impl Iterator<Item = io::Result<String>> {
    io::stdin().lines()
}

/// The terminal a person answers on, read and written directly.
pub struct Tty {
    reader: BufReader<File>,
    writer: File,
}

/// The terminal, or none: a cron job, a pipe on both ends.
#[must_use]
pub fn tty() -> Option<Tty> {
    let open = || OpenOptions::new().read(true).write(true).open("/dev/tty");
    let (reader, writer) = (open().ok()?, open().ok()?);
    Some(Tty {
        reader: BufReader::new(reader),
        writer,
    })
}

impl Tty {
    /// Shows one line on the terminal, ahead of a prompt.
    pub fn show(&mut self, line: &str) -> Result<(), Failure> {
        self.writer
            .write_all(line.as_bytes())
            .and_then(|()| self.writer.write_all(b"\n"))
            .and_then(|()| self.writer.flush())
            .map_err(|error| failed("showing the prompt", &super::cause(&error)))
    }

    /// Shows the text and reads what was typed, without its line end; none at the end of input.
    pub fn prompt(&mut self, text: &str) -> Result<Option<String>, Failure> {
        let asked = self
            .writer
            .write_all(text.as_bytes())
            .and_then(|()| self.writer.flush());
        asked.map_err(|error| failed("showing the prompt", &super::cause(&error)))?;
        let mut typed = String::new();
        match self.reader.read_line(&mut typed) {
            Ok(0) => {
                let _ = self.writer.write_all(b"\n");
                Ok(None)
            }
            Ok(_) => Ok(Some(typed.trim_end_matches(['\n', '\r']).to_owned())),
            Err(error) => Err(failed("reading the answer", &super::cause(&error))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn environment(vars: &[(&str, &str)]) -> Environment {
        Environment(
            vars.iter()
                .map(|(var, value)| ((*var).to_owned(), (*value).to_owned()))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn a_terminal_with_a_term_is_styled_animated_and_edited() {
        let look = Look::of(true, &environment(&[("TERM", "xterm-256color")]));
        assert_eq!(
            look,
            Look {
                styled: true,
                animated: true,
                editing: true
            }
        );
    }

    #[test]
    fn a_pipe_is_plain_and_still_edits_on_the_tty() {
        let look = Look::of(false, &environment(&[("TERM", "xterm")]));
        assert_eq!(
            look,
            Look {
                styled: false,
                animated: false,
                editing: true
            }
        );
    }

    #[test]
    fn no_color_takes_the_colour_and_leaves_the_movement() {
        let look = Look::of(true, &environment(&[("TERM", "xterm"), ("NO_COLOR", "1")]));
        assert_eq!(
            look,
            Look {
                styled: false,
                animated: true,
                editing: true
            }
        );
        let empty = Look::of(true, &environment(&[("TERM", "xterm"), ("NO_COLOR", "")]));
        assert!(empty.styled);
    }

    #[test]
    fn a_dumb_or_missing_term_is_plain_everywhere() {
        assert_eq!(
            Look::of(true, &environment(&[("TERM", "dumb")])),
            Look::default()
        );
        assert_eq!(Look::of(true, &environment(&[])), Look::default());
        assert_eq!(
            Look::of(true, &environment(&[("TERM", "")])),
            Look::default()
        );
    }
}
