//! The questions `evoke` asks of a text on its own account, beside each argument's: each one's id and its words.
//! In: a reflex, an argument, what the question is about. Out: a `QuestionId` under `weave.` and a `Question`.
//!
//! An id names what it asks about, never a position that moves with the set: a reflex and an argument by their
//! names, joined by two underscores; a listed word by its place in its own list; words of the request by where
//! they stand in it. One text asked under two sets is asked each question under one id.

use indexmap::IndexMap;

use crate::adapter::{Choice, Key, Question, QuestionId, Text};
use crate::digest::Digest;
use crate::name::{ArgName, LocalName, WeaveName};
use crate::plan::{none, unstated};
use crate::text::{Clean, Span, identity};
use crate::words::{Form, Spelled};

/// The most words of a list, and the most spelled forms, one argument is asked a yes or no about.
pub(crate) const MOST: usize = 3;

/// The reader's own question of a listed argument, which presupposes nothing the author's ask may.
const VIEW: &str = "Which of these does the request name?";
/// Its two sentinels: a word the list lacks, and no word at all.
const VIEW_NONE: &str = "One that is not in this list.";
const VIEW_UNSTATED: &str = "None: the request names none.";

/// The choice anchored on words of the request, which listed word they name: the words between guillemets stand
/// for `{words}`.
const WORDS: &str = "Which of these do the words {words} name?";
/// Its sentinel for words that name no listed word; the one for a word the list lacks is the second view's.
const WORDS_UNSTATED: &str = "None: the words name none.";

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

/// `weave.meant_<reflex>__<argument>_<start>_<end>`: a yes or no on the value the words at the span spell out,
/// as its argument's kind reads it and its examples shape it.
pub(crate) fn meant(reflex: &LocalName, arg: &ArgName, span: &Span) -> QuestionId {
    own(&format!("meant_{}_{}", pair(reflex, arg), ends(span)))
}

/// `weave.only_<reflex>__<argument>_<start>_<end>`: a yes or no on the one candidate of the argument's kind.
pub(crate) fn only(reflex: &LocalName, arg: &ArgName, span: &Span) -> QuestionId {
    own(&format!("only_{}_{}", pair(reflex, arg), ends(span)))
}

/// `weave.words_<reflex>__<argument>_<start>_<end>`: which listed word of the argument the words at the span name.
pub(crate) fn words(reflex: &LocalName, arg: &ArgName, span: &Span) -> QuestionId {
    own(&format!("words_{}_{}", pair(reflex, arg), ends(span)))
}

/// `weave.does_<reflex>_<from>_<to>`: what a run of the request's words does, by the places of its first and
/// last word among the request's.
pub(crate) fn does(reflex: &LocalName, from: usize, to: usize) -> QuestionId {
    own(&format!("does_{reflex}_{from}_{to}"))
}

/// `weave.first_<reflex>__<argument>_<held>`: where a text typed without quotes begins, among the words no
/// typed value holds; `held` stands for those values' places, 0 when there is none.
pub(crate) fn first(reflex: &LocalName, arg: &ArgName, held: u32) -> QuestionId {
    own(&format!("first_{}_{held}", pair(reflex, arg)))
}

/// `weave.run_<reflex>__<argument>_<held>`: the text chosen whole among the runs of those words.
pub(crate) fn run(reflex: &LocalName, arg: &ArgName, held: u32) -> QuestionId {
    own(&format!("run_{}_{held}", pair(reflex, arg)))
}

/// `weave.word_<reflex>__<argument>_<n>`: whether the word at place `n` of the request is part of the text.
pub(crate) fn word(reflex: &LocalName, arg: &ArgName, place: usize) -> QuestionId {
    own(&format!("word_{}_{place}", pair(reflex, arg)))
}

/// `weave.last_<reflex>__<argument>_<from>_<held>`: where the text ends, that begins at the word at `from`.
pub(crate) fn last(reflex: &LocalName, arg: &ArgName, from: usize, held: u32) -> QuestionId {
    own(&format!("last_{}_{from}_{held}", pair(reflex, arg)))
}

/// `weave.text_<reflex>__<argument>_<readings>`: the last choice among a text's readings, which `readings`
/// stands for.
pub(crate) fn last_choice(reflex: &LocalName, arg: &ArgName, readings: u32) -> QuestionId {
    own(&format!("text_{}_{readings}", pair(reflex, arg)))
}

/// `weave.against_<call>`: a call held against the request, where `call` stands for the call on one line,
/// however the request spells its values.
pub(crate) fn against(call: &str) -> QuestionId {
    own(&format!("against_{}", short(identity(call).as_str())))
}

/// A number that stands for a text in an id: the head of its digest.
pub(crate) fn short(text: &str) -> u32 {
    let digest = Digest::of(text.as_bytes()).to_string();
    let hex: String = digest
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .chars()
        .take(8)
        .collect();
    u32::from_str_radix(&hex, 16).unwrap_or(0)
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
    let kinds = [
        ("view_", 0),
        ("is_", 1),
        ("meant_", 2),
        ("only_", 2),
        ("words_", 2),
        ("first_", 1),
        ("run_", 1),
        ("word_", 1),
        ("last_", 2),
        ("text_", 1),
    ];
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

/// The choice anchored on the words at the span: the argument's own options as the second view offers them, asked
/// of the words themselves, a word the list lacks, and none.
pub(crate) fn words_question(asked: &Choice, span: &Span) -> Question {
    let mut options: IndexMap<Key, Text> = asked
        .options()
        .iter()
        .filter(|(key, _)| **key != none() && Some(*key) != asked.otherwise())
        .map(|(key, text)| (key.clone(), text.clone()))
        .collect();
    options.insert(none(), Text::Plain(clean(VIEW_NONE)));
    let ask = WORDS.replace("{words}", &format!("\u{ab}{}\u{bb}", span.text()));
    Question::Choice(Choice::closed(
        Clean::new(&ask).unwrap_or_else(|_| clean(VIEW)),
        options,
        (unstated(), Text::Plain(clean(WORDS_UNSTATED))),
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
pub(crate) fn meant_question(ask: &Clean, spelled: &Spelled) -> Question {
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
            meant(&reflex, &arg, &span),
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
