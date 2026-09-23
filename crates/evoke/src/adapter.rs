//! Who answers: the `Adapter` trait over the two built-ins, resolved by name. In: the project's adapter name and
//! its `[adapters.<name>]` table, the environment. Out: a boxed adapter, or every diagnostic in its way; a `Trace`
//! per call. `Jev` is the pure mapping plus the network and the policy loop; `Replay` is the recording
//! `EVOKE_ANSWERS` names plus its lookup. `declared` reads what an adapter declares without its credential, for
//! the commands that never ask.

use std::path::Path;

use evoke_adapters::jev::{self, Settings};
use evoke_adapters::replay::{self, Recording};
use evoke_core::name::{AdapterId, AdapterName, VarName};
use evoke_core::{Declared, Diagnostic, Digest, Fault, Fix, Json, Raw, Request};
use serde::{Deserialize, Serialize};

use crate::hosts::network::{self, Agent};
use crate::hosts::{Deadline, Environment, files};

/// The variable naming the recording `replay` answers from.
const ANSWERS: &str = "EVOKE_ANSWERS";

/// Shared by the threads of a batch — `test`, the thief test — so every built-in is `Sync`.
pub trait Adapter: Sync {
    /// Id, limits and gate.
    fn declared(&self) -> &Declared;

    /// Whether the answers stand for this plan, asked once before any call: a recording made against another plan
    /// is refused; an engine answers any plan.
    fn accepts(&self, plan: &Digest) -> Result<(), Diagnostic> {
        let _ = plan;
        Ok(())
    }

    /// Stateless; any subset of the plan's questions.
    fn answer(&self, request: &Request, deadline: Deadline) -> Result<Raw, Fault>;
}

/// One call as the host saw it: which adapter, how many questions, how long.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trace {
    pub adapter: AdapterId,
    pub questions: usize,
    pub ms: u64,
}

/// The adapter a project names, ready to answer: its settings validated, its credential or recording in hand.
pub fn resolve(
    name: &AdapterName,
    settings: Option<&Json>,
    environment: &Environment,
) -> Result<Box<dyn Adapter>, Vec<Diagnostic>> {
    match name.as_str() {
        "jev" => Jev::resolve(settings, environment).map(|jev| Box::new(jev) as Box<dyn Adapter>),
        "replay" => Replay::resolve(environment)
            .map(|replay| Box::new(replay) as Box<dyn Adapter>)
            .map_err(|problem| vec![problem]),
        other => Err(vec![unknown(other)]),
    }
}

/// What the project's adapter declares — id, limits, gate — without its credential in hand: what a plan is
/// compiled from when nothing is asked. `jev` declares from its settings; `replay` from its recording when
/// `EVOKE_ANSWERS` names one, else as a bare `replay`.
pub fn declared(
    name: &AdapterName,
    settings: Option<&Json>,
    environment: &Environment,
) -> Result<Declared, Vec<Diagnostic>> {
    match name.as_str() {
        "jev" => jev::settings(settings).map(|settings| settings.declared),
        "replay" if environment.get(ANSWERS).is_some() => Replay::resolve(environment)
            .map(|replay| replay.recording.declared)
            .map_err(|problem| vec![problem]),
        "replay" => Ok(Declared {
            id: AdapterId::new("replay").expect("replay is an id"),
            limits: None,
            gate: None,
        }),
        other => Err(vec![unknown(other)]),
    }
}

fn unknown(name: &str) -> Diagnostic {
    Diagnostic {
        reflex: None,
        at: None,
        message: format!("adapter \"{name}\" is unknown; the built-ins are jev and replay"),
        fix: Fix::Check,
    }
}

/// A credential variable that is not set: the export line for it.
fn unset(what: &str, var: VarName) -> Diagnostic {
    Diagnostic {
        reflex: None,
        at: None,
        message: format!("{what} needs {var}"),
        fix: Fix::ExportKey { var },
    }
}

struct Jev {
    settings: Settings,
    key: String,
    agent: Agent,
}

impl Jev {
    fn resolve(table: Option<&Json>, environment: &Environment) -> Result<Self, Vec<Diagnostic>> {
        let settings = jev::settings(table)?;
        let key = environment
            .get(settings.credential.as_str())
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                let mut needs = unset("jev", settings.credential.clone());
                needs.message.push_str(", a key from typesafe.ai");
                vec![needs]
            })?
            .to_owned();
        Ok(Self {
            settings,
            key,
            agent: Agent::new(),
        })
    }
}

impl Adapter for Jev {
    fn declared(&self) -> &Declared {
        &self.settings.declared
    }

    /// The policy loop: once more after a connect error or a retried status, never after a client error.
    fn answer(&self, request: &Request, deadline: Deadline) -> Result<Raw, Fault> {
        let body = jev::request(request).to_string();
        let policy = &self.settings.policy;
        let mut attempt = 0;
        loop {
            attempt += 1;
            let again = attempt <= policy.retries;
            let sent = network::post(
                &self.agent,
                &self.settings.url,
                &self.key,
                &body,
                deadline,
                policy.timeout,
            );
            match sent {
                Err(transport) if again && !transport.connected => {}
                Err(transport) => return Err(transport.into()),
                Ok(response) if again && policy.retried(response.status) => {}
                Ok(response) => {
                    return jev::answers(
                        response.status,
                        &response.body,
                        &self.settings.credential,
                    );
                }
            }
        }
    }
}

struct Replay {
    recording: Recording,
}

impl Replay {
    fn resolve(environment: &Environment) -> Result<Self, Diagnostic> {
        let path = environment
            .get(ANSWERS)
            .ok_or_else(|| unset("replay", answers()))?;
        let text = match files::read(Path::new(path)) {
            Ok(Some(text)) => Ok(text),
            Ok(None) => Err("there is no such file".to_owned()),
            Err(failure) => Err(failure.cause.unwrap_or(failure.what)),
        };
        let recording = text
            .and_then(|text| replay::recording(&text))
            .map_err(|why| Diagnostic {
                reflex: None,
                at: None,
                message: format!("{ANSWERS} names {path}: {why}"),
                fix: Fix::ExportKey { var: answers() },
            })?;
        Ok(Self { recording })
    }
}

impl Adapter for Replay {
    fn declared(&self) -> &Declared {
        &self.recording.declared
    }

    fn accepts(&self, plan: &Digest) -> Result<(), Diagnostic> {
        match self.recording.plan {
            Some(recorded) if recorded != *plan => Err(Diagnostic {
                reflex: None,
                at: None,
                message: format!("{ANSWERS} was recorded against plan {recorded}, not {plan}"),
                fix: Fix::ExportKey { var: answers() },
            }),
            _ => Ok(()),
        }
    }

    fn answer(&self, request: &Request, _deadline: Deadline) -> Result<Raw, Fault> {
        replay::answer(&self.recording, request)
    }
}

fn answers() -> VarName {
    VarName::new(ANSWERS).expect("EVOKE_ANSWERS is a variable name")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recording_accepts_only_the_plan_it_was_made_against() {
        let digest = |hex: &str| -> Digest {
            serde_json::from_value(Json::String(format!("h1:{}", hex.repeat(64)))).unwrap()
        };
        let any = Replay {
            recording: replay::recording("id = \"replay\"\n[answers]\n").unwrap(),
        };
        assert_eq!(any.accepts(&digest("a")), Ok(()));
        let made = Replay {
            recording: replay::recording(&format!(
                "id = \"replay\"\nplan = \"{}\"\n[answers]\n",
                digest("a")
            ))
            .unwrap(),
        };
        assert_eq!(made.accepts(&digest("a")), Ok(()));
        let refused = made.accepts(&digest("b")).unwrap_err();
        assert!(
            refused
                .message
                .starts_with("EVOKE_ANSWERS was recorded against plan h1:aaaa")
        );
        assert_eq!(refused.fix, Fix::ExportKey { var: answers() });
    }

    #[test]
    fn an_unknown_adapter_fixes_with_check() {
        let environment = Environment::of_process();
        let name = AdapterName::new("gpt").unwrap();
        let problems = resolve(&name, None, &environment).err().unwrap();
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].fix, Fix::Check);
        assert_eq!(declared(&name, None, &environment).unwrap_err(), problems);
    }

    #[test]
    fn jev_without_its_key_says_where_one_comes_from() {
        let environment = Environment(std::collections::BTreeMap::new());
        let problems = resolve(&AdapterName::new("jev").unwrap(), None, &environment)
            .err()
            .unwrap();
        assert_eq!(
            problems[0].message,
            "jev needs TYPESAFE_API_KEY, a key from typesafe.ai"
        );
        assert_eq!(
            problems[0].fix,
            Fix::ExportKey {
                var: VarName::new("TYPESAFE_API_KEY").unwrap()
            }
        );
    }

    #[test]
    fn a_declaration_needs_no_credential() {
        let environment = Environment(std::collections::BTreeMap::new());
        let jev = declared(&AdapterName::new("jev").unwrap(), None, &environment).unwrap();
        assert!(jev.gate.is_some());
        let replay = declared(&AdapterName::new("replay").unwrap(), None, &environment).unwrap();
        assert_eq!(replay.id.as_str(), "replay");
        assert!(replay.gate.is_none() && replay.limits.is_none());
    }
}
