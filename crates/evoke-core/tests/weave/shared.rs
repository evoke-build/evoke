//! The shared value under any plan, against the core's own planner: requests of one to six lookups of the outage
//! — errors, deploys, logs — each naming a service or not, joined by `and` and `then`, the engine's judgment of
//! every split point generated, through `weave::planning::plan` as it is. A lookup that names a service is
//! decided with it, or, now and then, without it: a word the classifier did not read. Each invariant quotes the
//! sentence of the design it checks.

use super::reference::outage;
use super::schedule::{Join, join, judgment, planned_under};
use evoke_core::Decision;
use evoke_core::name::LocalName;
use evoke_core::weave::{Outcome, Via, Weave};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::{FileFailurePersistence, TestCaseError};

/// The vector whose decisions the parts are patched from: a lookup that read its service, and one asking for it.
const VECTOR: &str =
    include_str!("../../../../spec/vectors/weave.plan/a-word-stated-once-reaches-every-step.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reflex {
    Errors,
    Deploys,
    Logs,
}

impl Reflex {
    fn name(self) -> &'static str {
        match self {
            Self::Errors => "errors",
            Self::Deploys => "deploys",
            Self::Logs => "logs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Service {
    Checkout,
    Payments,
}

impl Service {
    fn word(self) -> &'static str {
        match self {
            Self::Checkout => "checkout",
            Self::Payments => "payments",
        }
    }
}

/// One lookup of the request: its reflex; the service its words name, if any; and whether the classifier read
/// that word — a lookup naming a service it did not read asks for one, its own words holding the word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Lookup {
    reflex: Reflex,
    service: Option<Service>,
    read: bool,
}

impl Lookup {
    /// Words that carry no connective, pronoun or determiner: the reflex, then the service where one is named —
    /// «of» between them where the word goes unread, so the two readings of one wording never meet.
    fn text(self) -> String {
        match (self.service, self.read) {
            (None, _) => self.reflex.name().to_owned(),
            (Some(service), true) => format!("{} {}", self.reflex.name(), service.word()),
            (Some(service), false) => format!("{} of {}", self.reflex.name(), service.word()),
        }
    }

    fn of_text(text: &str) -> Self {
        let words: Vec<&str> = text.split(' ').collect();
        let reflex = [Reflex::Errors, Reflex::Deploys, Reflex::Logs]
            .into_iter()
            .find(|reflex| reflex.name() == words[0])
            .unwrap_or_else(|| panic!("no lookup reads «{text}»"));
        let service = |word: &str| {
            [Service::Checkout, Service::Payments]
                .into_iter()
                .find(|service| service.word() == word)
                .unwrap_or_else(|| panic!("no service reads «{word}»"))
        };
        match words.as_slice() {
            [_] => Self {
                reflex,
                service: None,
                read: false,
            },
            [_, word] => Self {
                reflex,
                service: Some(service(word)),
                read: true,
            },
            [_, "of", word] => Self {
                reflex,
                service: Some(service(word)),
                read: false,
            },
            _ => panic!("no lookup reads «{text}»"),
        }
    }

    /// The decision the foundation makes of the words: the vector's own decision of a lookup that read its
    /// service, or of one asking for it, patched to this reflex and word.
    fn decision(self) -> Decision {
        let vector: serde_json::Value = serde_json::from_str(VECTOR).expect("the vector reads");
        let read = self.service.filter(|_| self.read);
        let wanted = if read.is_some() {
            "check checkout's errors"
        } else {
            "deploys"
        };
        let mut json = vector["input"]["answers"]["decided"]
            .as_array()
            .expect("decided is a list")
            .iter()
            .find(|entry| entry[0]["text"] == wanted)
            .map(|entry| entry[1].clone())
            .expect("the vector decided the template");
        let was = json["reflex"].as_str().expect("a reflex").to_owned();
        let name = self.reflex.name();
        json["reflex"] = serde_json::Value::from(name);
        if let Some(service) = read {
            json["args"]["service"]["word"] = serde_json::Value::from(service.word());
            json["call"] =
                serde_json::Value::from(format!("{name} service=\"{}\"", service.word()));
            json["judgments"][1]["top"] = serde_json::Value::from(service.word());
        }
        for judgment in json["judgments"]
            .as_array_mut()
            .expect("judgments are a list")
        {
            if let Some(question) = judgment["question"].as_str()
                && let Some(arg) = question.strip_prefix(&format!("{was}."))
            {
                judgment["question"] = serde_json::Value::from(format!("{name}.{arg}"));
            }
        }
        serde_json::from_value(json).expect("the patched decision reads")
    }
}

/// A request: its lookups, the connectives between them, and the engine's judgment of each split point.
#[derive(Clone, Debug)]
struct Request {
    lookups: Vec<Lookup>,
    joins: Vec<Join>,
    judged: Vec<f64>,
}

impl Request {
    fn text(&self) -> String {
        let mut text = self.lookups[0].text();
        for (join, lookup) in self.joins.iter().zip(&self.lookups[1..]) {
            text.push_str(join.words());
            text.push_str(&lookup.text());
        }
        text
    }

    /// «A required argument takes the one distinct word of its vocabulary the request states, however often,
    /// when the step's own words hold no word of the vocabulary» — and only a word some step read: the word
    /// every lookup lacking one takes, if any.
    fn carried(&self) -> Option<Service> {
        let mut stated: Vec<Service> = Vec::new();
        for lookup in &self.lookups {
            if let Some(service) = lookup.service
                && !stated.contains(&service)
            {
                stated.push(service);
            }
        }
        let [word] = stated.as_slice() else {
            return None;
        };
        self.lookups
            .iter()
            .any(|lookup| lookup.read && lookup.service == Some(*word))
            .then_some(*word)
    }
}

fn lookup() -> impl Strategy<Value = Lookup> {
    let reflex = prop_oneof![
        Just(Reflex::Errors),
        Just(Reflex::Deploys),
        Just(Reflex::Logs)
    ];
    let service = prop_oneof![
        4 => Just(None),
        4 => Just(Some(Service::Checkout)),
        1 => Just(Some(Service::Payments))
    ];
    (
        reflex,
        service,
        prop_oneof![2 => Just(true), 1 => Just(false)],
    )
        .prop_map(|(reflex, service, read)| Lookup {
            reflex,
            service,
            read,
        })
}

fn request() -> impl Strategy<Value = Request> {
    (1usize..=6).prop_flat_map(|n| {
        (vec(lookup(), n), vec(join(), n - 1), vec(judgment(), n - 1)).prop_map(
            |(lookups, joins, judged)| Request {
                lookups,
                joins,
                judged,
            },
        )
    })
}

/// The request through the planner over the outage's plan: each lookup decided as the words say, a text narrowed
/// to one reflex — a fan-out tried on a doubted part — matching nothing.
fn planned(request: &Request) -> Weave {
    planned_under(&outage(), &request.text(), &request.judged, |text| {
        Lookup::of_text(text).decision()
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, failure_persistence: Some(Box::new(FileFailurePersistence::Direct("proptest-regressions/weave/shared.txt"))), ..ProptestConfig::default() })]

    /// «A word of a vocabulary the words state once reaches every step that lacks an argument of that vocabulary
    /// … a required argument is filled as a person's answer to the step's ask would be … a step whose own words
    /// hold a word of the vocabulary keeps it; two words stated, nothing is carried.»
    #[test]
    fn a_word_stated_once_reaches_every_step_that_lacks_it(request in request()) {
        let weave = planned(&request);
        prop_assert_eq!(weave.steps.len(), request.lookups.len());
        let carried = request.carried();
        for (step, lookup) in weave.steps.iter().zip(&request.lookups) {
            prop_assert_eq!(step.reflex.as_ref().map(LocalName::as_str), Some(lookup.reflex.name()));
            // No optional word is stated, so no step's words are rewritten.
            let text = lookup.text();
            prop_assert_eq!(step.text.as_str(), text.as_str());
            let service = match &step.decision {
                Decision::Run { chosen } => chosen.call.args.get("service").and_then(|value| value.text().map(str::to_owned)),
                Decision::Ask { .. } => None,
                Decision::Confirm { .. } | Decision::Abstain { .. } => return Err(TestCaseError::fail("a lookup runs or asks")),
            };
            match (lookup.service, lookup.read, carried) {
                // A step that read its own word keeps it, and shares nothing.
                (Some(own), true, _) => {
                    prop_assert_eq!(service.as_deref(), Some(own.word()));
                    prop_assert!(step.shared.is_empty(), "step {} read its own word", step.n);
                }
                // A step whose words hold the word it did not read asks for it: nothing reaches it.
                (Some(_), false, _) => {
                    prop_assert_eq!(service, None);
                    prop_assert!(step.shared.is_empty(), "step {} holds a word of the vocabulary", step.n);
                }
                // A step lacking the word takes the one word stated and read, as a fill.
                (None, _, Some(word)) => {
                    prop_assert_eq!(service.as_deref(), Some(word.word()));
                    let shared = step.shared.get("service").expect("the fill is on the step");
                    prop_assert_eq!(shared.word.as_str(), word.word());
                    prop_assert_eq!(shared.via, Via::Fill);
                }
                // Two words stated, or none read: nothing is carried, and the step asks.
                (None, _, None) => {
                    prop_assert_eq!(service, None);
                    prop_assert!(step.shared.is_empty(), "step {} took a word no step read, or one of two", step.n);
                }
            }
        }
        // «The verdict stands before anything runs»: an ask left is the plan's.
        let asks = weave.steps.iter().any(|step| matches!(step.decision, Decision::Ask { .. }));
        prop_assert_eq!(weave.verdict.outcome, if asks { Outcome::Ask } else { Outcome::Run });
        // No fan-out settles a lookup: every narrowed decision abstained, so no step is repaired.
        prop_assert!(weave.steps.iter().all(|step| step.repair.is_none()));
    }
}

/// The generator draws the shapes the invariant speaks of: a word carried to a step lacking it, two words stated,
/// one word stated and read by no step, and every step holding its own.
#[test]
fn the_shared_generator_reaches_every_shape() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let mut seen = [0usize; 4];
    for _ in 0..300 {
        let request = request()
            .new_tree(&mut runner)
            .expect("a request")
            .current();
        let lacking = request
            .lookups
            .iter()
            .any(|lookup| lookup.service.is_none());
        let mut stated: Vec<Service> = request.lookups.iter().filter_map(|l| l.service).collect();
        stated.dedup();
        let distinct = {
            let mut words = stated.clone();
            words.sort_by_key(|word| word.word());
            words.dedup();
            words.len()
        };
        seen[0] += usize::from(request.carried().is_some() && lacking);
        seen[1] += usize::from(distinct == 2 && lacking);
        seen[2] += usize::from(distinct == 1 && request.carried().is_none() && lacking);
        seen[3] += usize::from(!lacking && !request.lookups.is_empty());
    }
    let names = [
        "a word carried to a step lacking it",
        "two words stated",
        "one word stated, read by no step",
        "every step holding its own",
    ];
    for (count, name) in seen.iter().zip(names) {
        assert!(*count >= 15, "{name} drawn {count} times of 300");
    }
}
