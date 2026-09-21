//! The REPL's lines from the terminal. Where the terminal can edit, a line is edited in place — the cursor moved,
//! words killed and yanked, earlier lines recalled with the arrows and searched with `Ctrl-R` — and remembered in
//! the history file; where it cannot, a line is read plain, as a prompt's answer is. In: the prompt, the history
//! file. Out: the line typed, or the end of input.

use std::path::{Path, PathBuf};

use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Behavior, Config, Editor};

use super::{Look, Tty, tty};
use crate::hosts::{Failure, failed};

/// How many lines the history keeps.
const REMEMBERED: usize = 1000;

pub struct Repl {
    reader: Reader,
    history: PathBuf,
}

/// The editor is boxed: it is large, and the plain reader is not.
enum Reader {
    Editing(Box<Editor<(), FileHistory>>),
    Plain(Tty),
}

impl Repl {
    /// The terminal, or none: a cron job, a pipe on both ends. The history is loaded when the terminal edits.
    pub(super) fn open(history: PathBuf, look: Look) -> Option<Self> {
        let plain = tty()?;
        let reader = if look.editing {
            Reader::Editing(Box::new(editor(&history)?))
        } else {
            Reader::Plain(plain)
        };
        Some(Self { reader, history })
    }

    /// Shows the prompt and reads the line without its end; a line cancelled with `Ctrl-C` is an empty one; none
    /// at the end of input. A line that is not blank is remembered — best effort: a history that cannot be
    /// written costs nothing but the recall.
    pub fn line(&mut self, prompt: &str) -> Result<Option<String>, Failure> {
        match &mut self.reader {
            Reader::Plain(tty) => tty.prompt(prompt),
            Reader::Editing(editor) => match editor.readline(prompt) {
                Ok(line) => {
                    if !line.trim().is_empty() {
                        let _ = editor
                            .add_history_entry(line.as_str())
                            .and_then(|_| editor.append_history(&self.history));
                    }
                    Ok(Some(line))
                }
                Err(ReadlineError::Interrupted) => Ok(Some(String::new())),
                Err(ReadlineError::Eof) => Ok(None),
                Err(error) => Err(failed("reading the line", &error.to_string())),
            },
        }
    }
}

/// An editor on `/dev/tty`, never stdin or stdout, with the history file's lines; a history that is not there
/// yet is an empty one.
fn editor(history: &Path) -> Option<Editor<(), FileHistory>> {
    let config = Config::builder()
        .behavior(Behavior::PreferTerm)
        .auto_add_history(false)
        .max_history_size(REMEMBERED)
        .ok()?
        .history_ignore_dups(true)
        .ok()?
        .build();
    let remembered = FileHistory::with_config(&config);
    let mut editor = Editor::with_history(config, remembered).ok()?;
    let _ = editor.load_history(history);
    Some(editor)
}
