//! `evoke check`: the reflex in the working directory read with its lines to fix, its body's file present and
//! loaded by the runtime under its declaration — a declared path or program the machine lacks is a line to fix —
//! lint reported, `reflex.d.ts` written for a file body when it changed, and — when the directory sits in a git
//! repository whose newest version tag holds this reflex — the contract diffed against that tag, with the
//! version the next tag must carry; a `was` violation is refused. In: the working directory, the environment.
//! Out: `Exit`; the row, the findings and the contract on stdout, the write and the refusals on stderr.

use std::collections::BTreeMap;
use std::path::Path;

use evoke_core::contract::WasViolation;
use evoke_core::document::Text;
use evoke_core::manifest::Run;
use evoke_core::name::LocalName;
use evoke_core::needs::{self, Origin};
use evoke_core::plan::{DEADLINE, Millis};
use evoke_core::{
    Active, At, Call, Diagnostic, Document, Envelope, File, Fix, Input, Manifest, Version, diff,
    lint, manifest, reflex_dts, resolve,
};
use indexmap::IndexMap;

use super::{Exit, about, human};
use crate::args::Command;
use crate::hosts::contain;
use crate::hosts::processes::{self, Body, Probed};
use crate::hosts::state::State;
use crate::hosts::{Deadline, Environment, Failure, files, git, terminal};
use crate::report::{self, Paths};

pub fn run(command: &Command, environment: &Environment) -> Exit {
    let invoked = command.invoked("");
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            let failed = Exit::Failed(Failure {
                what: "finding the working directory".to_owned(),
                cause: Some(error.to_string()),
                fix: Fix::Rerun,
            });
            if let Some(line) = report::exit(&failed, &invoked, None) {
                terminal::note(&line);
            }
            return failed;
        }
    };
    // Location is identity: the directory names the reflex; one that is no name still gets a file to point at.
    let name = cwd
        .file_name()
        .and_then(|dir| dir.to_str())
        .and_then(|dir| LocalName::new(dir).ok())
        .unwrap_or_else(|| LocalName::new("reflex").expect("a name"));
    let paths = Paths {
        root: files::shown(&cwd, environment),
        reflexes: BTreeMap::from([(name.clone(), ".".to_owned())]),
        home: environment.get("HOME").map(str::to_owned),
    };
    let checking = Checking {
        cwd: &cwd,
        name: &name,
        paths: &paths,
        invoked: &invoked,
        environment,
    };
    let exit = checking.checked().unwrap_or_else(|exit| exit);
    if let Some(line) = report::exit(&exit, &invoked, Some(&paths)) {
        terminal::note(&line);
    }
    exit
}

/// What every step reads: where the reflex is, what it is called, how its lines print, what runs its body.
struct Checking<'a> {
    cwd: &'a Path,
    name: &'a LocalName,
    paths: &'a Paths,
    invoked: &'a str,
    environment: &'a Environment,
}

impl Checking<'_> {
    /// The straight sequence: parse, the body, the row, lint, the types, the contract against the newest tag.
    fn checked(&self) -> Result<Exit, Exit> {
        let text = files::read(&self.cwd.join("reflex.toml")).map_err(Exit::Failed)?;
        let Some(text) = text else {
            return Err(human("no reflex.toml here", Fix::New));
        };
        let checked = self.parsed(&text)?;
        if let Run::File(entrypoint) = &checked.run {
            self.loaded(&checked, &text, entrypoint.path().as_str())?;
        }
        terminal::answer(&report::checked_row(self.name, &checked));
        for finding in lint(&checked) {
            terminal::answer(&report::finding(self.name, &finding));
        }
        // An argv has no body to type.
        if matches!(checked.run, Run::File(_)) {
            let types = reflex_dts(&checked);
            let path = self.cwd.join("reflex.d.ts");
            if files::read(&path).map_err(Exit::Failed)?.as_deref() != Some(types.as_str()) {
                files::write(&path, &types).map_err(Exit::Failed)?;
                terminal::note(&report::created("reflex.d.ts"));
            }
        }
        if let Some((tag, previous)) = self.previous()? {
            let contract = diff(&previous, &checked);
            terminal::answer(&report::checked(tag, &contract));
            let violations: Vec<Diagnostic> = contract
                .violations
                .iter()
                .map(|violation| self.violated(violation))
                .collect();
            if let Some((last, rest)) = violations.split_last() {
                for problem in rest {
                    terminal::note(&report::diagnostic(problem, self.invoked, Some(self.paths)));
                }
                return Err(Exit::Human(last.clone()));
            }
        }
        Ok(Exit::Ran)
    }

    /// The body's file, present and loaded by the runtime under the declaration — its `{name}`s unstated, so
    /// dropped — with a function as its default export; what is wrong with it is the author's to fix, so it exits
    /// as a manifest's line does, and a declared path or program this machine lacks names its line.
    fn loaded(&self, checked: &Manifest, text: &str, run: &str) -> Result<(), Exit> {
        if !self.cwd.join(run).is_file() {
            return Err(about(
                self.name,
                format!("runs {run}, which does not exist"),
                Fix::Rerun,
            ));
        }
        let Some(found) = processes::find_runtime(self.environment) else {
            return Err(about(
                self.name,
                format!("needs node on PATH to load {run}"),
                Fix::Rerun,
            ));
        };
        let runtime = processes::real_runtime(&found, self.environment).map_err(Exit::Failed)?;
        let dir = std::fs::canonicalize(self.cwd).map_err(|error| {
            Exit::Failed(Failure {
                what: format!("finding {}", self.cwd.display()),
                cause: Some(crate::hosts::cause(&error)),
                fix: Fix::Rerun,
            })
        })?;
        let active = Active::of(checked, IndexMap::new());
        let call = Call {
            reflex: self.name.clone(),
            args: IndexMap::new(),
        };
        let home = self.environment.get("HOME").unwrap_or_default();
        let policy =
            resolve(&checked.needs, &call, &active, &IndexMap::new(), home).map_err(Exit::Human)?;
        let scratch = files::scratch(self.environment).map_err(Exit::Failed)?;
        let state = State::of(self.environment).map_err(Exit::Failed)?;
        let facts = match contain::facts(
            &policy,
            Some(&runtime),
            &dir,
            &scratch.path,
            &state,
            self.environment,
        )
        .map_err(Exit::Failed)?
        {
            Ok(facts) => facts,
            Err(missing) => {
                let origin = Origin::Local {
                    at: self.declared_at(text),
                };
                return Err(Exit::Human(needs::lacking(
                    &missing, self.name, &active, &origin, home,
                )));
            }
        };
        let envelope = Envelope {
            reflex: self.name.clone(),
            run: checked.run.clone(),
            args: IndexMap::new(),
            input: Input::new("").expect("nothing is under the cap"),
            config: IndexMap::new(),
            deadline: Millis(DEADLINE.0),
        };
        let what = format!("loading {run}");
        let body = Body {
            what: &what,
            envelope: &envelope,
            config: &IndexMap::new(),
            policy: &policy,
            facts: &facts,
            environment: self.environment,
            deadline: Deadline::after(DEADLINE),
        };
        let probed = processes::probe(&body, &runtime, &dir, run).map_err(Exit::Failed)?;
        match probed {
            Probed::Loads => Ok(()),
            Probed::DoesNotLoad(why) => Err(about(self.name, why, Fix::Rerun)),
        }
    }

    /// Where the manifest declares its needs, or its first line, where the table is added.
    fn declared_at(&self, text: &str) -> At {
        needs::declared_at(Document {
            file: File::Manifest {
                name: self.name.clone(),
            },
            text: Text::Toml(text),
        })
    }

    /// The manifest, or every line to fix: all but the last printed, the last the exit.
    fn parsed(&self, text: &str) -> Result<Manifest, Exit> {
        manifest(Document {
            file: File::Manifest {
                name: self.name.clone(),
            },
            text: Text::Toml(text),
        })
        .map_err(|mut problems| {
            let last = problems.pop().expect("the core names every problem");
            for problem in &problems {
                terminal::note(&report::diagnostic(problem, self.invoked, Some(self.paths)));
            }
            Exit::Human(last)
        })
    }

    /// The reflex as the repository's newest version tag holds it, when the directory sits in one and the tag has
    /// a `reflex.toml` there.
    fn previous(&self) -> Result<Option<(Version, Manifest)>, Exit> {
        let Some(repository) = git::repository(self.cwd).map_err(Exit::Failed)? else {
            return Ok(None);
        };
        let tags = git::local_tags(&repository).map_err(Exit::Failed)?;
        let Some(tag) = tags.last() else {
            return Ok(None);
        };
        let Some(tree) = git::tree_at(&repository, tag).map_err(Exit::Failed)? else {
            return Ok(None);
        };
        let text = tree
            .files
            .iter()
            .find(|(path, _)| path.as_str() == "reflex.toml")
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default();
        let previous = manifest(Document {
            file: File::Manifest {
                name: self.name.clone(),
            },
            text: Text::Toml(&text),
        })
        .map_err(|problems| {
            Exit::Failed(Failure {
                what: format!("reading {} at {}", self.name, tag.version),
                cause: problems.last().map(|problem| problem.message.clone()),
                fix: Fix::Rerun,
            })
        })?;
        Ok(Some((tag.version, previous)))
    }

    /// A `was` rule broken against the previous tag: fixed in the manifest, then checked again.
    fn violated(&self, violation: &WasViolation) -> Diagnostic {
        let message = match violation {
            WasViolation::Returned { arg } => {
                format!("args.{arg} is a retired name, back in use")
            }
            WasViolation::Dropped { arg } => format!("{arg} has gone from was"),
        };
        Diagnostic {
            reflex: Some(self.name.clone()),
            at: None,
            message,
            fix: Fix::Rerun,
        }
    }
}
