//! The System One wire as a pure mapping, behind two doors. In: a `Door` and its `[adapters.<name>]` table; a
//! `Request`; a response's status and body. Out: `Settings` or the problems to fix; the request body; `Raw`
//! answers or a `Fault`. Jev is the model behind both doors: `jev` posts to TypeSafe AI's own address under its
//! key, `openjev` to OpenJEV, an independent service that forwards requests to Jev, under a key of its own; one
//! body and one reading serve both.

use std::fmt;

use evoke_core::adapter::{
    Declared, Fault, Gate, Limits, Prob, Question, QuestionId, Raw, Request,
};
use evoke_core::diagnostic::{Diagnostic, Fix};
use evoke_core::document::Json;
use evoke_core::manifest::Range;
use evoke_core::name::{AdapterId, VarName};
use evoke_core::plan::Millis;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A door on the wire: which address, under which key, naming which model. On the wire, its adapter name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Door {
    /// TypeSafe AI's own API.
    Jev,
    /// OpenJEV, an independent service that forwards requests to Jev.
    OpenJev,
}

/// What tells one door from the other; the mapping behind them is one.
struct Constants {
    /// The adapter's name, as a project names it and as a problem's key path begins.
    name: &'static str,
    /// The model the body names, which is the adapter's id: it changes whenever answers could. OpenJEV's is an
    /// alias that follows the latest Jev, so behind that door answers may move under one id.
    model: &'static str,
    /// The endpoint both hosts post to.
    url: &'static str,
    /// The variable both hosts read the key from.
    credential: &'static str,
    /// Where a key comes from, as the line that asks for one says it.
    issuer: &'static str,
    /// The wait after connect. OpenJEV forwards each request over two more hops, from a function that may be
    /// cold, so its door waits twice as long; the decision's deadline bounds both.
    timeout: Millis,
}

const JEV: Constants = Constants {
    name: "jev",
    model: "jev-1.13.0",
    url: "https://api.typesafe.ai/v1/systemone",
    credential: "TYPESAFE_API_KEY",
    issuer: "typesafe.ai",
    timeout: Millis(1_500),
};

const OPENJEV: Constants = Constants {
    name: "openjev",
    model: "openjev",
    url: "https://api.openjev.sh/v1/systemone",
    credential: "OPENJEV_API_KEY",
    issuer: "openjev.sh",
    timeout: Millis(3_000),
};

impl Constants {
    /// The model as the adapter's id.
    fn id(&self) -> AdapterId {
        AdapterId::new(self.model).expect("the model name is an id")
    }

    fn credential(&self) -> VarName {
        VarName::new(self.credential).expect("the credential is a variable name")
    }
}

impl Door {
    const fn constants(self) -> &'static Constants {
        match self {
            Self::Jev => &JEV,
            Self::OpenJev => &OPENJEV,
        }
    }

    /// The adapter's name, as a project names it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.constants().name
    }

    /// The door a project's adapter name opens, if it names one.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        [Self::Jev, Self::OpenJev]
            .into_iter()
            .find(|door| door.name() == name)
    }
}

impl fmt::Display for Door {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The ceiling per `choice`, the same at both doors.
const OPTIONS: u32 = 255;

/// The gate's four numbers, each a probability.
#[derive(Clone, Copy)]
struct Floors {
    route: Prob,
    fits: Prob,
    read: Prob,
    write: Prob,
}

/// The floors both doors ship — `route`, `fits` for the runner-up, `read`, `write` — Jev's own calibration, which
/// OpenJEV forwards to and never rescales.
const DEFAULTS: Floors = Floors {
    route: floor(0.5),
    fits: floor(0.3),
    read: floor(0.6),
    write: floor(0.8),
};

/// A literal floor; the compiler evaluates it, so a literal outside `[0, 1]` fails the build.
const fn floor(p: f64) -> Prob {
    match Prob::new(p) {
        Some(p) => p,
        None => panic!("a floor is a probability"),
    }
}

/// What both hosts read before the first call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub declared: Declared,
    pub credential: VarName,
    /// Where a key comes from, as the line that asks for one says it.
    pub issuer: String,
    pub url: String,
    pub policy: Policy,
}

/// The transport policy as a value, executed by each host: the wait after connect, how many times a connect error
/// or a retried status is tried again, and which statuses those are.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    pub timeout: Millis,
    pub retries: u32,
    pub retry_statuses: Range<u16>,
}

impl Policy {
    /// Whether a response's status is tried again.
    #[must_use]
    pub fn retried(&self, status: u16) -> bool {
        (self.retry_statuses.min()..=self.retry_statuses.max()).contains(&status)
    }
}

/// The door's effective settings: its constants and the defaults, with the table's gate overrides; every problem
/// names its key under the door's name.
pub fn settings(door: Door, table: Option<&Json>) -> Result<Settings, Vec<Diagnostic>> {
    let constants = door.constants();
    let mut floors = DEFAULTS;
    let mut errors: Vec<Diagnostic> = table
        .map(|table| overrides(constants.name, table, &mut floors))
        .unwrap_or_default()
        .into_iter()
        .map(problem)
        .collect();
    let gate = Gate::new(floors.route, Some(floors.fits), floors.read, floors.write)
        .map_err(|why| errors.push(problem(format!("adapters.{}.gate: {why}", constants.name))))
        .ok();
    match gate {
        Some(gate) if errors.is_empty() => Ok(Settings {
            declared: Declared {
                id: constants.id(),
                limits: Some(Limits {
                    options: Some(OPTIONS),
                    tokens: None,
                }),
                gate: Some(gate),
            },
            credential: constants.credential(),
            issuer: constants.issuer.to_owned(),
            url: constants.url.to_owned(),
            policy: policy(constants.timeout),
        }),
        _ => Err(errors),
    }
}

/// The table is validated only once selected, so `evoke check` is where a problem shows.
fn problem(message: String) -> Diagnostic {
    Diagnostic {
        reflex: None,
        at: None,
        message,
        fix: Fix::Check,
    }
}

/// The table's overrides: `gate`, each of its keys a probability; every problem, naming its key.
fn overrides(name: &str, table: &Json, floors: &mut Floors) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(table) = table.as_object() else {
        return vec![format!("adapters.{name} must be a table")];
    };
    for (key, value) in table {
        if key != "gate" {
            problems.push(format!(
                "adapters.{name}.{key} is unknown; the keys are gate"
            ));
            continue;
        }
        let Some(gate) = value.as_object() else {
            problems.push(format!("adapters.{name}.gate must be a table"));
            continue;
        };
        for (key, value) in gate {
            let floor = match key.as_str() {
                "route" => &mut floors.route,
                "fits" => &mut floors.fits,
                "read" => &mut floors.read,
                "write" => &mut floors.write,
                _ => {
                    problems.push(format!(
                        "adapters.{name}.gate.{key} is unknown; the keys are route, fits, read and write"
                    ));
                    continue;
                }
            };
            match value.as_f64().and_then(Prob::new) {
                Some(p) => *floor = p,
                None => problems.push(format!(
                    "adapters.{name}.gate.{key} must be a probability, 0 to 1"
                )),
            }
        }
    }
    problems
}

/// The door's wait after connect; one retry on connect errors and server errors, never on a client error.
fn policy(timeout: Millis) -> Policy {
    Policy {
        timeout,
        retries: 1,
        retry_statuses: Range::new(500, 599).expect("500 is below 599"),
    }
}

/// The body of `POST /v1/systemone`: the door's model, the state, and each question as System One's `choice` or
/// `noul`.
#[must_use]
pub fn request(door: Door, request: &Request) -> Json {
    let questions: serde_json::Map<String, Json> = request
        .questions
        .iter()
        .map(|(id, question)| {
            let question = match question {
                Question::Choice(choice) => json!({
                    "type": "choice",
                    "instructions": choice.ask(),
                    "criteria": choice.options(),
                }),
                Question::YesNo { ask, yes, no } => json!({
                    "type": "noul",
                    "instructions": ask,
                    "criteria": { "true": yes, "false": no },
                }),
            };
            (id.to_string(), question)
        })
        .collect();
    json!({
        "model": door.constants().model,
        "state": { "request": request.state.request },
        "questions": questions,
    })
}

/// A response as `Raw` answers, the same at both doors: a `choice` as given, scaled back to 1 only when two-decimal
/// rounding left its sum a hair off; a `noul` as `{ "yes": p }`. A 401 is the key refused, named by the variable
/// that holds it; any other status is a fault of its own; a body that is not System One's is a transport fault; an
/// answer of the wrong shape is malformed for its question; a sum off by more than rounding reaches the core as
/// given, for `read` to refuse.
pub fn answers(status: u16, body: &str, credential: &VarName) -> Result<Raw, Fault> {
    if status == 401 {
        return Err(Fault::Refused {
            credential: credential.clone(),
        });
    }
    if status != 200 {
        return Err(Fault::Status { status });
    }
    let transport = |message: String| Fault::Transport { message };
    let body: Json = serde_json::from_str(body)
        .map_err(|error| transport(format!("the response is not JSON: {error}")))?;
    let answers = body
        .get("answers")
        .and_then(Json::as_object)
        .ok_or_else(|| transport("the response has no answers".to_owned()))?;
    let mut raw = IndexMap::new();
    for (id, answer) in answers {
        let question = QuestionId::parse(id).map_err(transport)?;
        let malformed = |message: String| Fault::Malformed {
            question: question.clone(),
            message,
        };
        let kind = answer
            .get("type")
            .and_then(Json::as_str)
            .unwrap_or_default();
        let probabilities = match kind {
            "choice" => {
                let given = answer
                    .get("probabilities")
                    .and_then(Json::as_object)
                    .ok_or_else(|| malformed("the choice has no probabilities".to_owned()))?;
                let mut probabilities: IndexMap<String, f64> = given
                    .iter()
                    .map(|(key, p)| p.as_f64().map(|p| (key.clone(), p)))
                    .collect::<Option<_>>()
                    .ok_or_else(|| malformed("a probability is not a number".to_owned()))?;
                // Jev prints two decimals, so a sum drifts by half a unit in the last place per option at most;
                // that much is normalized, bounded at 0.05, and a wider gap is the engine's, never hidden. The
                // comparison allows for the doubles' own error, so 0.51 + 0.50 is within 0.01.
                let sum: f64 = probabilities.values().sum();
                let rounding = u32::try_from(probabilities.len())
                    .map_or(0.05, |n| (0.005 * f64::from(n)).min(0.05));
                if sum.is_finite() && sum > 0.0 && (sum - 1.0).abs() <= rounding + 1e-9 {
                    for p in probabilities.values_mut() {
                        *p /= sum;
                    }
                }
                probabilities
            }
            "noul" => {
                let p = answer
                    .get("noul")
                    .and_then(Json::as_f64)
                    .ok_or_else(|| malformed("the noul has no probability".to_owned()))?;
                IndexMap::from([("yes".to_owned(), p)])
            }
            other => return Err(malformed(format!("the answer has type \"{other}\""))),
        };
        raw.insert(id.clone(), probabilities);
    }
    Ok(Raw(raw))
}

#[cfg(test)]
// The numbers under test are literals, compared exactly.
#[expect(clippy::float_cmp)]
mod tests {
    use evoke_core::diagnostic::File;
    use evoke_core::document::Text;
    use evoke_core::{Document, Project, project};

    use super::*;

    /// The project's `[adapters.<name>]` table as `evoke.toml` carries it.
    fn table(name: &str, toml: &str) -> Json {
        let text = format!("adapter = \"{name}\"\n\n{toml}");
        let project: Project = project(Document {
            file: File::Project,
            text: Text::Toml(&text),
        })
        .unwrap();
        project.adapters[name].clone()
    }

    fn gate_of(settings: &Settings) -> [f64; 4] {
        let gate = settings.declared.gate.unwrap();
        [
            gate.route().get(),
            gate.fits().unwrap().get(),
            gate.read().get(),
            gate.write().get(),
        ]
    }

    #[test]
    fn defaults_hold_without_a_table() {
        let settings = settings(Door::Jev, None).unwrap();
        assert_eq!(settings.declared.id.as_str(), "jev-1.13.0");
        assert_eq!(settings.declared.limits.unwrap().options, Some(255));
        assert_eq!(settings.declared.limits.unwrap().tokens, None);
        assert_eq!(gate_of(&settings), [0.5, 0.3, 0.6, 0.8]);
        assert_eq!(settings.credential.as_str(), "TYPESAFE_API_KEY");
        assert_eq!(settings.issuer, "typesafe.ai");
        assert_eq!(settings.url, "https://api.typesafe.ai/v1/systemone");
        assert_eq!(settings.policy.timeout, Millis(1_500));
        assert_eq!(settings.policy.retries, 1);
        assert!(settings.policy.retried(529));
        assert!(!settings.policy.retried(429));
        assert!(!settings.policy.retried(401));
        assert_eq!(
            serde_json::to_value(settings.policy).unwrap(),
            json!({ "timeout": 1500, "retries": 1, "retry_statuses": [500, 599] })
        );
    }

    #[test]
    fn openjev_is_a_second_door_on_the_same_wire() {
        let jev = settings(Door::Jev, None).unwrap();
        let openjev = settings(Door::OpenJev, None).unwrap();
        assert_eq!(openjev.declared.id.as_str(), "openjev");
        assert_eq!(openjev.declared.limits, jev.declared.limits);
        assert_eq!(openjev.declared.gate, jev.declared.gate);
        assert_eq!(openjev.credential.as_str(), "OPENJEV_API_KEY");
        assert_eq!(openjev.issuer, "openjev.sh");
        assert_eq!(openjev.url, "https://api.openjev.sh/v1/systemone");
        assert_eq!(openjev.policy.timeout, Millis(3_000));
        assert_eq!(openjev.policy.retries, jev.policy.retries);
        assert_eq!(openjev.policy.retry_statuses, jev.policy.retry_statuses);
        let tuned = settings(
            Door::OpenJev,
            Some(&table(
                "openjev",
                "[adapters.openjev]\ngate = { write = 0.9 }\n",
            )),
        )
        .unwrap();
        assert_eq!(gate_of(&tuned), [0.5, 0.3, 0.6, 0.9]);
        let problems: Vec<String> = settings(
            Door::OpenJev,
            Some(&table(
                "openjev",
                "[adapters.openjev]\ngate = { route = 2 }\n",
            )),
        )
        .unwrap_err()
        .into_iter()
        .map(|d| d.message)
        .collect();
        assert_eq!(
            problems,
            ["adapters.openjev.gate.route must be a probability, 0 to 1"]
        );
        let request: Request = serde_json::from_str(include_str!(
            "../../../spec/fixtures/request-kill-the-lights.json"
        ))
        .unwrap();
        let through_jev = super::request(Door::Jev, &request);
        let through_openjev = super::request(Door::OpenJev, &request);
        assert_eq!(through_openjev["model"], "openjev");
        assert_eq!(through_openjev["state"], through_jev["state"]);
        assert_eq!(through_openjev["questions"], through_jev["questions"]);
    }

    #[test]
    fn a_door_is_its_name_on_the_wire() {
        assert_eq!(serde_json::to_value(Door::Jev).unwrap(), "jev");
        assert_eq!(serde_json::to_value(Door::OpenJev).unwrap(), "openjev");
        assert_eq!(
            serde_json::from_value::<Door>(json!("openjev")).unwrap(),
            Door::OpenJev
        );
        assert!(serde_json::from_value::<Door>(json!("OpenJev")).is_err());
        assert_eq!(Door::named("jev"), Some(Door::Jev));
        assert_eq!(Door::named("openjev"), Some(Door::OpenJev));
        assert_eq!(Door::named("replay"), None);
        assert_eq!(Door::OpenJev.to_string(), "openjev");
    }

    #[test]
    fn the_table_overrides_the_gate() {
        let settings = settings(
            Door::Jev,
            Some(&table(
                "jev",
                "[adapters.jev]\ngate = { write = 0.85, fits = 0.4 }\n",
            )),
        )
        .unwrap();
        assert_eq!(gate_of(&settings), [0.5, 0.4, 0.6, 0.85]);
        let settings = settings_of("[adapters.jev]\n");
        assert_eq!(gate_of(&settings.unwrap()), [0.5, 0.3, 0.6, 0.8]);
    }

    fn settings_of(toml: &str) -> Result<Settings, Vec<Diagnostic>> {
        settings(Door::Jev, Some(&table("jev", toml)))
    }

    fn problems(toml: &str) -> Vec<String> {
        settings_of(toml)
            .unwrap_err()
            .into_iter()
            .map(|d| {
                assert_eq!(d.fix, Fix::Check);
                assert_eq!(d.at, None);
                d.message
            })
            .collect()
    }

    #[test]
    fn every_problem_names_its_key() {
        assert_eq!(
            problems("[adapters.jev]\ngate = { route = 1.5 }\n"),
            ["adapters.jev.gate.route must be a probability, 0 to 1"]
        );
        assert_eq!(
            problems("[adapters.jev]\ngate = { read = 0.9 }\n"),
            ["adapters.jev.gate: read 0.9 is above write 0.8"]
        );
        assert_eq!(
            problems("[adapters.jev]\ncolour = 1\ngate = { abstain = 0.5 }\n"),
            [
                "adapters.jev.colour is unknown; the keys are gate",
                "adapters.jev.gate.abstain is unknown; the keys are route, fits, read and write",
            ]
        );
        assert_eq!(
            settings(Door::Jev, Some(&json!({ "gate": { "write": "high" } })))
                .unwrap_err()
                .len(),
            1
        );
    }

    #[test]
    fn the_body_maps_the_request_near_identity() {
        let request: Request = serde_json::from_str(include_str!(
            "../../../spec/fixtures/request-kill-the-lights.json"
        ))
        .unwrap();
        let body = super::request(Door::Jev, &request);
        assert_eq!(body["model"], "jev-1.13.0");
        assert_eq!(body["state"], json!({ "request": "kill the lights" }));
        let questions = body["questions"].as_object().unwrap();
        let ids: Vec<&str> = questions.keys().map(String::as_str).collect();
        assert_eq!(
            ids,
            [
                "route",
                "fits.lights",
                "lights.room",
                "lights.state",
                "fits.timer"
            ]
        );
        let route = &questions["route"];
        assert_eq!(route["type"], "choice");
        assert_eq!(route["instructions"], "Which one does the request ask for?");
        assert_eq!(
            route["criteria"]["lights"]["not_for"],
            json!([
                "colour scenes and schedules",
                "asking whether a light is on"
            ])
        );
        assert_eq!(route["criteria"]["none"], "None of these.");
        assert_eq!(route.get("otherwise"), None);
        let fits = &questions["fits.lights"];
        assert_eq!(fits["type"], "noul");
        assert_eq!(
            fits["instructions"],
            "Does `lights` do the specific thing the request asks for?"
        );
        assert_eq!(
            fits["criteria"]["false"],
            "Something else, such as: colour scenes and schedules; asking whether a light is on."
        );
        assert_eq!(
            questions["lights.state"]["criteria"]["on"],
            json!({ "what": "Switch on.", "examples": ["turn on the kitchen lights"] })
        );
    }

    #[test]
    fn a_question_of_evokes_own_travels_like_any_other() {
        let mut request: Request = serde_json::from_str(include_str!(
            "../../../spec/fixtures/request-kill-the-lights.json"
        ))
        .unwrap();
        let clean = |text: &str| evoke_core::Clean::new(text).unwrap();
        request.questions.insert(
            QuestionId::parse("weave.split_0").unwrap(),
            Question::YesNo {
                ask: clean("At «and», does the request ask for two things?"),
                yes: evoke_core::adapter::Text::Plain(clean("Two things.")),
                no: evoke_core::adapter::Text::Plain(clean("One thing.")),
            },
        );
        let body = super::request(Door::Jev, &request);
        assert_eq!(body["questions"]["weave.split_0"]["type"], "noul");
        assert_eq!(
            body["questions"]["weave.split_0"]["criteria"]["true"],
            "Two things."
        );
        let raw = answers(
            200,
            r#"{"answers": {"weave.split_0": {"type": "noul", "noul": 0.73}}}"#,
        )
        .unwrap();
        assert_eq!(raw.0["weave.split_0"]["yes"], 0.73);
    }

    /// The shape Jev answers with, at either door: a choice with its probabilities, a noul with one number.
    const RESPONSE: &str = r#"{"model": "jev-1.13.0", "answers": {"fits.awake": {"type": "noul", "noul": 0.96}, "power.action": {"type": "choice", "choice": "unstated", "confidence": 0.94, "probabilities": {"sleep": 0.04, "shutdown": 0.0, "unstated": 0.96, "restart": 0.0}}, "route": {"type": "choice", "choice": "awake", "confidence": 0.99, "probabilities": {"awake": 0.99, "none": 0.01, "power": 0.0}}}, "usage": {"input_tokens": 5371}}"#;

    #[test]
    fn a_response_becomes_raw_answers() {
        let raw = answers(200, RESPONSE).unwrap();
        assert_eq!(raw.0["fits.awake"]["yes"], 0.96);
        assert_eq!(raw.0["route"]["awake"], 0.99);
        assert_eq!(raw.0["power.action"]["unstated"], 0.96);
        let rounded = answers(
            200,
            r#"{"answers": {"route": {"type": "choice", "probabilities": {"a": 0.33, "b": 0.33, "c": 0.33}}}}"#,
        )
        .unwrap();
        let third = rounded.0["route"]["a"];
        assert!((third - 1.0 / 3.0).abs() < 1e-12);
        assert!((rounded.0["route"].values().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_sum_at_the_rounding_boundary_is_normalized() {
        // 0.51 + 0.50 is 1.01 in print and a hair over in doubles: within half a unit per option either way.
        let answers = answers(
            200,
            r#"{"answers": {"route": {"type": "choice", "probabilities": {"a": 0.51, "b": 0.50}}}}"#,
        )
        .unwrap();
        let sum = answers.0["route"]["a"] + answers.0["route"]["b"];
        assert!((sum - 1.0).abs() < 1e-12, "{sum}");
    }

    #[test]
    fn a_sum_off_by_more_than_rounding_reaches_the_core_as_given() {
        let deflated = answers(
            200,
            r#"{"answers": {"route": {"type": "choice", "probabilities": {"a": 0.3}}}}"#,
        )
        .unwrap();
        assert_eq!(deflated.0["route"]["a"], 0.3);
        let inflated = answers(
            200,
            r#"{"answers": {"route": {"type": "choice", "probabilities": {"a": 3, "b": 1}}}}"#,
        )
        .unwrap();
        assert_eq!(inflated.0["route"]["a"], 3.0);
        assert_eq!(inflated.0["route"]["b"], 1.0);
    }

    /// The mapping with jev's credential, as every test but the refusals' calls it.
    fn answers(status: u16, body: &str) -> Result<Raw, Fault> {
        super::answers(status, body, &settings(Door::Jev, None).unwrap().credential)
    }

    #[test]
    fn a_refused_key_names_the_variable_that_holds_it() {
        for door in [Door::Jev, Door::OpenJev] {
            let credential = settings(door, None).unwrap().credential;
            assert_eq!(
                super::answers(401, "", &credential).unwrap_err(),
                Fault::Refused { credential }
            );
        }
    }

    #[test]
    fn a_bad_status_or_body_is_a_fault() {
        assert_eq!(answers(429, "").unwrap_err(), Fault::Status { status: 429 });
        assert!(matches!(
            answers(200, "not json").unwrap_err(),
            Fault::Transport { message } if message.starts_with("the response is not JSON")
        ));
        assert_eq!(
            answers(200, "{}").unwrap_err(),
            Fault::Transport {
                message: "the response has no answers".to_owned()
            }
        );
        assert_eq!(
            answers(
                200,
                r#"{"answers": {"route": {"type": "score", "score": 2}}}"#
            )
            .unwrap_err(),
            Fault::Malformed {
                question: QuestionId::Route,
                message: "the answer has type \"score\"".to_owned()
            }
        );
        assert!(matches!(
            answers(
                200,
                r#"{"answers": {"Route": {"type": "noul", "noul": 1}}}"#
            )
            .unwrap_err(),
            Fault::Transport { .. }
        ));
    }
}
