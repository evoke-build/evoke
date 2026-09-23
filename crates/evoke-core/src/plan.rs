//! Activation and compilation to question slots. In: `Installed` — what the host found — and the adapter's limits.
//! Out: a `Plan`: the active reflexes, the inactive ones with a fix per problem, every question that does not depend
//! on an input, and the digest that keys what is derived from it.

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::adapter::{Choice, Key, Limits, Question, QuestionId, Text};
use crate::diagnostic::{Diagnostic, Fix};
use crate::digest::Digest;
use crate::document::{self, Diagnostics, Form, Json};
use crate::manifest::{
    self, Argument, Assertion, Effect, Kind, Lookup, Manifest, Recognizer, Record, Run, Source,
    Template, Yield,
};
use crate::name::{
    AdapterId, ArgName, ConfigKey, FieldName, LocalName, Tag, VarName, VocabName, Word,
};
use crate::overlay::Effective;
use crate::project::{Setting, Version};
use crate::text::{Clean, NonEmpty};
use crate::vocabulary::Vocabulary;

/// Everything the host found: the fields are public because the host assembles them; `compile` alone judges activity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Installed {
    pub reflexes: IndexMap<LocalName, Item>,
    pub vocab: IndexMap<VocabName, Vocabulary>,
    pub adapter: AdapterId,
    pub evoke: Version,
}

/// One installed reflex as found: its wording or why it has none, the effect consented to, how its config is held.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    #[serde(with = "wording")]
    pub wording: Result<Effective, Vec<Diagnostic>>,
    pub consented: Effect,
    pub configured: IndexMap<ConfigKey, Held>,
}

/// How a set config key is held: the value itself, or a variable that is set or not. Never a secret's value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Held {
    Plain { value: String },
    Env { var: VarName, set: bool },
}

/// A duration in milliseconds; the host makes it an instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Millis(pub u64);

/// The core's one deadline: shared by the adapter call and the body it runs, never by a prompt; what a host
/// gives a body it loads.
pub const DEADLINE: Millis = Millis(30_000);

/// The sentinel every argument's choice carries, and its text.
const UNSTATED: &str = "unstated";
const UNSTATED_TEXT: &str = "The request does not say.";

/// The compiled set: read through its accessors, built by `compile` alone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawPlan")]
pub struct Plan {
    digest: Digest,
    active: IndexMap<LocalName, Active>,
    inactive: IndexMap<LocalName, NonEmpty<Diagnostic>>,
    /// The tags of the inactive reflexes whose manifest read, so a request narrowed by tag can say whether the
    /// tag names an inactive reflex or none at all.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    tagged: IndexMap<LocalName, Vec<Tag>>,
    slots: IndexMap<QuestionId, Slot>,
    values: IndexMap<VocabName, IndexMap<Word, String>>,
    deadline: Millis,
}

#[derive(Deserialize)]
struct RawPlan {
    digest: Digest,
    active: IndexMap<LocalName, Active>,
    inactive: IndexMap<LocalName, NonEmpty<Diagnostic>>,
    #[serde(default)]
    tagged: IndexMap<LocalName, Vec<Tag>>,
    slots: IndexMap<QuestionId, Slot>,
    values: IndexMap<VocabName, IndexMap<Word, String>>,
    deadline: Millis,
}

impl Plan {
    /// SHA-256 of the compact JSON of `Installed`: keys the decision cache and the baselines.
    #[must_use]
    pub fn digest(&self) -> Digest {
        self.digest
    }

    #[must_use]
    pub fn active(&self) -> &IndexMap<LocalName, Active> {
        &self.active
    }

    /// The tags an inactive reflex carries; none when its manifest did not read.
    #[must_use]
    pub fn tagged(&self, name: &LocalName) -> &[Tag] {
        self.tagged.get(name).map_or(&[], Vec::as_slice)
    }

    /// Every problem of every inactive reflex, each with its fix.
    #[must_use]
    pub fn inactive(&self) -> &IndexMap<LocalName, NonEmpty<Diagnostic>> {
        &self.inactive
    }

    /// `route`, then per active reflex `fits.<name>` and every argument, in that order.
    #[must_use]
    pub fn slots(&self) -> &IndexMap<QuestionId, Slot> {
        &self.slots
    }

    /// The route: a choice over the active reflexes and `none`.
    #[must_use]
    pub fn route(&self) -> &Choice {
        match self.slots.get(&QuestionId::Route) {
            Some(Slot::Ready(Question::Choice(route))) => route,
            // `compile` puts it there and `try_from` refuses a plan without it.
            _ => unreachable!("a plan has a route choice"),
        }
    }

    /// Per vocabulary, the words that carry a value: what a `Value::Word`'s `value` is read from.
    #[must_use]
    pub fn values(&self) -> &IndexMap<VocabName, IndexMap<Word, String>> {
        &self.values
    }

    #[must_use]
    pub fn deadline(&self) -> Millis {
        self.deadline
    }

    /// The reflex as the plan runs it: an inactive one answers with its first problem, one not installed with the
    /// list to look at.
    pub fn running(&self, reflex: &LocalName) -> Result<&Active, Diagnostic> {
        if let Some(active) = self.active.get(reflex) {
            return Ok(active);
        }
        Err(self.inactive.get(reflex).map_or_else(
            || Diagnostic {
                reflex: Some(reflex.clone()),
                at: None,
                message: format!("{reflex} is not installed"),
                fix: Fix::Show { reflex: None },
            },
            |problems| problems.first().clone(),
        ))
    }
}

impl TryFrom<RawPlan> for Plan {
    type Error = String;

    fn try_from(raw: RawPlan) -> Result<Self, String> {
        if !matches!(
            raw.slots.get(&QuestionId::Route),
            Some(Slot::Ready(Question::Choice(_)))
        ) {
            return Err("a plan has a route choice".to_owned());
        }
        for id in raw.slots.keys() {
            let owner = match id {
                QuestionId::Route => continue,
                // A question of evoke's own rides beside the plan's, never in it.
                QuestionId::Weave(_) => None,
                QuestionId::Fits(reflex) => raw.active.get(reflex).map(|_| true),
                QuestionId::Arg(reflex, arg) => raw
                    .active
                    .get(reflex)
                    .map(|active| active.args.contains_key(arg)),
            };
            if owner != Some(true) {
                return Err(format!("the question {id} names nothing active"));
            }
        }
        for (name, active) in &raw.active {
            let asked = |id: QuestionId| raw.slots.contains_key(&id);
            // An optional argument over an empty vocabulary has no question: never asked, never stated.
            let unasked = |argument: &Argument| {
                matches!(
                    argument.kind,
                    Kind::Value {
                        source: Source::Vocab(_),
                        optional: true
                    }
                )
            };
            if !asked(QuestionId::Fits(name.clone()))
                || !active.args.iter().all(|(arg, argument)| {
                    asked(QuestionId::Arg(name.clone(), arg.clone())) || unasked(argument)
                })
            {
                return Err(format!("{name} is active without all of its questions"));
            }
        }
        if let Some(name) = raw
            .inactive
            .keys()
            .find(|name| raw.active.contains_key(*name))
        {
            return Err(format!("{name} is both active and inactive"));
        }
        Ok(Self {
            digest: raw.digest,
            active: raw.active,
            inactive: raw.inactive,
            tagged: raw.tagged,
            slots: raw.slots,
            values: raw.values,
            deadline: raw.deadline,
        })
    }
}

/// An active reflex as the decision needs it; `effect` is the tighter of the manifest's and the consented one.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Active {
    pub effect: Effect,
    #[serde(skip_serializing_if = "Run::is_inline")]
    pub run: Run,
    pub confirm: Template,
    pub args: IndexMap<ArgName, Argument>,
    /// What the body's `data` yields for a later step to take, per field.
    pub yields: IndexMap<FieldName, Yield>,
    pub config: IndexMap<ConfigKey, Setting>,
    pub tags: Vec<Tag>,
}

impl Active {
    /// The argument a written name reaches, under its current name: a former one is followed through `was`; none
    /// is the manifest to look at. `reflex` names the diagnostic.
    pub fn argument(
        &self,
        reflex: &LocalName,
        written: &str,
    ) -> Result<(&ArgName, &Argument), Diagnostic> {
        self.args
            .get_key_value(written)
            .or_else(|| {
                self.args
                    .iter()
                    .find(|(_, argument)| argument.was.iter().any(|old| old.as_str() == written))
            })
            .ok_or_else(|| Diagnostic {
                reflex: Some(reflex.clone()),
                at: None,
                message: format!("{reflex} has no argument {written}"),
                fix: Fix::Show {
                    reflex: Some(reflex.clone()),
                },
            })
    }
}

impl<'de> Deserialize<'de> for Active {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let json = Json::deserialize(deserializer)?;
        let mut d = Diagnostics::new(None);
        let value = read_active(&mut d, &json);
        d.finish(value)
            .map_err(|errors| D::Error::custom(document::summary(&errors)))
    }
}

/// The wire form enters through the manifest's own walker, so an argument or a body is validated the same way.
fn read_active(d: &mut Diagnostics, json: &Json) -> Option<Active> {
    let mut top = d.table(document::wire(json))?;
    let effect = if let Some(node) = top.take("effect") {
        manifest::effect(d, &node)
    } else {
        d.fail(None, "effect is required");
        None
    };
    let mut unknown = Vec::new();
    let known = top
        .take("args")
        .map_or_else(IndexMap::new, |node| manifest::args(d, node, &mut unknown));
    let yields = top.take("yields").map_or_else(IndexMap::new, |node| {
        manifest::yields(d, node, &mut unknown)
    });
    let mut lookup = |name: &ArgName| match known.get_key_value(name) {
        None => Lookup::Unknown,
        Some((_, None)) => Lookup::Broken,
        Some((current, Some(arg))) => Lookup::Arg(current, arg),
    };
    let confirm = if let Some(node) = top.take("confirm") {
        manifest::template(d, &node, &mut lookup)
    } else {
        d.fail(None, "confirm is required");
        None
    };
    let run = manifest::run_of(d, top.take("run"), Form::Wire, &known, None);
    let config = match top.take("config") {
        Some(node) => match serde_json::from_value(node.json()) {
            Ok(config) => Some(config),
            Err(error) => {
                d.fail(None, format!("config: {error}"));
                None
            }
        },
        None => Some(IndexMap::new()),
    };
    let tags = top
        .take("tags")
        .map_or_else(Vec::new, |node| manifest::tags(d, node));
    for path in top.unknown().into_iter().chain(unknown) {
        d.fail(None, format!("{path} is not part of an active reflex"));
    }
    let args: Option<IndexMap<ArgName, Argument>> = known
        .into_iter()
        .map(|(name, arg)| Some((name, arg?)))
        .collect();
    Some(Active {
        effect: effect?,
        run: run?,
        confirm: confirm?,
        args: args?,
        yields,
        config: config?,
        tags,
    })
}

/// A question that does not depend on the input, or a pick whose options exist only per input.
#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Ready(Question),
    Pick {
        ask: Clean,
        pick: Recognizer,
        optional: bool,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename = "pick")]
struct RawPick {
    ask: Clean,
    pick: Recognizer,
    optional: bool,
}

impl Serialize for Slot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Ready(question) => question.serialize(serializer),
            Self::Pick {
                ask,
                pick,
                optional,
            } => RawPick {
                ask: ask.clone(),
                pick: *pick,
                optional: *optional,
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Slot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let json = Json::deserialize(deserializer)?;
        if json.get("type").and_then(Json::as_str) == Some("pick") {
            let RawPick {
                ask,
                pick,
                optional,
            } = serde_json::from_value(json).map_err(D::Error::custom)?;
            Ok(Self::Pick {
                ask,
                pick,
                optional,
            })
        } else {
            serde_json::from_value(json)
                .map(Self::Ready)
                .map_err(D::Error::custom)
        }
    }
}

/// The plan of an installed set; over `limits.options`, a diagnostic naming what to remove.
pub fn compile(set: &Installed, limits: Option<&Limits>) -> Result<Plan, Diagnostic> {
    let json = serde_json::to_vec(set).map_err(|error| Diagnostic {
        reflex: None,
        at: None,
        message: format!("the installed set has no wire form: {error}"),
        fix: Fix::Check,
    })?;
    let mut active = IndexMap::new();
    let mut inactive = IndexMap::new();
    let mut tagged = IndexMap::new();
    let mut route = IndexMap::new();
    let mut own = Vec::new();
    for (name, item) in &set.reflexes {
        match judge(name, item, &set.vocab) {
            Err(problems) => {
                if let Ok(effective) = &item.wording
                    && !effective.manifest.tags.is_empty()
                {
                    tagged.insert(name.clone(), effective.manifest.tags.clone());
                }
                inactive.insert(name.clone(), problems);
            }
            Ok((manifest, judged)) => {
                route.insert(Key::from(name), route_option(manifest));
                own.push((QuestionId::Fits(name.clone()), fits(name, manifest)));
                own.extend(argument_slots(name, manifest, &set.vocab));
                active.insert(name.clone(), judged);
            }
        }
    }
    let route = Choice::closed(
        clean("Which one does the request ask for?"),
        route,
        (key("none"), Text::Plain(clean("None of these."))),
    );
    let slots: IndexMap<QuestionId, Slot> =
        std::iter::once((QuestionId::Route, Slot::Ready(Question::Choice(route))))
            .chain(own)
            .collect();
    if let Some(max) = limits.and_then(|limits| limits.options) {
        for (id, slot) in &slots {
            if let Slot::Ready(Question::Choice(choice)) = slot
                && choice.options().len() > max as usize
            {
                return Err(too_many_options(id, choice.options().len(), max, &active));
            }
        }
    }
    let values = set
        .vocab
        .iter()
        .filter_map(|(name, words)| {
            let valued: IndexMap<Word, String> = words
                .iter()
                .filter_map(|(word, meaning)| Some((word.clone(), meaning.value.clone()?)))
                .collect();
            (!valued.is_empty()).then(|| (name.clone(), valued))
        })
        .collect();
    Ok(Plan {
        digest: Digest::of(&json),
        active,
        inactive,
        tagged,
        slots,
        values,
        deadline: DEADLINE,
    })
}

/// Active with what the decision needs, or every problem with its fix.
fn judge<'a>(
    name: &LocalName,
    item: &'a Item,
    vocab: &IndexMap<VocabName, Vocabulary>,
) -> Result<(&'a Manifest, Active), NonEmpty<Diagnostic>> {
    let problem = |message: String, fix: Fix| Diagnostic {
        reflex: Some(name.clone()),
        at: None,
        message,
        fix,
    };
    let manifest = match &item.wording {
        Ok(effective) => &effective.manifest,
        Err(errors) => {
            return Err(NonEmpty::try_from(errors.clone()).unwrap_or_else(|_| {
                NonEmpty::new(
                    problem("the wording could not be read".to_owned(), Fix::Check),
                    Vec::new(),
                )
            }));
        }
    };
    let mut problems = Vec::new();
    let mut named = Vec::new();
    for arg in manifest.args.values() {
        let Kind::Value {
            source: Source::Vocab(vocabulary),
            optional,
        } = &arg.kind
        else {
            continue;
        };
        // An optional argument over an empty vocabulary is never asked and never stated: the reflex stays active.
        if *optional || named.contains(&vocabulary) {
            continue;
        }
        named.push(vocabulary);
        if vocab.get(vocabulary).is_none_or(|words| words.is_empty()) {
            problems.push(problem(
                format!("vocabulary \"{vocabulary}\" is empty"),
                Fix::VocabAdd {
                    vocab: vocabulary.clone(),
                },
            ));
        }
    }
    let mut config = IndexMap::new();
    for (key, spec) in &manifest.config {
        let by_env = Fix::ConfigEnv {
            reflex: name.clone(),
            key: key.clone(),
        };
        match item.configured.get(key) {
            None => problems.push(problem(
                format!("config \"{key}\" is not set"),
                if spec.secret {
                    by_env
                } else {
                    Fix::ConfigSet {
                        reflex: name.clone(),
                        key: key.clone(),
                    }
                },
            )),
            Some(Held::Plain { .. }) if spec.secret => problems.push(problem(
                format!("config \"{key}\" is a secret held plain; set it from a variable"),
                by_env,
            )),
            Some(Held::Env { var, set: false }) => problems.push(problem(
                format!("{var} is not set"),
                Fix::ExportKey { var: var.clone() },
            )),
            Some(Held::Plain { value }) => {
                config.insert(
                    key.clone(),
                    Setting::Plain {
                        value: value.clone(),
                    },
                );
            }
            Some(Held::Env { var, set: true }) => {
                config.insert(key.clone(), Setting::Env { var: var.clone() });
            }
        }
    }
    match NonEmpty::try_from(problems) {
        Ok(problems) => Err(problems),
        Err(_) => Ok((
            manifest,
            Active {
                effect: manifest.effect.max(item.consented),
                run: manifest.run.clone(),
                confirm: manifest.confirm.clone(),
                args: manifest.args.clone(),
                yields: manifest.yields.clone(),
                config,
                tags: manifest.tags.clone(),
            },
        )),
    }
}

/// The route's option for a reflex: its description, what it is not for with its `false` examples, and its examples.
fn route_option(manifest: &Manifest) -> Text {
    let mut not_for = manifest.not_for.clone();
    let mut examples = Vec::new();
    for (_, (utterance, record)) in manifest.examples.iter() {
        match record {
            Record::Never => not_for.push(utterance.clone()),
            Record::Asserts(_) => examples.push(utterance.clone()),
        }
    }
    Text::Rich {
        what: clean(&manifest.description.to_string()),
        not_for,
        examples,
    }
}

/// `fits.<name>`: the description as yes, what it is not for as no.
fn fits(name: &LocalName, manifest: &Manifest) -> Slot {
    let no = if manifest.not_for.is_empty() {
        "Something else.".to_owned()
    } else {
        let listed: Vec<&str> = manifest.not_for.iter().map(Clean::as_str).collect();
        format!("Something else, such as: {}.", listed.join("; "))
    };
    Slot::Ready(Question::YesNo {
        ask: clean(&format!(
            "Does `{name}` do the specific thing the request asks for?"
        )),
        yes: Text::Plain(clean(&manifest.description.to_string())),
        no: Text::Plain(clean(&no)),
    })
}

/// A question over the adapter's option limit: the line names what to remove. For a vocabulary argument the words
/// are the user's, so the fix is a shorter vocabulary, not a removed reflex; for the route it is the last reflex
/// installed.
fn too_many_options(
    id: &QuestionId,
    count: usize,
    max: u32,
    active: &IndexMap<LocalName, Active>,
) -> Diagnostic {
    let options = format!("{id} has {count} options; the adapter takes at most {max}");
    let (message, fix) = match id {
        QuestionId::Arg(reflex, arg) => match active
            .get(reflex)
            .and_then(|active| active.args.get(arg))
            .map(|argument| &argument.kind)
        {
            Some(Kind::Value {
                source: Source::Vocab(vocab),
                ..
            }) => (
                format!(
                    "{id} offers {count} words of \"{vocab}\"; the adapter takes at most {max}"
                ),
                Fix::VocabRemove {
                    vocab: vocab.clone(),
                },
            ),
            _ => (
                options,
                Fix::Remove {
                    reflex: reflex.clone(),
                },
            ),
        },
        QuestionId::Route | QuestionId::Fits(_) | QuestionId::Weave(_) => (
            options,
            active
                .keys()
                .last()
                .map_or(Fix::Check, |reflex| Fix::Remove {
                    reflex: reflex.clone(),
                }),
        ),
    };
    Diagnostic {
        reflex: id.reflex().cloned(),
        at: None,
        message,
        fix,
    }
}

/// One slot per argument: a choice with `unstated`, its examples attached to the options they assert; or a pick.
/// An optional argument over an empty vocabulary has none: never asked, never stated.
fn argument_slots(
    name: &LocalName,
    manifest: &Manifest,
    vocab: &IndexMap<VocabName, Vocabulary>,
) -> Vec<(QuestionId, Slot)> {
    let taught = taught(manifest);
    manifest
        .args
        .iter()
        .filter_map(|(arg, argument)| {
            let teach =
                |key: &str, what: &Clean| match taught.get(arg).and_then(|keys| keys.get(key)) {
                    Some(examples) => Text::Rich {
                        what: what.clone(),
                        not_for: Vec::new(),
                        examples: examples.clone(),
                    },
                    None => Text::Plain(what.clone()),
                };
            let ask = argument.ask.clone();
            let sentinel = || (unstated(), teach(UNSTATED, &unstated_text()));
            let slot = match &argument.kind {
                Kind::Flag => Slot::Ready(Question::Choice(Choice::closed(
                    ask,
                    [
                        (
                            key("yes"),
                            teach("yes", &clean("The request asks for this.")),
                        ),
                        (
                            key("no"),
                            Text::Plain(clean("The request asks for the opposite.")),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    sentinel(),
                ))),
                Kind::Value {
                    source: Source::Pick(pick),
                    optional,
                } => Slot::Pick {
                    ask,
                    pick: pick.recognizer(),
                    optional: *optional,
                },
                Kind::Value {
                    source: Source::Options(options),
                    ..
                } => Slot::Ready(Question::Choice(Choice::closed(
                    ask,
                    options
                        .iter()
                        .map(|(key, what)| (Key::from(key), teach(key.as_str(), what)))
                        .collect(),
                    sentinel(),
                ))),
                Kind::Value {
                    source: Source::Vocab(vocabulary),
                    optional,
                } => {
                    let words: Vec<_> = vocab
                        .get(vocabulary)
                        .into_iter()
                        .flat_map(|words| words.iter())
                        .collect();
                    if *optional && words.is_empty() {
                        return None;
                    }
                    Slot::Ready(Question::Choice(Choice::closed(
                        ask,
                        words
                            .into_iter()
                            .map(|(word, meaning)| {
                                (Key::from(word), teach(word.as_str(), &meaning.what))
                            })
                            .collect(),
                        sentinel(),
                    )))
                }
            };
            Some((QuestionId::Arg(name.clone(), arg.clone()), slot))
        })
        .collect()
}

/// Per argument and key, the example utterances that assert it.
fn taught(manifest: &Manifest) -> IndexMap<ArgName, IndexMap<String, Vec<Clean>>> {
    let mut taught: IndexMap<ArgName, IndexMap<String, Vec<Clean>>> = IndexMap::new();
    for (_, (utterance, record)) in manifest.examples.iter() {
        let Record::Asserts(asserts) = record else {
            continue;
        };
        for (arg, assertion) in asserts {
            let key = match assertion {
                Assertion::Unstated => "unstated".to_owned(),
                Assertion::Option(key) => key.to_string(),
                Assertion::Word(word) => word.to_string(),
                Assertion::Flag => "yes".to_owned(),
                Assertion::Span(_) => continue,
            };
            taught
                .entry(arg.clone())
                .or_default()
                .entry(key)
                .or_default()
                .push(utterance.clone());
        }
    }
    taught
}

/// The sentinel of every argument's choice, and its text.
pub(crate) fn unstated() -> Key {
    key(UNSTATED)
}

pub(crate) fn unstated_text() -> Clean {
    clean(UNSTATED_TEXT)
}

fn key(literal: &str) -> Key {
    Key::new(literal).expect("a key of this module is not empty")
}

/// Text built from this module's literals and clean parts; a line feed is allowed.
fn clean(text: &str) -> Clean {
    Clean::new(text).expect("built from clean parts")
}

/// `Result` on the wire: `{ "ok": … }` or `{ "err": … }`, and never an error without a diagnostic.
mod wording {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::diagnostic::Diagnostic;
    use crate::overlay::Effective;
    use crate::text::NonEmpty;

    #[derive(Serialize)]
    #[serde(rename_all = "lowercase")]
    enum Out<'a> {
        Ok(&'a Effective),
        Err(&'a Vec<Diagnostic>),
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum In {
        Ok(Box<Effective>),
        Err(NonEmpty<Diagnostic>),
    }

    pub(super) fn serialize<S: Serializer>(
        value: &Result<Effective, Vec<Diagnostic>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Ok(effective) => Out::Ok(effective),
            Err(errors) => Out::Err(errors),
        }
        .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Result<Effective, Vec<Diagnostic>>, D::Error> {
        Ok(match In::deserialize(deserializer)? {
            In::Ok(effective) => Ok(*effective),
            In::Err(errors) => Err(errors.into_iter().collect()),
        })
    }
}
