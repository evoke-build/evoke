//! `evoke add <ref>… [--as name]`: each remote ref fetched at its pin or its newest tag and kept in the store — one
//! fetch per repository and tag, however many reflexes come from it — and each local ref read where it is; every
//! manifest read and linted; the installed examples routed over the new set, a few at a time, to name each phrase
//! a newcomer steals; then the project, the lock and `evoke.d.ts` written, the runtime recorded, and for each
//! newcomer its row, then the lint, theft and inactive lines. Nothing is written until every newcomer is in hand;
//! a theft test the adapter could not finish is reported with `evoke test`, never a refusal. In: the refs, one name
//! or none, the environment. Out: `Exit`.

use std::path::PathBuf;

use evoke_core::document::Text;
use evoke_core::name::LocalName;
use evoke_core::project::{Location, Locked, Reference};
use evoke_core::{
    Case, Diagnostic, Document, Fault, File, Finding, Fix, Item, Manifest, Scope, Table, Theft,
    Version, add_entry, cases, compile, effective, lint, manifest, read, request, thieves,
};

use super::session::{self, Opening, Session};
use super::{Exit, about, default_name, human};
use crate::adapter;
use crate::args::{Command, Ref};
use crate::hosts::{Deadline, Environment, files, git, terminal, threads};
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

/// A reflex read and, when remote, kept in the store: not yet installed.
struct Newcomer {
    name: LocalName,
    written: String,
    location: Location,
    manifest: Manifest,
    findings: Vec<Finding>,
    /// The lock entry a remote reflex takes; a local one is never locked.
    locked: Option<Locked>,
}

fn added(session: &mut Session<'_>, input: &str, refs: &[Ref], name: Option<&LocalName>) -> Exit {
    let mut remotes = git::Remotes::default();
    let mut newcomers = Vec::new();
    for r in refs {
        let found = match &r.location {
            Location::Local { .. } => local(session, input, r, name).map(|one| vec![one]),
            Location::Remote { reference, pin } => {
                fetched(session, input, &mut remotes, r, reference, *pin, name)
            }
        };
        match found {
            Ok(found) => newcomers.extend(found),
            Err(exit) => return exit,
        }
    }
    if let Err(exit) = placed(session, &newcomers) {
        return exit;
    }
    let (thefts, unfinished) = match stolen(session, input, &newcomers) {
        Ok(thefts) => (thefts, None),
        Err(Exit::Adapter(fault)) => (Vec::new(), Some(fault)),
        Err(exit) => return exit,
    };
    for newcomer in &newcomers {
        if let Err(exit) = session.land(input, &add_entry(&newcomer.name, &newcomer.location)) {
            return exit;
        }
    }
    let locked: Vec<(&LocalName, &Locked)> = newcomers
        .iter()
        .filter_map(|newcomer| Some((&newcomer.name, newcomer.locked.as_ref()?)))
        .collect();
    if !locked.is_empty() {
        let mut lock = session.lock.clone().unwrap_or_else(|| session.new_lock());
        lock.adapter = session.locked_adapter();
        for (name, entry) in locked {
            lock.reflexes.insert(name.clone(), entry.clone());
        }
        if let Err(exit) = session.write_lock(&lock) {
            return exit;
        }
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
    if let Some(fault) = unfinished {
        session.reporter.note(input, &unfinished_line(&fault));
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

/// A local ref: the directory as `evoke.toml` will name it, relative to the project; its manifest read and
/// linted where it is. Nothing is fetched or kept.
fn local(
    session: &mut Session<'_>,
    input: &str,
    r: &Ref,
    name: Option<&LocalName>,
) -> Result<Newcomer, Exit> {
    let Some(path) = files::relative(&session.root.path, &r.written).map_err(Exit::Failed)? else {
        return Err(human(
            format!("{} is not a directory", r.written),
            Fix::Rerun,
        ));
    };
    let dir: PathBuf = session.root.path.join(&path).components().collect();
    let text = files::read(&dir.join("reflex.toml")).map_err(Exit::Failed)?;
    let Some(text) = text else {
        return Err(human(format!("no reflex.toml in {}", r.written), Fix::New));
    };
    let local = if let Some(name) = name {
        name.clone()
    } else {
        let last = path.rsplit('/').next().unwrap_or(&path);
        LocalName::new(last).map_err(|why| {
            human(
                format!("{}: {why}", r.written),
                Fix::AddRef {
                    reference: r.written.clone(),
                    name: None,
                },
            )
        })?
    };
    session
        .reporter
        .paths
        .reflexes
        .insert(local.clone(), path.clone());
    let manifest = parsed(session, input, &local, &text)?;
    let findings = lint(&manifest);
    Ok(Newcomer {
        name: local,
        written: r.written.clone(),
        location: Location::Local { path },
        manifest,
        findings,
        locked: None,
    })
}

/// One remote ref's reflexes, fetched at the tag it pins or the newest, kept in the store, read and linted.
fn fetched(
    session: &mut Session<'_>,
    input: &str,
    remotes: &mut git::Remotes,
    r: &Ref,
    reference: &Reference,
    pin: Option<Version>,
    name: Option<&LocalName>,
) -> Result<Vec<Newcomer>, Exit> {
    let tags = remotes.tags(reference).map_err(Exit::Failed)?;
    let tag = match pin {
        Some(pin) => tags
            .iter()
            .find(|tag| tag.version == pin)
            .ok_or_else(|| human(format!("{reference} has no tag {pin}"), Fix::Rerun))?,
        None => tags.last().ok_or_else(|| {
            human(
                format!(
                    "{reference} has no version tag; its author publishes one with git push --tags"
                ),
                Fix::Rerun,
            )
        })?,
    };
    let fetched = remotes.fetch(reference, tag).map_err(Exit::Failed)?;
    if fetched.reflexes.is_empty() {
        return Err(human(
            format!("no reflex.toml at {reference} {}", tag.version),
            Fix::Rerun,
        ));
    }
    let one = fetched.reflexes.len() == 1 && fetched.reflexes[0].dir == reference.dir;
    if !one && name.is_some() {
        return Err(human(
            format!("{reference} is a collection; --as names one reflex"),
            Fix::Rerun,
        ));
    }
    let mut found = Vec::new();
    for tree in fetched.reflexes {
        let reference = Reference {
            repo: reference.repo.clone(),
            dir: tree.dir.clone(),
        };
        let location = Location::Remote {
            reference: reference.clone(),
            pin,
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
        let manifest = parsed(session, input, &local, &text)?;
        let findings = lint(&manifest);
        found.push(Newcomer {
            locked: Some(Locked {
                reference,
                tag: tag.version,
                commit: fetched.commit.clone(),
                h1: kept.h1,
                effect: manifest.effect,
            }),
            name: local,
            written,
            location,
            manifest,
            findings,
        });
    }
    Ok(found)
}

/// A newcomer's manifest, or every line to fix: all but the last printed, the last the exit.
fn parsed(
    session: &Session<'_>,
    input: &str,
    name: &LocalName,
    text: &str,
) -> Result<Manifest, Exit> {
    manifest(Document {
        file: File::Manifest { name: name.clone() },
        text: Text::Toml(text),
    })
    .map_err(|problems| session.reporter.human(input, problems))
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
/// the adapter, a few at a time; nothing is asked when nothing is installed or no newcomer is active.
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
    let winners = {
        let _busy = terminal::busy("checking for thefts");
        threads::try_each(&examples, |case| {
            let request = request(&plan, case.utterance.text().as_str(), &[], Scope::Route)
                .map_err(Exit::Human)?;
            let raw = adapter
                .answer(&request, Deadline::after(plan.deadline()))
                .map_err(Exit::Adapter)?;
            let reading = read(&plan, &request, raw).map_err(Exit::Adapter)?;
            Ok(reading.winner.map(|winner| winner.reflex))
        })?
    };
    let routed: Vec<(Case, Option<LocalName>)> = examples.into_iter().zip(winners).collect();
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

/// The theft test ended in a fault: the install stands, and `evoke test` routes every example again.
fn unfinished_line(fault: &Fault) -> Diagnostic {
    Diagnostic {
        reflex: None,
        at: None,
        message: format!("the theft test did not finish: {fault}"),
        fix: Fix::Test,
    }
}
