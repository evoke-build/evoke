//! The System One wire as a pure mapping, behind four doors. In: a `Door` and its `[adapters.<name>]` table; the
//! account a door's address names; a `Request`; a response's status and body. Out: `Settings` or the problems to
//! fix; the address; the request's bodies; `Raw` answers or a `Fault`. Jev is the model behind two doors, named by
//! version in every body: `jev` posts to TypeSafe AI's own address under its key, `openjev` to OpenJEV, an
//! independent service that forwards the request to Jev, under a key of its own. Clef and Clef-flash, Cloudflare's
//! models on the same wire, answer behind the two others: `clef` and `clef_flash` post to Workers AI under a token,
//! at the account the token belongs to. One body and one reading serve all four.

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
#[serde(rename_all = "snake_case")]
pub enum Door {
    /// TypeSafe AI's own API.
    Jev,
    /// OpenJEV, an independent service that forwards requests to Jev.
    #[serde(rename = "openjev")]
    OpenJev,
    /// Clef, on Cloudflare's Workers AI.
    Clef,
    /// Clef-flash, Clef's smaller model, on Cloudflare's Workers AI.
    ClefFlash,
}

/// What tells one door from another; the mapping behind them is one.
struct Constants {
    /// The adapter's name, as a project names it and as a problem's key path begins.
    name: &'static str,
    /// The model every body names.
    model: &'static str,
    /// The adapter's id: what answers behind the door, which changes whenever answers could.
    id: &'static str,
    /// The endpoint both hosts post to; at a door whose address names an account, the address up to its id.
    url: &'static str,
    /// The account the address names, at a door whose service keeps its models per account.
    account: Option<Account>,
    /// The variable both hosts read the key from.
    credential: &'static str,
    /// Where a key comes from, as the line that asks for one says it.
    issuer: &'static str,
    /// The wait after connect; the decision's deadline bounds it at every door.
    timeout: Millis,
    /// The most questions one body carries, where the service sets a ceiling.
    questions: Option<usize>,
    /// The floors the door ships.
    floors: Floors,
    /// Whether an answer arrives in the service's own envelope, under `result`.
    enveloped: bool,
}

/// An account in a door's address: the variable both hosts read its id from, and the address after it.
#[derive(Clone, Copy)]
struct Account {
    var: &'static str,
    path: &'static str,
}

impl Account {
    fn var(self) -> VarName {
        VarName::new(self.var).expect("the account is a variable name")
    }
}

/// Jev by version, the adapter's id at both of its doors. OpenJEV forwards the name it is given, so the door
/// pins it rather than take the service's alias, which follows the latest Jev.
const JEV_MODEL: &str = "jev-1.13.0";

const JEV: Constants = Constants {
    name: "jev",
    model: JEV_MODEL,
    id: JEV_MODEL,
    url: "https://api.typesafe.ai/v1/systemone",
    account: None,
    credential: "TYPESAFE_API_KEY",
    issuer: "typesafe.ai",
    timeout: Millis(1_500),
    questions: None,
    floors: JEVS,
    enveloped: false,
};

/// OpenJEV forwards each request over two more hops, from a function that may be cold, so its door waits twice
/// as long as `jev`'s.
const OPENJEV: Constants = Constants {
    name: "openjev",
    model: JEV_MODEL,
    id: JEV_MODEL,
    url: "https://api.openjev.sh/v1/systemone",
    account: None,
    credential: "OPENJEV_API_KEY",
    issuer: "openjev.sh",
    timeout: Millis(3_000),
    questions: None,
    floors: JEVS,
    enveloped: false,
};

/// Workers AI names no version of a model, so each of its doors' ids carries the day its answers were pinned.
const CLEF: Constants = Constants {
    name: "clef",
    model: "clef",
    id: "clef-2026-10-03",
    url: WORKERS_AI,
    account: Some(Account {
        var: ACCOUNT,
        path: "/ai/run/@cf/cloudflare/clef",
    }),
    credential: "CLOUDFLARE_API_TOKEN",
    issuer: "dash.cloudflare.com",
    timeout: Millis(6_000),
    questions: Some(64),
    floors: Floors::NONE,
    enveloped: true,
};

const CLEF_FLASH: Constants = Constants {
    name: "clef_flash",
    model: "clef-flash",
    id: "clef-flash-2026-10-03",
    url: WORKERS_AI,
    account: Some(Account {
        var: ACCOUNT,
        path: "/ai/run/@cf/cloudflare/clef-flash",
    }),
    credential: "CLOUDFLARE_API_TOKEN",
    issuer: "dash.cloudflare.com",
    timeout: Millis(3_000),
    questions: Some(64),
    floors: Floors::NONE,
    enveloped: true,
};

/// Workers AI's address, up to the account's id.
const WORKERS_AI: &str = "https://api.cloudflare.com/client/v4/accounts/";

/// The variable that holds the account's id at Cloudflare's doors.
const ACCOUNT: &str = "CLOUDFLARE_ACCOUNT_ID";

impl Constants {
    fn id(&self) -> AdapterId {
        AdapterId::new(self.id).expect("the id is not empty")
    }

    fn credential(&self) -> VarName {
        VarName::new(self.credential).expect("the credential is a variable name")
    }

    fn account(&self) -> Option<VarName> {
        self.account.map(Account::var)
    }
}

impl Door {
    /// Every door, in the order a list of the built-ins names them.
    pub const ALL: [Self; 4] = [Self::Jev, Self::OpenJev, Self::Clef, Self::ClefFlash];

    const fn constants(self) -> &'static Constants {
        match self {
            Self::Jev => &JEV,
            Self::OpenJev => &OPENJEV,
            Self::Clef => &CLEF,
            Self::ClefFlash => &CLEF_FLASH,
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
        Self::ALL.into_iter().find(|door| door.name() == name)
    }
}

impl fmt::Display for Door {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The ceiling per `choice`, the same at every door.
const OPTIONS: u32 = 255;

/// The gate's five numbers, each a probability; one a door does not ship is the table's to give.
#[derive(Clone, Copy)]
struct Floors {
    route: Option<Prob>,
    fits: Option<Prob>,
    read: Option<Prob>,
    write: Option<Prob>,
    whole: Option<Prob>,
}

impl Floors {
    /// No floor: a door ships none until they are fitted on its own engine's answers, and a call through it
    /// waits for a yes unless the table gives them.
    const NONE: Self = Self {
        route: None,
        fits: None,
        read: None,
        write: None,
        whole: None,
    };
}

/// The floors Jev's doors ship — `route`, `fits` for a newcomer at `add`, `read`, `write`, `whole` for a call
/// held against its request — over Jev's own calibration, which OpenJEV forwards to and never rescales.
const JEVS: Floors = Floors {
    route: Some(floor(0.5)),
    fits: Some(floor(0.3)),
    read: Some(floor(0.8)),
    write: Some(floor(0.9)),
    whole: Some(floor(0.3)),
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
    /// The variable that holds the account's id, at a door whose address names an account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<VarName>,
    /// Where a key comes from, as the line that asks for one says it.
    pub issuer: String,
    pub policy: Transport,
}

/// The transport policy as a value, executed by each host: the wait after connect, how many times a connect error
/// or a retried status is tried again, and which statuses those are.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transport {
    pub timeout: Millis,
    pub retries: u32,
    pub retry_statuses: Range<u16>,
}

impl Transport {
    /// Whether a response's status is tried again.
    #[must_use]
    pub fn retried(&self, status: u16) -> bool {
        (self.retry_statuses.min()..=self.retry_statuses.max()).contains(&status)
    }
}

/// The door's effective settings: its constants and the floors it ships, with the table's gate overrides; every
/// problem names its key under the door's name. A door that ships no floors declares no gate, unless the table
/// gives `route`, `read` and `write`.
pub fn settings(door: Door, table: Option<&Json>) -> Result<Settings, Vec<Diagnostic>> {
    let constants = door.constants();
    let mut floors = constants.floors;
    let mut errors: Vec<Diagnostic> = table
        .map(|table| overrides(constants.name, table, &mut floors))
        .unwrap_or_default()
        .into_iter()
        .map(problem)
        .collect();
    let gate = match floors {
        Floors {
            route: Some(route),
            fits,
            read: Some(read),
            write: Some(write),
            whole,
        } => Gate::new(route, fits, read, write)
            .map(|gate| gate.holding(whole))
            .map_err(|why| errors.push(problem(format!("adapters.{}.gate: {why}", constants.name))))
            .ok(),
        Floors {
            route: None,
            fits: None,
            read: None,
            write: None,
            whole: None,
        } => None,
        _ => {
            errors.push(problem(format!(
                "adapters.{name}.gate needs route, read and write, since {name} ships no floors",
                name = constants.name
            )));
            None
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Settings {
        declared: Declared {
            id: constants.id(),
            limits: Some(Limits {
                options: Some(OPTIONS),
                tokens: None,
            }),
            gate,
        },
        credential: constants.credential(),
        account: constants.account(),
        issuer: constants.issuer.to_owned(),
        policy: policy(constants.timeout),
    })
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
                "whole" => &mut floors.whole,
                _ => {
                    problems.push(format!(
                        "adapters.{name}.gate.{key} is unknown; the keys are route, fits, read, write and whole"
                    ));
                    continue;
                }
            };
            match value.as_f64().and_then(Prob::new) {
                Some(p) => *floor = Some(p),
                None => problems.push(format!(
                    "adapters.{name}.gate.{key} must be a probability, 0 to 1"
                )),
            }
        }
    }
    problems
}

/// The door's wait after connect; one retry on connect errors and server errors, never on a client error.
fn policy(timeout: Millis) -> Transport {
    Transport {
        timeout,
        retries: 1,
        retry_statuses: Range::new(500, 599).expect("500 is below 599"),
    }
}

/// The address a door posts to: its own, built in. At a door whose address names an account, the id its
/// variable holds goes in, 32 hex digits in lowercase and nothing else, so the variable chooses an account and
/// never another address; a missing or malformed id is a problem fixed by exporting the variable.
pub fn address(door: Door, account: Option<&str>) -> Result<String, Diagnostic> {
    let constants = door.constants();
    let Some(named) = constants.account else {
        return Ok(constants.url.to_owned());
    };
    let problem = |message: String| Diagnostic {
        reflex: None,
        at: None,
        message,
        fix: Fix::ExportKey { var: named.var() },
    };
    // An empty id is no id.
    match account.filter(|id| !id.is_empty()) {
        None => Err(problem(format!(
            "{} needs {}, an account id from {}",
            constants.name, named.var, constants.issuer
        ))),
        Some(id)
            if id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) =>
        {
            Ok(format!("{}{id}{}", constants.url, named.path))
        }
        Some(_) => Err(problem(format!(
            "{} is not an account id: 32 hex digits, in lowercase",
            named.var
        ))),
    }
}

/// The bodies of a request, in order: each the door's model, the state, and questions as System One's `choice`
/// or `noul`. One body carries every question, except at a door whose service takes so many a body at most: there
/// the questions go out in the request's order, that many a body, and the answers of all the bodies are the
/// request's.
#[must_use]
pub fn request(door: Door, request: &Request) -> Vec<Json> {
    let constants = door.constants();
    let questions: Vec<(String, Json)> = request
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
    let body = |questions: &[(String, Json)]| {
        json!({
            "model": constants.model,
            "state": { "request": request.state.request },
            "questions": questions.iter().cloned().collect::<serde_json::Map<String, Json>>(),
        })
    };
    match constants.questions {
        Some(ceiling) if questions.len() > ceiling => questions.chunks(ceiling).map(body).collect(),
        _ => vec![body(&questions)],
    }
}

/// A response as `Raw` answers, the same at every door once out of a service's envelope: a `choice` as given,
/// scaled back to 1 only when rounding left its sum a hair off; a `noul` as `{ "yes": p }`. A 401 is the key
/// refused, named by the variable that holds it, and by the account's where the address names one; any other
/// status is a fault of its own; a body that is not System One's is a transport fault; an answer of the wrong
/// shape is malformed for its question; a sum off by more than rounding reaches the core as given, for `read` to
/// refuse.
pub fn answers(door: Door, status: u16, body: &str) -> Result<Raw, Fault> {
    let constants = door.constants();
    if status == 401 {
        return Err(Fault::Refused {
            credential: constants.credential(),
            account: constants.account(),
        });
    }
    if status != 200 {
        return Err(Fault::Status { status });
    }
    let transport = |message: String| Fault::Transport { message };
    let body: Json = serde_json::from_str(body)
        .map_err(|error| transport(format!("the response is not JSON: {error}")))?;
    let answered = if constants.enveloped {
        body.get("result")
    } else {
        Some(&body)
    };
    let answers = answered
        .and_then(|answered| answered.get("answers"))
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
                // An engine prints its numbers rounded, two decimals at the least, so a sum drifts by half a
                // unit in the last place per option at most; that much is normalized, bounded at 0.05, and a
                // wider gap is the engine's, never hidden. The comparison allows for the doubles' own error, so
                // 0.51 + 0.50 is within 0.01.
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

    fn gate_of(settings: &Settings) -> [f64; 5] {
        let gate = settings.declared.gate.unwrap();
        [
            gate.route().get(),
            gate.fits().unwrap().get(),
            gate.read().get(),
            gate.write().get(),
            gate.whole().unwrap().get(),
        ]
    }

    /// A door's problems with a table, as their messages.
    fn problems_at(door: Door, toml: &str) -> Vec<String> {
        settings(door, Some(&table(door.name(), toml)))
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
    fn defaults_hold_without_a_table() {
        let settings = settings(Door::Jev, None).unwrap();
        assert_eq!(settings.declared.id.as_str(), "jev-1.13.0");
        assert_eq!(settings.declared.limits.unwrap().options, Some(255));
        assert_eq!(settings.declared.limits.unwrap().tokens, None);
        assert_eq!(gate_of(&settings), [0.5, 0.3, 0.8, 0.9, 0.3]);
        assert_eq!(settings.credential.as_str(), "TYPESAFE_API_KEY");
        assert_eq!(settings.account, None);
        assert_eq!(settings.issuer, "typesafe.ai");
        assert_eq!(
            address(Door::Jev, None).unwrap(),
            "https://api.typesafe.ai/v1/systemone"
        );
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
        assert_eq!(openjev.declared.id, jev.declared.id);
        assert_eq!(openjev.declared.id.as_str(), "jev-1.13.0");
        assert_eq!(openjev.declared.limits, jev.declared.limits);
        assert_eq!(openjev.declared.gate, jev.declared.gate);
        assert_eq!(openjev.credential.as_str(), "OPENJEV_API_KEY");
        assert_eq!(openjev.issuer, "openjev.sh");
        assert_eq!(
            address(Door::OpenJev, None).unwrap(),
            "https://api.openjev.sh/v1/systemone"
        );
        assert_eq!(openjev.policy.timeout, Millis(3_000));
        assert_eq!(openjev.policy.retries, jev.policy.retries);
        assert_eq!(openjev.policy.retry_statuses, jev.policy.retry_statuses);
        let tuned = settings(
            Door::OpenJev,
            Some(&table(
                "openjev",
                "[adapters.openjev]\ngate = { write = 0.95 }\n",
            )),
        )
        .unwrap();
        assert_eq!(gate_of(&tuned), [0.5, 0.3, 0.8, 0.95, 0.3]);
        assert_eq!(
            problems_at(Door::OpenJev, "[adapters.openjev]\ngate = { route = 2 }\n"),
            ["adapters.openjev.gate.route must be a probability, 0 to 1"]
        );
    }

    #[test]
    fn clef_is_two_doors_at_one_service_with_no_floors_of_their_own() {
        let clef = settings(Door::Clef, None).unwrap();
        let flash = settings(Door::ClefFlash, None).unwrap();
        assert_eq!(clef.declared.id.as_str(), "clef-2026-10-03");
        assert_eq!(flash.declared.id.as_str(), "clef-flash-2026-10-03");
        for door in [&clef, &flash] {
            assert_eq!(door.declared.limits.unwrap().options, Some(255));
            assert_eq!(door.declared.gate, None);
            assert_eq!(door.credential.as_str(), "CLOUDFLARE_API_TOKEN");
            assert_eq!(
                door.account.as_ref().map(VarName::as_str),
                Some("CLOUDFLARE_ACCOUNT_ID")
            );
            assert_eq!(door.issuer, "dash.cloudflare.com");
            assert_eq!(door.policy.retries, 1);
        }
        assert_eq!(clef.policy.timeout, Millis(6_000));
        assert_eq!(flash.policy.timeout, Millis(3_000));
        // An empty table gives no floor either.
        let bare = settings(Door::Clef, Some(&table("clef", "[adapters.clef]\n"))).unwrap();
        assert_eq!(bare.declared.gate, None);
    }

    #[test]
    fn a_table_gives_the_floors_a_door_does_not_ship() {
        let fitted = settings(
            Door::Clef,
            Some(&table(
                "clef",
                "[adapters.clef]\ngate = { route = 0.6, read = 0.85, write = 0.95 }\n",
            )),
        )
        .unwrap();
        let gate = fitted.declared.gate.unwrap();
        assert_eq!(
            [gate.route().get(), gate.read().get(), gate.write().get()],
            [0.6, 0.85, 0.95]
        );
        assert_eq!((gate.fits(), gate.whole()), (None, None));
        let whole = settings(
            Door::ClefFlash,
            Some(&table(
                "clef_flash",
                "[adapters.clef_flash]\ngate = { route = 0.6, fits = 0.4, read = 0.85, write = 0.95, whole = 0.5 }\n",
            )),
        )
        .unwrap();
        assert_eq!(gate_of(&whole), [0.6, 0.4, 0.85, 0.95, 0.5]);
        assert_eq!(
            problems_at(Door::Clef, "[adapters.clef]\ngate = { write = 0.95 }\n"),
            ["adapters.clef.gate needs route, read and write, since clef ships no floors"]
        );
        assert_eq!(
            problems_at(
                Door::Clef,
                "[adapters.clef]\ngate = { route = 0.6, read = 0.97, write = 0.95 }\n"
            ),
            ["adapters.clef.gate: read 0.97 is above write 0.95"]
        );
    }

    #[test]
    fn an_address_names_the_account_its_variable_holds() {
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            address(Door::Clef, Some(id)).unwrap(),
            "https://api.cloudflare.com/client/v4/accounts/0123456789abcdef0123456789abcdef/ai/run/@cf/cloudflare/clef"
        );
        assert_eq!(
            address(Door::ClefFlash, Some(id)).unwrap(),
            "https://api.cloudflare.com/client/v4/accounts/0123456789abcdef0123456789abcdef/ai/run/@cf/cloudflare/clef-flash"
        );
        // A door with an address of its own takes no account, whatever the variable holds.
        assert_eq!(
            address(Door::Jev, Some("anything")).unwrap(),
            "https://api.typesafe.ai/v1/systemone"
        );
        let var = VarName::new("CLOUDFLARE_ACCOUNT_ID").unwrap();
        for missing in [None, Some("")] {
            let problem = address(Door::Clef, missing).unwrap_err();
            assert_eq!(
                problem.message,
                "clef needs CLOUDFLARE_ACCOUNT_ID, an account id from dash.cloudflare.com"
            );
            assert_eq!(problem.fix, Fix::ExportKey { var: var.clone() });
        }
        // Nothing but 32 hex digits in lowercase reaches the address: no path, no query, no other host.
        for malformed in [
            "0123456789ABCDEF0123456789ABCDEF",
            "0123456789abcdef0123456789abcde",
            "0123456789abcdef0123456789abcdef0",
            "0123456789abcdef/../../89abcdef0",
            "0123456789abcdef0123456789abcde?",
            "0123456789abcdef@evil.example/aa",
            " 123456789abcdef0123456789abcdef",
        ] {
            let problem = address(Door::ClefFlash, Some(malformed)).unwrap_err();
            assert_eq!(
                problem.message,
                "CLOUDFLARE_ACCOUNT_ID is not an account id: 32 hex digits, in lowercase"
            );
            assert_eq!(problem.fix, Fix::ExportKey { var: var.clone() });
        }
    }

    #[test]
    fn a_door_is_its_name_on_the_wire() {
        assert_eq!(serde_json::to_value(Door::Jev).unwrap(), "jev");
        assert_eq!(serde_json::to_value(Door::OpenJev).unwrap(), "openjev");
        assert_eq!(serde_json::to_value(Door::Clef).unwrap(), "clef");
        assert_eq!(serde_json::to_value(Door::ClefFlash).unwrap(), "clef_flash");
        assert_eq!(
            serde_json::from_value::<Door>(json!("clef_flash")).unwrap(),
            Door::ClefFlash
        );
        assert!(serde_json::from_value::<Door>(json!("clef-flash")).is_err());
        assert_eq!(
            serde_json::from_value::<Door>(json!("openjev")).unwrap(),
            Door::OpenJev
        );
        assert!(serde_json::from_value::<Door>(json!("OpenJev")).is_err());
        assert_eq!(Door::named("jev"), Some(Door::Jev));
        assert_eq!(Door::named("openjev"), Some(Door::OpenJev));
        assert_eq!(Door::named("clef"), Some(Door::Clef));
        assert_eq!(Door::named("clef_flash"), Some(Door::ClefFlash));
        assert_eq!(Door::named("replay"), None);
        assert_eq!(Door::OpenJev.to_string(), "openjev");
        assert_eq!(Door::ClefFlash.to_string(), "clef_flash");
        // A door's wire name is the name a project gives it.
        for door in Door::ALL {
            assert_eq!(serde_json::to_value(door).unwrap(), door.name());
        }
    }

    #[test]
    fn the_table_overrides_the_gate() {
        let settings = settings(
            Door::Jev,
            Some(&table(
                "jev",
                "[adapters.jev]\ngate = { write = 0.95, fits = 0.4, whole = 0.5 }\n",
            )),
        )
        .unwrap();
        assert_eq!(gate_of(&settings), [0.5, 0.4, 0.8, 0.95, 0.5]);
        let settings = settings_of("[adapters.jev]\n");
        assert_eq!(gate_of(&settings.unwrap()), [0.5, 0.3, 0.8, 0.9, 0.3]);
    }

    fn settings_of(toml: &str) -> Result<Settings, Vec<Diagnostic>> {
        settings(Door::Jev, Some(&table("jev", toml)))
    }

    fn problems(toml: &str) -> Vec<String> {
        problems_at(Door::Jev, toml)
    }

    #[test]
    fn every_problem_names_its_key() {
        assert_eq!(
            problems("[adapters.jev]\ngate = { route = 1.5 }\n"),
            ["adapters.jev.gate.route must be a probability, 0 to 1"]
        );
        assert_eq!(
            problems("[adapters.jev]\ngate = { read = 0.95 }\n"),
            ["adapters.jev.gate: read 0.95 is above write 0.9"]
        );
        assert_eq!(
            problems("[adapters.jev]\ncolour = 1\ngate = { abstain = 0.5 }\n"),
            [
                "adapters.jev.colour is unknown; the keys are gate",
                "adapters.jev.gate.abstain is unknown; the keys are route, fits, read, write and whole",
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
        let bodies = super::request(Door::Jev, &request);
        assert_eq!(bodies.len(), 1);
        let body = &bodies[0];
        assert_eq!(body["model"], "jev-1.13.0");
        assert_eq!(body["state"], json!({ "request": "kill the lights" }));
        let questions = body["questions"].as_object().unwrap();
        // Every question of the request, under its own id and in the request's order.
        let ids: Vec<&str> = questions.keys().map(String::as_str).collect();
        let asked: Vec<String> = request.questions.keys().map(ToString::to_string).collect();
        assert_eq!(ids, asked);
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
        assert_eq!(
            questions["lights.state"]["criteria"]["on"],
            json!({ "what": "Switch on.", "examples": ["turn on the kitchen lights"] })
        );
        // A phrase checked for thefts asks how each reflex fits, a yes or no.
        let phrase: Request = serde_json::from_str(include_str!(
            "../../../spec/fixtures/request-a-phrase-checked-for-thefts.json"
        ))
        .unwrap();
        let body = &super::request(Door::OpenJev, &phrase)[0];
        let questions = body["questions"].as_object().unwrap();
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
        let body = &super::request(Door::Jev, &request)[0];
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

    /// The mapping at jev's door, as every test but the other doors' calls it.
    fn answers(status: u16, body: &str) -> Result<Raw, Fault> {
        super::answers(Door::Jev, status, body)
    }

    #[test]
    fn a_refused_key_names_the_variable_that_holds_it() {
        for door in Door::ALL {
            let Settings {
                credential,
                account,
                ..
            } = settings(door, None).unwrap();
            assert_eq!(
                super::answers(door, 401, "").unwrap_err(),
                Fault::Refused {
                    credential,
                    account
                }
            );
        }
        assert_eq!(
            super::answers(Door::Clef, 401, "").unwrap_err().to_string(),
            "the key in CLOUDFLARE_API_TOKEN was refused for the account in CLOUDFLARE_ACCOUNT_ID"
        );
    }

    /// A request of `count` questions: the fixture's own, then yes-or-no questions of evoke's own.
    fn request_of(count: usize) -> Request {
        let mut request: Request = serde_json::from_str(include_str!(
            "../../../spec/fixtures/request-kill-the-lights.json"
        ))
        .unwrap();
        let clean = |text: &str| evoke_core::Clean::new(text).unwrap();
        for n in request.questions.len()..count {
            request.questions.insert(
                QuestionId::parse(&format!("weave.split_{n}")).unwrap(),
                Question::YesNo {
                    ask: clean("At «and», does the request ask for two things?"),
                    yes: evoke_core::adapter::Text::Plain(clean("Two things.")),
                    no: evoke_core::adapter::Text::Plain(clean("One thing.")),
                },
            );
        }
        request
    }

    #[test]
    fn a_clef_body_names_its_model_and_carries_sixty_four_questions_at_most() {
        let request = request_of(8);
        for (door, model) in [(Door::Clef, "clef"), (Door::ClefFlash, "clef-flash")] {
            let bodies = super::request(door, &request);
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies[0]["model"], model);
            // But for the model, the body is the one jev's door posts.
            let mut jevs = super::request(Door::Jev, &request).remove(0);
            jevs["model"] = json!(model);
            assert_eq!(bodies[0], jevs);
        }
        let asked = |body: &Json| -> Vec<String> {
            body["questions"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        };
        assert_eq!(super::request(Door::Clef, &request_of(64)).len(), 1);
        let request = request_of(130);
        let bodies = super::request(Door::Clef, &request);
        assert_eq!(
            bodies
                .iter()
                .map(|body| asked(body).len())
                .collect::<Vec<_>>(),
            [64, 64, 2]
        );
        // Every question once, in the request's order, the state in every body.
        let all: Vec<String> = bodies.iter().flat_map(asked).collect();
        let ids: Vec<String> = request.questions.keys().map(ToString::to_string).collect();
        assert_eq!(all, ids);
        for body in &bodies {
            assert_eq!(body["state"], json!({ "request": "kill the lights" }));
        }
        // A door without a ceiling posts one body, however many the questions.
        assert_eq!(super::request(Door::Jev, &request).len(), 1);
    }

    /// The shape Workers AI answers with: System One's answer under `result`, in the service's envelope.
    const ENVELOPED: &str = r#"{"result": {"model": "clef", "answers": {"route": {"type": "choice", "choice": "lights", "probabilities": {"lights": 0.8913, "timer": 0.0312, "none": 0.0775}, "confidence": 0.6541}, "fits.lights": {"type": "noul", "noul": 0.9627}}, "usage": {"input_tokens": 1092, "output_tokens": 0}}, "success": true, "errors": [], "messages": []}"#;

    #[test]
    fn a_clef_answer_is_read_from_its_envelope() {
        for door in [Door::Clef, Door::ClefFlash] {
            let raw = super::answers(door, 200, ENVELOPED).unwrap();
            assert_eq!(raw.0["route"]["lights"], 0.8913);
            assert_eq!(raw.0["fits.lights"]["yes"], 0.9627);
        }
        // The envelope is that service's: a door without one reads no answer in it, nor a bare answer with one.
        assert_eq!(
            answers(200, ENVELOPED).unwrap_err(),
            Fault::Transport {
                message: "the response has no answers".to_owned()
            }
        );
        assert_eq!(
            super::answers(Door::Clef, 200, RESPONSE).unwrap_err(),
            Fault::Transport {
                message: "the response has no answers".to_owned()
            }
        );
        // What the service says of a body it refuses is a status, as at any door.
        assert_eq!(
            super::answers(
                Door::Clef,
                422,
                r#"{"result": null, "success": false, "errors": [{"code": 5012, "message": "AiError"}], "messages": []}"#
            )
            .unwrap_err(),
            Fault::Status { status: 422 }
        );
        assert_eq!(
            super::answers(
                Door::Clef,
                200,
                r#"{"result": null, "success": false, "errors": [], "messages": []}"#
            )
            .unwrap_err(),
            Fault::Transport {
                message: "the response has no answers".to_owned()
            }
        );
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
