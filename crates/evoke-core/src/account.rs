//! The account of a text: the runs of its words that no value holds, and what each does in the request. In: the
//! input and the spans its values hold. Out: the runs, each with the question that asks what it does, and what an
//! answer says of it.

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
    })
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
}
