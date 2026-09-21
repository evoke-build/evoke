//! `evoke add <ref>… [--as name]`: each ref fetched at its pin or its newest tag and kept in the store, its manifest
//! read and linted; the installed examples routed over the new set, to name each phrase a newcomer steals; then the
//! project and the lock written, the runtime recorded, and for each newcomer its line, then the lint, theft and
//! inactive lines. Nothing is written until every newcomer is in hand. In: the refs, one name or none, the
//! environment. Out: `Exit`.

use evoke_core::document::Text;
use evoke_core::name::LocalName;
use evoke_core::project::{Commit, Location, Locked, Reference};
use evoke_core::{
    Case, Diagnostic, Digest, Document, File, Finding, Fix, Item, Manifest, Scope, Table, Theft,
    Version, add_entry, cases, compile, effective, lint, manifest, read, request, thieves,
};

use super::session::{self, Opening, Session};
use super::{Exit, about, default_name, human};
use crate::adapter;
use crate::args::{Command, Ref};
use crate::hosts::{Deadline, Environment, files, git, terminal};
use crate::report::{self, Gutter};

pub fn run(
    command: &Command,
    refs: &[Ref],
    name: Option<&LocalName>,
    environment: &Environment,
) -> Exit {
    let mut session = match session::open(command, false, environment, Opening::Installing) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.placeholder();
    let exit = added(&mut session, &input, refs, name);
    session.reporter.exit(&input, exit)
}

/// A reflex fetched, kept and read, not yet installed.
struct Newcomer {
    name: LocalName,
    written: String,
    location: Location,
    reference: Reference,
    tag: Version,
    commit: Commit,
    h1: Digest,
    manifest: Manifest,
    findings: Vec<Finding>,
}

fn added(session: &mut Session<'_>, input: &str, refs: &[Ref], name: Option<&LocalName>) -> Exit {
    let mut newcomers = Vec::new();
    for r in refs {
        match fetched(session, input, r, name) {
            Ok(found) => newcomers.extend(found),
            Err(exit) => return exit,
        }
    }
    if let Err(exit) = placed(session, &newcomers) {
        return exit;
    }
    let thefts = match stolen(session, input, &newcomers) {
        Ok(thefts) => thefts,
        Err(exit) => return exit,
    };
    for newcomer in &newcomers {
        if let Err(exit) = session.land(input, &add_entry(&newcomer.name, &newcomer.location)) {
            return exit;
        }
    }
    let mut lock = session.lock.clone().unwrap_or_else(|| session.new_lock());
    lock.adapter = session.locked_adapter();
    for newcomer in &newcomers {
        lock.reflexes.insert(
            newcomer.name.clone(),
            Locked {
                reference: newcomer.reference.clone(),
                tag: newcomer.tag,
                commit: newcomer.commit.clone(),
                h1: newcomer.h1,
                effect: newcomer.manifest.effect,
            },
        );
    }
    if let Err(exit) = session.write_lock(&lock) {
        return exit;
    }
    if let Err(exit) = session.reload(input).and_then(|()| session.write_types()) {
        return exit;
    }
    let runtime = match session.record_runtime() {
        Ok(runtime) => runtime,
        Err(exit) => return exit,
    };
    let names: Vec<&LocalName> = newcomers.iter().map(|newcomer| &newcomer.name).collect();
    terminal::note(&report::rows(
        &session.rows(names.iter().copied()),
        Gutter::Added,
    ));
    for newcomer in &newcomers {
        for finding in &newcomer.findings {
            terminal::note(&report::finding(&newcomer.name, finding));
        }
    }
    for theft in &thefts {
        session.reporter.note(input, &stolen_line(theft));
    }
    let problems = session.inactive_of(names.iter().copied());
    if !problems.is_empty() {
        terminal::note(&report::inactive(
            &problems,
            &session.reporter.command.placeholder(),
            &session.reporter.paths,
        ));
    }
    runtime.map_or(Exit::Ran, Exit::Human)
}

/// One ref's reflexes, fetched at the tag it pins or the newest, kept in the store, read and linted.
fn fetched(
    session: &mut Session<'_>,
    input: &str,
    r: &Ref,
    name: Option<&LocalName>,
) -> Result<Vec<Newcomer>, Exit> {
    let tags = git::tags(&r.reference).map_err(Exit::Failed)?;
    let tag = match r.pin {
        Some(pin) => tags
            .iter()
            .find(|tag| tag.version == pin)
            .ok_or_else(|| human(format!("{} has no tag {pin}", r.reference), Fix::Rerun))?,
        None => tags.last().ok_or_else(|| {
            human(
                format!(
                    "{} has no version tag; its author publishes one with git push --tags",
                    r.reference
                ),
                Fix::Rerun,
            )
        })?,
    };
    let fetched = git::fetch(&r.reference, tag).map_err(Exit::Failed)?;
    if fetched.reflexes.is_empty() {
        return Err(human(
            format!("no reflex.toml at {} {}", r.reference, tag.version),
            Fix::Rerun,
        ));
    }
    let one = fetched.reflexes.len() == 1 && fetched.reflexes[0].dir == r.reference.dir;
    if !one && name.is_some() {
        return Err(human(
            format!("{} is a collection; --as names one reflex", r.reference),
            Fix::Rerun,
        ));
    }
    let mut found = Vec::new();
    for tree in fetched.reflexes {
        let reference = Reference {
            repo: r.reference.repo.clone(),
            dir: tree.dir.clone(),
        };
        let location = Location::Remote {
            reference: reference.clone(),
            pin: r.pin,
        };
        let written = if one {
            r.written.clone()
        } else {
            location.to_string()
        };
        let local = match name {
            Some(name) => name.clone(),
            None => default_name(&reference).map_err(|why| {
                human(
                    format!("{reference}: {why}"),
                    Fix::AddRef {
                        reference: written.clone(),
                        name: None,
                    },
                )
            })?,
        };
        let text = tree
            .files
            .iter()
            .find(|(path, _)| path.as_str() == "reflex.toml")
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default();
        let kept = session.store.keep(tree.files).map_err(Exit::Failed)?;
        let shown = files::shown(&kept.dir, session.environment());
        session.reporter.paths.reflexes.insert(local.clone(), shown);
        let manifest = manifest(Document {
            file: File::Manifest {
                name: local.clone(),
            },
            text: Text::Toml(&text),
        })
        .map_err(|problems| session.reporter.human(input, problems))?;
        let findings = lint(&manifest);
        found.push(Newcomer {
            name: local,
            written,
            location,
            reference,
            tag: tag.version,
            commit: fetched.commit.clone(),
            h1: kept.h1,
            manifest,
            findings,
        });
    }
    Ok(found)
}

/// Every newcomer's name is free: not another newcomer's, not an installed reflex's from elsewhere, not one
/// already locked. An entry the project names but the lock lacks is taken over.
fn placed(session: &Session<'_>, newcomers: &[Newcomer]) -> Result<(), Exit> {
    for (i, newcomer) in newcomers.iter().enumerate() {
        let choose = Fix::AddRef {
            reference: newcomer.written.clone(),
            name: None,
        };
        if newcomers[..i]
            .iter()
            .any(|other| other.name == newcomer.name)
        {
            return Err(human(
                format!("two of these would be named {}", newcomer.name),
                choose,
            ));
        }
        let locked = session
            .lock
            .as_ref()
            .is_some_and(|lock| lock.reflexes.contains_key(&newcomer.name));
        match session.project.reflexes.get(&newcomer.name) {
            Some(existing) if *existing != newcomer.location => {
                return Err(about(
                    &newcomer.name,
                    format!("{} is already installed from {existing}", newcomer.name),
                    choose,
                ));
            }
            Some(_) if locked => {
                return Err(about(
                    &newcomer.name,
                    format!("{} is already installed", newcomer.name),
                    Fix::Update {
                        reflex: Some(newcomer.name.clone()),
                    },
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// The route-only conflict test: every installed example routed over the set with the newcomers in it, through
/// the adapter; nothing is asked when nothing is installed or no newcomer is active.
fn stolen(session: &Session<'_>, input: &str, newcomers: &[Newcomer]) -> Result<Vec<Theft>, Exit> {
    let names: Vec<LocalName> = newcomers
        .iter()
        .map(|newcomer| newcomer.name.clone())
        .collect();
    let examples: Vec<Case> = cases(&session.installed)
        .into_iter()
        .filter(|case| case.from == Table::Examples && !names.contains(&case.reflex))
        .collect();
    if examples.is_empty() {
        return Ok(Vec::new());
    }
    let mut set = session.installed.clone();
    for newcomer in newcomers {
        set.reflexes.insert(
            newcomer.name.clone(),
            Item {
                wording: Ok(effective(&newcomer.manifest, None)),
                consented: newcomer.manifest.effect,
                configured: session.configured(&newcomer.name),
            },
        );
    }
    let plan = compile(&set, session.declared.limits.as_ref()).map_err(Exit::Human)?;
    if !plan.active().keys().any(|name| names.contains(name)) {
        return Ok(Vec::new());
    }
    let adapter = adapter::resolve(
        &session.project.adapter,
        session.project.adapters.get(&session.project.adapter),
        session.environment(),
    )
    .map_err(|problems| session.reporter.human(input, problems))?;
    adapter.accepts(&plan.digest()).map_err(Exit::Human)?;
    let mut routed = Vec::new();
    for case in examples {
        let request = request(&plan, case.utterance.text().as_str(), &[], Scope::Route)
            .map_err(Exit::Human)?;
        let raw = adapter
            .answer(&request, Deadline::after(plan.deadline()))
            .map_err(Exit::Adapter)?;
        let reading = read(&plan, &request, raw).map_err(Exit::Adapter)?;
        routed.push((case, reading.winner.map(|winner| winner.reflex)));
    }
    Ok(thieves(&names, &routed))
}

/// A theft as a line: the thief, the phrase and its owner, and the lesson that settles it.
fn stolen_line(theft: &Theft) -> Diagnostic {
    Diagnostic {
        reflex: Some(theft.thief.clone()),
        at: None,
        message: format!("steals \"{}\" from {}", theft.phrase.text(), theft.owner),
        fix: Fix::TeachNot {
            utterance: theft.phrase.text().to_string(),
            reflex: theft.thief.clone(),
        },
    }
}
