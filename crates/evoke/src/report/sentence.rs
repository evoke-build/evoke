//! A sentence as it was read, for `try` and `why`: the sentence, where it was cut and what stands out of the plan;
//! then each step with its reflex, each value with what it stands on, the call, what became of it and why; then
//! what the adapter was asked. And the line the log keeps of a sentence, after its steps' lines. In: that line
//! and the steps' lines. Out: `Text`; the line as JSON.

use std::fmt::Write as _;

use evoke_core::adapter::QuestionId;
use evoke_core::call::Value;
use evoke_core::decide::{Basis, Cap, Choices, Judgment, Missing, View};
use evoke_core::manifest::{Effect, Recognizer};
use evoke_core::name::{AdapterId, ArgName};
use evoke_core::propose::PickValue;
use evoke_core::weave::reading::{Split, Stretch};
use evoke_core::weave::{Aside, Because, Count, Folded, Repair, Status};
use evoke_core::words::{Form, How};
use evoke_core::{Call, Decision, Does, Gate, Proposed, Raw, Span, render};
use serde::{Deserialize, Serialize};

use super::{Line, call, held, own, plain, quoted, stopped, verdict};
use crate::adapter::Trace;
use crate::commands::session::Woven;
use crate::hosts::terminal::{Role, Text};

/// The rows' labels.
const SEVERAL: &str = "one thing, or several";
const SET_ASIDE: &str = "set aside";
const NOT_IN_PLAN: &str = "not in the plan";
const LEFT_OUT: &str = "left out";
const WHICH: &str = "which reflex";
const HOLDS: &str = "holds all you said";
const SAME_CALL: &str = "the same call";
const PLAYBOOK_STEP: &str = "the playbook's step";
const PLAYBOOK: &str = "the playbook";
const TAKES: &str = "takes";
const RUNS_IF: &str = "runs if";
const READ_WITH: &str = "read with";
const READ_APART: &str = "read apart";

/// How a step's words were settled, where a rule settled them, after the row's label.
const NARROWED: &str = "the reflex of the part before it, as another item of its list";
const SPLICED: &str = "the words of the part before it, as one request";
const MERGED: &str = "the part beside it, as one request: alone it matches nothing";
const CORRECTED: &str = "the part beside it, which says what not to do";
const SPLIT: &str = "as a call of its own, where the parts were kept whole";

/// An answer's key as a person reads it: of the question about the reflex; of an argument, by what it takes.
const NONE_OF_THEM: &str = "none of them";
const NOT_SAID: &str = "not said";
const NOT_LISTED: &str = "said, and not on the list";
const NOT_QUOTED: &str = "said, without quotes";
const NOT_READ: &str = "said, in a form evoke does not read";

/// Under a call held against the request, the words that say what is wanted from the result: one run, and
/// several.
const ABOUT_RESULT: &str = "says what is wanted from the result";
const ABOUT_RESULTS: &str = "say what is wanted from the result";

/// What a part out of the plan does, by each answer.
const A_REMARK: &str = "a reason or a remark";
const A_DETAIL: &str = "adds a detail";
const ASKS: &str = "asks for something";
const THANKS: &str = "a word of thanks";
/// What a part code set aside by its words is: the person's own action, or a courtesy.
const OWN_ACTION: &str = "your own action";
const A_COURTESY: &str = "a courtesy";
const NOT_TO_DO: &str = "what you said not to do";

/// Under a value: a switch that is set, a word said once, the person's own answer, what an ask recalled.
const YES: &str = "yes";
const ONCE_FOR_SEVERAL: &str = "said once in the sentence, for several steps";
const YOUR_ANSWER: &str = "your answer";
const RECALLED: &str = "recalled";
const FROM: &str = "from";

/// How the words of the request hold a listed word that they do not spell as it is listed.
const ANOTHER_FORM: &str = " (another form of it)";
const SAME_WORD: &str = " (the same word)";
const BY_SPELLING: &str = " (by its spelling)";
const BY_MEANING: &str = " (by what it means)";

/// What became of a call, as `why` says it, and what would, as `try` does.
const RAN: &str = "ran";
const RAN_AT_YES: &str = "ran at your yes";
const FAILED: &str = "failed";
const CANCELLED: &str = "cancelled";
const DECLINED: &str = "declined";
const WAITS: &str = "waits for a yes";
const ASKED: &str = "asked";
const REFUSED: &str = "refused";
const SKIPPED: &str = "skipped";
const UNANSWERED: &str = "unanswered";
const RUNS: &str = "runs";
const ASKS_FOR: &str = "asks";

/// The width of a row's label: the longest, and two spaces.
const LABEL: usize = 24;
/// The answers a row shows: the first, and the two after it.
const SHOWN: usize = 3;
/// Under this, a share prints as `0.00`, and its answer is not shown.
const LEAST: f64 = 0.005;

/// What was read of a sentence as a whole, logged once it has been handled, after its steps' lines: the sentence
/// as typed, how many lines before this one are its own, how many things it asks and where it could be cut, what
/// was folded, set aside or left out, why it was refused where it holds no step, the bars it was held to, and
/// what the adapter was asked for it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sentence {
    #[serde(rename = "sentence")]
    pub typed: String,
    pub lines: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<Count>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splits: Vec<Split>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folded: Vec<Folded>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asides: Vec<Aside>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub because: Vec<Because>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<Gate>,
    pub asked: Asked,
}

/// What the adapter was asked for a sentence: by which adapter, how many questions, in how many rounds; nothing
/// where the cache or a plan file answered.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asked {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<AdapterId>,
    pub questions: usize,
    pub rounds: usize,
}

impl Asked {
    /// One more plan's calls: a plan made again asks only what the cache lacks.
    pub fn more(&mut self, trace: &[Trace], rounds: usize) {
        if let Some(call) = trace.first() {
            self.adapter.get_or_insert_with(|| call.adapter.clone());
        }
        self.questions += trace.iter().map(|call| call.questions).sum::<usize>();
        self.rounds += rounds;
    }
}

impl Sentence {
    /// The sentence of a plan, before any of its lines is logged.
    #[must_use]
    pub fn of(woven: &Woven, gate: Option<&Gate>) -> Self {
        let weave = &woven.weave;
        Self {
            typed: weave.input.clone(),
            lines: 0,
            count: weave.count,
            splits: weave.splits.clone(),
            folded: weave.folded.clone(),
            asides: weave.asides.clone(),
            excluded: weave.excluded.clone(),
            because: if weave.steps.is_empty() {
                weave.verdict.because.clone()
            } else {
                Vec::new()
            },
            gate: gate.copied(),
            asked: Asked::default(),
        }
    }

    /// The line the log keeps.
    #[must_use]
    pub fn log(&self) -> String {
        serde_json::to_string(self).expect("a sentence serializes")
    }

    /// The log's line read back; none where the line is a step's or a decision's.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }
}

/// A row: its label, then what it says; what stands under its first line stays under it.
fn row(label: &str, body: Text) -> Text {
    let width = LABEL.max(label.chars().count() + 1);
    let mut text = Text::from(format!("{label:<width$}"));
    text.append(body.hang(width));
    text
}

/// A share as a row shows it.
fn share(p: f64) -> String {
    format!("{p:.2}")
}

/// A question's answers, most probable first; ties keep the adapter's order.
fn sorted(answers: &Raw, question: &str) -> Vec<(String, f64)> {
    let mut sorted: Vec<(String, f64)> = answers
        .0
        .get(question)
        .map(|answer| answer.iter().map(|(key, p)| (key.clone(), *p)).collect())
        .unwrap_or_default();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
    sorted
}

/// The first answers of a question, each as `shown` writes its key: the first weighted, the ones after it
/// between brackets.
fn answers(sorted: &[(String, f64)], shown: impl Fn(&str) -> String) -> Text {
    let mut text = Text::new();
    let mut rest: Vec<String> = Vec::new();
    for (i, (key, p)) in sorted.iter().take(SHOWN).enumerate() {
        if i == 0 {
            text.roled(Role::Top, &shown(key))
                .push(&format!(" {}", share(*p)));
        } else if *p >= LEAST {
            rest.push(format!("{} {}", shown(key), share(*p)));
        }
    }
    if !rest.is_empty() {
        text.push(&format!("   ({})", rest.join(" · ")));
    }
    text
}

/// What an argument takes, as far as a decision says: what decides the words of its `none`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Takes {
    Listed,
    Quoted,
    Typed,
}

/// What the argument of a decision takes: by the value the call holds, else by what its ask offers.
fn takes(value: Option<&Value>, missing: Option<&Missing>) -> Takes {
    match (value, missing.map(|missing| &missing.choices)) {
        (Some(Value::Option { .. } | Value::Word { .. }), _)
        | (None, Some(Choices::Options { .. } | Choices::Vocab { .. })) => Takes::Listed,
        (
            Some(Value::Pick {
                value: PickValue::Quoted { .. },
                ..
            }),
            _,
        )
        | (
            None,
            Some(Choices::Pick {
                pick: Recognizer::Quoted,
                ..
            }),
        ) => Takes::Quoted,
        _ => Takes::Typed,
    }
}

/// An argument's answer as a person reads it: a span by its words, a sentinel by what it means.
fn answer_of(key: &str, takes: Takes, proposed: &[Proposed]) -> String {
    match (key, takes) {
        ("unstated", _) => NOT_SAID.to_owned(),
        ("none", Takes::Listed) => NOT_LISTED.to_owned(),
        ("none", Takes::Quoted) => NOT_QUOTED.to_owned(),
        ("none", Takes::Typed) => NOT_READ.to_owned(),
        _ => proposed
            .iter()
            .find(|proposed| format!("{}-{}", proposed.span.start(), proposed.span.end()) == key)
            .map_or_else(
                || key.to_owned(),
                |proposed| said(proposed.span.text().as_str()),
            ),
    }
}

/// Words of the request, between quotes.
fn said(words: &str) -> String {
    plain(&quoted(words))
}

/// How words hold a listed word, where they do not spell it as listed.
/// What the words that hold a listed value name, asked of them alone.
fn anchored_on(words: &str, shown: &str, p: f64) -> String {
    format!("asked of {words} alone: {shown} {}", share(p))
}

fn how(how: How) -> &'static str {
    match how {
        How::Same => "",
        How::Form => ANOTHER_FORM,
        How::Stem => SAME_WORD,
        How::Spelling => BY_SPELLING,
        How::Meaning => BY_MEANING,
    }
}

/// The reading of a sentence: `why`'s, from the log, with what became of each step; `try`'s, from the plan, with
/// what would.
#[must_use]
pub fn reading(sentence: Option<&Sentence>, lines: &[Line], tried: bool) -> Text {
    let mut shown: Vec<Text> = Vec::new();
    let typed = sentence.map_or_else(
        || {
            lines
                .first()
                .map_or_else(String::new, |line| line.input.as_str().to_owned())
        },
        |sentence| sentence.typed.clone(),
    );
    shown.push(Text::from(said(&typed)));
    shown.push(Text::new());
    let head = sentence
        .map(|sentence| whole(sentence, lines.len()))
        .unwrap_or_default();
    if !head.is_empty() {
        shown.extend(head);
        shown.push(Text::new());
    }
    let several = lines
        .iter()
        .any(|line| line.step.is_some() || line.expansion.is_some());
    for line in lines {
        let mut block = step(sentence, line, tried);
        if several {
            let words = plain(line.typed.as_deref().unwrap_or(line.input.as_str()));
            shown.push(Text::from(match (&line.step, &line.expansion) {
                (Some(step), _) => {
                    let width = step.of.to_string().len();
                    format!("{:>width$} · {words}", step.n)
                }
                (None, Some(expansion)) => {
                    format!(
                        "the {} plan, {} steps · {words}",
                        expansion.playbook, expansion.of
                    )
                }
                (None, None) => format!("1 · {words}"),
            }));
            block = block
                .into_iter()
                .map(|row| {
                    let mut under = Text::from("  ");
                    under.append(row.hang(2));
                    under
                })
                .collect();
        }
        shown.extend(block);
        shown.push(Text::new());
    }
    if let Some(sentence) = sentence
        && lines.is_empty()
    {
        for because in &sentence.because {
            shown.push(Text::from(format!("→ {REFUSED}: {}", verdict(because))));
        }
        shown.push(Text::new());
    }
    shown.push(closing(sentence, lines));
    // An empty line stays empty.
    Text::lines(shown.into_iter().map(|line| {
        if line.is_empty() {
            return line;
        }
        let mut indented = Text::from("  ");
        indented.append(line.hang(2));
        indented
    }))
}

/// The rows of the sentence as a whole: how many things it asks, where that changed the reading; each part set
/// aside or kept out of the plan; what was left out.
fn whole(sentence: &Sentence, steps: usize) -> Vec<Text> {
    let mut rows = Vec::new();
    if let Some(count) = sentence.count {
        let cuts: Vec<String> = sentence
            .splits
            .iter()
            .filter(|split| split.cut)
            .map(|split| {
                let chars: Vec<char> = sentence.typed.chars().collect();
                let word: String = chars
                    .get(split.start..split.end)
                    .map_or_else(|| split.word.clone(), |chars| chars.iter().collect());
                match split.p {
                    Some(p) => format!("{} {}", said(word.trim()), share(p.get())),
                    None => said(word.trim()),
                }
            })
            .collect();
        if count.as_one && !sentence.splits.is_empty() {
            rows.push(row(
                SEVERAL,
                Text::from(format!(
                    "one thing {}: read as one step, and no cut is made",
                    share(count.one.get())
                )),
            ));
        } else if steps > 1 && !cuts.is_empty() {
            rows.push(row(
                SEVERAL,
                Text::from(format!(
                    "several {}: cut at {}",
                    share(1.0 - count.one.get()),
                    cuts.join(", ")
                )),
            ));
        }
    }
    for aside in &sentence.asides {
        let label = if aside.remark { SET_ASIDE } else { NOT_IN_PLAN };
        let does = match (aside.by, aside.does) {
            (Some(Stretch::Own), _) => OWN_ACTION.to_owned(),
            (Some(Stretch::Courtesy | Stretch::Contrast), _) => A_COURTESY.to_owned(),
            (None, None) => THANKS.to_owned(),
            (None, Some(does)) => {
                let mut shares = [
                    (A_REMARK, does.aside.get()),
                    (A_DETAIL, does.detail.get()),
                    (ASKS, does.asks.get()),
                ];
                shares.sort_by(|a, b| b.1.total_cmp(&a.1));
                shares
                    .iter()
                    .map(|(what, p)| format!("{what} {}", share(*p)))
                    .collect::<Vec<_>>()
                    .join(" · ")
            }
        };
        rows.push(row(
            label,
            Text::from(format!("{}: {does}", said(&aside.text))),
        ));
    }
    for part in &sentence.excluded {
        rows.push(row(
            LEFT_OUT,
            Text::from(format!("{}: {NOT_TO_DO}", said(part))),
        ));
    }
    rows
}

/// One step's rows: the reflex, each value, how far the call holds all that was said, what a rule did, then what
/// became of the call and why.
fn step(sentence: Option<&Sentence>, line: &Line, tried: bool) -> Vec<Text> {
    let mut rows = Vec::new();
    let route = sorted(&line.answers, "route");
    if !route.is_empty() {
        let mut body = answers(&route, |key| match key {
            "none" => NONE_OF_THEM.to_owned(),
            _ => key.to_owned(),
        });
        if let Some(summary) = &line.summary {
            body.push("\n").push(&plain(summary));
        }
        rows.push(row(WHICH, body));
    }
    rows.extend(values(sentence, line));
    if let Some(row) = holds(sentence, &line.decision) {
        rows.push(row);
    }
    rows.extend(rules(sentence, line));
    rows.extend(outcome(sentence, line, tried));
    rows
}

/// What a decision holds or asks of an argument.
struct Of<'a> {
    value: Option<&'a Value>,
    basis: Option<&'a Basis>,
    missing: Option<&'a Missing>,
}

fn of<'a>(decision: &'a Decision, arg: &str) -> Of<'a> {
    let (args, basis, missing) = match decision {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
            (Some(&chosen.call.args), Some(&chosen.basis), None)
        }
        Decision::Ask { asking, missing } => {
            (Some(&asking.args), Some(&asking.basis), Some(missing))
        }
        Decision::Abstain { .. } => (None, None, None),
    };
    Of {
        value: args.and_then(|args| {
            args.iter()
                .find(|(name, _)| name.as_str() == arg)
                .map(|(_, value)| value)
        }),
        basis: basis.and_then(|basis| {
            basis
                .iter()
                .find(|(name, _)| name.as_str() == arg)
                .map(|(_, basis)| basis)
        }),
        missing: missing.and_then(|missing| missing.iter().find(|m| m.arg.as_str() == arg)),
    }
}

/// A row for each argument the call holds or asks, and each the request says something of.
fn values(sentence: Option<&Sentence>, line: &Line) -> Vec<Text> {
    let Some(reflex) = line.reflex() else {
        return Vec::new();
    };
    let mut names: Vec<(String, Option<&String>)> = line
        .answers
        .0
        .keys()
        .filter_map(|question| match QuestionId::parse(question) {
            Ok(QuestionId::Arg(of, arg)) if of == *reflex => {
                Some((arg.to_string(), Some(question)))
            }
            _ => None,
        })
        .collect();
    let held: Vec<&ArgName> = match &line.decision {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
            chosen.call.args.keys().collect()
        }
        Decision::Ask { asking, .. } => asking.args.keys().collect(),
        Decision::Abstain { .. } => Vec::new(),
    };
    for arg in held {
        if !names.iter().any(|(name, _)| name == arg.as_str()) {
            names.push((arg.to_string(), None));
        }
    }
    names
        .into_iter()
        .filter_map(|(arg, question)| value(sentence, line, &arg, question))
        .collect()
}

/// One argument's row: what was answered of it, or the value where no answer gave it; under it, what the value
/// stands on and where it came from.
fn value(
    sentence: Option<&Sentence>,
    line: &Line,
    arg: &str,
    question: Option<&String>,
) -> Option<Text> {
    let Of {
        value,
        basis,
        missing,
    } = of(&line.decision, arg);
    let answered = question
        .map(|question| sorted(&line.answers, question))
        .unwrap_or_default();
    let quiet = answered.first().is_none_or(|(top, _)| top == "unstated");
    // An argument the request says nothing of, which the call neither holds nor asks, is no row.
    if value.is_none() && missing.is_none() && quiet {
        return None;
    }
    let takes = takes(value, missing);
    let shown = value.map_or_else(String::new, |value| match value {
        Value::Flag => YES.to_owned(),
        Value::Pick { .. } => said(value.text().unwrap_or_default()),
        _ => value.text().unwrap_or_default().to_owned(),
    });
    let bound = line
        .step
        .iter()
        .flat_map(|step| &step.bound)
        .find(|bound| bound.arg.as_str() == arg);
    let shared = line
        .step
        .as_ref()
        .is_some_and(|step| step.shared.keys().any(|name| name.as_str() == arg));
    // A value a part folded into this step gave it.
    let n = line.step.as_ref().map_or(1, |step| step.n);
    let gave = sentence
        .iter()
        .flat_map(|sentence| &sentence.folded)
        .find(|folded| folded.into == n && folded.gave.iter().any(|name| name.as_str() == arg));
    let raw = || {
        if answered.is_empty() {
            alone(&shown, None)
        } else {
            answers(&answered, |key| answer_of(key, takes, &line.proposed))
        }
    };
    let (mut body, mut details): (Text, Vec<String>) = match (value, basis, bound) {
        // What an earlier step hands: where it comes from.
        (held, _, Some(bound)) => {
            let from = format!("taken from step {}, its {}", bound.from, bound.field);
            match held {
                Some(_) => (alone(&shown, None), vec![from]),
                None => (Text::from(from), Vec::new()),
            }
        }
        // A word the sentence says once for several steps.
        (Some(_), basis, None) if shared => (
            alone(&shown, basis.map(|basis| basis.sure().get())),
            vec![ONCE_FOR_SEVERAL.to_owned()],
        ),
        // A value a part folded into the step gave: read from that part's words.
        (Some(_), basis, None) if gave.is_some() => (
            alone(&shown, basis.map(|basis| basis.sure().get())),
            vec![format!(
                "{FROM} {}",
                said(gave.map(|folded| folded.text.as_str()).unwrap_or_default())
            )],
        ),
        // A value spelled out in words, or another part's: the yes that took it.
        (
            Some(value),
            Some(basis @ (Basis::Spelled { yes, .. } | Basis::Shared { yes, .. })),
            None,
        ) => (
            alone(&shown, Some(yes.get())),
            stands(basis, value, &shown, takes, &line.proposed),
        ),
        // A value read from the request: what was answered of it, and what it stands on.
        (Some(value), Some(basis), None) => {
            (raw(), stands(basis, value, &shown, takes, &line.proposed))
        }
        // A switch, which its own question gives; an argument that is asked.
        (Some(Value::Flag), None, None) | (None, _, None) => (raw(), Vec::new()),
        // A value the person gave at the prompt: what was read before it, and the answer.
        (Some(_), None, None) if quiet => (alone(&shown, None), vec![YOUR_ANSWER.to_owned()]),
        (Some(_), None, None) => {
            let mut details: Vec<String> = words_for(&line.decision, arg).into_iter().collect();
            details.push(format!("{YOUR_ANSWER}: {shown}"));
            (raw(), details)
        }
    };
    if let (Some(missing), None) = (missing, bound) {
        details.extend(super::because(missing));
    }
    if let Some(recalled) = line
        .recalled
        .iter()
        .find(|(name, _)| name.as_str() == arg)
        .map(|(_, values)| values)
    {
        details.push(format!("{RECALLED}: {}", recalled.join(" · ")));
    }
    for detail in details {
        body.push("\n").push(&detail);
    }
    Some(row(arg, body))
}

/// A value alone, weighted, with the share of what gave it where one did.
fn alone(shown: &str, p: Option<f64>) -> Text {
    let mut text = Text::new();
    text.roled(Role::Top, shown);
    if let Some(p) = p {
        text.push(&format!(" {}", share(p)));
    }
    text
}

/// Whether a line's step is one a playbook wrote.
fn from_a_playbook(line: &Line) -> bool {
    line.step.as_ref().is_some_and(|step| !step.from.is_empty())
}

/// The words of the request that answer an argument's ask, where the reading found them and took no value.
fn words_for(decision: &Decision, arg: &str) -> Option<String> {
    let left = match decision {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => &chosen.left,
        Decision::Ask { asking, .. } => &asking.left,
        Decision::Abstain { .. } => return None,
    };
    left.iter()
        .find(|run| matches!(&run.does, Does::Answers { arg: of } if of.as_str() == arg))
        .map(|run| format!("your words: {}", said(run.words.text().as_str())))
}

/// What a value read from the request stands on, a line each.
fn stands(
    basis: &Basis,
    value: &Value,
    shown: &str,
    takes: Takes,
    proposed: &[Proposed],
) -> Vec<String> {
    let words = |words: &Span| said(words.text().as_str());
    match basis {
        Basis::Ask { .. } => Vec::new(),
        Basis::Views {
            reader,
            yes,
            anchored,
            ..
        } => {
            let mut lines = vec![format!(
                "asked a second way: {shown} {}; the less sure of the two counts",
                share(reader.get())
            )];
            if let Some(yes) = yes {
                lines.push(format!("is it {shown}? yes {}", share(yes.get())));
            }
            if let Some(anchored) = anchored {
                lines.push(anchored_on(
                    &words(&anchored.words),
                    shown,
                    anchored.p.get(),
                ));
            }
            lines
        }
        Basis::View {
            view,
            p,
            other,
            words: held,
            how: by,
            anchored,
        } => {
            let second = match view {
                View::Ask => format!(
                    "asked a second way: {}",
                    answer_of(other.as_str(), takes, proposed)
                ),
                View::Reader => format!("asked a second way: {shown} {}", share(p.get())),
            };
            let mut lines = vec![
                second,
                format!("in your words: {}{}", words(held), how(*by)),
            ];
            if let Some(anchored) = anchored {
                lines.push(anchored_on(&words(held), shown, anchored.get()));
            }
            lines
        }
        Basis::Words {
            words: held,
            how: by,
            yes,
        } => vec![
            format!("in your words: {}{}", words(held), how(*by)),
            format!("is {} {shown}? yes {}", words(held), share(yes.get())),
        ],
        Basis::Spelled { form, .. } => match value {
            Value::Pick { span, .. } => {
                let way = match form {
                    Form::Aloud => "said aloud",
                    Form::Misspelt => "misspelt",
                    Form::Spaced => "typed with a space",
                };
                vec![format!("{way}: {}", words(span))]
            }
            _ => Vec::new(),
        },
        Basis::Only { yes } => vec![format!(
            "the only one of its kind: yes {}",
            share(yes.get())
        )],
        Basis::Text { p, others } => {
            let mut lines = vec![format!(
                "read from your own words: {shown} {}",
                share(p.get())
            )];
            if !others.is_empty() {
                lines.push(format!(
                    "other readings: {}",
                    others.iter().map(words).collect::<Vec<_>>().join(" · ")
                ));
            }
            lines
        }
        Basis::Shared { from, .. } => vec![format!(
            "said once, in {}: it is this part's too",
            said(from)
        )],
    }
}

/// How far the call holds all that was said, where it was held against the request; under the bar, the words
/// it leaves out.
fn holds(sentence: Option<&Sentence>, decision: &Decision) -> Option<Text> {
    let (whole, args, basis, left, unconsumed) = match decision {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => (
            chosen.whole?,
            &chosen.call.args,
            &chosen.basis,
            &chosen.left,
            &chosen.unconsumed,
        ),
        Decision::Ask { asking, .. } => (
            asking.whole?,
            &asking.args,
            &asking.basis,
            &asking.left,
            &asking.unconsumed,
        ),
        Decision::Abstain { .. } => return None,
    };
    let mut body = share(whole.get());
    let floor = sentence
        .and_then(|sentence| sentence.gate.as_ref())
        .and_then(Gate::whole);
    if floor.is_some_and(|floor| whole < floor) {
        // Words that answered an ask the person then settled are not left out; words that may be part of a
        // value are named by the line that says why the call waits.
        let mut out: Vec<&str> = left
            .iter()
            .filter(|run| run.left_out(args, basis))
            .map(|run| run.words.text().as_str())
            .collect();
        for span in unconsumed {
            if !out.iter().any(|words| words.contains(span.text().as_str())) {
                out.push(span.text().as_str());
            }
        }
        if !out.is_empty() {
            let named: Vec<String> = out.into_iter().map(said).collect();
            let _ = write!(body, ", leaves out {}", named.join(", "));
        }
        // Words about the result are named apart, never as left out; those that may be a text the call lacks
        // are named by the line that says why the call waits.
        let text_left = |words: &Span| {
            matches!(decision, Decision::Confirm { because, .. }
                if because.iter().any(|cap| matches!(cap, Cap::TextLeft { words: left, .. } if left == words)))
        };
        let about: Vec<String> = left
            .iter()
            .filter(|run| run.does == Does::Result && !text_left(&run.words))
            .map(|run| said(run.words.text().as_str()))
            .collect();
        if !about.is_empty() {
            let says = if about.len() == 1 {
                ABOUT_RESULT
            } else {
                ABOUT_RESULTS
            };
            let _ = write!(body, "; {} {says}", about.join(", "));
        }
    }
    Some(row(HOLDS, Text::from(body)))
}

/// What a rule did to the step: a part folded into it, how its words were settled, the playbook that wrote it,
/// the whole results it takes, what picks it.
fn rules(sentence: Option<&Sentence>, line: &Line) -> Vec<Text> {
    let mut rows = Vec::new();
    let n = line.step.as_ref().map_or(1, |step| step.n);
    let written = from_a_playbook(line) || line.expansion.is_some();
    for folded in sentence
        .iter()
        .flat_map(|sentence| &sentence.folded)
        .filter(|folded| folded.into == n && line.expansion.is_none())
    {
        let part = said(&folded.text);
        let (label, says) = match (&folded.picked, written) {
            (_, true) => (
                PLAYBOOK_STEP,
                format!("{part} is a step the playbook writes"),
            ),
            (Some(word), false) => (
                SAME_CALL,
                format!("{} picks this call: {part} adds to it", said(word)),
            ),
            (None, false) => (SAME_CALL, format!("{part} adds to this call")),
        };
        let mut body = Text::from(says);
        if !folded.gave.is_empty() {
            let gave: Vec<&str> = folded.gave.iter().map(ArgName::as_str).collect();
            body.push("\n")
                .push(&format!("it gives {}", gave.join(", ")));
        }
        rows.push(row(label, body));
    }
    if let Some(repair) = line.repair {
        let (label, says) = match repair {
            Repair::Narrowed => (READ_WITH, NARROWED),
            Repair::Spliced => (READ_WITH, SPLICED),
            Repair::Merged => (READ_WITH, MERGED),
            Repair::Corrected => (READ_WITH, CORRECTED),
            Repair::Split => (READ_APART, SPLIT),
        };
        rows.push(row(label, Text::from(says)));
    }
    if let Some(step) = &line.step {
        for from in &step.from {
            rows.push(row(
                PLAYBOOK,
                Text::from(format!("step {} of {}", from.step, from.playbook)),
            ));
        }
        // A value bound into an argument stands under the argument's row; a whole result has a row here.
        let call_holds = |arg: &ArgName| match &line.decision {
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
                chosen.call.args.contains_key(arg)
            }
            Decision::Ask { asking, missing } => {
                asking.args.contains_key(arg) || missing.iter().any(|m| m.arg == *arg)
            }
            Decision::Abstain { .. } => false,
        };
        let taken: Vec<String> = step
            .bound
            .iter()
            .filter(|bound| !call_holds(&bound.arg))
            .map(|bound| format!("{} from step {}", bound.field, bound.from))
            .collect();
        if !taken.is_empty() {
            rows.push(row(TAKES, Text::from(taken.join(", "))));
        }
        if let Some(when) = &step.when {
            rows.push(row(
                RUNS_IF,
                Text::from(format!(
                    "step {} yields {} {}",
                    when.step,
                    when.field,
                    said(when.is.as_str())
                )),
            ));
        }
    }
    rows
}

/// What became of the call, or what would: the word, the call as the plan printed it, and under it why.
fn outcome(sentence: Option<&Sentence>, line: &Line, tried: bool) -> Vec<Text> {
    let word = if let (Some(_), Decision::Run { .. } | Decision::Confirm { .. }) =
        (&line.review, &line.decision)
    {
        // A playbook's call is its plan, which waits for one yes: the steps say what became of it.
        WAITS
    } else if tried {
        match &line.decision {
            Decision::Run { .. } => RUNS,
            Decision::Confirm { .. } => WAITS,
            Decision::Ask { missing, .. } if all_bound(line, missing) => RUNS,
            Decision::Ask { .. } => ASKS_FOR,
            Decision::Abstain { .. } => REFUSED,
        }
    } else {
        became(line)
    };
    let mut head = Text::from(format!("→ {word}: "));
    let mut under: Vec<String> = Vec::new();
    match (&line.decision, &line.review) {
        (Decision::Run { chosen } | Decision::Confirm { chosen, .. }, Some(review)) => {
            head.append(own(chosen, &review.own));
            if !review.reason.is_empty() {
                under.push(plain(&review.reason));
            }
        }
        _ => said_of(sentence, line, word, &mut head, &mut under),
    }
    if let Some(contained) = &line.contained {
        held(&mut head, contained);
    }
    if !tried {
        if let Some(error) = &line.error {
            under.push(plain(error));
        }
        if let Some(step) = &line.step
            && step.status != Status::Declined
            && let Some(why) = &step.why
        {
            under.push(stopped(why, step.when.as_ref()));
        }
        under.extend(line.frames.iter().map(|frame| plain(frame)));
    }
    for line in under {
        head.push("\n  ").roled(Role::Weak, &line);
    }
    vec![head]
}

/// A decision's call as the plan printed it, on the end of `head`, and why it runs, waits, asks or is refused.
fn said_of(
    sentence: Option<&Sentence>,
    line: &Line,
    word: &str,
    head: &mut Text,
    under: &mut Vec<String>,
) {
    match &line.decision {
        Decision::Run { chosen } => {
            head.append(call(&chosen.call));
            if let Some(judged) = &chosen.judged {
                head.push("  ")
                    .roled(Role::Weak, &share(judged.confidence().get()));
                // Why it runs, where it did or would: a call that was stopped says what stopped it.
                if word == RUNS || word == RAN {
                    under.extend(clears(
                        sentence.and_then(|sentence| sentence.gate.as_ref()),
                        chosen.effect,
                        judged.weakest(),
                        chosen.whole.map(evoke_core::Prob::get),
                    ));
                }
            }
        }
        Decision::Confirm { chosen, prompt, .. } => {
            head.append(own(chosen, &prompt.own));
            if !prompt.reason.is_empty() {
                under.push(plain(&prompt.reason));
            }
        }
        Decision::Ask { asking, missing } => {
            let mut text = render(&Call {
                reflex: asking.reflex.clone(),
                args: asking.args.clone(),
            });
            let bound: Vec<&Missing> = missing
                .iter()
                .filter(|m| is_bound(line, m.arg.as_str()))
                .collect();
            let needs: Vec<&str> = missing
                .iter()
                .filter(|m| !is_bound(line, m.arg.as_str()))
                .map(|m| m.arg.as_str())
                .collect();
            for arg in &needs {
                let _ = write!(text, " {arg}=?");
            }
            head.append(call_of(&text, asking.reflex.as_str()));
            let takes: Vec<String> = bound
                .iter()
                .filter_map(|m| {
                    line.step
                        .iter()
                        .flat_map(|step| &step.bound)
                        .find(|bound| bound.arg == m.arg)
                })
                .map(|bound| format!("{} from {}", bound.field, bound.from))
                .collect();
            if !takes.is_empty() {
                head.push(&format!(" · takes {}", takes.join(", ")));
            }
            if !needs.is_empty() {
                under.push(format!("it needs {}", needs.join(", ")));
            }
        }
        Decision::Abstain { judgments, .. } => {
            let floor = sentence
                .and_then(|sentence| sentence.gate.as_ref())
                .map(Gate::route);
            let winner = judgments
                .iter()
                .find(|judgment| judgment.question == QuestionId::Route)
                .filter(|judgment| judgment.top.as_str() != "none");
            *head = Text::from(format!("→ {word}: "));
            match (winner, floor) {
                (Some(winner), Some(floor)) => head.push(&format!(
                    "{} is at {}, and a reflex is picked at {} or more",
                    winner.top.as_str(),
                    share(winner.p.get()),
                    share(floor.get())
                )),
                _ => head.push("no reflex serves it"),
            };
        }
    }
}

/// Whether an argument asked is one the plan binds a step's result into.
fn is_bound(line: &Line, arg: &str) -> bool {
    line.step
        .iter()
        .flat_map(|step| &step.bound)
        .any(|bound| bound.arg.as_str() == arg)
}

/// Whether every argument asked is bound.
fn all_bound(line: &Line, missing: &evoke_core::text::NonEmpty<Missing>) -> bool {
    missing.iter().all(|m| is_bound(line, m.arg.as_str()))
}

/// A call written out, the reflex's name carrying the weight.
fn call_of(text: &str, name: &str) -> Text {
    let mut call = Text::new();
    call.roled(Role::Call, name).push(&text[name.len()..]);
    call
}

/// Why a call ran: the bar of its effect with the least sure judgment, and the bar it was held against the
/// request at, where it was.
fn clears(
    gate: Option<&Gate>,
    effect: Effect,
    weakest: &Judgment,
    whole: Option<f64>,
) -> Option<String> {
    let gate = gate?;
    let (kind, floor) = match effect {
        Effect::Read => ("a read", gate.read()),
        Effect::Write => ("a write", gate.write()),
        Effect::Destructive => return None,
    };
    let about = match weakest.question {
        QuestionId::Route => WHICH.to_owned(),
        _ => weakest.about(),
    };
    let mut line = format!(
        "{kind} runs at {} or more, and its least sure judgment is {about}, {}",
        share(floor.get()),
        share(weakest.p.get())
    );
    if let (Some(whole), Some(floor)) = (whole, gate.whole()) {
        let _ = write!(
            line,
            "; it holds all you said at {}, and a call runs at {} or more",
            share(whole),
            share(floor.get())
        );
    }
    Some(line)
}

/// What became of a line's call, in the word `why` prints.
fn became(line: &Line) -> &'static str {
    let status = line.step.as_ref().map(|step| step.status).or(line.status);
    match (&line.decision, status) {
        (Decision::Abstain { .. }, _) | (_, Some(Status::Refused)) => REFUSED,
        (_, Some(Status::Skipped)) => SKIPPED,
        (_, Some(Status::Declined)) => DECLINED,
        (_, Some(Status::Unanswered)) => UNANSWERED,
        (_, Some(Status::Failed)) => FAILED,
        (Decision::Ask { .. }, _) => ASKED,
        (Decision::Confirm { .. }, Some(Status::Ran)) => RAN_AT_YES,
        (Decision::Run { .. }, Some(Status::Ran)) => RAN,
        (Decision::Run { .. } | Decision::Confirm { .. }, None) => {
            if line.cancelled {
                CANCELLED
            } else if line.error.is_some() {
                FAILED
            } else if line.result.is_some() {
                match line.decision {
                    Decision::Confirm { .. } => RAN_AT_YES,
                    _ => RAN,
                }
            } else {
                WAITS
            }
        }
    }
}

/// What the adapter was asked for the sentence: the plan file it came from, the questions and the rounds, or
/// that the cache answered.
fn closing(sentence: Option<&Sentence>, lines: &[Line]) -> Text {
    if let Some(pinned) = lines.iter().find_map(|line| line.pinned.as_ref()) {
        return Text::from(format!("from {}", pinned.file));
    }
    let asked = sentence.map_or_else(
        || {
            // A line logged before the log kept its sentence: its own calls, one round each.
            let mut asked = Asked::default();
            for line in lines {
                asked.more(&line.trace, line.trace.len());
            }
            asked
        },
        |sentence| sentence.asked.clone(),
    );
    match asked.adapter {
        Some(adapter) if asked.questions > 0 => {
            let questions = match asked.questions {
                1 => "1 question".to_owned(),
                n => format!("{n} questions"),
            };
            let rounds = match asked.rounds {
                1 => "1 round".to_owned(),
                n => format!("{n} rounds"),
            };
            Text::from(format!("{adapter} answered {questions} in {rounds}"))
        }
        _ => Text::from("answered from the cache"),
    }
}
