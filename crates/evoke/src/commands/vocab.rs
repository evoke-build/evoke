//! `evoke vocab <name> [add <word> "<meaning>" [--value v] | remove <word>]`: your words. In: the vocabulary's
//! name, what changes or nothing, the environment. Out: `Exit`. Nothing to change lists the words, on stdout; a
//! change lands in `vocab/<name>.toml` when the file still reads. Adding a word that is there replaces its meaning; removing
//! one that is not is refused.

use evoke_core::name::VocabName;
use evoke_core::{Diagnostic, Fix, VocabChange, vocab_edit};

use super::Exit;
use super::session::{self, Opening, Session};
use crate::args::Command;
use crate::hosts::{Environment, terminal};
use crate::report;

pub fn run(
    command: &Command,
    name: &VocabName,
    change: Option<&VocabChange>,
    environment: &Environment,
) -> Exit {
    let mut session = match session::open(command, false, environment, Opening::Tuning) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.stand_in();
    let exit = changed(&mut session, &input, name, change);
    session.reporter.exit(&input, exit)
}

fn changed(
    session: &mut Session<'_>,
    input: &str,
    name: &VocabName,
    change: Option<&VocabChange>,
) -> Exit {
    let words = session.installed.vocab.get(name);
    match change {
        None => match words.filter(|words| !words.is_empty()) {
            Some(words) => {
                terminal::answer(&report::vocabulary(words));
                Exit::Ran
            }
            None => Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message: format!("vocabulary \"{name}\" is empty"),
                fix: Fix::VocabAdd {
                    vocab: name.clone(),
                },
            }),
        },
        Some(VocabChange::Remove { word })
            if !words.is_some_and(|words| words.contains_key(word)) =>
        {
            Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message: format!("\"{word}\" is not in vocabulary \"{name}\""),
                fix: Fix::Rerun,
            })
        }
        Some(change) => match session.apply(input, &vocab_edit(name.clone(), change.clone())) {
            Ok(_) => Exit::Ran,
            Err(exit) => exit,
        },
    }
}
