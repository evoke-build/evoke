//! `evoke config <name> <key> <value> | --env VAR`: one setting of one reflex. In: the reflex, the key, the setting,
//! the environment. Out: `Exit`. The key must be one the manifest declares, and a secret is set only from a
//! variable; the setting lands under `[config.<name>]` in `evoke.toml` when the file still reads. A path the
//! declaration reads or writes that is not there yet is said, since the body will not start without it.

use std::path::Path;

use evoke_core::name::{ConfigKey, LocalName};
use evoke_core::needs::{self, Entry};
use evoke_core::project::Setting;
use evoke_core::{Diagnostic, Fix, set_config};

use super::Exit;
use super::session::{self, Opening, Session};
use crate::args::Command;
use crate::hosts::{Environment, terminal};
use crate::report;

pub fn run(
    command: &Command,
    reflex: &LocalName,
    key: &ConfigKey,
    setting: &Setting,
    environment: &Environment,
) -> Exit {
    let mut session = match session::open(command, false, environment, Opening::Tuning) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.stand_in();
    let exit = set(&mut session, &input, reflex, key, setting);
    session.reporter.exit(&input, exit)
}

fn set(
    session: &mut Session<'_>,
    input: &str,
    reflex: &LocalName,
    key: &ConfigKey,
    setting: &Setting,
) -> Exit {
    let Some(item) = session.installed.reflexes.get(reflex) else {
        return Exit::Human(Diagnostic {
            reflex: Some(reflex.clone()),
            at: None,
            message: "is not installed".to_owned(),
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
                message: "is not in the store".to_owned(),
                fix: Fix::Sync,
            });
        }
    };
    let Some(spec) = manifest.config.get(key) else {
        return Exit::Human(Diagnostic {
            reflex: Some(reflex.clone()),
            at: None,
            message: format!("declares no config \"{key}\""),
            fix: Fix::Show {
                reflex: Some(reflex.clone()),
            },
        });
    };
    let declared = manifest
        .needs
        .reads
        .iter()
        .chain(&manifest.needs.writes)
        .any(|entry| matches!(entry, Entry::Value(name) if name.as_str() == key.as_str()));
    match set_config(reflex.clone(), key.clone(), setting.clone(), spec) {
        Ok(edit) => match session.apply(input, &edit) {
            Ok(_) => {
                if let Setting::Plain { value } = setting
                    && declared
                    && let Some(shown) = not_there(value, session.environment())
                {
                    terminal::note(&report::not_there_yet(reflex, &shown));
                }
                Exit::Ran
            }
            Err(problem) => problem,
        },
        Err(refused) => Exit::Human(refused),
    }
}

/// A path a declaration reads or writes, set to somewhere that is not there yet: the path as shown. A value that
/// is no path is the run's own refusal, not this line's.
fn not_there(value: &str, environment: &Environment) -> Option<String> {
    let home = environment.get("HOME")?;
    let path = match value.strip_prefix("~/") {
        Some(rest) => format!("{home}/{rest}"),
        None if value.starts_with('/') => value.to_owned(),
        None => return None,
    };
    (!Path::new(&path).exists()).then(|| needs::shown(&path, home))
}
