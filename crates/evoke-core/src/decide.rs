//! Build the request; validate and read the answers, failing closed; confidence; the gate; fill; a call by name.
//! In: a `Plan`, an input, tags and a `Scope`; `Raw` answers; the adapter's `Gate`; a `Written` call. Out:
//! `Request`, `Reading`, `Decision`.

use std::fmt::Write as _;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::{
    Choice, Fault, Gate, Key, Prob, Question, QuestionId, Raw, Request, State, Text,
};
use crate::call::{Call, Value, Written, quoted, render};
use crate::diagnostic::{Diagnostic, Fix};
use crate::manifest::{Argument, Effect, Kind, Pick, Piece, Range, Recognizer, Source};
use crate::name::{ArgName, LocalName, OptionKey, Tag, VocabName, Word};
use crate::plan::{Active, Plan, Slot, unstated as unstated_key, unstated_text};
use crate::propose::{PickValue, propose};
use crate::text::{Clean, Input, NonEmpty, Span};

/// What a request asks: everything, or the route alone — the conflict test at `add`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Full,
    Route,
}

/// What the answers said: the reflexes ranked, every choice read, and the winner with its values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    pub ranking: Vec<Contender>,
    pub judgments: Vec<Judgment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<Winner>,
}

/// A reflex in the ranking: its route probability, and its `fits` when that was asked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contender {
    pub reflex: LocalName,
    pub route: Prob,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fits: Option<Prob>,
}

/// The reflex that won the route, with what its arguments read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Winner {
    pub reflex: LocalName,
    pub args: IndexMap<ArgName, Value>,
    pub missing: Vec<Missing>,
    pub unconsumed: Vec<Span>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_up: Option<Contender>,
}

/// One choice read: which key came out on top, and how probable it was.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Judgment {
    pub question: QuestionId,
    pub top: Key,
    pub p: Prob,
}

/// An argument without a usable value, and what a person may choose from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Missing {
    pub arg: ArgName,
    pub ask: Clean,
    pub because: Why,
    pub choices: Choices,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Why {
    Unstated,
    OutOfRange { span: Span, range: Range<f64> },
}

/// What a person may answer with; a vocabulary also prompts to add a word.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Choices {
    Options { options: IndexMap<OptionKey, Clean> },
    Vocab { words: IndexMap<Word, Clean> },
    Pick { pick: Recognizer },
}

/// The outcome, tagged by `outcome` on the wire, the chosen call's fields flattened beside it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum Decision {
    Abstain {
        contenders: Vec<Contender>,
        judgments: Vec<Judgment>,
    },
    Run {
        #[serde(flatten)]
        chosen: Chosen,
    },
    Confirm {
        #[serde(flatten)]
        chosen: Chosen,
        prompt: Prompt,
        because: NonEmpty<Cap>,
    },
    Ask {
        #[serde(flatten)]
        asking: Asking,
        missing: NonEmpty<Missing>,
    },
}

/// A complete call with its effect and, unless called by name, what the classifier judged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawChosen")]
pub struct Chosen {
    #[serde(flatten)]
    pub call: Call,
    pub effect: Effect,
    #[serde(flatten)]
    pub judged: Option<Judged>,
}

/// The judgment fields as plain options, so a partial or lying set is refused rather than read as none.
#[derive(Deserialize)]
struct RawChosen {
    #[serde(flatten)]
    call: Call,
    effect: Effect,
    confidence: Option<Prob>,
    weakest: Option<Judgment>,
    judgments: Option<NonEmpty<Judgment>>,
    runner_up: Option<Contender>,
    contenders: Option<Vec<Contender>>,
}

impl TryFrom<RawChosen> for Chosen {
    type Error = String;

    fn try_from(raw: RawChosen) -> Result<Self, String> {
        let judged =
            match (raw.confidence, raw.weakest, raw.judgments, raw.contenders) {
                (None, None, None, None) if raw.runner_up.is_none() => None,
                (Some(confidence), Some(weakest), Some(judgments), Some(contenders)) => {
                    Some(Judged::try_from(RawJudged {
                        confidence,
                        weakest,
                        judgments,
                        runner_up: raw.runner_up,
                        contenders,
                    })?)
                }
                _ => return Err(
                    "a judged call carries confidence, weakest, judgments and contenders together"
                        .to_owned(),
                ),
            };
        Ok(Self {
            call: raw.call,
            effect: raw.effect,
            judged,
        })
    }
}

/// The judgments a decision rests on: confidence is the weakest one's probability, over the route and every
/// argument question of the winner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawJudged")]
pub struct Judged {
    confidence: Prob,
    weakest: Judgment,
    judgments: NonEmpty<Judgment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runner_up: Option<Contender>,
    contenders: Vec<Contender>,
}

#[derive(Deserialize)]
struct RawJudged {
    confidence: Prob,
    weakest: Judgment,
    judgments: NonEmpty<Judgment>,
    #[serde(default)]
    runner_up: Option<Contender>,
    contenders: Vec<Contender>,
}

impl Judged {
    /// The judgments of a reading; `None` when it judged nothing.
    #[must_use]
    pub fn of(reading: &Reading) -> Option<Self> {
        let judgments = NonEmpty::try_from(reading.judgments.clone()).ok()?;
        let weakest = weakest(&judgments).clone();
        Some(Self {
            confidence: weakest.p,
            weakest,
            judgments,
            runner_up: reading
                .winner
                .as_ref()
                .and_then(|winner| winner.runner_up.clone()),
            contenders: reading.ranking.clone(),
        })
    }

    #[must_use]
    pub fn confidence(&self) -> Prob {
        self.confidence
    }

    #[must_use]
    pub fn weakest(&self) -> &Judgment {
        &self.weakest
    }

    #[must_use]
    pub fn judgments(&self) -> &NonEmpty<Judgment> {
        &self.judgments
    }

    #[must_use]
    pub fn runner_up(&self) -> Option<&Contender> {
        self.runner_up.as_ref()
    }

    #[must_use]
    pub fn contenders(&self) -> &[Contender] {
        &self.contenders
    }

    fn route(&self) -> Option<Prob> {
        self.judgments
            .iter()
            .find(|judgment| judgment.question == QuestionId::Route)
            .map(|judgment| judgment.p)
    }
}

/// The first judgment with the lowest probability.
fn weakest(judgments: &NonEmpty<Judgment>) -> &Judgment {
    judgments
        .iter()
        .min_by(|a, b| a.p.get().total_cmp(&b.p.get()))
        .unwrap_or(judgments.first())
}

impl TryFrom<RawJudged> for Judged {
    type Error = String;

    fn try_from(raw: RawJudged) -> Result<Self, String> {
        let weakest = weakest(&raw.judgments);
        if raw.weakest != *weakest || raw.confidence != weakest.p {
            return Err("confidence is the weakest judgment's probability".to_owned());
        }
        Ok(Self {
            confidence: raw.confidence,
            weakest: raw.weakest,
            judgments: raw.judgments,
            runner_up: raw.runner_up,
            contenders: raw.contenders,
        })
    }
}

/// An ask in flight: what the gate reads again once the missing values are given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asking {
    pub reflex: LocalName,
    pub args: IndexMap<ArgName, Value>,
    pub unconsumed: Vec<Span>,
    #[serde(flatten)]
    pub judged: Judged,
}

/// Why a decision stops at confirm; `because` lists them in this order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Cap {
    Destructive,
    NoGate,
    UnderFloor { judgment: Judgment, floor: Prob },
    UnconsumedSpan { span: Span },
    TwoThings { contender: Contender },
}

/// The confirm prompt: `evoke`'s own line, then the manifest's template filled in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prompt {
    pub own: String,
    pub template: Clean,
}

impl Prompt {
    fn of(chosen: &Chosen, active: &Active, because: &[Cap]) -> Self {
        let mut own = format!("{} · {}", render(&chosen.call), chosen.effect);
        if let Some(judged) = &chosen.judged {
            let weakest = &judged.weakest;
            let name = match &weakest.question {
                QuestionId::Arg(_, arg) => arg.to_string(),
                question => question.to_string(),
            };
            let _ = write!(own, " · weakest: {name} {:.2}", weakest.p.get());
        }
        for cap in because {
            match cap {
                Cap::NoGate => own.push_str(" · no gate"),
                Cap::UnconsumedSpan { span } => {
                    let _ = write!(own, " · unused {}", quoted(span.text().as_str()));
                }
                Cap::TwoThings { contender } => {
                    let judged = match contender.fits {
                        Some(fits) => format!("fits {:.2}", fits.get()),
                        None => format!("route {:.2}", contender.route.get()),
                    };
                    let _ = write!(own, " · also {} ({judged})", contender.reflex);
                }
                Cap::Destructive | Cap::UnderFloor { .. } => {}
            }
        }
        let template = active
            .confirm
            .pieces()
            .iter()
            .map(|piece| match piece {
                Piece::Text(text) => text.as_str().to_owned(),
                Piece::Arg(name) => chosen
                    .call
                    .args
                    .get(name)
                    .and_then(Value::text)
                    .map_or_else(|| format!("{{{name}}}"), str::to_owned),
            })
            .collect::<String>();
        Self {
            own,
            // Every piece is clean text: the template's, a key, a word or a span.
            template: Clean::new(&template).expect("filled from clean pieces"),
        }
    }
}

/// The one call of `answer`: an input over the cap is refused; `--tag` narrows the set, or `only` narrows it to
/// one reflex — what a weave decides a fragment or a rewritten step by; a pick is asked over its candidates, or
/// not at all.
pub fn request(
    plan: &Plan,
    input: &str,
    tags: &[Tag],
    only: Option<&LocalName>,
    scope: Scope,
) -> Result<Request, Diagnostic> {
    let input = Input::new(input).map_err(|why| Diagnostic {
        reflex: None,
        at: None,
        message: why.to_string(),
        fix: Fix::Rerun,
    })?;
    if let Some(only) = only {
        plan.running(only)?;
    }
    let narrowed: Vec<&LocalName> = plan
        .active()
        .iter()
        .filter(|(name, active)| match only {
            Some(only) => *name == only,
            None => tags.is_empty() || active.tags.iter().any(|tag| tags.contains(tag)),
        })
        .map(|(name, _)| name)
        .collect();
    if narrowed.is_empty() {
        return Err(nothing_to_ask(plan, tags));
    }
    let route = plan
        .route()
        .narrowed(|key| narrowed.iter().any(|name| name.as_str() == key.as_str()));
    let mut questions = IndexMap::new();
    questions.insert(QuestionId::Route, Question::Choice(route));
    let proposed = if scope == Scope::Full {
        propose(&input)
    } else {
        Vec::new()
    };
    if scope == Scope::Full {
        for (id, slot) in plan.slots() {
            if !id.reflex().is_some_and(|name| narrowed.contains(&name)) {
                continue;
            }
            let question = match slot {
                Slot::Ready(question) => question.clone(),
                Slot::Pick { ask, pick, .. } => {
                    let options: IndexMap<Key, Text> = proposed
                        .iter()
                        .filter(|proposed| proposes(*pick, &proposed.value))
                        .map(|proposed| {
                            (
                                candidate(&proposed.span),
                                Text::Plain(proposed.span.text().clone()),
                            )
                        })
                        .collect();
                    if options.is_empty() {
                        continue;
                    }
                    Question::Choice(Choice::closed(
                        ask.clone(),
                        options,
                        (unstated_key(), Text::Plain(unstated_text())),
                    ))
                }
            };
            questions.insert(id.clone(), question);
        }
    }
    Ok(Request {
        state: State { request: input },
        questions,
        proposed,
    })
}

/// Nothing to ask about: the first inactive reflex's first problem, nothing installed, or a tag no reflex carries.
fn nothing_to_ask(plan: &Plan, tags: &[Tag]) -> Diagnostic {
    if let Some((_, problems)) = plan.inactive().first() {
        return problems.first().clone();
    }
    let (message, fix) = if plan.active().is_empty() {
        ("no reflexes are installed".to_owned(), Fix::Add)
    } else {
        let tags: Vec<&str> = tags.iter().map(Tag::as_str).collect();
        (
            format!("no reflex is tagged {}", tags.join(", ")),
            Fix::Rerun,
        )
    };
    Diagnostic {
        reflex: None,
        at: None,
        message,
        fix,
    }
}

/// A pick candidate's key: `<start>-<end>`.
pub(crate) fn candidate(span: &Span) -> Key {
    // Two numbers and a dash are never empty.
    Key::new(&format!("{}-{}", span.start(), span.end())).expect("a candidate key has text")
}

/// Whether a candidate is one the recognizer proposes.
fn proposes(recognizer: Recognizer, value: &PickValue) -> bool {
    matches!(
        (recognizer, value),
        (Recognizer::Number, PickValue::Number { .. })
            | (Recognizer::Duration, PickValue::Seconds { .. })
            | (Recognizer::Email, PickValue::Email { .. })
            | (Recognizer::Url, PickValue::Url { .. })
            | (Recognizer::Quoted, PickValue::Quoted { .. })
    )
}

/// Every answer validated against its question and resolved to the keys offered.
pub(crate) type Answers<'a> = IndexMap<&'a QuestionId, IndexMap<Key, Prob>>;

/// The answers read against their request: every asked question answered, keys among those offered,
/// probabilities in `[0, 1]`, a choice summing to 1 — then the ranking, the judgments and the winner's values.
pub fn read(plan: &Plan, request: &Request, raw: Raw) -> Result<Reading, Fault> {
    let unanswered = Fault::Unanswered {
        question: QuestionId::Route,
    };
    let reader = Reader {
        plan,
        request,
        answers: validated(request, raw)?,
    };
    let Some(Question::Choice(route)) = request.questions.get(&QuestionId::Route) else {
        return Err(unanswered);
    };
    let answer = reader.answers.get(&QuestionId::Route).ok_or(unanswered)?;
    let ranking = reader.ranking(route, answer)?;
    let (winner, p) = top(route, answer)
        .ok_or_else(|| malformed(QuestionId::Route, "the route has no options"))?;
    let judgment = Judgment {
        question: QuestionId::Route,
        top: winner.clone(),
        p,
    };
    if Some(&winner) == route.otherwise() {
        return Ok(Reading {
            ranking,
            judgments: vec![judgment],
            winner: None,
        });
    }
    let reflex =
        LocalName::new(winner.as_str()).map_err(|why| malformed(QuestionId::Route, why))?;
    let (judgments, winner) = reader.winner(reflex, judgment, ranking.get(1).cloned())?;
    Ok(Reading {
        ranking,
        judgments,
        winner: Some(winner),
    })
}

fn malformed(question: QuestionId, message: impl Into<String>) -> Fault {
    Fault::Malformed {
        question,
        message: message.into(),
    }
}

/// The answers taken apart: every asked question's answer, and nothing that was not asked. A host that caches
/// answers checks them here before keeping them, and again on a hit, so a malformed answer is never served twice.
pub fn validated(request: &Request, raw: Raw) -> Result<Answers<'_>, Fault> {
    let mut raw = raw.0;
    let mut answers = IndexMap::new();
    for (id, question) in &request.questions {
        let given = raw
            .shift_remove(&id.to_string())
            .ok_or_else(|| Fault::Unanswered {
                question: id.clone(),
            })?;
        let offered: Vec<&str> = match question {
            Question::Choice(choice) => choice.options().keys().map(Key::as_str).collect(),
            Question::YesNo { .. } => vec!["yes", "no"],
        };
        let mut answer = IndexMap::new();
        let mut sum = 0.0;
        for (key, p) in given {
            if !offered.contains(&key.as_str()) {
                return Err(malformed(id.clone(), format!("\"{key}\" was not offered")));
            }
            let p = Prob::new(p)
                .ok_or_else(|| malformed(id.clone(), format!("{p} is not a probability")))?;
            answer.insert(Key::new(&key).map_err(|why| malformed(id.clone(), why))?, p);
            sum += p.get();
        }
        match question {
            Question::Choice(_) if (sum - 1.0).abs() > 1e-6 => {
                return Err(malformed(
                    id.clone(),
                    format!("the probabilities sum to {}, not 1", shown(sum)),
                ));
            }
            Question::YesNo { .. } if !answer.contains_key("yes") => {
                return Err(Fault::Unanswered {
                    question: id.clone(),
                });
            }
            _ => {}
        }
        answers.insert(id, answer);
    }
    if let Some(key) = raw.keys().next() {
        return Err(match QuestionId::parse(key) {
            Ok(question) => malformed(question, "the question was not asked"),
            // A key that is no question id at all is reported under the route.
            Err(_) => malformed(QuestionId::Route, format!("\"{key}\" is not a question")),
        });
    }
    Ok(answers)
}

/// A sum to six decimals, trailing zeros dropped: `1.2`, not `1.2000000000000002`.
fn shown(sum: f64) -> String {
    let text = format!("{sum:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// An omitted key reads as 0.
fn probability(answer: &IndexMap<Key, Prob>, key: &Key) -> Prob {
    answer.get(key).copied().unwrap_or(Prob::ZERO)
}

/// The first offered key with the highest probability.
fn top(choice: &Choice, answer: &IndexMap<Key, Prob>) -> Option<(Key, Prob)> {
    choice
        .options()
        .keys()
        .map(|key| (key.clone(), probability(answer, key)))
        .reduce(|best, next| if next.1 > best.1 { next } else { best })
}

/// The validated answers over the request and the plan they were asked from.
struct Reader<'a> {
    plan: &'a Plan,
    request: &'a Request,
    answers: Answers<'a>,
}

/// What one argument's judgment settles: a value that may consume a span, a missing value, or nothing.
enum Settled<'a> {
    Value(Value, Option<&'a Span>),
    Missing(Missing),
    Nothing,
}

impl<'a> Reader<'a> {
    /// Every reflex offered, by route probability, with its `fits` when that was asked.
    fn ranking(
        &self,
        route: &Choice,
        answer: &IndexMap<Key, Prob>,
    ) -> Result<Vec<Contender>, Fault> {
        let mut ranking = Vec::new();
        for key in route.options().keys() {
            if Some(key) == route.otherwise() {
                continue;
            }
            let reflex =
                LocalName::new(key.as_str()).map_err(|why| malformed(QuestionId::Route, why))?;
            let fits = self
                .answers
                .get(&QuestionId::Fits(reflex.clone()))
                .and_then(|answer| answer.get("yes"))
                .copied();
            ranking.push(Contender {
                reflex,
                route: probability(answer, key),
                fits,
            });
        }
        ranking.sort_by(|a, b| b.route.get().total_cmp(&a.route.get()));
        Ok(ranking)
    }

    /// The route winner's arguments read in order: a judgment per question asked, and what each settled.
    fn winner(
        &self,
        reflex: LocalName,
        route: Judgment,
        runner_up: Option<Contender>,
    ) -> Result<(Vec<Judgment>, Winner), Fault> {
        let active =
            self.plan.active().get(&reflex).ok_or_else(|| {
                malformed(QuestionId::Route, format!("\"{reflex}\" is not active"))
            })?;
        let mut judgments = vec![route];
        let mut args = IndexMap::new();
        let mut missing = Vec::new();
        let mut consumed: Vec<&Span> = Vec::new();
        for (arg, argument) in &active.args {
            let question = QuestionId::Arg(reflex.clone(), arg.clone());
            let asked = self
                .request
                .questions
                .get(&question)
                .zip(self.answers.get(&question));
            let Some((Question::Choice(choice), answer)) = asked else {
                // A pick with no candidate is not asked and reads unstated.
                if let Some(source) = required(argument) {
                    missing.push(unstated(self.plan, &reflex, arg, argument, source));
                }
                continue;
            };
            let judgment = judge(question, argument, choice, answer)?;
            match self.settle(&reflex, arg, argument, choice, &judgment)? {
                Settled::Value(value, span) => {
                    args.insert(arg.clone(), value);
                    consumed.extend(span);
                }
                Settled::Missing(unsettled) => missing.push(unsettled),
                Settled::Nothing => {}
            }
            judgments.push(judgment);
        }
        let unconsumed = self
            .request
            .proposed
            .iter()
            .filter(|proposed| proposed.value.is_typed() && !consumed.contains(&&proposed.span))
            .map(|proposed| proposed.span.clone())
            .collect();
        Ok((
            judgments,
            Winner {
                reflex,
                args,
                missing,
                unconsumed,
                runner_up,
            },
        ))
    }

    /// The judgment as a value of the argument's source, or what is missing when it read unstated or out of range.
    fn settle(
        &self,
        reflex: &LocalName,
        arg: &ArgName,
        argument: &Argument,
        choice: &Choice,
        judgment: &Judgment,
    ) -> Result<Settled<'a>, Fault> {
        let top = &judgment.top;
        let malformed = |message: String| malformed(judgment.question.clone(), message);
        let source = match &argument.kind {
            Kind::Flag if top.as_str() == "yes" => return Ok(Settled::Value(Value::Flag, None)),
            Kind::Flag => return Ok(Settled::Nothing),
            Kind::Value { source, .. } => source,
        };
        if Some(top) == choice.otherwise() {
            return Ok(match required(argument) {
                Some(source) => {
                    Settled::Missing(unstated(self.plan, reflex, arg, argument, source))
                }
                None => Settled::Nothing,
            });
        }
        let value = match source {
            Source::Options(options) => {
                let (key, _) = options
                    .get_key_value(top.as_str())
                    .ok_or_else(|| malformed(format!("\"{top}\" is not an option")))?;
                Value::Option { key: key.clone() }
            }
            Source::Vocab(vocabulary) => worded(
                self.plan,
                vocabulary,
                Word::new(top.as_str()).map_err(malformed)?,
            ),
            Source::Pick(pick) => {
                let proposed = self
                    .request
                    .proposed
                    .iter()
                    .find(|proposed| {
                        candidate(&proposed.span) == *top
                            && proposes(pick.recognizer(), &proposed.value)
                    })
                    .ok_or_else(|| malformed(format!("\"{top}\" is not a candidate")))?;
                if let Some(range) = out_of_range(pick, &proposed.value) {
                    return Ok(Settled::Missing(Missing {
                        arg: arg.clone(),
                        ask: argument.ask.clone(),
                        because: Why::OutOfRange {
                            span: proposed.span.clone(),
                            range,
                        },
                        choices: Choices::Pick {
                            pick: pick.recognizer(),
                        },
                    }));
                }
                return Ok(Settled::Value(
                    Value::Pick {
                        span: proposed.span.clone(),
                        value: proposed.value.clone(),
                    },
                    Some(&proposed.span),
                ));
            }
        };
        Ok(Settled::Value(value, None))
    }
}

/// One argument's judgment: the top key and its probability; a flag reads `yes` at or above one half, against the
/// rest.
fn judge(
    question: QuestionId,
    argument: &Argument,
    choice: &Choice,
    answer: &IndexMap<Key, Prob>,
) -> Result<Judgment, Fault> {
    let (top, p) = match &argument.kind {
        Kind::Flag => {
            let yes = answer.get("yes").copied().unwrap_or(Prob::ZERO);
            let (top, p) = if yes.get() >= 0.5 {
                ("yes", yes)
            } else {
                ("no", yes.complement())
            };
            (
                Key::new(top).map_err(|why| malformed(question.clone(), why))?,
                p,
            )
        }
        Kind::Value { .. } => {
            top(choice, answer).ok_or_else(|| malformed(question.clone(), "no options"))?
        }
    };
    Ok(Judgment { question, top, p })
}

/// The source of a required argument; a flag and an optional value are never missing.
fn required(argument: &Argument) -> Option<&Source> {
    match &argument.kind {
        Kind::Value {
            source,
            optional: false,
        } => Some(source),
        Kind::Value { optional: true, .. } | Kind::Flag => None,
    }
}

/// An argument the request did not state.
fn unstated(
    plan: &Plan,
    reflex: &LocalName,
    arg: &ArgName,
    argument: &Argument,
    source: &Source,
) -> Missing {
    Missing {
        arg: arg.clone(),
        ask: argument.ask.clone(),
        because: Why::Unstated,
        choices: choices(plan, reflex, arg, source),
    }
}

/// What a person may answer for an argument: the author's options, the vocabulary's words, or the pick's kind.
fn choices(plan: &Plan, reflex: &LocalName, arg: &ArgName, source: &Source) -> Choices {
    match source {
        Source::Options(options) => Choices::Options {
            options: (**options).clone(),
        },
        Source::Pick(pick) => Choices::Pick {
            pick: pick.recognizer(),
        },
        Source::Vocab(_) => Choices::Vocab {
            words: words(plan, reflex, arg),
        },
    }
}

/// A word with the value the plan carries for it, if any; a value is never minted by the host.
fn worded(plan: &Plan, vocabulary: &VocabName, word: Word) -> Value {
    let value = plan
        .values()
        .get(vocabulary)
        .and_then(|words| words.get(&word))
        .cloned();
    Value::Word { word, value }
}

/// The vocabulary of an argument as the plan compiled it: the words its question offers, with what each means.
pub(crate) fn words(plan: &Plan, reflex: &LocalName, arg: &ArgName) -> IndexMap<Word, Clean> {
    let slot = plan
        .slots()
        .get(&QuestionId::Arg(reflex.clone(), arg.clone()));
    match slot {
        Some(Slot::Ready(Question::Choice(choice))) => choice
            .options()
            .iter()
            .filter(|(key, _)| Some(*key) != choice.otherwise())
            .filter_map(|(key, text)| {
                Word::new(key.as_str())
                    .ok()
                    .map(|word| (word, text.what().clone()))
            })
            .collect(),
        _ => IndexMap::new(),
    }
}

/// The range a pick's value falls outside of, if it has one and does.
fn out_of_range(pick: &Pick, value: &PickValue) -> Option<Range<f64>> {
    let (value, range) = match (pick, value) {
        (Pick::Number(Some(range)), PickValue::Number { value }) => (*value, *range),
        (Pick::Duration(Some(range)), PickValue::Seconds { value }) => (
            seconds(*value),
            Range::new(seconds(range.min()), seconds(range.max()))?,
        ),
        _ => return None,
    };
    (value < range.min() || value > range.max()).then_some(range)
}

/// Seconds as a number, to compare with a range.
#[allow(clippy::cast_precision_loss)]
fn seconds(seconds: u64) -> f64 {
    seconds as f64
}

/// The outcome of a reading under the adapter's gate: abstain, ask, confirm or run.
#[must_use]
pub fn gate(plan: &Plan, reading: Reading, gate: Option<&Gate>) -> Decision {
    let judged = Judged::of(&reading);
    let Reading {
        ranking,
        judgments,
        winner,
    } = reading;
    let abstain = Decision::Abstain {
        contenders: ranking,
        judgments,
    };
    let (Some(winner), Some(judged)) = (winner, judged) else {
        return abstain;
    };
    let Some(route) = judged.route() else {
        return abstain;
    };
    if gate.is_some_and(|gate| route < gate.route()) {
        return abstain;
    }
    let Some(active) = plan.active().get(&winner.reflex) else {
        return abstain;
    };
    let (args, missing) = settled(plan, &winner.reflex, active, &winner.args, &winner.missing);
    decided(
        active,
        Asking {
            reflex: winner.reflex,
            args,
            unconsumed: winner.unconsumed,
            judged,
        },
        missing,
        gate,
    )
}

/// An ask with its values given: the same gate over the judgments the ask carries.
#[must_use]
pub fn fill(
    plan: &Plan,
    asking: Asking,
    given: IndexMap<ArgName, Value>,
    gate: Option<&Gate>,
) -> Decision {
    let Asking {
        reflex,
        mut args,
        unconsumed,
        judged,
    } = asking;
    let Some(active) = plan.active().get(&reflex) else {
        return Decision::Abstain {
            contenders: judged.contenders,
            judgments: judged.judgments.into_iter().collect(),
        };
    };
    args.extend(given);
    let (args, missing) = settled(plan, &reflex, active, &args, &[]);
    decided(
        active,
        Asking {
            reflex,
            args,
            unconsumed,
            judged,
        },
        missing,
        gate,
    )
}

/// Ask while anything is missing; else the call is complete, and the caps decide between confirm and run.
fn decided(
    active: &Active,
    asking: Asking,
    missing: Vec<Missing>,
    gate: Option<&Gate>,
) -> Decision {
    if let Ok(missing) = NonEmpty::try_from(missing) {
        return Decision::Ask { asking, missing };
    }
    let chosen = Chosen {
        call: Call {
            reflex: asking.reflex,
            args: asking.args,
        },
        effect: active.effect,
        judged: Some(asking.judged),
    };
    capped(active, chosen, asking.unconsumed, gate)
}

/// The values a reflex can use, in its argument order, and what it still lacks: a value of the wrong kind or out
/// of range is asked again; a required argument without one keeps the reason it was read with, else is unstated.
fn settled(
    plan: &Plan,
    reflex: &LocalName,
    active: &Active,
    values: &IndexMap<ArgName, Value>,
    read: &[Missing],
) -> (IndexMap<ArgName, Value>, Vec<Missing>) {
    let mut args = IndexMap::new();
    let mut missing = Vec::new();
    for (arg, argument) in &active.args {
        let Some(value) = values.get(arg) else {
            match read.iter().find(|unsettled| unsettled.arg == *arg) {
                Some(unsettled) => missing.push(unsettled.clone()),
                None => {
                    if let Some(source) = required(argument) {
                        missing.push(unstated(plan, reflex, arg, argument, source));
                    }
                }
            }
            continue;
        };
        match (
            typed(plan, reflex, arg, argument, value),
            &argument.kind,
            value,
        ) {
            // A word's value is the plan's, never a caller's.
            (
                Ok(()),
                Kind::Value {
                    source: Source::Vocab(vocabulary),
                    ..
                },
                Value::Word { word, .. },
            ) => {
                args.insert(arg.clone(), worded(plan, vocabulary, word.clone()));
            }
            (Ok(()), ..) => {
                args.insert(arg.clone(), value.clone());
            }
            (Err(because), Kind::Value { source, .. }, _) => missing.push(Missing {
                arg: arg.clone(),
                ask: argument.ask.clone(),
                because,
                choices: choices(plan, reflex, arg, source),
            }),
            // A flag given anything but itself is absent.
            (Err(_), Kind::Flag, _) => {}
        }
    }
    (args, missing)
}

/// Whether a value is of the kind its argument's source gives, and why it is asked again when not.
fn typed(
    plan: &Plan,
    reflex: &LocalName,
    arg: &ArgName,
    argument: &Argument,
    value: &Value,
) -> Result<(), Why> {
    let fits = match (&argument.kind, value) {
        (Kind::Flag, Value::Flag) => true,
        (
            Kind::Value {
                source: Source::Options(options),
                ..
            },
            Value::Option { key },
        ) => options.contains_key(key),
        (
            Kind::Value {
                source: Source::Vocab(_),
                ..
            },
            Value::Word { word, .. },
        ) => words(plan, reflex, arg).contains_key(word),
        (
            Kind::Value {
                source: Source::Pick(pick),
                ..
            },
            Value::Pick { span, value },
        ) => {
            if !proposes(pick.recognizer(), value) {
                false
            } else if let Some(range) = out_of_range(pick, value) {
                return Err(Why::OutOfRange {
                    span: span.clone(),
                    range,
                });
            } else {
                true
            }
        }
        _ => false,
    };
    if fits { Ok(()) } else { Err(Why::Unstated) }
}

/// Run, or confirm for every named cap in the fixed order.
fn capped(active: &Active, chosen: Chosen, unconsumed: Vec<Span>, gate: Option<&Gate>) -> Decision {
    let floors = gate.zip(chosen.judged.as_ref());
    let mut because = Vec::new();
    if chosen.effect == Effect::Destructive {
        because.push(Cap::Destructive);
    }
    if gate.is_none() {
        because.push(Cap::NoGate);
    }
    if let Some((gate, judged)) = floors {
        let floor = match chosen.effect {
            Effect::Read => Some(gate.read()),
            Effect::Write => Some(gate.write()),
            Effect::Destructive => None,
        };
        if let Some(floor) = floor
            && judged.confidence < floor
        {
            because.push(Cap::UnderFloor {
                judgment: judged.weakest.clone(),
                floor,
            });
        }
    }
    because.extend(
        unconsumed
            .into_iter()
            .map(|span| Cap::UnconsumedSpan { span }),
    );
    if let Some((gate, judged)) = floors
        && let (Some(floor), Some(runner_up)) = (gate.fits(), &judged.runner_up)
        && runner_up.fits.is_some_and(|fits| fits >= floor)
    {
        because.push(Cap::TwoThings {
            contender: runner_up.clone(),
        });
    }
    let prompt = Prompt::of(&chosen, active, &because);
    match NonEmpty::try_from(because) {
        Ok(because) => Decision::Confirm {
            chosen,
            prompt,
            because,
        },
        Err(_) => Decision::Run { chosen },
    }
}

/// Typed text run through a pick's recognizer, as the prompt and a call by name read it: the first candidate of
/// its kind; a quoted pick takes the whole text when nothing is quoted.
#[must_use]
pub fn picked(text: &str, recognizer: Recognizer) -> Option<Value> {
    let input = Input::new(text).ok()?;
    let found = propose(&input)
        .into_iter()
        .find(|proposed| proposes(recognizer, &proposed.value));
    match (found, recognizer) {
        (Some(proposed), _) => Some(Value::Pick {
            span: proposed.span,
            value: proposed.value,
        }),
        (None, Recognizer::Quoted) => {
            let span = Span::of(&input, 0, input.as_str().chars().count())?;
            let value = span.text().clone();
            Some(Value::Pick {
                span,
                value: PickValue::Quoted { value },
            })
        }
        (None, _) => None,
    }
}

/// `evoke run <call>`: no classifier. Every name is followed through `was` and every value typed by its argument's
/// source; a required argument left out, a word the vocabulary lacks, an option not offered, a pick that does not
/// read or is out of range, a valued flag or a bare value is refused. A read or write call runs; a destructive one
/// confirms, `Destructive` its one reason; nothing is judged.
pub fn by_name(plan: &Plan, written: Written) -> Result<Decision, Diagnostic> {
    let reflex = written.reflex;
    let active = plan.running(&reflex)?;
    let mut given = IndexMap::new();
    for (name, text) in &written.args {
        let (current, argument) = active.argument(&reflex, name.as_str())?;
        let value = named(plan, &reflex, current, argument, text.as_deref())?;
        given.insert(current.clone(), value);
    }
    let needed: Vec<&str> = active
        .args
        .iter()
        .filter(|(arg, argument)| required(argument).is_some() && !given.contains_key(*arg))
        .map(|(arg, _)| arg.as_str())
        .collect();
    if !needed.is_empty() {
        return Err(refused(
            &reflex,
            format!("{reflex} needs {}", needed.join(", ")),
            Fix::Show {
                reflex: Some(reflex.clone()),
            },
        ));
    }
    let args = active
        .args
        .keys()
        .filter_map(|arg| given.shift_remove_entry(arg))
        .collect();
    let chosen = Chosen {
        call: Call { reflex, args },
        effect: active.effect,
        judged: None,
    };
    if chosen.effect != Effect::Destructive {
        return Ok(Decision::Run { chosen });
    }
    let prompt = Prompt::of(&chosen, active, &[Cap::Destructive]);
    Ok(Decision::Confirm {
        chosen,
        prompt,
        because: NonEmpty::new(Cap::Destructive, Vec::new()),
    })
}

/// A written value typed by its argument's source, or why it cannot be.
fn named(
    plan: &Plan,
    reflex: &LocalName,
    arg: &ArgName,
    argument: &Argument,
    text: Option<&str>,
) -> Result<Value, Diagnostic> {
    let show = || Fix::Show {
        reflex: Some(reflex.clone()),
    };
    let (source, text) = match (&argument.kind, text) {
        (Kind::Flag, None) => return Ok(Value::Flag),
        (Kind::Flag, Some(_)) => {
            return Err(refused(
                reflex,
                format!("{arg} is a flag; write it bare"),
                show(),
            ));
        }
        (Kind::Value { .. }, None) => {
            return Err(refused(reflex, format!("{arg} takes a value"), show()));
        }
        (Kind::Value { source, .. }, Some(text)) => (source, text),
    };
    match source {
        Source::Options(options) => {
            let Some(key) = OptionKey::new(text)
                .ok()
                .filter(|key| options.contains_key(key))
            else {
                let keys: Vec<&str> = options.keys().map(OptionKey::as_str).collect();
                return Err(refused(
                    reflex,
                    format!("\"{text}\" is not an option of {arg}: {}", keys.join(", ")),
                    show(),
                ));
            };
            Ok(Value::Option { key })
        }
        Source::Vocab(vocabulary) => {
            let Some(word) = Word::new(text)
                .ok()
                .filter(|word| words(plan, reflex, arg).contains_key(word))
            else {
                return Err(refused(
                    reflex,
                    format!("\"{text}\" is not in vocabulary \"{vocabulary}\""),
                    Fix::VocabAdd {
                        vocab: vocabulary.clone(),
                    },
                ));
            };
            Ok(worded(plan, vocabulary, word))
        }
        Source::Pick(pick) => {
            let Some(value) = picked(text, pick.recognizer()) else {
                return Err(refused(
                    reflex,
                    format!("\"{text}\" is not {}", pick.recognizer().wants()),
                    Fix::Rerun,
                ));
            };
            if let Value::Pick { span, value: read } = &value
                && let Some(range) = out_of_range(pick, read)
            {
                return Err(refused(
                    reflex,
                    format!("{} is outside {}–{}", span.text(), range.min(), range.max()),
                    Fix::Rerun,
                ));
            }
            Ok(value)
        }
    }
}

fn refused(reflex: &LocalName, message: String, fix: Fix) -> Diagnostic {
    Diagnostic {
        reflex: Some(reflex.clone()),
        at: None,
        message,
        fix,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value as Json, json};

    use super::*;

    /// `spec/fixtures/plan.json`, which the vectors read through `$ref`.
    fn plan() -> Plan {
        serde_json::from_str(include_str!("../../../spec/fixtures/plan.json")).unwrap()
    }

    /// The expected decision of a gate vector.
    fn expected(vector: &str) -> Json {
        let case: Json = serde_json::from_str(vector).unwrap();
        case["expect"].clone()
    }

    #[test]
    fn a_decision_whose_confidence_lies_is_refused() {
        let run = expected(include_str!("../../../spec/vectors/gate/run.json"));
        assert!(matches!(
            serde_json::from_value::<Decision>(run.clone()).unwrap(),
            Decision::Run { chosen } if chosen.judged.is_some()
        ));
        let mut lying = run.clone();
        lying["confidence"] = json!(0.99);
        assert!(serde_json::from_value::<Decision>(lying).is_err());
        let mut partial = run;
        partial.as_object_mut().unwrap().remove("contenders");
        assert!(serde_json::from_value::<Decision>(partial).is_err());
        let by_name = json!({
            "outcome": "run", "reflex": "lights", "effect": "write",
            "args": { "state": { "type": "option", "key": "off" } }
        });
        assert!(matches!(
            serde_json::from_value::<Decision>(by_name).unwrap(),
            Decision::Run { chosen } if chosen.judged.is_none()
        ));
    }

    #[test]
    fn a_given_value_the_plan_cannot_type_is_asked_again() {
        let case: Json = serde_json::from_str(include_str!(
            "../../../spec/vectors/fill/a-filled-ask-is-gated-again.json"
        ))
        .unwrap();
        let asking: Asking = serde_json::from_value(case["input"]["asking"].clone()).unwrap();
        let gate: Gate = serde_json::from_value(case["input"]["gate"].clone()).unwrap();
        let given = |given: Json| serde_json::from_value(given).unwrap();
        let unknown_word = given(json!({ "room": { "type": "word", "word": "garage" } }));
        let Decision::Ask { missing, .. } =
            fill(&plan(), asking.clone(), unknown_word, Some(&gate))
        else {
            panic!("a word the vocabulary lacks is asked again");
        };
        assert_eq!(missing.first().arg.as_str(), "room");
        assert_eq!(missing.first().because, Why::Unstated);
        let over_range = given(json!({
            "room": { "type": "word", "word": "den" },
            "brightness": { "type": "pick", "span": { "start": 0, "end": 3, "text": "150" },
                            "value": { "type": "number", "value": 150 } }
        }));
        let Decision::Ask { missing, .. } = fill(&plan(), asking, over_range, Some(&gate)) else {
            panic!("an out-of-range pick is asked again, optional or not");
        };
        assert_eq!(missing.first().arg.as_str(), "brightness");
        assert!(matches!(missing.first().because, Why::OutOfRange { .. }));
    }

    #[test]
    fn a_words_value_is_the_plans_never_a_callers() {
        let forged = json!({ "type": "word", "word": "office", "value": "forged" });
        let room = |decision: Decision| match decision {
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
                chosen.call.args.get("room").cloned()
            }
            Decision::Abstain { .. } | Decision::Ask { .. } => None,
        };
        let plans = Some(Value::Word {
            word: Word::new("office").unwrap(),
            value: Some("group-7".into()),
        });
        let filled: Json = serde_json::from_str(include_str!(
            "../../../spec/vectors/fill/a-filled-ask-is-gated-again.json"
        ))
        .unwrap();
        let asking: Asking = serde_json::from_value(filled["input"]["asking"].clone()).unwrap();
        let gated: Gate = serde_json::from_value(filled["input"]["gate"].clone()).unwrap();
        let given = serde_json::from_value(json!({ "room": forged })).unwrap();
        assert_eq!(room(fill(&plan(), asking, given, Some(&gated))), plans);
        let mut run: Json =
            serde_json::from_str(include_str!("../../../spec/vectors/gate/run.json")).unwrap();
        run["input"]["reading"]["winner"]["args"]["room"] = forged;
        let reading: Reading = serde_json::from_value(run["input"]["reading"].clone()).unwrap();
        let gated: Gate = serde_json::from_value(run["input"]["gate"].clone()).unwrap();
        assert_eq!(room(gate(&plan(), reading, Some(&gated))), plans);
    }
}
