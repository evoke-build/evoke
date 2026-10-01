//! The account of a text: the runs of its words that no value holds, and what each does in the request; and the
//! texts it puts in quotes that no value holds. In: the input and the spans its values hold. Out: the runs, each
//! with the question that asks what it does and what an answer says of it; whether a run follows a value; the
//! quotes.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::{Choice, Key, Prob, Question, Text};
use crate::manifest::{Argument, Kind};
use crate::name::ArgName;
use crate::text::{Clean, Input, Span};
use crate::words::{self, Token};

/// The words that join a value to what is asked: a run of words left over is cut before each.
pub(crate) const INTRODUCES: [&str; 25] = [
    "about",
    "regarding",
    "concerning",
    "re",
    "called",
    "named",
    "titled",
    "labelled",
    "labeled",
    "entitled",
    "subject",
    "saying",
    "that",
    "for",
    "to",
    "from",
    "into",
    "in",
    "on",
    "at",
    "with",
    "by",
    "as",
    "until",
    "till",
];

/// The marks that close a run and are no part of it.
const CLOSING: [char; 6] = [',', '.', ';', ':', '!', '?'];

/// The most runs a text is asked about.
const MOST: usize = 6;

/// What the question asks of a run, and what each answer says.
const ACTION: &str = "They say what to do, or to what.";
const ANSWER: &str = "They answer:";
const COURTESY: &str = "They are politeness, a reason or an aside, and ask for nothing.";
const MORE: &str = "They ask for another thing as well.";

/// The marks that open and close a text in quotes, each with its pair: double, single and typographic. A mark
/// opens only at a word's start and closes only at a word's end, so an apostrophe inside a word, «Sam's», is none.
const QUOTES: [(char, char); 6] = [
    ('"', '"'),
    ('\u{201c}', '\u{201d}'),
    ('\u{2018}', '\u{2019}'),
    ('\'', '\''),
    ('\u{ab}', '\u{bb}'),
    ('\u{201e}', '\u{201c}'),
];

/// A run of the input's words that no value holds: the places of its first and last word among the input's,
/// and the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Run {
    pub from: usize,
    pub to: usize,
    pub words: Span,
}

/// What a run of words does in the request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "does", rename_all = "lowercase")]
pub enum Does {
    /// It says what to do, or to what.
    Action,
    /// It answers an argument's ask.
    Answers { arg: ArgName },
    /// It is politeness, a reason or an aside, and asks for nothing.
    Nothing,
    /// It asks for another thing as well.
    More,
}

/// A run of words no value holds, with what it does and how sure that is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Left {
    pub words: Span,
    #[serde(flatten)]
    pub does: Does,
    pub p: Prob,
    /// Whether its words answer an argument that holds a typed value and stand right after that value's words,
    /// so that the value may be cut short of them: found by code where the words are read.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
}

/// The runs of the input that none of the spans holds, six at most: cut where a held word stands and before a
/// word that introduces a value, the words that carry nothing dropped at both ends, a run of such words alone
/// left out.
pub(crate) fn runs(input: &Input, held: &[&Span]) -> Vec<Run> {
    let tokens = words::tokens(input.as_str());
    let covered = |token: &Token| {
        held.iter()
            .any(|span| token.start < span.end() && token.end > span.start())
    };
    let mut runs = Vec::new();
    let mut from: Option<usize> = None;
    for (i, token) in tokens.iter().enumerate() {
        match from {
            _ if covered(token) => {
                runs.extend(
                    from.take()
                        .and_then(|from| run(input, &tokens, from, i - 1)),
                );
            }
            None => from = Some(i),
            Some(open) if i > open && INTRODUCES.contains(&token.plain.as_str()) => {
                runs.extend(run(input, &tokens, open, i - 1));
                from = Some(i);
            }
            Some(_) => {}
        }
    }
    if let (Some(open), Some(last)) = (from, tokens.len().checked_sub(1)) {
        runs.extend(run(input, &tokens, open, last));
    }
    runs.truncate(MOST);
    runs
}

/// The run between two places, less the words that carry nothing at its ends and the marks that close it.
fn run(input: &Input, tokens: &[Token], from: usize, to: usize) -> Option<Run> {
    let carries = |i: &usize| !words::function(&tokens[*i].plain);
    let first = (from..=to).find(carries)?;
    let last = (from..=to).rev().find(carries)?;
    let chars: Vec<char> = input.as_str().chars().collect();
    let mut end = tokens[last].end;
    while end > tokens[first].start && CLOSING.contains(&chars[end - 1]) {
        end -= 1;
    }
    Some(Run {
        from: first,
        to: last,
        words: Span::of(input, tokens[first].start, end)?,
    })
}

/// The question of a run: what its words do, among saying what to do, answering one of the reflex's asks,
/// asking for nothing, and asking for another thing.
pub(crate) fn question(run: &Run, args: &IndexMap<ArgName, Argument>) -> Question {
    let mut options: IndexMap<Key, Text> = IndexMap::new();
    options.insert(key("action"), plain(ACTION));
    for (name, argument) in args {
        if argument.kind != Kind::Flag {
            options.insert(
                key(&format!("arg:{name}")),
                plain(&format!("{ANSWER} {}", argument.ask)),
            );
        }
    }
    options.insert(key("courtesy"), plain(COURTESY));
    options.insert(key("more"), plain(MORE));
    let ask = format!(
        "In the request, what do the words \u{ab}{}\u{bb} do?",
        run.words.text()
    );
    Question::Choice(
        Choice::new(clean(&ask), options, None).expect("a choice without a sentinel is one"),
    )
}

/// What an answer says a run does: the first of its most probable options.
pub(crate) fn read(run: &Run, question: &Question, answer: &IndexMap<Key, Prob>) -> Option<Left> {
    let Question::Choice(choice) = question else {
        return None;
    };
    let (top, p) = choice
        .options()
        .keys()
        .map(|key| (key, answer.get(key).copied().unwrap_or(Prob::ZERO)))
        .reduce(|best, next| if next.1 > best.1 { next } else { best })?;
    let does = match top.as_str() {
        "action" => Does::Action,
        "courtesy" => Does::Nothing,
        "more" => Does::More,
        other => Does::Answers {
            arg: ArgName::new(other.strip_prefix("arg:")?).ok()?,
        },
    };
    Some(Left {
        words: run.words.clone(),
        does,
        p,
        cut: false,
    })
}

/// Whether a run stands right after a value's words: only words that carry nothing, and marks, between the two.
pub(crate) fn follows(input: &Input, value: &Span, run: &Span) -> bool {
    run.start() >= value.end()
        && words::tokens(input.as_str())
            .iter()
            .filter(|token| token.start >= value.end() && token.end <= run.start())
            .all(|token| token.plain.is_empty() || words::function(&token.plain))
}

/// The texts the input puts in quotes that none of the spans holds whole, each with its marks: a mark of
/// `QUOTES` at a word's start opens, its pair at a word's end closes, and the words between are the text.
pub(crate) fn quotes(input: &Input, held: &[&Span]) -> Vec<Span> {
    let chars: Vec<char> = input.as_str().chars().collect();
    let letter = |i: usize| chars.get(i).is_some_and(|c| c.is_alphanumeric());
    let space = |i: usize| chars.get(i).is_none_or(|c| c.is_whitespace());
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let opened = QUOTES
            .iter()
            .find(|(open, _)| *open == chars[i])
            .filter(|_| (i == 0 || !letter(i - 1)) && !space(i + 1));
        let closed = opened.and_then(|(_, close)| {
            (i + 2..chars.len()).find(|&j| chars[j] == *close && !space(j - 1) && !letter(j + 1))
        });
        let Some(end) = closed else {
            i += 1;
            continue;
        };
        // The words inside the marks, which a value holds whole or not at all.
        let unheld = !held
            .iter()
            .any(|span| span.start() <= i + 1 && span.end() >= end);
        if unheld {
            found.extend(Span::of(input, i, end + 1));
        }
        i = end + 1;
    }
    found
}

fn key(text: &str) -> Key {
    Key::new(text).expect("a key of this module has text")
}

fn plain(text: &str) -> Text {
    Text::Plain(clean(text))
}

fn clean(text: &str) -> Clean {
    Clean::new(text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn left(input: &str, held: &[(usize, usize)]) -> Vec<String> {
        let input = Input::new(input).unwrap();
        let spans: Vec<Span> = held
            .iter()
            .map(|(start, end)| Span::of(&input, *start, *end).unwrap())
            .collect();
        let held: Vec<&Span> = spans.iter().collect();
        runs(&input, &held)
            .into_iter()
            .map(|run| format!("{}-{} {}", run.from, run.to, run.words.text()))
            .collect()
    }

    #[test]
    fn a_run_is_cut_at_a_held_word_and_before_a_word_that_introduces() {
        // «10 minutes» is held
        assert_eq!(
            left(
                "set a timer for 10 minutes called tea, please.",
                &[(16, 26)]
            ),
            ["0-2 set a timer", "6-7 called tea"]
        );
    }

    #[test]
    fn words_that_carry_nothing_make_no_run() {
        assert_eq!(left("and then the den", &[(13, 16)]), Vec::<String>::new());
        assert_eq!(left("please, to me", &[]), Vec::<String>::new());
    }

    #[test]
    fn quotes_are_found_at_a_word_s_edges() {
        let found = |text: &str| -> Vec<String> {
            let input = Input::new(text).unwrap();
            quotes(&input, &[])
                .into_iter()
                .map(|span| span.text().to_string())
                .collect()
        };
        assert_eq!(found("send it 'running 10 late'"), ["'running 10 late'"]);
        assert_eq!(
            found("\u{201c}tea\u{201d} and \u{2018}eggs\u{2019}"),
            ["\u{201c}tea\u{201d}", "\u{2018}eggs\u{2019}"]
        );
        assert_eq!(found("note \"a\" and \"b\"."), ["\"a\"", "\"b\""]);
        assert_eq!(found("'it's late', she said"), ["'it's late'"]);
        assert!(
            found("don't lock Sam's laptop").is_empty(),
            "an apostrophe inside a word"
        );
        assert!(
            found("the kids' room").is_empty(),
            "a closing mark with no opening one"
        );
        assert!(
            found("' spaced '").is_empty(),
            "a mark before a space opens nothing"
        );
        let input = Input::new("send it 'running 10 late'").unwrap();
        let inner = Span::of(&input, 9, 24).unwrap();
        assert!(
            quotes(&input, &[&inner]).is_empty(),
            "a value that holds the words inside"
        );
        let part = Span::of(&input, 9, 16).unwrap();
        assert_eq!(
            quotes(&input, &[&part]).len(),
            1,
            "a value holding part of them"
        );
    }

    #[test]
    fn a_run_follows_a_value_across_words_that_carry_nothing() {
        let input = Input::new("friday, 2 ocotber and the rest, to be sure").unwrap();
        let value = Span::of(&input, 0, 6).unwrap();
        let after = Span::of(&input, 8, 17).unwrap();
        let far = Span::of(&input, 26, 30).unwrap();
        assert!(follows(&input, &value, &after));
        assert!(!follows(&input, &value, &far), "«2 ocotber» carries");
        let input = Input::new("the next 3 hours").unwrap();
        let value = Span::of(&input, 9, 16).unwrap();
        let before = Span::of(&input, 4, 8).unwrap();
        assert!(!follows(&input, &value, &before), "before the value");
    }
}
