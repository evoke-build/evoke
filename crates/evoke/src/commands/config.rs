//! `evoke config <name> <key> <value> | --env VAR`: one setting of one reflex. In: the reflex, the key, the setting,
//! the environment. Out: `Exit`. The key must be one the manifest declares, and a secret is set only from a
//! variable; the setting lands under `[config.<name>]` in `evoke.toml` when the file still reads.

use evoke_core::name::{ConfigKey, LocalName};
use evoke_core::project::Setting;
use evoke_core::{Diagnostic, Fix, set_config};

use super::Exit;
use super::session::{self, Opening, Session};
use crate::args::Command;
use crate::hosts::Environment;

pub fn run(
    command: &Command,
    reflex: &LocalName,
    key: &ConfigKey,
    setting: &Setting,
    environment: &Environment,
) -> Exit {
    let session = match session::open(command, false, environment, Opening::Tuning) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.stand_in();
    let exit = set(&session, &input, reflex, key, setting);
    session.reporter.exit(&input, exit)
}

fn set(
    session: &Session<'_>,
    input: &str,
    reflex: &LocalName,
    key: &ConfigKey,
    setting: &Setting,
) -> Exit {
    let Some(item) = session.installed.reflexes.get(reflex) else {
        return Exit::Human(Diagnostic {
            reflex: Some(reflex.clone()),
            at: None,
            message: format!("{reflex} is not installed"),
            fix: Fix::Show { reflex: None },
        });
    };
    let manifest = match (&item.wording, session.shipped.get(reflex)) {
        (Err(problems), _) => return session.reporter.human(input, problems.clone()),
        (Ok(_), Some((manifest, _))) => manifest,
        (Ok(_), None) => {
            return Exit::Human(Diagnostic {
                reflex: Some(reflex.clone()),
                at: None,
                message: format!("{reflex} is not in the store"),
                fix: Fix::Sync,
            });
        }
    };
    let Some(spec) = manifest.config.get(key) else {
        return Exit::Human(Diagnostic {
            reflex: Some(reflex.clone()),
            at: None,
            message: format!("{reflex} declares no config \"{key}\""),
            fix: Fix::Show {
                reflex: Some(reflex.clone()),
            },
        });
    };
    match set_config(reflex.clone(), key.clone(), setting.clone(), spec) {
        Ok(edit) => match session.apply(input, &edit) {
            Ok(_) => Exit::Ran,
            Err(problem) => problem,
        },
        Err(refused) => Exit::Human(refused),
    }
}
