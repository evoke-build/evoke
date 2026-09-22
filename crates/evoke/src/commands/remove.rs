//! `evoke remove <name>`: the reflex's line gone from the project and its entry from the lock; your overlay,
//! vocabulary and settings stay, as does the store's copy; `evoke.d.ts` follows the set. In: the name, the
//! environment. Out: `Exit`, the reflex's line behind `-`.

use evoke_core::name::LocalName;
use evoke_core::{Fix, remove_entry};

use super::session::{self, Opening, Session};
use super::{Exit, about};
use crate::args::Command;
use crate::hosts::{Environment, terminal};
use crate::report::{self, Gutter};

pub fn run(command: &Command, name: &LocalName, environment: &Environment) -> Exit {
    let mut session = match session::open(command, false, environment, Opening::Installing) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.stand_in();
    let exit = removed(&mut session, &input, name);
    session.reporter.exit(&input, exit)
}

fn removed(session: &mut Session<'_>, input: &str, name: &LocalName) -> Exit {
    if !session.project.reflexes.contains_key(name) {
        return about(
            name,
            format!("{name} is not installed"),
            Fix::Show { reflex: None },
        );
    }
    let rows = session.rows([name]);
    if let Err(exit) = session.land(input, &remove_entry(name)) {
        return exit;
    }
    if let Some(mut lock) = session.lock.clone()
        && lock.reflexes.shift_remove(name).is_some()
        && let Err(exit) = session.write_lock(&lock)
    {
        return exit;
    }
    if let Err(exit) = session.reload(input).and_then(|()| session.write_types()) {
        return exit;
    }
    terminal::note(&report::rows(&rows, Gutter::Removed));
    Exit::Ran
}
