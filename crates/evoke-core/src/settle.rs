//! One argument of the winner, settled from every answer about it. In: the argument with its own question and
//! answer, the request, the answers so far. Out: a value with what it stands on, a missing value, nothing, or the
//! questions it waits for.
//!
//! A listed argument is read by two views and by the words of the request that hold a listed word; a typed one by
//! its own question, then by what code proposed.

use indexmap::IndexMap;

use crate::adapter::{Choice, Key, Prob, Question, QuestionId, Request};
use crate::call::Value;
use crate::decide::{
    Answers, Basis, Choices, Judgment, Missing, View, Why, candidate, choices, judge, only,
    out_of_range, probability, proposes, required, taken_as, top, unstated, word_question, worded,
};
use crate::manifest::{Argument, Pick, Source};
use crate::name::{ArgName, LocalName, Word};
use crate::pins;
use crate::plan::{Plan, none};
use crate::propose::Proposed;
use crate::text::Span;
use crate::words::{self, Listed, Spelled};
use crate::{Fault, adapter};

/// At this share a yes takes what code proposed: a listed word the request's words hold, a value it spells
/// out, the one candidate of a kind.
const PROPOSED: f64 = 0.5;

/// At this share a yes keeps a listed word that both views gave and no word of the request holds.
const UNHELD: f64 = 0.5;

/// And `none` may not hold this share beside it.
const BESIDE: f64 = 0.3;

/// At this share the reader's view alone gives a listed word that the request holds by its meaning only.
const READER_ALONE: f64 = 0.8;

/// A spelled value is taken when its yes leads the next one by this much.
const LEAD: f64 = 0.2;

/// What one argument's answers settle: a value with what it stands on and the spans it consumes — a pick's
/// own, or every typed candidate a word covers — a missing value, nothing, or nothing yet, a question being open.
pub(crate) enum Settled<'a> {
    Value(Value, Option<Basis>, Vec<&'a Span>),
    Missing(Missing),
    Nothing,
    Open,
}

/// One argument as it is read: the plan, the request and the answers so far; its names, its manifest, its own
/// question and the answer to it.
pub(crate) struct One<'a, 'r> {
    pub plan: &'a Plan,
    pub request: &'a Request,
    pub answers: &'r Answers<'a>,
    pub reflex: &'r LocalName,
    pub arg: &'r ArgName,
    pub argument: &'r Argument,
    pub choice: &'r Choice,
    pub answer: &'r IndexMap<Key, Prob>,
}

/// The questions an argument waits for, by id.
type Open = IndexMap<QuestionId, Question>;

impl<'a> One<'a, '_> {
    fn id(&self) -> QuestionId {
        QuestionId::Arg(self.reflex.clone(), self.arg.clone())
    }

    /// The argument asked of the person, for the reason given.
    fn asked(&self, source: &Source, because: Why) -> Settled<'a> {
        Settled::Missing(Missing {
            arg: self.arg.clone(),
            ask: self.argument.ask.clone(),
            because,
            words: None,
            choices: choices(self.plan, self.reflex, self.arg, source, self.recalled()),
        })
    }

    /// The argument as the request leaves it unsaid: asked when the reflex cannot run without it.
    fn unsaid(&self) -> Settled<'a> {
        match required(self.argument) {
            Some(source) => Settled::Missing(unstated(
                self.plan,
                self.reflex,
                self.arg,
                self.argument,
                source,
                self.recalled(),
            )),
            None => Settled::Nothing,
        }
    }

    /// The values the request kept for a pick's ask from the session's results, when it kept any.
    fn recalled(&self) -> Option<Vec<String>> {
        self.request.recent.get(&self.id()).cloned()
    }

    /// The yes a question of evoke's own was answered with; where it was not asked, the question is opened.
    fn yes(
        &self,
        id: &QuestionId,
        question: impl FnOnce() -> Question,
        open: &mut Open,
    ) -> Option<Prob> {
        let Some(answer) = self.answers.get(id) else {
            open.insert(id.clone(), question());
            return None;
        };
        Some(answer.get("yes").copied().unwrap_or(Prob::ZERO))
    }

    /// The yes on one listed word of the argument, with the question that holds it.
    fn is(&self, key: &Key, open: &mut Open) -> Option<(QuestionId, Prob)> {
        let (id, question) = word_question(self.reflex, self.arg, self.argument, self.choice, key)?;
        let yes = self.yes(&id, || question, open)?;
        Some((id, yes))
    }

    /// A listed word as a value of its source, with what it stands on.
    fn word(&self, source: &Source, key: &Key, stands: Basis) -> Result<Settled<'a>, Fault> {
        let malformed = |message: String| adapter::Fault::Malformed {
            question: self.id(),
            message,
        };
        Ok(match source {
            Source::Options(options) => {
                let (key, _) = options
                    .get_key_value(key.as_str())
                    .ok_or_else(|| malformed(format!("\"{key}\" is not an option")))?;
                Settled::Value(Value::Option { key: key.clone() }, Some(stands), Vec::new())
            }
            Source::Vocab(vocabulary) => {
                let word = Word::new(key.as_str()).map_err(malformed)?;
                // A word said and used is no unused span: every typed candidate that is its text is consumed,
                // so `monday` under a vocabulary of days is the word, and the date it reads as caps nothing.
                let spoken = word.as_str().to_lowercase();
                let covered: Vec<&Span> = self
                    .request
                    .proposed
                    .iter()
                    .filter(|proposed| {
                        proposed.value.is_typed()
                            && proposed.span.text().as_str().to_lowercase() == spoken
                    })
                    .map(|proposed| &proposed.span)
                    .collect();
                Settled::Value(worded(self.plan, vocabulary, word), Some(stands), covered)
            }
            Source::Pick(_) => Settled::Nothing,
        })
    }

    /// A typed value inside its argument's range, or the argument asked with the range it fell out of.
    fn ranged(
        &self,
        pick: &Pick,
        value: Value,
        stands: Basis,
        consumed: Vec<&'a Span>,
    ) -> Settled<'a> {
        if let Value::Pick {
            span, value: read, ..
        } = &value
            && let Some(range) = out_of_range(pick, read)
        {
            return Settled::Missing(Missing {
                arg: self.arg.clone(),
                ask: self.argument.ask.clone(),
                because: Why::OutOfRange {
                    span: span.clone(),
                    range,
                },
                words: None,
                choices: Choices::Pick {
                    pick: pick.recognizer(),
                    recent: self.recalled(),
                },
            });
        }
        Settled::Value(value, Some(stands), consumed)
    }
}

/// One view of a listed argument: the key on top and its share, as a judgment under the view's question.
struct Seen {
    key: Key,
    p: Prob,
    judgment: Judgment,
}

fn seen(
    question: QuestionId,
    choice: &Choice,
    answer: &IndexMap<Key, Prob>,
) -> Result<Seen, Fault> {
    let (key, p) = top(choice, answer).ok_or_else(|| adapter::Fault::Malformed {
        question: question.clone(),
        message: "no options".to_owned(),
    })?;
    Ok(Seen {
        judgment: Judgment {
            question,
            top: key.clone(),
            p,
        },
        key,
        p,
    })
}

/// A listed argument. The word stands where both views give it; where one view gives it and the request's words
/// hold it, it is taken on that view; where neither gives one and the words hold one alone, a yes takes it;
/// where both give a word no word of the request holds, its own yes decides; else the argument is asked, or
/// reads unstated.
pub(crate) fn listed<'a>(
    one: &One<'a, '_>,
    source: &Source,
    open: &mut Open,
) -> Result<(Judgment, Settled<'a>), Fault> {
    let ask = seen(one.id(), one.choice, one.answer)?;
    let view = pins::view(one.reflex, one.arg);
    let Some(second) = one.answers.get(&view) else {
        open.insert(view, pins::view_question(one.choice));
        return Ok((ask.judgment, Settled::Open));
    };
    let reader = seen(view, one.choice, second)?;
    let found: &[Listed] = one.request.listed.get(&one.id()).map_or(&[], Vec::as_slice);
    let names = |key: &Key| *key != none() && Some(key) != one.choice.otherwise();
    match (names(&ask.key), names(&reader.key)) {
        (true, true) if ask.key == reader.key => agreed(one, source, found, ask, reader, open),
        (true, true) => Ok((ask.judgment, one.asked(source, Why::Unsettled))),
        (true, false) => alone(one, source, found, View::Ask, ask, &reader),
        (false, true) => alone(one, source, found, View::Reader, reader, &ask),
        (false, false) if ask.key == none() && reader.key == none() => {
            Ok((ask.judgment, one.asked(source, Why::NotOffered)))
        }
        // One view says a word the list lacks, the other that none is said: it reads unstated.
        (false, false) if ask.key == none() || reader.key == none() => {
            Ok((ask.judgment, one.unsaid()))
        }
        (false, false) => unviewed(one, source, found, ask, open),
    }
}

/// Both views gave one word: it stands where a word of the request holds it, or where its own yes keeps it
/// and `none` holds little beside; it is as sure as the less sure of the two views.
fn agreed<'a>(
    one: &One<'a, '_>,
    source: &Source,
    found: &[Listed],
    ask: Seen,
    reader: Seen,
    open: &mut Open,
) -> Result<(Judgment, Settled<'a>), Fault> {
    let yes = if found.iter().any(|held| held.key == ask.key) {
        None
    } else {
        let Some((_, yes)) = one.is(&ask.key, open) else {
            return Ok((ask.judgment, Settled::Open));
        };
        Some(yes)
    };
    let beside = probability(one.answer, &none());
    if yes.is_some_and(|yes| yes.get() < UNHELD || beside.get() >= BESIDE) {
        return Ok((ask.judgment, one.asked(source, Why::Unsettled)));
    }
    let stands = Basis::Views {
        ask: ask.p,
        reader: reader.p,
        yes,
    };
    let settled = one.word(source, &ask.key, stands)?;
    let weaker = if reader.p < ask.p { reader } else { ask };
    Ok((weaker.judgment, settled))
}

/// One view gave a word and the other none: it is taken where the request's words hold it, by the reader's
/// view only where they hold it firmly or the view is sure; else the argument is asked.
fn alone<'a>(
    one: &One<'a, '_>,
    source: &Source,
    found: &[Listed],
    view: View,
    gave: Seen,
    other: &Seen,
) -> Result<(Judgment, Settled<'a>), Fault> {
    let held = found
        .iter()
        .find(|held| held.key == gave.key)
        .filter(|held| view == View::Ask || held.how.firm() || gave.p.get() >= READER_ALONE);
    let Some(held) = held else {
        let asked = match view {
            View::Ask => gave.judgment,
            View::Reader => other.judgment.clone(),
        };
        return Ok((asked, one.asked(source, Why::Unsettled)));
    };
    let stands = Basis::View {
        view,
        p: gave.p,
        other: other.key.clone(),
        words: held.span.clone(),
        how: held.how,
    };
    let settled = one.word(source, &gave.key, stands)?;
    Ok((gave.judgment, settled))
}

/// Neither view gave a word: the one word the request holds firmly is taken where a yes says it is meant.
fn unviewed<'a>(
    one: &One<'a, '_>,
    source: &Source,
    found: &[Listed],
    ask: Seen,
    open: &mut Open,
) -> Result<(Judgment, Settled<'a>), Fault> {
    let mut firm = found.iter().filter(|held| held.how.firm());
    let (Some(held), None) = (firm.next(), firm.next()) else {
        return Ok((ask.judgment, one.unsaid()));
    };
    let Some((question, yes)) = one.is(&held.key, open) else {
        return Ok((ask.judgment, Settled::Open));
    };
    if yes.get() < PROPOSED {
        return Ok((ask.judgment, one.unsaid()));
    }
    let stands = Basis::Words {
        words: held.span.clone(),
        how: held.how,
        yes,
    };
    let settled = one.word(source, &held.key, stands)?;
    Ok((said_yes(question, yes), settled))
}

/// A typed argument: the candidate its own question gave; else a value the request spells out, or the one
/// candidate of its kind, where a yes takes it; else asked where the request states one that is not among
/// the candidates, and unstated otherwise. A candidate made of courtesy alone is no value.
pub(crate) fn typed<'a>(
    one: &One<'a, '_>,
    pick: &Pick,
    open: &mut Open,
) -> Result<(Judgment, Settled<'a>), Fault> {
    let asked = judge(one.id(), one.argument, one.choice, one.answer)?;
    let stated = asked.top == none();
    if !stated
        && Some(&asked.top) != one.choice.otherwise()
        && let Some(settled) = offered(one, pick, &asked)?
    {
        return Ok((asked, settled));
    }
    match proposed(one, pick, open) {
        Err(Waits) => Ok((asked, Settled::Open)),
        Ok(Some(taken)) => Ok(taken),
        Ok(None) if stated => {
            let source = Source::Pick(pick.clone());
            Ok((asked, one.asked(&source, Why::NotOffered)))
        }
        Ok(None) => Ok((asked, one.unsaid())),
    }
}

/// The candidate the argument's own question gave, as its value; none when it is a courtesy and no value.
fn offered<'a>(
    one: &One<'a, '_>,
    pick: &Pick,
    asked: &Judgment,
) -> Result<Option<Settled<'a>>, Fault> {
    let recognizer = pick.recognizer();
    let found: &Proposed = one
        .request
        .proposed
        .iter()
        .find(|proposed| {
            candidate(&proposed.span) == asked.top && proposes(recognizer, &proposed.value)
        })
        .ok_or_else(|| adapter::Fault::Malformed {
            question: one.id(),
            message: format!("\"{}\" is not a candidate", asked.top),
        })?;
    if words::courtesy(found.span.text().as_str()) {
        return Ok(None);
    }
    let value = Value::Pick {
        span: found.span.clone(),
        value: taken_as(recognizer, &found.value),
        typed: None,
    };
    let stands = Basis::Ask { p: asked.p };
    Ok(Some(one.ranged(pick, value, stands, vec![&found.span])))
}

/// A yes that was not asked yet, its question opened.
struct Waits;

/// What code proposed for a typed argument its own question gave no value to: the value the request spells out
/// whose yes leads, else the one candidate of the kind, each taken where a yes says it is meant; none where
/// code proposed nothing a yes takes.
fn proposed<'a>(
    one: &One<'a, '_>,
    pick: &Pick,
    open: &mut Open,
) -> Result<Option<(Judgment, Settled<'a>)>, Waits> {
    let recognizer = pick.recognizer();
    let spelled: &[Spelled] = one
        .request
        .spelled
        .get(&one.id())
        .map_or(&[], Vec::as_slice);
    let said: Vec<(&Spelled, QuestionId, Option<Prob>)> = spelled
        .iter()
        .take(pins::MOST)
        .map(|spelled| {
            let id = pins::said(one.reflex, one.arg, &spelled.span);
            let yes = one.yes(
                &id,
                || pins::said_question(&one.argument.ask, spelled),
                open,
            );
            (spelled, id, yes)
        })
        .collect();
    let alone: Option<(&Proposed, QuestionId, Option<Prob>)> =
        only(recognizer, &one.request.proposed)
            .filter(|only| !words::courtesy(only.span.text().as_str()))
            .map(|only| {
                let id = pins::only(one.reflex, one.arg, &only.span);
                let yes = one.yes(
                    &id,
                    || pins::only_question(&one.argument.ask, &only.span),
                    open,
                );
                (only, id, yes)
            });
    let waits = said.iter().any(|(.., yes)| yes.is_none())
        || alone.as_ref().is_some_and(|(.., yes)| yes.is_none());
    if waits {
        return Err(Waits);
    }
    let mut said: Vec<(&Spelled, QuestionId, Prob)> = said
        .into_iter()
        .filter_map(|(spelled, id, yes)| Some((spelled, id, yes?)))
        .collect();
    said.sort_by(|a, b| b.2.get().total_cmp(&a.2.get()));
    let leads = match said.as_slice() {
        [best, next, ..] => best.2.get() - next.2.get() >= LEAD,
        _ => true,
    };
    if let Some((spelled, id, yes)) = said.into_iter().next()
        && yes.get() >= PROPOSED
        && leads
    {
        let value = Value::Pick {
            span: spelled.span.clone(),
            value: taken_as(recognizer, &spelled.value),
            typed: Some(spelled.typed.clone()),
        };
        let stands = Basis::Spelled {
            form: spelled.form,
            yes,
        };
        let settled = one.ranged(pick, value, stands, Vec::new());
        return Ok(Some((said_yes(id, yes), settled)));
    }
    if let Some((only, id, Some(yes))) = alone
        && yes.get() >= PROPOSED
    {
        let value = Value::Pick {
            span: only.span.clone(),
            value: taken_as(recognizer, &only.value),
            typed: None,
        };
        let settled = one.ranged(pick, value, Basis::Only { yes }, vec![&only.span]);
        return Ok(Some((said_yes(id, yes), settled)));
    }
    Ok(None)
}

/// The judgment of a yes that took a value.
fn said_yes(question: QuestionId, p: Prob) -> Judgment {
    Judgment {
        question,
        top: Key::new("yes").expect("a key of this module has text"),
        p,
    }
}
