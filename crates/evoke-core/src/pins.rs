//! The questions `evoke` asks of a text on its own account, beside each argument's: each one's id and its words.
//! In: a reflex, an argument, what the question is about. Out: a `QuestionId` under `weave.` and a `Question`.
//!
//! An id names what it asks about, never a position that moves with the set: a reflex and an argument by their
//! names, joined by two underscores; a listed word by its place in its own list; words of the request by where
//! they stand in it. One text asked under two sets is asked each question under one id.

use indexmap::IndexMap;

use crate::adapter::{Choice, Key, Question, QuestionId, Text};
use crate::name::{ArgName, LocalName, WeaveName};
use crate::plan::{none, unstated};
use crate::text::{Clean, Span};
use crate::words::{Form, Spelled};

/// The most words of a list, and the most spelled forms, one argument is asked a yes or no about.
pub(crate) const MOST: usize = 3;

/// The reader's own question of a listed argument, which presupposes nothing the author's ask may.
const VIEW: &str = "Which of these does the request name?";
/// Its two sentinels: a word the list lacks, and no word at all.
const VIEW_NONE: &str = "One that is not in this list.";
const VIEW_UNSTATED: &str = "None: the request names none.";

/// The no of a yes or no on a listed word.
const ANOTHER: &str = "Another one, or none.";
/// The no of a yes or no on words of the request.
const OTHER_WORDS: &str = "Other words, or none.";

/// `weave.view_<reflex>__<argument>`: the second view of a listed argument.
pub(crate) fn view(reflex: &LocalName, arg: &ArgName) -> QuestionId {
    own(&format!("view_{}", pair(reflex, arg)))
}

/// `weave.is_<reflex>__<argument>_<n>`: a yes or no on the listed word at place `n` of the argument's list.
pub(crate) fn is(reflex: &LocalName, arg: &ArgName, place: usize) -> QuestionId {
    own(&format!("is_{}_{place}", pair(reflex, arg)))
}

/// `weave.said_<reflex>__<argument>_<start>_<end>`: a yes or no on a value the words at the span spell out.
pub(crate) fn said(reflex: &LocalName, arg: &ArgName, span: &Span) -> QuestionId {
    own(&format!("said_{}_{}", pair(reflex, arg), ends(span)))
}

/// `weave.only_<reflex>__<argument>_<start>_<end>`: a yes or no on the one candidate of the argument's kind.
pub(crate) fn only(reflex: &LocalName, arg: &ArgName, span: &Span) -> QuestionId {
    own(&format!("only_{}_{}", pair(reflex, arg), ends(span)))
}

/// `weave.left_<reflex>_<from>_<to>`: what a run of the request's words does, by the places of its first and
/// last word among the request's.
pub(crate) fn left(reflex: &LocalName, from: usize, to: usize) -> QuestionId {
    own(&format!("left_{reflex}_{from}_{to}"))
}

/// A reflex and its argument as one name. Two pairs of a set never share one: `compile` refuses the second.
pub(crate) fn pair(reflex: &LocalName, arg: &ArgName) -> String {
    format!("{reflex}__{arg}")
}

/// The argument a question of evoke's own is about, by its name; none for a question about no argument.
#[must_use]
pub fn argument(question: &QuestionId) -> Option<ArgName> {
    let QuestionId::Weave(name) = question else {
        return None;
    };
    // The kind, then how many places follow the pair.
    let kinds = [("view_", 0), ("is_", 1), ("said_", 2), ("only_", 2)];
    let (rest, places) = kinds
        .iter()
        .find_map(|(kind, places)| Some((name.as_str().strip_prefix(kind)?, *places)))?;
    let mut pair = rest;
    for _ in 0..places {
        let (head, place) = pair.rsplit_once('_')?;
        if place.is_empty() || !place.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        pair = head;
    }
    let (_, arg) = pair.split_once("__")?;
    ArgName::new(arg).ok()
}

fn ends(span: &Span) -> String {
    format!("{}_{}", span.start(), span.end())
}

/// The second view: the argument's own options under the reader's question and sentinels.
pub(crate) fn view_question(asked: &Choice) -> Question {
    let mut options: IndexMap<Key, Text> = asked
        .options()
        .iter()
        .filter(|(key, _)| **key != none() && Some(*key) != asked.otherwise())
        .map(|(key, text)| (key.clone(), text.clone()))
        .collect();
    options.insert(none(), Text::Plain(clean(VIEW_NONE)));
    Question::Choice(Choice::closed(
        clean(VIEW),
        options,
        (unstated(), Text::Plain(clean(VIEW_UNSTATED))),
    ))
}

/// A yes or no on one listed word, under the argument's own ask: the word's meaning, or another one.
pub(crate) fn is_question(ask: &Clean, meaning: &Clean) -> Question {
    Question::YesNo {
        ask: ask.clone(),
        yes: Text::Plain(meaning.clone()),
        no: Text::Plain(clean(ANOTHER)),
    }
}

/// A yes or no on a value the request spells out, under the argument's own ask: the value as typed with the
/// words that gave it, or other words.
pub(crate) fn said_question(ask: &Clean, spelled: &Spelled) -> Question {
    let how = match spelled.form {
        Form::Aloud => "said",
        Form::Misspelt | Form::Spaced => "written",
    };
    let yes = format!(
        "\u{ab}{}\u{bb}, {how} as \u{ab}{}\u{bb}.",
        spelled.typed,
        spelled.span.text()
    );
    Question::YesNo {
        ask: ask.clone(),
        yes: Text::Plain(Clean::new(&yes).unwrap_or_else(|_| spelled.typed.clone())),
        no: Text::Plain(clean(OTHER_WORDS)),
    }
}

/// A yes or no on the one candidate of an argument's kind, under the argument's own ask.
pub(crate) fn only_question(ask: &Clean, span: &Span) -> Question {
    let yes = format!("\u{ab}{}\u{bb}", span.text());
    Question::YesNo {
        ask: ask.clone(),
        yes: Text::Plain(Clean::new(&yes).unwrap_or_else(|_| span.text().clone())),
        no: Text::Plain(clean(OTHER_WORDS)),
    }
}

fn own(name: &str) -> QuestionId {
    QuestionId::Weave(WeaveName::new(name).expect("a name of this module is one"))
}

fn clean(text: &str) -> Clean {
    Clean::new(text).expect("the texts of this module are clean")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_about_an_argument_names_it() {
        let reflex = LocalName::new("roll_back").unwrap();
        let arg = ArgName::new("service").unwrap();
        let input = crate::text::Input::new("roll payments back").unwrap();
        let span = Span::of(&input, 5, 13).unwrap();
        for question in [
            view(&reflex, &arg),
            is(&reflex, &arg, 12),
            said(&reflex, &arg, &span),
            only(&reflex, &arg, &span),
        ] {
            assert_eq!(argument(&question), Some(arg.clone()), "{question}");
        }
        assert_eq!(argument(&QuestionId::Route), None);
    }

    #[test]
    fn an_id_names_the_reflex_and_the_argument() {
        let reflex = LocalName::new("roll_back").unwrap();
        let arg = ArgName::new("service").unwrap();
        assert_eq!(
            view(&reflex, &arg).to_string(),
            "weave.view_roll_back__service"
        );
        assert_eq!(
            is(&reflex, &arg, 2).to_string(),
            "weave.is_roll_back__service_2"
        );
    }
}
