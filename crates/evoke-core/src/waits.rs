//! Why a call waits for a yes, in one line a person reads under the call. In: the chosen call and what holds it;
//! a playbook's own line and the effect its steps reach. Out: the line, one phrase a cause.

use crate::account::Does;
use crate::call::quoted;
use crate::decide::{Basis, Cap, Chosen};
use crate::manifest::Effect;
use crate::text::{Clean, Span};

/// A call that cannot be undone.
const UNDONE: &str = "it cannot be undone, so it always waits for a yes";
/// An adapter that ships no bars.
const NO_BARS: &str = "the adapter ships no bars, so nothing runs on its own";
/// A judgment under the bar of the call's effect: the effect as named below, and its bar.
const UNDER: &str = "{effect} runs at {floor} or more";
/// The two effects a bar is set for.
const A_READ: &str = "a read";
const A_WRITE: &str = "a write";
/// A value read from words that do not state it as it is listed or typed: the argument, and the words.
const READ_FROM: &str = "{arg} was read from {words}";
/// A text read from words typed without quotes: the argument.
const UNQUOTED: &str = "{arg} was typed without quotes, which always waits for a yes";
/// Words that ask for another thing than the call does.
const MORE: &str = "{words} asks for another thing";
/// Words right after a value that answer its argument: the words, and the argument.
const CUT: &str = "{words} may be part of the {arg}";
/// A text in quotes that no value of the call holds: the text with its marks.
const QUOTED: &str = "{words} is in quotes and the call does not hold it";
/// Words read as about the result, in a call that takes a text and holds none: the words, and the argument.
const TEXT_LEFT: &str = "{words} may be the {arg}";
/// A call that holds less than the request says: how far it holds all of it, and the bar.
const LESS: &str = "it holds all you said at {p}, and a call runs at {floor} or more";
/// The words such a call leaves out, after the phrase above.
const LEAVES_OUT: &str = "; it leaves out {words}";
/// The words of such a call that say what is wanted from the result, after the phrases above: one run, and
/// several.
const ABOUT_RESULT: &str = "; {words} says what is wanted from the result";
const ABOUT_RESULTS: &str = "; {words} say what is wanted from the result";
/// A part out of the plan that may add a detail to the call.
const DETAIL: &str = "{words} may add a detail this call does not hold";
/// Words that match nothing alone, read as the call's reflex in the whole request: the words, the reflex, its share.
const NAMED: &str = "{words} matches nothing alone, and the whole request reads it as {reflex} at {p}, which always waits for a yes";
/// A call held for a cause that has no words of its own.
const WAITS: &str = "it waits for a yes";
/// A playbook, whose plan is reviewed whole.
const PLAYBOOK: &str = "a playbook's plan waits for one yes over its steps";
/// The effect a playbook's steps reach, over the playbook's own.
const REACH: &str = "its steps reach {effect}";
/// Between two causes, and between two runs of words.
const THEN: &str = "; ";
const AND: &str = ", ";

/// A phrase with each `{name}` replaced by its value.
fn said(phrase: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(phrase.to_owned(), |phrase, (name, value)| {
            phrase.replace(&format!("{{{name}}}"), value)
        })
}

/// A share as a line shows it.
fn share(p: f64) -> String {
    format!("{p:.2}")
}

/// The line under a call that waits: each cause in the order the decision lists them.
pub(crate) fn reason(chosen: &Chosen, because: &[Cap]) -> String {
    let causes: Vec<String> = because
        .iter()
        .map(|cap| cause(chosen, because, cap))
        .collect();
    causes.join(THEN)
}

fn cause(chosen: &Chosen, because: &[Cap], cap: &Cap) -> String {
    match cap {
        Cap::Destructive => UNDONE.to_owned(),
        Cap::NoGate => NO_BARS.to_owned(),
        Cap::UnderFloor { floor, .. } => {
            let effect = match chosen.effect {
                Effect::Read => A_READ,
                Effect::Write | Effect::Destructive => A_WRITE,
            };
            said(UNDER, &[("effect", effect), ("floor", &share(floor.get()))])
        }
        Cap::OneView { arg } | Cap::Respelt { arg } => from(chosen, arg).map_or_else(
            || WAITS.to_owned(),
            |words| {
                said(
                    READ_FROM,
                    &[("arg", arg.as_str()), ("words", &quoted(words.as_str()))],
                )
            },
        ),
        Cap::TextRead { arg } => said(UNQUOTED, &[("arg", arg.as_str())]),
        Cap::More { words } => said(MORE, &[("words", &quoted(words.text().as_str()))]),
        Cap::Cut { arg, words } => said(
            CUT,
            &[
                ("words", &quoted(words.text().as_str())),
                ("arg", arg.as_str()),
            ],
        ),
        Cap::Quoted { words } => said(QUOTED, &[("words", words.text().as_str())]),
        Cap::TextLeft { arg, words } => said(
            TEXT_LEFT,
            &[
                ("words", &quoted(words.text().as_str())),
                ("arg", arg.as_str()),
            ],
        ),
        Cap::Whole { p, floor } => {
            let mut line = said(
                LESS,
                &[("p", &share(p.get())), ("floor", &share(floor.get()))],
            );
            let out = left_out(chosen);
            if !out.is_empty() {
                let named: Vec<String> = out.into_iter().map(quoted).collect();
                line.push_str(&said(LEAVES_OUT, &[("words", &named.join(AND))]));
            }
            // Words that may be a text the call lacks have their own phrase, and are not said to be about the
            // result beside it.
            let text_left = |words: &Span| {
                because
                    .iter()
                    .any(|cap| matches!(cap, Cap::TextLeft { words: left, .. } if left == words))
            };
            let about: Vec<String> = chosen
                .left
                .iter()
                .filter(|run| run.does == Does::Result && !text_left(&run.words))
                .map(|run| quoted(run.words.text().as_str()))
                .collect();
            if !about.is_empty() {
                let phrase = if about.len() == 1 {
                    ABOUT_RESULT
                } else {
                    ABOUT_RESULTS
                };
                line.push_str(&said(phrase, &[("words", &about.join(AND))]));
            }
            line
        }
        Cap::Detail { words } => said(DETAIL, &[("words", &quoted(words))]),
        // The person's words last: nothing in them is read as a place to fill.
        Cap::Named { words, p } => said(
            NAMED,
            &[
                ("reflex", chosen.call.reflex.as_str()),
                ("p", &share(p.get())),
                ("words", &quoted(words)),
            ],
        ),
    }
}

/// The words a call leaves out (`Left::left_out`), and the values typed that no argument took.
fn left_out(chosen: &Chosen) -> Vec<&str> {
    let mut out: Vec<&str> = chosen
        .left
        .iter()
        .filter(|run| run.left_out(&chosen.call.args, &chosen.basis))
        .map(|run| run.words.text().as_str())
        .collect();
    for span in &chosen.unconsumed {
        if !out.iter().any(|words| words.contains(span.text().as_str())) {
            out.push(span.text().as_str());
        }
    }
    out
}

/// The words of the request a value was read from, where it is not the words as they stand.
fn from<'a>(chosen: &'a Chosen, arg: &crate::name::ArgName) -> Option<&'a Clean> {
    use crate::call::Value;
    match (chosen.basis.get(arg)?, chosen.call.args.get(arg)?) {
        (Basis::View { words, .. } | Basis::Words { words, .. }, _) => Some(words.text()),
        (Basis::Spelled { .. }, Value::Pick { span, .. }) => Some(span.text()),
        _ => None,
    }
}

/// The line under a playbook's call: what holds the call itself, or that a playbook's plan is reviewed whole;
/// then the effect its steps reach, where it is over the playbook's own.
pub(crate) fn reviewed(own: &str, reach: Option<Effect>) -> String {
    let mut causes = vec![if own.is_empty() {
        PLAYBOOK.to_owned()
    } else {
        own.to_owned()
    }];
    if let Some(effect) = reach {
        causes.push(said(REACH, &[("effect", &effect.to_string())]));
    }
    causes.join(THEN)
}
