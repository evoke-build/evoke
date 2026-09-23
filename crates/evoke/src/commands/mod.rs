//! One file per command word, and the session they share; orchestration only: a straight sequence of host and
//! core calls, each failure mapped to `Exit` explicitly. In: a `Command`, the environment. Out: `Exit`, everything
//! already printed.

pub mod add;
pub mod check;
pub mod config;
pub mod help;
pub mod new;
pub mod remove;
pub mod run;
pub mod session;
pub mod show;
pub mod sync;
pub mod teach;
pub mod test;
pub mod trust;
pub mod r#try;
pub mod update;
pub mod r#use;
pub mod vocab;
pub mod why;

use evoke_core::name::LocalName;
use evoke_core::project::{Reference, Repo};
use evoke_core::{Diagnostic, Fault, Fix};

use crate::args::Command;
use crate::hosts::{Environment, Failure, terminal};
use crate::report::{self, Paths};

/// How a command ended: 0 ran · 1 failed · 2 declined · 3 needs a human · 4 the adapter failed.
#[derive(Clone, Debug, PartialEq)]
pub enum Exit {
    Ran,
    Failed(Failure),
    Declined(Decline),
    Human(Diagnostic),
    Adapter(Fault),
}

/// Nothing ran, by decision: the classifier abstained, or the person said no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decline {
    Abstained,
    Refused,
}

impl Exit {
    #[must_use]
    pub fn code(&self) -> i32 {
        match self {
            Self::Ran => 0,
            Self::Failed(_) => 1,
            Self::Declined(_) => 2,
            Self::Human(_) => 3,
            Self::Adapter(_) => 4,
        }
    }
}

pub fn dispatch(command: &Command, environment: &Environment) -> Exit {
    match command {
        Command::Use(arguments) => r#use::run(command, arguments, environment),
        Command::Try(arguments) => r#try::run(command, arguments, environment),
        Command::Why => why::run(environment),
        Command::Run { written, json } => run::run(command, written, *json, environment),
        Command::Teach { spoken, lesson } => teach::run(command, spoken, lesson, environment),
        Command::Show(name) => show::run(command, name.as_ref(), environment),
        Command::Vocab { name, change } => vocab::run(command, name, change.as_ref(), environment),
        Command::Config {
            reflex,
            key,
            setting,
        } => config::run(command, reflex, key, setting, environment),
        Command::Add { refs, name } => add::run(command, refs, name.as_ref(), environment),
        Command::Remove(name) => remove::run(command, name, environment),
        Command::Update { reflex, accept } => {
            update::run(command, reflex.as_ref(), accept.as_ref(), environment)
        }
        Command::Sync => sync::run(command, environment),
        Command::Trust => trust::run(environment),
        Command::New(name) => new::run(command, name),
        Command::Check => check::run(command, environment),
        Command::Test(name) => test::run(command, name.as_ref(), environment),
        Command::Help => help::run(),
        Command::Version => help::version(),
    }
}

/// A host fact that needs a human, minted by a command: what, and the command that fixes it.
pub(crate) fn human(message: impl Into<String>, fix: Fix) -> Exit {
    Exit::Human(Diagnostic {
        reflex: None,
        at: None,
        message: message.into(),
        fix,
    })
}

/// Nothing is installed, where a command has nothing to act on: the add line.
pub(crate) fn nothing_installed() -> Exit {
    human("no reflexes are installed", Fix::Add)
}

/// The same, about one reflex.
pub(crate) fn about(reflex: &LocalName, message: impl Into<String>, fix: Fix) -> Exit {
    Exit::Human(Diagnostic {
        reflex: Some(reflex.clone()),
        at: None,
        message: message.into(),
        fix,
    })
}

/// The local name a ref takes without `--as`: its last segment, or the repository's name.
pub(crate) fn default_name(reference: &Reference) -> Result<LocalName, String> {
    let segment = match (&reference.dir, &reference.repo) {
        (Some(dir), _) => dir.as_str().rsplit('/').next().unwrap_or(dir.as_str()),
        (None, Repo::GitHub { name, .. }) => name.as_str(),
        (None, Repo::Url { url }) => url
            .as_str()
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".git"),
    };
    LocalName::new(segment)
}

/// How a command's problems reach the terminal: the line `Rerun` repeats, whether `--json` is on, and the owned
/// files' paths as shown.
pub struct Reporter<'a> {
    pub command: &'a Command,
    pub json: bool,
    pub paths: Paths,
}

impl Reporter<'_> {
    /// The exit's line, when it has one, for the input it concerns: text on stderr, or under `--json` one object
    /// on stdout.
    pub fn exit(&self, input: &str, exit: Exit) -> Exit {
        if self.json {
            if let Some(object) = report::exit_json(&exit) {
                terminal::result(&object);
            }
        } else if let Some(line) =
            report::exit(&exit, &self.command.invoked(input), Some(&self.paths))
        {
            terminal::note(&line);
        }
        exit
    }

    /// Every problem but the last noted with its fix; the last is the exit, not yet printed.
    pub fn human(&self, input: &str, mut problems: Vec<Diagnostic>) -> Exit {
        let last = problems.pop().expect("the core names every problem");
        for problem in &problems {
            self.note(input, problem);
        }
        Exit::Human(last)
    }

    /// Every problem, each with its fix; the last one is the exit.
    pub fn problems(&self, input: &str, problems: Vec<Diagnostic>) -> Exit {
        let exit = self.human(input, problems);
        self.exit(input, exit)
    }

    pub fn note(&self, input: &str, problem: &Diagnostic) {
        terminal::note(&report::diagnostic(
            problem,
            &self.command.invoked(input),
            Some(&self.paths),
        ));
    }
}

/// `<what> needs a terminal`: the stop when a prompt has none to read.
#[must_use]
pub fn needs_terminal(what: &str) -> Diagnostic {
    Diagnostic {
        reflex: None,
        at: None,
        message: format!("{what} needs a terminal"),
        fix: Fix::Rerun,
    }
}

/// Every non-blank line of stdin acted on in turn: the first non-zero exit is the command's, and a read error
/// stops it, reported as the failure it is.
pub fn each_line(json: bool, mut act: impl FnMut(&str) -> Exit) -> Exit {
    let mut first = Exit::Ran;
    for line in terminal::stdin_lines() {
        let exit = match line {
            Ok(input) if input.trim().is_empty() => continue,
            Ok(input) => act(&input),
            Err(error) => {
                let failed = Exit::Failed(Failure {
                    what: "reading stdin".to_owned(),
                    cause: Some(error.to_string()),
                    fix: Fix::Rerun,
                });
                if json {
                    if let Some(object) = report::exit_json(&failed) {
                        terminal::result(&object);
                    }
                } else if let Some(line) = report::exit(&failed, "<input>", None) {
                    terminal::note(&line);
                }
                return if first == Exit::Ran { failed } else { first };
            }
        };
        if first == Exit::Ran {
            first = exit;
        }
    }
    first
}
