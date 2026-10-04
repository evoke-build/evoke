//! A call held against the request it was read from. In: the reflex as the plan holds it, the values read. Out:
//! the call read back in the manifest's own words, and the question that holds it against the request.

use indexmap::IndexMap;

use crate::adapter::{Choice, Key, Question, QuestionId, Text};
use crate::call::{Call, Value, render};
use crate::manifest::Kind;
use crate::name::{ArgName, LocalName};
use crate::pins;
use crate::plan::{Active, Plan};
use crate::text::Clean;

/// How the reading stands to the request, and its three answers.
const READ_AS: &str = "The request was read as:";
const STANDS: &str = "How does that reading stand to the request?";
const WHOLE: &str = "The reading holds everything the request says.";
const LESS: &str = "The request says something more, which the reading leaves out.";
const OTHER: &str = "The request asks for something else.";

/// What is read back for an argument the request gives no value for, and for a flag it states.
const NOT_SAID: &str = "(not said)";
const YES: &str = "Yes.";

/// The answer that says the reading holds all the request says.
pub(crate) const HOLDS: &str = "whole";

/// The call as a sentence: what the reflex does, by its description whole, then each argument's ask with the
/// value the reading holds, or that none is said; a flag only where the request states it.
pub(crate) fn read_back(
    plan: &Plan,
    reflex: &LocalName,
    active: &Active,
    args: &IndexMap<ArgName, Value>,
) -> String {
    let summary = plan
        .route()
        .options()
        .get(reflex.as_str())
        .map_or_else(|| reflex.to_string(), |text| one_line(text.what().as_str()));
    let mut parts = vec![summary];
    for (arg, argument) in &active.args {
        let value = args.get(arg);
        match (&argument.kind, value.and_then(Value::text)) {
            (Kind::Flag, _) if value.is_some() => parts.push(format!("{} {YES}", argument.ask)),
            (Kind::Flag, _) => {}
            (Kind::Value { .. }, Some(text)) => parts.push(format!("{} {text}.", argument.ask)),
            (Kind::Value { .. }, None) => parts.push(format!("{} {NOT_SAID}", argument.ask)),
        }
    }
    parts.join(" ")
}

/// A description as one line: each line trimmed, a blank one dropped, the rest joined by a space.
pub(crate) fn one_line(what: &str) -> String {
    what.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The question that holds a call against its request: the call read back, under an id that stands for the
/// call, its reflex and the values it holds, so that one text is never answered under two calls, and one call
/// under one id however its values are spelled. None where the call's words cannot be asked about.
pub(crate) fn question(
    plan: &Plan,
    reflex: &LocalName,
    active: &Active,
    args: &IndexMap<ArgName, Value>,
) -> Option<(QuestionId, Question)> {
    let read_back = read_back(plan, reflex, active, args);
    let call = render(&Call {
        reflex: reflex.clone(),
        args: args.clone(),
    });
    let ask = Clean::new(&format!("{READ_AS} {read_back} {STANDS}")).ok()?;
    let options: IndexMap<Key, Text> = [(HOLDS, WHOLE), ("less", LESS), ("other", OTHER)]
        .into_iter()
        .map(|(answer, text)| Some((Key::new(answer).ok()?, Text::Plain(Clean::new(text).ok()?))))
        .collect::<Option<_>>()?;
    let choice = Choice::new(ask, options, None).ok()?;
    Some((pins::against(&call), Question::Choice(choice)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_reads_as_one_line() {
        assert_eq!(one_line("Lock the screen."), "Lock the screen.");
        assert_eq!(
            one_line("Lock the screen.\nEverything keeps running."),
            "Lock the screen. Everything keeps running."
        );
        assert_eq!(
            one_line("Lock the screen. \n\n  Everything keeps running.\n"),
            "Lock the screen. Everything keeps running."
        );
    }
}
