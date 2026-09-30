//! A text typed without quotes, read by its bounds. In: the input, the argument's ask, the words other values
//! hold, the answers so far. Out: the questions to ask next, the text with the share that chose it, or nothing.
//!
//! The text is a run of the input's words, copied. Its first word and its last are asked apart, its words are
//! judged one by one, the run is chosen whole; then the readings stand side by side as one last choice.

use indexmap::IndexMap;

use crate::account::INTRODUCES;
use crate::adapter::{Choice, Key, Prob, Question, QuestionId, Text};
use crate::decide::Answers;
use crate::name::{ArgName, LocalName};
use crate::pins;
use crate::plan::{unstated, unstated_text};
use crate::text::{Clean, Input, Span};
use crate::words::{self, Token};

/// The most words of the input a text is looked for among.
const MOST_WORDS: usize = 48;

/// The most runs of words offered as the text chosen whole, the shortest first.
const MOST_RUNS: usize = 250;

/// How many words each side of a word are shown with it.
const BESIDE: usize = 2;

/// At this share a word is part of the text.
const WORD: f64 = 0.5;

/// At this share the last choice takes a reading.
const FINAL: f64 = 0.5;

/// The marks that close a text and are no part of it.
const CLOSING: [char; 6] = [',', '.', ';', ':', '!', '?'];

/// What follows the action's name and is no part of the text: «note that …», «note this: …».
const AFTER_NAME: [&str; 2] = ["that", "this"];

/// A note addressed to oneself: «note to self …».
const TO_SELF: [&str; 2] = ["to", "self"];

/// Whether a word is part of the text, and its two answers.
const PART: &str = "part of it?";
const IS_PART: &str = "It is part of it.";
const NO_PART: &str = "It is no part of it.";

/// The last choice's answer for a text that none of the readings is.
const OTHER: &str = "Other words than these.";

/// One argument's text as it is looked for: the input and its words, the reflex and the argument, the
/// argument's ask, the places of the words no other value holds, and what makes their set its own.
pub(crate) struct Sought<'a> {
    pub input: &'a Input,
    pub reflex: &'a LocalName,
    pub arg: &'a ArgName,
    pub ask: &'a Clean,
    tokens: Vec<Token>,
    free: Vec<usize>,
    held: u32,
}

/// What looking for a text came to.
pub(crate) enum Found {
    /// Questions were put, and the text waits for their answers.
    Open,
    /// The text, the last choice that took it with its share, and the readings beside it.
    Text {
        span: Span,
        chose: (QuestionId, Key),
        p: Prob,
        others: Vec<Span>,
    },
    /// The request holds no text the answers agree on.
    None,
}

impl<'a> Sought<'a> {
    /// The text of an argument, looked for among the words no typed value holds.
    pub(crate) fn new(
        input: &'a Input,
        reflex: &'a LocalName,
        arg: &'a ArgName,
        ask: &'a Clean,
        held: &[&Span],
    ) -> Self {
        let tokens = words::tokens(input.as_str());
        let free = tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| {
                !held
                    .iter()
                    .any(|span| token.start < span.end() && token.end > span.start())
            })
            .map(|(i, _)| i)
            .take(MOST_WORDS)
            .collect();
        let places: Vec<String> = held
            .iter()
            .map(|span| format!("{}-{}", span.start(), span.end()))
            .collect();
        Self {
            input,
            reflex,
            arg,
            ask,
            tokens,
            free,
            held: if places.is_empty() {
                0
            } else {
                pins::short(&places.join(","))
            },
        }
    }

    /// The words from one place to another as they stand in the input, the marks that close them dropped.
    fn span(&self, from: usize, to: usize) -> Option<Span> {
        let chars: Vec<char> = self.input.as_str().chars().collect();
        let start = self.tokens.get(from)?.start;
        let mut end = self.tokens.get(to)?.end;
        while end > start && CLOSING.contains(&chars[end - 1]) {
            end -= 1;
        }
        Span::of(self.input, start, end)
    }

    /// The first round's questions: where the text begins, the run chosen whole, and each word by itself.
    fn first_round(&self) -> IndexMap<QuestionId, Question> {
        let mut asked = IndexMap::new();
        let last = self.tokens.len().saturating_sub(1);
        let begins: IndexMap<Key, Text> = self
            .free
            .iter()
            .filter_map(|&i| Some((key(&format!("s{i}")), plain(self.span(i, last)?.text()))))
            .collect();
        asked.insert(
            pins::first(self.reflex, self.arg, self.held),
            closed(self.ask, begins),
        );
        let mut runs: Vec<(Key, Span)> = Vec::new();
        for (a, &from) in self.free.iter().enumerate() {
            for (b, &to) in self.free.iter().enumerate().skip(a) {
                if to - from != b - a {
                    break;
                }
                runs.extend(
                    self.span(from, to)
                        .map(|span| (key(&format!("{from}-{to}")), span)),
                );
            }
        }
        runs.sort_by_key(|(_, span)| span.text().as_str().encode_utf16().count());
        runs.truncate(MOST_RUNS);
        asked.insert(
            pins::run(self.reflex, self.arg, self.held),
            closed(
                self.ask,
                runs.into_iter()
                    .map(|(key, span)| (key, plain(span.text())))
                    .collect(),
            ),
        );
        for &i in &self.free {
            let from = i.saturating_sub(BESIDE);
            let to = (i + BESIDE).min(last);
            let Some(context) = self.span(from, to) else {
                continue;
            };
            let ask = format!(
                "{} Is the word \u{ab}{}\u{bb}, in \u{ab}{}\u{bb}, {PART}",
                self.ask,
                self.tokens[i].plain,
                context.text()
            );
            asked.insert(
                pins::word(self.reflex, self.arg, i),
                Question::YesNo {
                    ask: clean(&ask),
                    yes: Text::Plain(clean(IS_PART)),
                    no: Text::Plain(clean(NO_PART)),
                },
            );
        }
        asked
    }

    /// Where the text ends, asked once its first word is known: each run of free words from that word on.
    fn last_round(&self, from: usize) -> (QuestionId, Question) {
        let ends: IndexMap<Key, Text> = self
            .free
            .iter()
            .skip_while(|&&i| i < from)
            .enumerate()
            .take_while(|(n, i)| **i - from == *n)
            .filter_map(|(_, &i)| Some((key(&format!("e{i}")), plain(self.span(from, i)?.text()))))
            .collect();
        (
            pins::last(self.reflex, self.arg, from, self.held),
            closed(self.ask, ends),
        )
    }

    /// The best run of the words judged one by one: each word scored by the log odds of its yes, the run of
    /// greatest sum taken, none when no word passes. Words that do not follow one another break a run.
    fn decoded(&self, answers: &Answers<'_>) -> Option<(usize, usize)> {
        let mut best: Option<(f64, (usize, usize))> = None;
        let mut here = 0.0;
        let mut start = 0;
        let mut last: Option<usize> = None;
        for &i in &self.free {
            let id = pins::word(self.reflex, self.arg, i);
            let yes = answers
                .get(&id)
                .and_then(|answer| answer.get("yes"))
                .map_or(0.0, |yes| yes.get())
                .clamp(0.01, 0.99);
            let score = (yes / (1.0 - yes)).ln() - (WORD / (1.0 - WORD)).ln();
            if last.is_some_and(|last| i != last + 1) {
                here = 0.0;
            }
            if here <= 0.0 {
                here = score;
                start = i;
            } else {
                here += score;
            }
            last = Some(i);
            if here > best.map_or(0.0, |(sum, _)| sum) {
                best = Some((here, (start, i)));
            }
        }
        best.map(|(_, run)| run)
    }

    /// The text less what is no part of it: the action's own name before it, with the word or the mark that
    /// follows a name; a word that introduces a text, with the words before it that carry nothing; a
    /// courtesy after it.
    fn trimmed(&self, from: usize, to: usize) -> Option<Span> {
        let plain = |i: usize| self.tokens[i].plain.as_str();
        let chars: Vec<char> = self.input.as_str().chars().collect();
        let mark = |i: usize| {
            let token = &self.tokens[i];
            token.from == token.to
                && chars[token.start..token.end]
                    .iter()
                    .all(|c| "-:".contains(*c))
        };
        let (mut a, mut b) = (from, to);
        if a < b && words::names(plain(a), self.reflex.as_str()) {
            a += 1;
            if a < b && (AFTER_NAME.contains(&plain(a)) || mark(a)) {
                a += 1;
            } else if a + 1 < b && [plain(a), plain(a + 1)] == TO_SELF {
                a += 2;
            }
        }
        // A word that introduces the text, and the words before it that carry nothing: «its for the tea».
        let carries =
            (a..b).find(|&i| !words::function(plain(i)) || INTRODUCES.contains(&plain(i)));
        if let Some(i) = carries
            && INTRODUCES.contains(&plain(i))
        {
            a = i + 1;
        }
        while b > a && words::courtesy(plain(b)) {
            b -= 1;
        }
        self.span(a, b)
    }
}

/// The text of an argument, as far as the answers go: the first round's questions, then where it ends, then
/// the readings side by side; a question not asked yet is put in `open`. The first round is asked before it
/// is known whether the request states a text; the rounds after it only where it does.
pub(crate) fn read(
    sought: &Sought<'_>,
    answers: &Answers<'_>,
    stated: bool,
    open: &mut IndexMap<QuestionId, Question>,
) -> Found {
    if sought.free.is_empty() {
        return Found::None;
    }
    let first = sought.first_round();
    if first.keys().any(|id| !answers.contains_key(id)) {
        open.extend(first);
        return Found::Open;
    }
    if !stated {
        return Found::None;
    }
    let top = |id: &QuestionId, question: &Question| -> Option<(Key, Prob)> {
        let (Question::Choice(choice), Some(answer)) = (question, answers.get(id)) else {
            return None;
        };
        choice
            .options()
            .keys()
            .map(|key| (key.clone(), answer.get(key).copied().unwrap_or(Prob::ZERO)))
            .reduce(|best, next| if next.1 > best.1 { next } else { best })
            .filter(|(key, _)| *key != unstated())
    };
    let mut readings: Vec<Span> = Vec::new();
    let mut add = |span: Option<Span>| {
        if let Some(span) = span
            && !readings.contains(&span)
        {
            readings.push(span);
        }
    };
    let begins = pins::first(sought.reflex, sought.arg, sought.held);
    if let Some((begin, _)) = first
        .get(&begins)
        .and_then(|question| top(&begins, question))
        && let Some(from) = begin
            .as_str()
            .strip_prefix('s')
            .and_then(|i| i.parse::<usize>().ok())
    {
        let (id, question) = sought.last_round(from);
        if !answers.contains_key(&id) {
            open.insert(id, question);
            return Found::Open;
        }
        let to = top(&id, &question)
            .and_then(|(end, _)| end.as_str().strip_prefix('e')?.parse::<usize>().ok());
        add(to.and_then(|to| sought.span(from, to)));
    }
    add(sought
        .decoded(answers)
        .and_then(|(from, to)| sought.span(from, to)));
    let whole = pins::run(sought.reflex, sought.arg, sought.held);
    add(first
        .get(&whole)
        .and_then(|question| top(&whole, question))
        .and_then(|(run, _)| {
            let (from, to) = run.as_str().split_once('-')?;
            sought.span(from.parse().ok()?, to.parse().ok()?)
        }));
    if readings.is_empty() {
        return Found::None;
    }
    chosen(sought, &readings, answers, open)
}

/// The last choice among the readings: the one taken, where its share is enough.
fn chosen(
    sought: &Sought<'_>,
    readings: &[Span],
    answers: &Answers<'_>,
    open: &mut IndexMap<QuestionId, Question>,
) -> Found {
    let places: Vec<String> = readings
        .iter()
        .map(|span| format!("{}-{}", span.start(), span.end()))
        .collect();
    let id = pins::last_choice(sought.reflex, sought.arg, pins::short(&places.join(",")));
    let mut options: IndexMap<Key, Text> = readings
        .iter()
        .enumerate()
        .map(|(n, span)| {
            (
                key(&format!("r{}", n + 1)),
                Text::Plain(clean(&format!("\u{ab}{}\u{bb}", span.text()))),
            )
        })
        .collect();
    options.insert(key("other"), Text::Plain(clean(OTHER)));
    let question = closed(sought.ask, options);
    let Some(answer) = answers.get(&id) else {
        open.insert(id, question);
        return Found::Open;
    };
    let Question::Choice(choice) = &question else {
        return Found::None;
    };
    let best = choice
        .options()
        .keys()
        .map(|key| (key, answer.get(key).copied().unwrap_or(Prob::ZERO)))
        .reduce(|best, next| if next.1 > best.1 { next } else { best });
    let taken = best.and_then(|(key, p)| {
        let n: usize = key.as_str().strip_prefix('r')?.parse().ok()?;
        (p.get() >= FINAL).then_some((n.checked_sub(1)?, key.clone(), p))
    });
    let Some((n, took, p)) = taken else {
        return Found::None;
    };
    let read = &readings[n];
    let tokens = &sought.tokens;
    let from = tokens.iter().position(|token| token.start == read.start());
    let to = tokens.iter().rposition(|token| token.start < read.end());
    let span = from
        .zip(to)
        .and_then(|(from, to)| sought.trimmed(from, to))
        .unwrap_or_else(|| read.clone());
    let others = readings
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != n)
        .map(|(_, span)| span.clone())
        .collect();
    Found::Text {
        span,
        chose: (id, took),
        p,
        others,
    }
}

/// A choice under the argument's own ask, closed by *the request does not say*.
fn closed(ask: &Clean, options: IndexMap<Key, Text>) -> Question {
    Question::Choice(Choice::closed(
        ask.clone(),
        options,
        (unstated(), Text::Plain(unstated_text())),
    ))
}

fn key(text: &str) -> Key {
    Key::new(text).expect("a key of this module has text")
}

fn plain(text: &Clean) -> Text {
    Text::Plain(text.clone())
}

fn clean(text: &str) -> Clean {
    Clean::new(text).unwrap_or_default()
}
