//! `evoke sync`: the lock realised on this machine — every remote reflex the store lacks fetched at its locked tag,
//! refused unless the tag still names the locked commit and the tree still hashes to the lock — and the runtime
//! recorded. Nothing in the project changes. In: the environment. Out: `Exit`, a `+` line per reflex realised.

use evoke_core::Fix;
use evoke_core::name::LocalName;
use evoke_core::project::Location;

use super::session::{self, Opening, Session};
use super::{Exit, about};
use crate::args::Command;
use crate::hosts::git;
use crate::hosts::{Environment, terminal};
use crate::report::{self, Gutter};

pub fn run(command: &Command, environment: &Environment) -> Exit {
    let mut session = match session::open(command, false, environment, Opening::Installing) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.placeholder();
    let exit = synced(&mut session, &input);
    session.reporter.exit(&input, exit)
}

fn synced(session: &mut Session<'_>, input: &str) -> Exit {
    let mut realised = Vec::new();
    let remotes: Vec<(LocalName, Location)> = session
        .project
        .reflexes
        .iter()
        .filter(|(_, location)| matches!(location, Location::Remote { .. }))
        .map(|(name, location)| (name.clone(), location.clone()))
        .collect();
    for (name, location) in remotes {
        match realise(session, &name, &location) {
            Ok(true) => realised.push(name),
            Ok(false) => {}
            Err(exit) => return exit,
        }
    }
    if let Err(exit) = session.reload(input) {
        return exit;
    }
    let runtime = match session.record_runtime() {
        Ok(runtime) => runtime,
        Err(exit) => return exit,
    };
    if !realised.is_empty() {
        terminal::note(&report::rows(&session.rows(realised.iter()), Gutter::Added));
    }
    runtime.map_or(Exit::Ran, Exit::Human)
}

/// One remote reflex placed in the store from its locked tag; `false` when it was there already.
fn realise(session: &Session<'_>, name: &LocalName, location: &Location) -> Result<bool, Exit> {
    let Location::Remote { reference, .. } = location else {
        return Ok(false);
    };
    let Some(locked) = session
        .lock
        .as_ref()
        .and_then(|lock| lock.reflexes.get(name))
    else {
        return Err(about(
            name,
            format!("{name} is not locked"),
            Fix::AddRef {
                reference: location.to_string(),
                name: Some(name.clone()),
            },
        ));
    };
    if session
        .store
        .entry(&locked.h1)
        .map_err(Exit::Failed)?
        .is_some()
    {
        return Ok(false);
    }
    let moved = |what: &str| {
        about(
            name,
            format!("{reference} {} {what}", locked.tag),
            Fix::Update {
                reflex: Some(name.clone()),
            },
        )
    };
    let tags = git::tags(reference).map_err(Exit::Failed)?;
    let Some(tag) = tags.iter().find(|tag| tag.version == locked.tag) else {
        return Err(moved("is gone"));
    };
    let fetched = git::fetch(reference, tag).map_err(Exit::Failed)?;
    if fetched.commit != locked.commit {
        return Err(moved("no longer names the locked commit"));
    }
    let Some(tree) = fetched
        .reflexes
        .into_iter()
        .find(|tree| tree.dir == reference.dir)
    else {
        return Err(moved("has no reflex.toml"));
    };
    let kept = session.store.keep(tree.files).map_err(Exit::Failed)?;
    if kept.h1 != locked.h1 {
        return Err(moved("does not hash to the lock"));
    }
    Ok(true)
}
