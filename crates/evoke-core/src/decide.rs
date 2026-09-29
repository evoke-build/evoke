//! Build the request; validate and read the answers, failing closed; confidence; the gate; fill; a call by name.
//! In: a `Plan`, an input, tags and a `Scope`; `Raw` answers; the adapter's `Gate`; a `Written` call. Out:
//! `Request`, `Reading`, `Decision`.

use std::fmt::Write as _;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::account::{self, Does, Left};
use crate::adapter::{
    Choice, Fault, Gate, Key, Prob, Question, QuestionId, Raw, Request, State, Text,
};
use crate::call::{Call, Value, Written, quoted, render};
use crate::diagnostic::{Diagnostic, Fix};
use crate::document::Json;
use crate::manifest::{
    self, Argument, Effect, Kind, Pick, Piece, Range, Recognizer, Source, Yield,
};
use crate::name::{self, ArgName, FieldName, LocalName, OptionKey, Tag, VocabName, Word};
use crate::pins;
use crate::plan::{
    Active, Plan, Slot, none as none_key, not_among, unstated as unstated_key, unstated_text,
};
use crate::propose::{PickValue, Proposed, propose};
use crate::settle::{self, One, Settled};
use crate::text::{Clean, Input, NonEmpty, Span};
use crate::words::{self, Form, How, Listed, Spelled};

/// The most values an ask lists from the session's results.
const MOST_RECENT: usize = 5;

/// A result a body of this session returned, as the host hands it to `request`: the reflex, and its `data`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recent {
    pub reflex: LocalName,
    pub data: Json,
}

/// What a request asks: everything, or the route alone — the conflict test at `add`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Full,
    Route,
    /// The route and every reflex's `fits`: what the thief test at `add` asks.
    Fits,
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

/// What reading a text's answers comes to: the reading, or the questions its answers open, to be asked of the
/// same text and read with the rest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Read {
    Open {
        request: Request,
    },
    Done {
        #[serde(flatten)]
        reading: Reading,
    },
}

impl Read {
    /// The reading, when nothing is left to ask.
    #[must_use]
    pub fn done(self) -> Option<Reading> {
        match self {
            Self::Done { reading } => Some(reading),
            Self::Open { .. } => None,
        }
    }
}

/// The reflex that won the route, with what its arguments read, and what each value read stands on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Winner {
    pub reflex: LocalName,
    pub args: IndexMap<ArgName, Value>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub basis: IndexMap<ArgName, Basis>,
    pub missing: Vec<Missing>,
    /// The runs of the request's words that no value holds, with what each does.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<Left>,
    pub unconsumed: Vec<Span>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_up: Option<Contender>,
}

/// One of the two views of a listed word: the argument's own question, and the reader's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Ask,
    Reader,
}

/// What a value stands on: the answers that gave it, and the words of the request that hold it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum Basis {
    /// The argument's own question gave it.
    Ask { p: Prob },
    /// Both views of a listed word gave it. `yes` is the word's own yes, asked where no word of the request
    /// holds it.
    Views {
        ask: Prob,
        reader: Prob,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        yes: Option<Prob>,
    },
    /// One view gave it, and words of the request hold it; `other` is what the other view answered.
    View {
        view: View,
        p: Prob,
        other: Key,
        words: Span,
        how: How,
    },
    /// No view gave it: words of the request hold it, and a yes says it is meant.
    Words { words: Span, how: How, yes: Prob },
    /// The request spells it out in a form code reads, and a yes says it is meant.
    Spelled { form: Form, yes: Prob },
    /// The one candidate of its kind, and a yes says it is meant.
    Only { yes: Prob },
}

impl Basis {
    /// How sure the value is: the share of what gave it, the less sure of two views.
    #[must_use]
    pub fn sure(&self) -> Prob {
        match self {
            Self::Ask { p } | Self::View { p, .. } => *p,
            Self::Views { ask, reader, .. } => {
                if reader < ask {
                    *reader
                } else {
                    *ask
                }
            }
            Self::Words { yes, .. } | Self::Spelled { yes, .. } | Self::Only { yes } => *yes,
        }
    }

    /// Why a call that holds the value waits for a yes whatever its number, when it does.
    fn cap(&self, arg: &ArgName) -> Option<Cap> {
        match self {
            Self::View { .. } | Self::Words { .. } => Some(Cap::OneView { arg: arg.clone() }),
            Self::Spelled {
                form: Form::Misspelt | Form::Spaced,
                ..
            } => Some(Cap::Respelt { arg: arg.clone() }),
            Self::Ask { .. } | Self::Views { .. } | Self::Spelled { .. } | Self::Only { .. } => {
                None
            }
        }
    }
}

/// One choice read: which key came out on top, and how probable it was.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Judgment {
    pub question: QuestionId,
    pub top: Key,
    pub p: Prob,
}

impl Judgment {
    /// What the judgment is about, as a person names it: the argument, where its question is about one.
    #[must_use]
    pub fn about(&self) -> String {
        match &self.question {
            QuestionId::Arg(_, arg) => arg.to_string(),
            question => {
                pins::argument(question).map_or_else(|| question.to_string(), |arg| arg.to_string())
            }
        }
    }
}

/// An argument without a usable value, and what a person may choose from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Missing {
    pub arg: ArgName,
    pub ask: Clean,
    pub because: Why,
    /// The words of the request that answer the ask, where the reading found them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<Span>,
    pub choices: Choices,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Why {
    Unstated,
    OutOfRange {
        span: Span,
        range: Range<f64>,
    },
    /// Stated, but not among what is offered: a word the list lacks, a form no recognizer reads.
    NotOffered,
    /// Stated, and read two ways that do not agree.
    Unsettled,
    /// Stated in words that answer the ask, which nothing read as a value.
    Unread,
}

/// What a person may answer with; a vocabulary also prompts to add a word; a pick that names a yielded field
/// lists the values the session's results returned under it, when there are any.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Choices {
    Options {
        options: IndexMap<OptionKey, Clean>,
    },
    Vocab {
        words: IndexMap<Word, Clean>,
    },
    Pick {
        pick: Recognizer,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recent: Option<Vec<String>>,
    },
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
    /// What each value read from the request stands on; a value a person gave has none.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub basis: IndexMap<ArgName, Basis>,
    /// The runs of the request's words that no value holds, with what each does.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<Left>,
    #[serde(flatten)]
    pub judged: Option<Judged>,
}

/// The judgment fields as plain options, so a partial or lying set is refused rather than read as none.
#[derive(Deserialize)]
struct RawChosen {
    #[serde(flatten)]
    call: Call,
    effect: Effect,
    #[serde(default)]
    basis: IndexMap<ArgName, Basis>,
    #[serde(default)]
    left: Vec<Left>,
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
            basis: raw.basis,
            left: raw.left,
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

    /// The same, without the judgments of the arguments a person has just answered for: their `unstated` doubt is
    /// settled by the answer, so it leaves the gate. The route always stays, so the judgments are never empty.
    fn without(self, reflex: &LocalName, given: &IndexMap<ArgName, Value>) -> Self {
        let kept: Vec<Judgment> = self
            .judgments
            .iter()
            .filter(|judgment| {
                !matches!(&judgment.question, QuestionId::Arg(of, arg) if of == reflex && given.contains_key(arg))
            })
            .cloned()
            .collect();
        let judgments = NonEmpty::try_from(kept).unwrap_or(self.judgments);
        let weakest = weakest(&judgments).clone();
        Self {
            confidence: weakest.p,
            weakest,
            judgments,
            runner_up: self.runner_up,
            contenders: self.contenders,
        }
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
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub basis: IndexMap<ArgName, Basis>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<Left>,
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
    UnderFloor {
        judgment: Judgment,
        floor: Prob,
    },
    /// A listed word that one view alone gave, or none, and words of the request hold.
    OneView {
        arg: ArgName,
    },
    /// A value read from words that do not spell it as it is typed: a day misspelt, a code typed with spaces.
    Respelt {
        arg: ArgName,
    },
    /// Words of the request that ask for another thing, which the call does not hold.
    More {
        words: Span,
    },
    UnconsumedSpan {
        span: Span,
    },
    TwoThings {
        contender: Contender,
    },
    /// A weave merged a part that matched nothing on its own back into these words: never run unasked.
    Merged,
}

/// The confirm prompt: `evoke`'s own line, then the manifest's template filled in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prompt {
    pub own: String,
    pub template: Clean,
}

impl Prompt {
    /// The prompt of a chosen call under its caps: the call, its effect, its weakest judgment, then each cap that
    /// has words; the template filled from the call's values.
    #[must_use]
    pub fn of(chosen: &Chosen, active: &Active, because: &[Cap]) -> Self {
        let mut own = format!("{} · {}", render(&chosen.call), chosen.effect);
        if let Some(judged) = &chosen.judged {
            let weakest = &judged.weakest;
            let _ = write!(
                own,
                " · weakest: {} {:.2}",
                weakest.about(),
                weakest.p.get()
            );
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
                Cap::Merged => own.push_str(" · merged"),
                Cap::More { words } => {
                    let _ = write!(own, " · also {}", quoted(words.text().as_str()));
                }
                Cap::OneView { arg } | Cap::Respelt { arg } => {
                    if let Some(words) = from(chosen, arg) {
                        let _ = write!(own, " · {arg} from {}", quoted(words.as_str()));
                    }
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
            // Every piece is clean text: the template's, a key, a word or a span; the template itself otherwise.
            template: Clean::new(&template).unwrap_or_else(|_| {
                Clean::new(&active.confirm.to_string()).unwrap_or_else(|_| Clean::default())
            }),
        }
    }
}

/// The words of the request a value was read from, where it is not the words as they stand.
fn from<'a>(chosen: &'a Chosen, arg: &ArgName) -> Option<&'a Clean> {
    match (chosen.basis.get(arg)?, chosen.call.args.get(arg)?) {
        (Basis::View { words, .. } | Basis::Words { words, .. }, _) => Some(words.text()),
        (Basis::Spelled { .. }, Value::Pick { span, .. }) => Some(span.text()),
        _ => None,
    }
}

/// The one call of `answer`: an input over the cap is refused; `--tag` narrows the set, or `only` narrows it to
/// one reflex — what a weave decides a fragment or a rewritten step by; a pick is asked over its candidates.
/// `recent` is what the session's bodies returned, newest first: a pick that names a yielded field
/// keeps the values under it that its recognizer reads whole, for its ask to list; no question offers them.
pub fn request(
    plan: &Plan,
    input: &str,
    tags: &[Tag],
    only: Option<&LocalName>,
    scope: Scope,
    recent: &[Recent],
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
    if scope == Scope::Fits {
        for (id, slot) in plan.slots() {
            if let (QuestionId::Fits(name), Slot::Ready(question)) = (id, slot)
                && narrowed.contains(&name)
            {
                questions.insert(id.clone(), question.clone());
            }
        }
    }
    let mut recalled = IndexMap::new();
    if scope == Scope::Full {
        for (id, slot) in plan.slots() {
            if !id.reflex().is_some_and(|name| narrowed.contains(&name)) {
                continue;
            }
            let question = asked(plan, id, slot, &proposed, recent, &mut recalled);
            questions.insert(id.clone(), question);
        }
    }
    let (listed, spelled) = if scope == Scope::Full {
        found(plan, &narrowed, &input, &proposed)
    } else {
        (IndexMap::new(), IndexMap::new())
    };
    let mut request = Request {
        state: State { request: input },
        questions,
        proposed,
        recent: recalled,
        listed,
        spelled,
    };
    if scope == Scope::Full {
        let own = ahead(plan, &narrowed, &request);
        request.questions.extend(own);
    }
    Ok(request)
}

/// The questions of evoke's own that depend on no answer, for every reflex asked about: the second view of
/// each listed argument, a yes or no on each listed word and each spelled value code found, and on the one
/// candidate of a typed argument's kind.
fn ahead(
    plan: &Plan,
    narrowed: &[&LocalName],
    request: &Request,
) -> IndexMap<QuestionId, Question> {
    let mut own = IndexMap::new();
    for reflex in narrowed {
        let Some(active) = plan.active().get(*reflex) else {
            continue;
        };
        for (arg, argument) in &active.args {
            let id = QuestionId::Arg((*reflex).clone(), arg.clone());
            match (&argument.kind, request.questions.get(&id)) {
                (
                    Kind::Value {
                        source: Source::Options(_) | Source::Vocab(_),
                        ..
                    },
                    Some(Question::Choice(choice)),
                ) => {
                    own.insert(pins::view(reflex, arg), pins::view_question(choice));
                    let found = request.listed.get(&id).map_or(&[][..], Vec::as_slice);
                    for held in found.iter().take(pins::MOST) {
                        own.extend(word_question(reflex, arg, argument, choice, &held.key));
                    }
                }
                (
                    Kind::Value {
                        source: Source::Pick(pick),
                        ..
                    },
                    Some(Question::Choice(_)),
                ) => {
                    let said = request.spelled.get(&id).map_or(&[][..], Vec::as_slice);
                    for spelled in said.iter().take(pins::MOST) {
                        own.insert(
                            pins::said(reflex, arg, &spelled.span),
                            pins::said_question(&argument.ask, spelled),
                        );
                    }
                    if let Some(only) = only(pick.recognizer(), &request.proposed) {
                        own.insert(
                            pins::only(reflex, arg, &only.span),
                            pins::only_question(&argument.ask, &only.span),
                        );
                    }
                }
                _ => {}
            }
        }
    }
    own
}

/// The yes or no on one listed word of an argument, by the word's place in the argument's list.
pub(crate) fn word_question(
    reflex: &LocalName,
    arg: &ArgName,
    argument: &Argument,
    choice: &Choice,
    key: &Key,
) -> Option<(QuestionId, Question)> {
    let (place, _, text) = choice.options().get_full(key)?;
    Some((
        pins::is(reflex, arg, place),
        pins::is_question(&argument.ask, text.what()),
    ))
}

/// The one candidate of a kind among the input's, when it holds one and no more.
pub(crate) fn only(recognizer: Recognizer, proposed: &[Proposed]) -> Option<&Proposed> {
    let mut of_kind = proposed
        .iter()
        .filter(|proposed| proposes(recognizer, &proposed.value));
    let first = of_kind.next()?;
    of_kind.next().is_none().then_some(first)
}

/// One slot of the plan as the request asks it: a question that does not depend on the input as it stands; a
/// pick over the input's candidates of its kind, and over `none` when the input states one that is not among
/// them. Beside it, for a pick that names a yielded field, the values the session's results returned under it.
fn asked(
    plan: &Plan,
    id: &QuestionId,
    slot: &Slot,
    proposed: &[Proposed],
    recent: &[Recent],
    recalled: &mut IndexMap<QuestionId, Vec<String>>,
) -> Question {
    let (ask, pick, field) = match slot {
        Slot::Ready(question) => return question.clone(),
        Slot::Pick {
            ask, pick, recent, ..
        } => (ask, *pick, recent),
    };
    if let (Some(field), QuestionId::Arg(reflex, arg)) = (field, id) {
        let values = plan
            .active()
            .get(reflex)
            .and_then(|active| active.args.get(arg))
            .map(|argument| match &argument.kind {
                Kind::Value {
                    source: Source::Pick(pick),
                    ..
                } => recalled_values(plan, recent, field, pick),
                _ => Vec::new(),
            })
            .unwrap_or_default();
        if !values.is_empty() {
            recalled.insert(id.clone(), values);
        }
    }
    let mut options: IndexMap<Key, Text> = proposed
        .iter()
        .filter(|proposed| proposes(pick, &proposed.value))
        .map(|proposed| {
            (
                candidate(&proposed.span),
                Text::Plain(proposed.span.text().clone()),
            )
        })
        .collect();
    let said = not_among(pick, options.len());
    options.insert(none_key(), Text::Plain(said));
    Question::Choice(Choice::closed(
        ask.clone(),
        options,
        (unstated_key(), Text::Plain(unstated_text())),
    ))
}

/// What code finds in the input's own words for the arguments of the reflexes asked about, by each argument's
/// question: the listed words they hold, and the values they spell out in a form no recognizer reads as typed.
/// A word that is the reflex's own name says what to do and is no value; a word the reflex's own words hold is
/// no part of a code typed with spaces.
fn found(
    plan: &Plan,
    narrowed: &[&LocalName],
    input: &Input,
    proposed: &[Proposed],
) -> (
    IndexMap<QuestionId, Vec<Listed>>,
    IndexMap<QuestionId, Vec<Spelled>>,
) {
    let mut listed = IndexMap::new();
    let mut spelled = IndexMap::new();
    for (id, slot) in plan.slots() {
        let QuestionId::Arg(reflex, arg) = id else {
            continue;
        };
        if !narrowed.contains(&reflex) {
            continue;
        }
        let flag = plan
            .active()
            .get(reflex)
            .and_then(|active| active.args.get(arg))
            .is_some_and(|argument| argument.kind == Kind::Flag);
        match slot {
            Slot::Ready(Question::Choice(choice)) if !flag => {
                let list: IndexMap<Key, Clean> = choice
                    .options()
                    .iter()
                    .filter(|(key, _)| !sentinel(key))
                    .map(|(key, text)| (key.clone(), text.what().clone()))
                    .collect();
                let held: Vec<Listed> = words::listed(input, &list)
                    .into_iter()
                    .filter(|held| !words::names(held.span.text().as_str(), reflex.as_str()))
                    .collect();
                if !held.is_empty() {
                    listed.insert(id.clone(), held);
                }
            }
            Slot::Pick { pick, .. } => {
                let own = own_words(plan, reflex);
                let said: Vec<Spelled> = words::spelled(input, *pick, proposed)
                    .into_iter()
                    .filter(|said| {
                        said.form != Form::Spaced
                            || !words::tokens(said.span.text().as_str())
                                .iter()
                                .any(|token| own.contains(&token.plain))
                    })
                    .collect();
                if !said.is_empty() {
                    spelled.insert(id.clone(), said);
                }
            }
            Slot::Ready(_) => {}
        }
    }
    (listed, spelled)
}

/// Whether a key is one of the sentinels a choice carries, which no list holds.
fn sentinel(key: &Key) -> bool {
    name::RESERVED.contains(&key.as_str())
}

/// The words a reflex says of itself, lowered: its name, its description, its examples and its asks.
fn own_words(plan: &Plan, reflex: &LocalName) -> Vec<String> {
    let mut texts: Vec<String> = vec![reflex.to_string()];
    if let Some(Text::Rich { what, examples, .. }) = plan.route().options().get(reflex.as_str()) {
        texts.push(what.to_string());
        texts.extend(examples.iter().map(ToString::to_string));
    }
    if let Some(active) = plan.active().get(reflex) {
        texts.extend(
            active
                .args
                .values()
                .map(|argument| argument.ask.to_string()),
        );
    }
    texts
        .iter()
        .flat_map(|text| words::tokens(text))
        .map(|token| token.plain)
        .collect()
}

/// The values of `field` among the session's results, newest first: each read whole by the pick's recognizer
/// as a body yields it and inside its range, distinct, five at most. A result of an inactive reflex, and a
/// field its manifest does not yield, count for nothing.
fn recalled_values(plan: &Plan, results: &[Recent], field: &FieldName, pick: &Pick) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for result in results {
        let Some(active) = plan.active().get(&result.reflex) else {
            continue;
        };
        for text in under_field(&active.yields, field, &result.data) {
            if kept.len() == MOST_RECENT {
                return kept;
            }
            if kept.contains(&text) {
                continue;
            }
            let Some(Value::Pick { value, .. }) = yielded(&text, pick.recognizer()) else {
                continue;
            };
            if out_of_range(pick, &value).is_none() {
                kept.push(text);
            }
        }
    }
    kept
}

/// The values a result holds under `field`, strung as a bound value is: the field itself, or the field of each
/// record of a list, by the kind the manifest yields it as.
fn under_field(yields: &IndexMap<FieldName, Yield>, field: &FieldName, data: &Json) -> Vec<String> {
    let Some(data) = data.as_object() else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for (name, yield_) in yields {
        match yield_ {
            Yield::Kind(kind) if name == field => {
                values.extend(
                    data.get(name.as_str())
                        .and_then(|value| scalar(value, *kind)),
                );
            }
            Yield::Each(fields) => {
                if let Some(kind) = fields.get(field)
                    && let Some(records) = data.get(name.as_str()).and_then(Json::as_array)
                {
                    values.extend(records.iter().filter_map(|record| {
                        record
                            .get(field.as_str())
                            .and_then(|value| scalar(value, *kind))
                    }));
                }
            }
            Yield::Kind(_) => {}
        }
    }
    values
}

/// A string or a number of a result's data as text a recognizer reads: a number under a `duration` field as
/// `<n> seconds`; anything else is nothing to take.
#[must_use]
pub fn scalar(value: &Json, kind: Recognizer) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Number(number) => {
            let text = number
                .as_i64()
                .map_or_else(|| number.to_string(), |i| i.to_string());
            Some(if kind == Recognizer::Duration {
                format!("{text} seconds")
            } else {
                text
            })
        }
        _ => None,
    }
}

/// Text a body yielded, or a branch listed, read whole by the kind: as `picked` reads it, but a date only with
/// its year and a time only as `HH:MM`, since a value that depends on the day names nothing a step can take.
#[must_use]
pub fn yielded(text: &str, kind: Recognizer) -> Option<Value> {
    let value = picked(text, kind)?;
    let stands = match &value {
        Value::Pick {
            value: PickValue::Date { value },
            ..
        } => value.is_absolute(),
        Value::Pick {
            value: PickValue::Time { value },
            ..
        } => text.trim() == value.to_string(),
        _ => true,
    };
    stands.then_some(value)
}

/// Nothing to ask about: the first inactive reflex's first problem, nothing installed, or a tag no reflex carries.
/// Why there is nothing to ask. An inactive reflex the tags name — any inactive one, when nothing narrows —
/// explains itself, since making it active is the way in; else the tags name nothing, or nothing is installed.
fn nothing_to_ask(plan: &Plan, tags: &[Tag]) -> Diagnostic {
    let named = |name: &LocalName| {
        tags.is_empty() || plan.tagged(name).iter().any(|tag| tags.contains(tag))
    };
    if let Some((_, problems)) = plan.inactive().iter().find(|(name, _)| named(name)) {
        return problems.first().clone();
    }
    let (message, fix) = if plan.active().is_empty() && plan.inactive().is_empty() {
        ("no reflexes are installed".to_owned(), Fix::Add)
    } else {
        let tags: Vec<&str> = tags.iter().map(Tag::as_str).collect();
        (
            format!("no reflex is tagged {}", tags.join(", ")),
            Fix::Show { reflex: None },
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

/// Whether a candidate is one the recognizer proposes; a code argument also takes a quoted candidate whose text
/// reads as a code whole.
pub(crate) fn proposes(recognizer: Recognizer, value: &PickValue) -> bool {
    match (recognizer, value) {
        (Recognizer::Number, PickValue::Number { .. })
        | (Recognizer::Duration, PickValue::Seconds { .. })
        | (Recognizer::Email, PickValue::Email { .. })
        | (Recognizer::Url, PickValue::Url { .. })
        | (Recognizer::Quoted, PickValue::Quoted { .. })
        | (Recognizer::Date, PickValue::Date { .. })
        | (Recognizer::Time, PickValue::Time { .. })
        | (Recognizer::Amount, PickValue::Amount { .. })
        | (Recognizer::Code, PickValue::Code { .. }) => true,
        (Recognizer::Code, PickValue::Quoted { value }) => coded(value).is_some(),
        _ => false,
    }
}

/// The text between quotes as a code, when it reads as one whole.
fn coded(text: &Clean) -> Option<Clean> {
    let input = Input::new(text.as_str()).ok()?;
    let whole = input.as_str().chars().count();
    propose(&input)
        .into_iter()
        .find_map(|proposed| match proposed.value {
            PickValue::Code { value }
                if proposed.span.start() == 0 && proposed.span.end() == whole =>
            {
                Some(value)
            }
            _ => None,
        })
}

/// A candidate's value as the argument's pick takes it: a quoted code as the code, every other as read.
pub(crate) fn taken_as(recognizer: Recognizer, value: &PickValue) -> PickValue {
    match (recognizer, value) {
        (Recognizer::Code, PickValue::Quoted { value }) => PickValue::Code {
            value: value.clone(),
        },
        _ => value.clone(),
    }
}

/// Every answer validated against its question and resolved to the keys offered.
pub(crate) type Answers<'a> = IndexMap<&'a QuestionId, IndexMap<Key, Prob>>;

/// The answers so far read against every question asked so far: each question answered, keys among those
/// offered, probabilities in `[0, 1]`, a choice summing to 1 — then the ranking, the judgments and the winner's
/// values; or the questions the answers open, which the reading waits for.
pub fn read(plan: &Plan, request: &Request, raw: Raw) -> Result<Read, Fault> {
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
        return Ok(Read::Done {
            reading: Reading {
                ranking,
                judgments: vec![judgment],
                winner: None,
            },
        });
    }
    let reflex =
        LocalName::new(winner.as_str()).map_err(|why| malformed(QuestionId::Route, why))?;
    let mut open = IndexMap::new();
    let (judgments, winner) =
        reader.winner(reflex, judgment, ranking.get(1).cloned(), &mut open)?;
    if !open.is_empty() {
        return Ok(Read::Open {
            request: Request {
                state: request.state.clone(),
                questions: open,
                proposed: Vec::new(),
                recent: IndexMap::new(),
                listed: IndexMap::new(),
                spelled: IndexMap::new(),
            },
        });
    }
    Ok(Read::Done {
        reading: Reading {
            ranking,
            judgments,
            winner: Some(winner),
        },
    })
}

/// A text read to its end: the request's questions answered by `answer`, then each round of questions the
/// answers open, until nothing is left to ask. Out: every question asked, every answer, and the reading.
pub fn reading<E>(
    plan: &Plan,
    mut request: Request,
    mut answer: impl FnMut(&Request) -> Result<Raw, E>,
    fault: impl Fn(Fault) -> E,
) -> Result<(Request, Raw, Reading), E> {
    let mut answers = answer(&request)?;
    loop {
        match read(plan, &request, answers.clone()).map_err(&fault)? {
            Read::Done { reading } => return Ok((request, answers, reading)),
            Read::Open { request: round } => {
                answers.0.extend(answer(&round)?.0);
                request.questions.extend(round.questions);
            }
        }
    }
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
            Question::YesNo { .. } if answer.contains_key("no") && (sum - 1.0).abs() > 1e-6 => {
                return Err(malformed(
                    id.clone(),
                    format!("yes and no sum to {}, not 1", shown(sum)),
                ));
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
pub(crate) fn probability(answer: &IndexMap<Key, Prob>, key: &Key) -> Prob {
    answer.get(key).copied().unwrap_or(Prob::ZERO)
}

/// The first offered key with the highest probability.
pub(crate) fn top(choice: &Choice, answer: &IndexMap<Key, Prob>) -> Option<(Key, Prob)> {
    choice
        .options()
        .keys()
        .map(|key| (key.clone(), probability(answer, key)))
        .reduce(|best, next| if next.1 > best.1 { next } else { best })
}

/// At this share a run of words answers an argument's ask, or asks for another thing.
const LEFT: f64 = 0.5;

/// The validated answers over the request and the plan they were asked from.
struct Reader<'a> {
    plan: &'a Plan,
    request: &'a Request,
    answers: Answers<'a>,
}

impl Reader<'_> {
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

    /// The route winner's arguments read in order: a judgment per argument, and what each settled. A question
    /// an argument waits for is put in `open`.
    fn winner(
        &self,
        reflex: LocalName,
        route: Judgment,
        runner_up: Option<Contender>,
        open: &mut IndexMap<QuestionId, Question>,
    ) -> Result<(Vec<Judgment>, Winner), Fault> {
        let active =
            self.plan.active().get(&reflex).ok_or_else(|| {
                malformed(QuestionId::Route, format!("\"{reflex}\" is not active"))
            })?;
        let mut judgments = vec![route];
        let mut args = IndexMap::new();
        let mut basis = IndexMap::new();
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
                // An optional argument over an empty vocabulary is not asked and reads unstated.
                if let Some(source) = required(argument) {
                    let recent = self.recalled(&question);
                    missing.push(unstated(self.plan, &reflex, arg, argument, source, recent));
                }
                continue;
            };
            let one = One {
                plan: self.plan,
                request: self.request,
                answers: &self.answers,
                reflex: &reflex,
                arg,
                argument,
                choice,
                answer,
            };
            let (judgment, settled) = match &argument.kind {
                Kind::Flag => {
                    let judgment = judge(question, argument, choice, answer)?;
                    let settled = if judgment.top.as_str() == "yes" {
                        Settled::Value(Value::Flag, None, Vec::new())
                    } else {
                        Settled::Nothing
                    };
                    (judgment, settled)
                }
                Kind::Value {
                    source: Source::Pick(pick),
                    ..
                } => settle::typed(&one, pick, open)?,
                Kind::Value { source, .. } => settle::listed(&one, source, open)?,
            };
            match settled {
                Settled::Value(value, stands, spans) => {
                    args.insert(arg.clone(), value);
                    if let Some(stands) = stands {
                        basis.insert(arg.clone(), stands);
                    }
                    consumed.extend(spans);
                }
                Settled::Missing(unsettled) => missing.push(unsettled),
                Settled::Nothing | Settled::Open => {}
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
        let left = self.account(&reflex, active, &args, &basis, open);
        self.answered(&reflex, active, &args, &left, &mut missing);
        Ok((
            judgments,
            Winner {
                reflex,
                args,
                basis,
                missing,
                left,
                unconsumed,
                runner_up,
            },
        ))
    }

    /// Words that answer an ask the call holds no value for: the argument is asked, with the words.
    fn answered(
        &self,
        reflex: &LocalName,
        active: &Active,
        args: &IndexMap<ArgName, Value>,
        left: &[Left],
        missing: &mut Vec<Missing>,
    ) {
        for run in left {
            let Does::Answers { arg } = &run.does else {
                continue;
            };
            if run.p.get() < LEFT || args.contains_key(arg) {
                continue;
            }
            if let Some(asked) = missing.iter_mut().find(|asked| asked.arg == *arg) {
                asked.words.get_or_insert_with(|| run.words.clone());
            } else if let Some(argument) = active.args.get(arg)
                && let Kind::Value { source, .. } = &argument.kind
            {
                missing.push(Missing {
                    arg: arg.clone(),
                    ask: argument.ask.clone(),
                    because: Why::Unread,
                    words: Some(run.words.clone()),
                    choices: choices(self.plan, reflex, arg, source, None),
                });
            }
        }
    }

    /// The account of the winner's text: the runs of its words that no value holds, each read from the answer
    /// to what it does; a run not asked about yet is put in `open`, and says nothing.
    fn account(
        &self,
        reflex: &LocalName,
        active: &Active,
        args: &IndexMap<ArgName, Value>,
        basis: &IndexMap<ArgName, Basis>,
        open: &mut IndexMap<QuestionId, Question>,
    ) -> Vec<Left> {
        let held: Vec<&Span> = args
            .iter()
            .filter_map(|(arg, value)| self.held(reflex, arg, value, basis.get(arg)))
            .collect();
        let mut left = Vec::new();
        for run in account::runs(&self.request.state.request, &held) {
            let id = pins::left(reflex, run.from, run.to);
            let question = account::question(&run, &active.args);
            match self.answers.get(&id) {
                Some(answer) => left.extend(account::read(&run, &question, answer)),
                None => {
                    open.insert(id, question);
                }
            }
        }
        left
    }

    /// The words of the request that hold a value: a pick's own span, a listed word's where words hold it.
    fn held<'v>(
        &'v self,
        reflex: &LocalName,
        arg: &ArgName,
        value: &'v Value,
        stands: Option<&'v Basis>,
    ) -> Option<&'v Span> {
        match (value, stands) {
            (Value::Pick { span, .. }, _) => Some(span),
            (_, Some(Basis::View { words, .. } | Basis::Words { words, .. })) => Some(words),
            (Value::Option { key }, _) => self.word_held(reflex, arg, key.as_str()),
            (Value::Word { word, .. }, _) => self.word_held(reflex, arg, word.as_str()),
            (Value::Flag, _) => None,
        }
    }

    /// The words code found that hold a listed word of an argument.
    fn word_held(&self, reflex: &LocalName, arg: &ArgName, key: &str) -> Option<&Span> {
        self.request
            .listed
            .get(&QuestionId::Arg(reflex.clone(), arg.clone()))?
            .iter()
            .find(|held| held.key.as_str() == key)
            .map(|held| &held.span)
    }

    /// The values the request kept for a pick's ask from the session's results, when it kept any.
    fn recalled(&self, question: &QuestionId) -> Option<Vec<String>> {
        self.request.recent.get(question).cloned()
    }
}

/// At this share `none` asks its argument; under it the argument reads as the best of the other answers.
const NOT_OFFERED: f64 = 0.5;

/// One argument's judgment: the top key and its probability; a flag reads `yes` at or above one half, against the
/// rest; a value reads `none` only from one half up, and as the best of the other answers under it.
pub(crate) fn judge(
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
            let none = none_key();
            let stated = probability(answer, &none).get() >= NOT_OFFERED;
            choice
                .options()
                .keys()
                .filter(|key| stated || **key != none)
                .map(|key| (key.clone(), probability(answer, key)))
                .reduce(|best, next| if next.1 > best.1 { next } else { best })
                .ok_or_else(|| malformed(question.clone(), "no options"))?
        }
    };
    Ok(Judgment { question, top, p })
}

/// The source of a required argument; a flag and an optional value are never missing.
pub(crate) fn required(argument: &Argument) -> Option<&Source> {
    match &argument.kind {
        Kind::Value {
            source,
            optional: false,
        } => Some(source),
        Kind::Value { optional: true, .. } | Kind::Flag => None,
    }
}

/// An argument the request did not state; `recent` is what the session's results returned for a pick's ask to
/// list.
pub(crate) fn unstated(
    plan: &Plan,
    reflex: &LocalName,
    arg: &ArgName,
    argument: &Argument,
    source: &Source,
    recent: Option<Vec<String>>,
) -> Missing {
    Missing {
        arg: arg.clone(),
        ask: argument.ask.clone(),
        because: Why::Unstated,
        words: None,
        choices: choices(plan, reflex, arg, source, recent),
    }
}

/// What a person may answer for an argument: the author's options, the vocabulary's words, or the pick's kind
/// with the session's values under the field it names.
pub(crate) fn choices(
    plan: &Plan,
    reflex: &LocalName,
    arg: &ArgName,
    source: &Source,
    recent: Option<Vec<String>>,
) -> Choices {
    match source {
        Source::Options(options) => Choices::Options {
            options: (**options).clone(),
        },
        Source::Pick(pick) => Choices::Pick {
            pick: pick.recognizer(),
            recent,
        },
        Source::Vocab(_) => Choices::Vocab {
            words: words(plan, reflex, arg),
        },
    }
}

/// A word with the value the plan carries for it, if any; a value is never minted by the host.
pub(crate) fn worded(plan: &Plan, vocabulary: &VocabName, word: Word) -> Value {
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
pub(crate) fn out_of_range(pick: &Pick, value: &PickValue) -> Option<Range<f64>> {
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
#[expect(clippy::cast_precision_loss)]
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
            basis: winner.basis,
            left: winner.left,
            unconsumed: winner.unconsumed,
            judged,
        },
        missing,
        gate,
    )
}

/// An ask with its values given: the same gate over the judgments the ask carries, less those of the arguments
/// just answered for — an answer settles its argument, as one input naming it would have.
#[must_use]
pub fn fill(
    plan: &Plan,
    asking: Asking,
    given: IndexMap<ArgName, Value>,
    gate: Option<&Gate>,
) -> Decision {
    carry(plan, asking, given, IndexMap::new(), gate)
}

/// An ask filled with values another part of the request states: as `fill`, but each value stands on what it
/// stood on where it was read, so a value that waits for a yes there waits here.
#[must_use]
pub fn carry(
    plan: &Plan,
    asking: Asking,
    given: IndexMap<ArgName, Value>,
    stands: IndexMap<ArgName, Basis>,
    gate: Option<&Gate>,
) -> Decision {
    let Asking {
        reflex,
        mut args,
        mut basis,
        left,
        unconsumed,
        judged,
    } = asking;
    let Some(active) = plan.active().get(&reflex) else {
        return Decision::Abstain {
            contenders: judged.contenders,
            judgments: judged.judgments.into_iter().collect(),
        };
    };
    let judged = judged.without(&reflex, &given);
    // A value a person gave stands on their answer, and on nothing that was read.
    basis.retain(|arg, _| !given.contains_key(arg));
    basis.extend(stands);
    args.extend(given);
    let (args, missing) = settled(plan, &reflex, active, &args, &[]);
    basis.retain(|arg, _| args.contains_key(arg));
    decided(
        active,
        Asking {
            reflex,
            args,
            basis,
            left,
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
        basis: asking.basis,
        left: asking.left,
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
                        missing.push(unstated(plan, reflex, arg, argument, source, None));
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
                words: None,
                choices: choices(plan, reflex, arg, source, None),
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
            Value::Pick { span, value, .. },
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
        chosen
            .basis
            .iter()
            .filter_map(|(arg, stands)| stands.cap(arg)),
    );
    because.extend(
        chosen
            .left
            .iter()
            .filter(|run| run.does == Does::More && run.p.get() >= LEFT)
            .map(|run| Cap::More {
                words: run.words.clone(),
            }),
    );
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

/// A step a weave merged back from a part that matched nothing on its own never runs unasked: a run confirms
/// with the cap, a confirm gains it, once; an ask stands, capped once it is answered; an abstain stands.
#[must_use]
pub fn merged(plan: &Plan, decision: Decision) -> Decision {
    let active = match &decision {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
            plan.active().get(&chosen.call.reflex)
        }
        Decision::Ask { .. } | Decision::Abstain { .. } => None,
    };
    let Some(active) = active else {
        return decision;
    };
    let (chosen, because) = match decision {
        Decision::Run { chosen } => (chosen, NonEmpty::new(Cap::Merged, Vec::new())),
        Decision::Confirm {
            chosen,
            mut because,
            ..
        } => {
            if !because.iter().any(|cap| *cap == Cap::Merged) {
                because.push(Cap::Merged);
            }
            (chosen, because)
        }
        Decision::Ask { .. } | Decision::Abstain { .. } => return decision,
    };
    let caps: Vec<Cap> = because.iter().cloned().collect();
    let prompt = Prompt::of(&chosen, active, &caps);
    Decision::Confirm {
        chosen,
        prompt,
        because,
    }
}

/// Typed text run through a pick's recognizer, as the prompt and a call by name read it: a candidate of its kind
/// that covers the whole text, spaces at the ends aside — `1e3` and `1 hour 30 minutes` read as nothing, so a call
/// by name is refused and a prompt asks again, never trimmed to the part that read; a quoted pick takes the whole
/// text when no quotes enclose it whole, and a code pick takes a code typed in quotes.
#[must_use]
pub fn picked(text: &str, recognizer: Recognizer) -> Option<Value> {
    let input = Input::new(text.trim()).ok()?;
    let whole = input.as_str().chars().count();
    let found = propose(&input).into_iter().find(|proposed| {
        proposes(recognizer, &proposed.value)
            && match (recognizer, &proposed.value) {
                // A quoted span stands inside its quotes.
                (Recognizer::Quoted, _) | (Recognizer::Code, PickValue::Quoted { .. }) => {
                    proposed.span.start() == 1 && proposed.span.end() + 1 == whole
                }
                _ => proposed.span.start() == 0 && proposed.span.end() == whole,
            }
    });
    match (found, recognizer) {
        (Some(proposed), _) => Some(Value::Pick {
            value: taken_as(recognizer, &proposed.value),
            span: proposed.span,
            typed: None,
        }),
        (None, Recognizer::Quoted) => {
            let span = Span::of(&input, 0, input.as_str().chars().count())?;
            let value = span.text().clone();
            Some(Value::Pick {
                span,
                value: PickValue::Quoted { value },
                typed: None,
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
    // A playbook expands inside a plan: it has no body to call, so a call by name never reaches one.
    if !active.steps.is_empty() {
        return Err(refused(
            &reflex,
            format!("{reflex} is a plan of steps; say it in a sentence"),
            Fix::Show {
                reflex: Some(reflex.clone()),
            },
        ));
    }
    // A whole result is handed by the plan alone: no one types one, so a call by name never reaches a taker.
    if !active.takes.is_empty() {
        let names: Vec<&str> = active.takes.values().map(FieldName::as_str).collect();
        return Err(refused(
            &reflex,
            format!(
                "{reflex} takes {}, which a step before it in the same request returns",
                manifest::words(&names)
            ),
            Fix::Show {
                reflex: Some(reflex.clone()),
            },
        ));
    }
    let mut given = IndexMap::new();
    for (name, text) in &written.args {
        let (current, argument) = active.argument(&reflex, name.as_str())?;
        if given.contains_key(current) {
            return Err(refused(
                &reflex,
                format!("{name} and {current} are one argument; it is given twice"),
                Fix::Show {
                    reflex: Some(reflex.clone()),
                },
            ));
        }
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
        basis: IndexMap::new(),
        left: Vec::new(),
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
            if let Value::Pick {
                span, value: read, ..
            } = &value
                && let Some(range) = out_of_range(pick, read)
            {
                return Err(refused(
                    reflex,
                    format!("{} is outside {}–{}", span.text(), range.min(), range.max()),
                    Fix::Show {
                        reflex: Some(reflex.clone()),
                    },
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
