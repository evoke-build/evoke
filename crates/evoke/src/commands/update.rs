//! `evoke update [name] [--accept name]`: every unpinned remote reflex to its newest tag, a pinned one to its pin;
//! each move read as the contract diff and what it means for your files, the consented effect kept until accepted;
//! a reflex new to a repository you use reported; the lock written, your files never. What a move leaves inactive
//! prints at the next run, with its fix. A reflex that cannot move — its new manifest does not read — is reported
//! at the new tree, kept in the store, and skipped; the rest move. In: a name or none, a name whose looser effect
//! to accept or none, the environment. Out: `Exit`, a block per reflex moved, or `up to date`.

use std::collections::BTreeMap;

use evoke_core::contract::{Change, WasViolation};
use evoke_core::document::Text;
use evoke_core::manifest::{Effect, Record, Records};
use evoke_core::name::{ArgName, LocalName, RelPath};
use evoke_core::project::{Location, Lock, Locked, Reference};
use evoke_core::{
    Consent, ContractDiff, Diagnostic, Document, File, Fix, KeyPath, Level, Manifest, Overlay,
    Report, consent, diff, manifest, overlay, report,
};

use super::session::{self, Opening, Session};
use super::{Exit, about, default_name, nothing_installed};
use crate::args::Command;
use crate::hosts::store::Entry;
use crate::hosts::{Environment, files, git, terminal};
use crate::report::{self as render, Updated};

pub fn run(
    command: &Command,
    reflex: Option<&LocalName>,
    accept: Option<&LocalName>,
    environment: &Environment,
) -> Exit {
    let mut session = match session::open(command, false, environment, Opening::Installing) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.stand_in();
    let exit = updated(&mut session, &input, reflex, accept);
    session.reporter.exit(&input, exit)
}

/// One reflex moved: its block, and the accept line when upstream loosened the effect.
struct Moved {
    block: Updated,
    unaccepted: Option<Diagnostic>,
}

fn updated(
    session: &mut Session<'_>,
    input: &str,
    reflex: Option<&LocalName>,
    accept: Option<&LocalName>,
) -> Exit {
    if session.project.reflexes.is_empty() {
        return nothing_installed();
    }
    let scope: Vec<LocalName> = match reflex {
        Some(name) => vec![name.clone()],
        None => session
            .project
            .reflexes
            .iter()
            .filter(|(_, location)| matches!(location, Location::Remote { .. }))
            .map(|(name, _)| name.clone())
            .collect(),
    };
    let mut lock = session.lock.clone().unwrap_or_else(|| session.new_lock());
    let mut moved = Vec::new();
    let mut new = BTreeMap::new();
    let mut remotes = git::Remotes::default();
    // A reflex that cannot move is skipped, its problem the exit; the rest move.
    let mut skipped: Option<Exit> = None;
    for name in &scope {
        match one(
            session,
            input,
            name,
            accept == Some(name),
            &mut lock,
            &mut new,
            &mut remotes,
        ) {
            Ok(Some(one)) => moved.push(one),
            Ok(None) => {}
            Err(Exit::Human(problem)) => {
                if let Some(Exit::Human(earlier)) = skipped.replace(Exit::Human(problem)) {
                    session.reporter.note(input, &earlier);
                }
            }
            Err(exit) => return exit,
        }
    }
    if !moved.is_empty() {
        lock.adapter = session.locked_adapter();
        if let Err(exit) = session.write_lock(&lock) {
            return exit;
        }
        if let Err(exit) = session.reload(input).and_then(|()| session.write_types()) {
            return exit;
        }
    }
    if moved.is_empty() && skipped.is_none() {
        terminal::note(&render::up_to_date());
    }
    for one in &moved {
        terminal::note(&render::updated(&one.block));
        if let Some(problem) = &one.unaccepted {
            session.reporter.note(input, problem);
        }
    }
    for problem in new.values() {
        session.reporter.note(input, problem);
    }
    skipped.unwrap_or(Exit::Ran)
}

/// One reflex brought to its target tag, when it is not there already; `--accept` at the current tag takes
/// upstream's looser effect on.
fn one(
    session: &mut Session<'_>,
    input: &str,
    name: &LocalName,
    accept: bool,
    lock: &mut Lock,
    new: &mut BTreeMap<String, Diagnostic>,
    remotes: &mut git::Remotes,
) -> Result<Option<Moved>, Exit> {
    let Some(location) = session.project.reflexes.get(name) else {
        return Err(about(
            name,
            format!("{name} is not installed"),
            Fix::Show { reflex: None },
        ));
    };
    let Location::Remote { reference, pin } = location else {
        return Err(about(
            name,
            format!("{name} is local; nothing to update"),
            Fix::Show {
                reflex: Some(name.clone()),
            },
        ));
    };
    // Owned, so the session is free for the move to report at the tree it keeps.
    let (reference, pin) = (reference.clone(), *pin);
    let Some(locked) = lock.reflexes.get(name).cloned() else {
        return Err(about(
            name,
            format!("{name} is not locked"),
            Fix::AddRef {
                reference: location.to_string(),
                name: Some(name.clone()),
            },
        ));
    };
    let by_looking = Fix::Show {
        reflex: Some(name.clone()),
    };
    let tags = remotes.tags(&reference).map_err(Exit::Failed)?;
    let target = match pin {
        Some(pin) => tags
            .iter()
            .find(|tag| tag.version == pin)
            .ok_or_else(|| about(name, format!("{reference} has no tag {pin}"), by_looking))?,
        None => tags.last().ok_or_else(|| {
            about(
                name,
                format!(
                    "{reference} has no version tag; its author publishes one with git push --tags"
                ),
                by_looking,
            )
        })?,
    };
    let Some(previous) = session.store.entry(&locked.h1).map_err(Exit::Failed)? else {
        return Err(about(
            name,
            format!("{name} is not in the store"),
            Fix::Sync,
        ));
    };
    let previous_manifest = parsed(session, input, name, &previous.files)?;
    if target.version == locked.tag {
        if accept && previous_manifest.effect < locked.effect {
            lock.reflexes[name].effect = previous_manifest.effect;
            return Ok(Some(accepted(name, &locked, previous_manifest.effect)));
        }
        return Ok(None);
    }
    let r#move = Move {
        name,
        reference: &reference,
        locked: &locked,
        target,
        previous: &previous,
        previous_manifest: &previous_manifest,
    };
    moved(session, input, &r#move, accept, lock, new, remotes).map(Some)
}

/// A move in hand: what the reflex is locked at and what it goes to.
struct Move<'a> {
    name: &'a LocalName,
    reference: &'a Reference,
    locked: &'a Locked,
    target: &'a git::Tag,
    previous: &'a Entry,
    previous_manifest: &'a Manifest,
}

/// The target fetched and kept, the contract diffed, your overlay read against both, the lock entry replaced.
/// The tree is kept before its manifest is read, so a problem in it is reported where it now is.
fn moved(
    session: &mut Session<'_>,
    input: &str,
    r#move: &Move<'_>,
    accept: bool,
    lock: &mut Lock,
    new: &mut BTreeMap<String, Diagnostic>,
    remotes: &mut git::Remotes,
) -> Result<Moved, Exit> {
    let Move {
        name,
        reference,
        locked,
        target,
        previous,
        previous_manifest,
    } = *r#move;
    let fetched = remotes.fetch(reference, target).map_err(Exit::Failed)?;
    for dir in &fetched.all {
        siblings(session, reference, dir.as_ref(), new);
    }
    let Some(tree) = fetched
        .reflexes
        .into_iter()
        .find(|tree| tree.dir == reference.dir)
    else {
        return Err(about(
            name,
            format!("no reflex.toml at {reference} {}", target.version),
            Fix::Remove {
                reflex: name.clone(),
            },
        ));
    };
    let kept = session.store.keep(tree.files).map_err(Exit::Failed)?;
    let shown = files::shown(&kept.dir, session.environment());
    session.reporter.paths.reflexes.insert(name.clone(), shown);
    let next = parsed(session, input, name, &kept.files)?;
    let code_changed = code(&previous.files) != code(&kept.files);
    let contract = diff(previous_manifest, &next);
    let consent = consent(locked.effect, next.effect);
    let yours = session.overlay_text(name)?;
    let yours_next = yours
        .as_deref()
        .and_then(|text| overlay_of(name, text, &next));
    let yours_previous = yours
        .as_deref()
        .and_then(|text| overlay_of(name, text, previous_manifest));
    let report = report(previous_manifest, &next, yours_next.as_ref());
    let lines = details(
        previous_manifest,
        &next,
        yours_previous.as_ref(),
        &contract,
        &report,
        &consent,
    );
    let (effect, unaccepted) = match consent {
        Consent::Kept { effect } | Consent::Tightened { effect } => (effect, None),
        Consent::NeedsAccept { upstream, .. } if accept => (upstream, None),
        Consent::NeedsAccept { locked, upstream } => (
            locked,
            Some(Diagnostic {
                reflex: Some(name.clone()),
                at: None,
                message: format!("upstream loosened the effect to {upstream}; {locked} kept"),
                fix: Fix::Accept {
                    reflex: name.clone(),
                },
            }),
        ),
    };
    lock.reflexes.insert(
        name.clone(),
        Locked {
            reference: reference.clone(),
            tag: target.version,
            commit: fetched.commit,
            h1: kept.h1,
            effect,
        },
    );
    Ok(Moved {
        block: Updated {
            name: name.to_string(),
            from: locked.tag,
            to: target.version,
            level: contract.level,
            code_changed,
            lines,
        },
        unaccepted,
    })
}

/// `--accept` at the current tag: the looser effect upstream declares, taken on.
fn accepted(name: &LocalName, locked: &Locked, effect: Effect) -> Moved {
    Moved {
        block: Updated {
            name: name.to_string(),
            from: locked.tag,
            to: locked.tag,
            level: Level::Same,
            code_changed: false,
            lines: vec![(
                "effect".to_owned(),
                format!("{effect} accepted; was {}", locked.effect),
            )],
        },
        unaccepted: None,
    }
}

/// A reflex directory of the repository that the project does not install: reported once, with its add line.
fn siblings(
    session: &Session<'_>,
    reference: &Reference,
    dir: Option<&RelPath>,
    new: &mut BTreeMap<String, Diagnostic>,
) {
    if dir == reference.dir.as_ref() {
        return;
    }
    let sibling = Reference {
        repo: reference.repo.clone(),
        dir: dir.cloned(),
    };
    let installed = session.project.reflexes.values().any(
        |location| matches!(location, Location::Remote { reference, .. } if *reference == sibling),
    );
    if installed {
        return;
    }
    let text = sibling.to_string();
    new.entry(text.clone()).or_insert_with(|| Diagnostic {
        reflex: None,
        at: None,
        message: format!("{text} is new"),
        fix: Fix::AddRef {
            reference: text,
            name: default_name(&sibling).ok(),
        },
    });
}

/// The manifest among a tree's files, read for this reflex.
fn parsed(
    session: &Session<'_>,
    input: &str,
    name: &LocalName,
    files: &[(RelPath, Vec<u8>)],
) -> Result<Manifest, Exit> {
    let text = files
        .iter()
        .find(|(path, _)| path.as_str() == "reflex.toml")
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .unwrap_or_default();
    manifest(Document {
        file: File::Manifest { name: name.clone() },
        text: Text::Toml(&text),
    })
    .map_err(|problems| session.reporter.human(input, problems))
}

/// Your overlay read against a manifest; one that does not read is none here and inactive after.
fn overlay_of(name: &LocalName, text: &str, of: &Manifest) -> Option<Overlay> {
    overlay(
        Document {
            file: File::Overlay { name: name.clone() },
            text: Text::Toml(text),
        },
        of,
    )
    .ok()
}

/// A tree's files but the manifest, in path order: what "code changed" compares.
fn code(files: &[(RelPath, Vec<u8>)]) -> Vec<(&RelPath, &Vec<u8>)> {
    let mut code: Vec<(&RelPath, &Vec<u8>)> = files
        .iter()
        .filter(|(path, _)| path.as_str() != "reflex.toml")
        .map(|(path, bytes)| (path, bytes))
        .collect();
    code.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    code
}

/// What the move means, one line each: the description when rewritten, every contract change, every `was`
/// violation, what upstream changed that you override, what yours no longer addresses, the effect when it moved.
fn details(
    previous: &Manifest,
    next: &Manifest,
    yours: Option<&Overlay>,
    contract: &ContractDiff,
    report: &Report,
    consent: &Consent,
) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    let description = KeyPath::new(["description"]);
    if previous.description != next.description {
        let what = if report.stale.contains(&description) {
            "rewritten, yours kept"
        } else {
            "rewritten, taken (you have no override)"
        };
        lines.push(("description".to_owned(), what.to_owned()));
    }
    for change in &contract.changes {
        let (key, mut what) = render::change(change);
        if let Change::ArgRenamed { from, .. } = change {
            what.push_str(&following(yours, from));
        }
        lines.push((key, what));
    }
    for violation in &contract.violations {
        lines.push(match violation {
            WasViolation::Returned { arg } => (
                format!("args.{arg}"),
                "a retired name, back in use".to_owned(),
            ),
            WasViolation::Dropped { arg } => (format!("args.{arg}"), "gone from was".to_owned()),
        });
    }
    for path in report.stale.iter().filter(|path| **path != description) {
        lines.push((path.to_string(), "changed upstream, yours kept".to_owned()));
    }
    for path in &report.orphaned {
        lines.push((path.to_string(), "addresses nothing, skipped".to_owned()));
    }
    match consent {
        Consent::Kept { .. } => {}
        Consent::Tightened { effect } => {
            lines.push(("effect".to_owned(), format!("tightened to {effect}")));
        }
        Consent::NeedsAccept { locked, upstream } => lines.push((
            "effect".to_owned(),
            format!("{upstream} upstream; {locked} kept until you accept"),
        )),
    }
    lines
}

/// `; your 1 example follows`, `; your 2 examples and 1 test follow`: your records that named the old argument.
fn following(yours: Option<&Overlay>, from: &ArgName) -> String {
    let Some(yours) = yours else {
        return String::new();
    };
    let count = |records: &Records| {
        records
            .iter()
            .filter(|(_, (_, record))| {
                matches!(record, Record::Asserts(asserts) if asserts.contains_key(from))
            })
            .count()
    };
    let plural = |n: usize, one: &str| {
        if n == 1 {
            format!("{n} {one}")
        } else {
            format!("{n} {one}s")
        }
    };
    let (examples, tests) = (count(&yours.examples), count(&yours.tests));
    let parts: Vec<String> = [(examples, "example"), (tests, "test")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, one)| plural(n, one))
        .collect();
    if parts.is_empty() {
        return String::new();
    }
    let verb = if examples + tests == 1 {
        "follows"
    } else {
        "follow"
    };
    format!("; your {} {verb}", parts.join(" and "))
}
