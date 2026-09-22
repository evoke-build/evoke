//! Reading a request: the words' own order, where it may split, which parts are left out, and which part refers to
//! an earlier one — code alone from the literal words, and the two questions the engine settles: whether a split
//! point separates two things, and which earlier step a reference names. In: the request, and segments. Out:
//! splits, segments, references, and the two requests with what their answers say.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::{Choice, Key, Prob, Question, QuestionId, Raw, Request, State, Text};
use crate::decide::validated;
use crate::name::WeaveName;
use crate::text::{Clean, Input};

/// What a connective does: `then` orders what follows after what precedes; the rest coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Then,
    And,
}

/// A place the request may split: the connective's span in characters, its word, whether it orders, and the
/// engine's probability that the two sides are two things — `1` when a negation follows and code took it alone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub start: usize,
    pub end: usize,
    pub word: String,
    pub order: Order,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<Prob>,
}

/// A segment of the request: its text and where it sits, in characters; one that begins with a negation is left
/// out — never decided, never run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub excluded: bool,
}

/// The reference word in a step's text, in characters of that text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Where {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// How a reference was found: a pronoun, a determiner and a noun, or the engine choosing among steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum How {
    Pronoun,
    Phrase,
    Engine,
}

/// A reference in one step to earlier ones: the words, the steps it may name, how it was found, how surely.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ref {
    pub span: Where,
    /// The steps it may name, zero-based; several for a plural pronoun.
    pub from: Vec<usize>,
    pub how: How,
    /// `the <noun>`: a reference only when something takes it; otherwise plain words.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub weak: bool,
    /// A phrase's noun, singular — `png` in «the png» — which may name a field of the source's result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noun: Option<String>,
    /// Plural — «them», «each of them», «each client», «the clients»: over a result of several records, every one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub many: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<Prob>,
}

/// Connectives that order, longest first so `and then` wins over `and`.
const THEN: [&str; 11] = [
    "and then",
    "and after that",
    "and afterwards",
    "and afterward",
    "and next",
    "then",
    "after that",
    "afterwards",
    "afterward",
    "after which",
    "next",
];
/// Connectives that coordinate — `meanwhile` and `at the same time` say what `and` already allows.
const AND: [&str; 12] = [
    "and also",
    "and meanwhile",
    "and in the meantime",
    "and at the same time",
    "meanwhile",
    "in the meantime",
    "at the same time",
    "and",
    "but",
    "plus",
    "as well as",
    "also",
];
/// A segment that begins so is left out.
const NEGATION: [&str; 6] = ["not", "don't", "do not", "never", "without", "nor"];
const PRONOUNS: [&str; 9] = [
    "both of them",
    "each of them",
    "all of them",
    "the two",
    "it",
    "them",
    "those",
    "these",
    "both",
];
const DETERMINERS: [&str; 8] = [
    "that", "this", "those", "these", "the", "its", "each", "every",
];
const ADJECTIVES: [&str; 3] = ["same", "resulting", "new"];
/// A noun after a determiner that names no thing.
const STOP: [&str; 10] = [
    "one", "same", "other", "way", "time", "first", "second", "last", "next", "rest",
];

/// The words a subject may be written as before a `before` or `after` clause: `you`, `we`, `i`, with a tense.
const YOU: [&str; 3] = ["you", "we", "i"];
const TENSE: [&str; 4] = ["'ve", "'d", " have", " had"];

/// A `before` or `after` clause, leading or trailing, in the words' own order: «after you X, Y» and «Y after you X»
/// both read «X, then Y»; «before you X, Y» and «Y before you X» read «Y, then X». Every word stays the person's;
/// only the order and one connective change, so the rest of the reading needs no second path.
#[must_use]
pub fn canonical(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    if let Some((word, clause, rest)) = leading(&chars) {
        return if word == "before" {
            format!("{rest}, then {clause}")
        } else {
            format!("{clause}, then {rest}")
        };
    }
    if let Some((first, word, clause)) = trailing(&chars) {
        return if word == "before" {
            format!("{first}, then {clause}")
        } else {
            format!("{clause}, then {first}")
        };
    }
    input.to_owned()
}

/// «before you X, Y»: the word, X and Y; the subject is optional, and a clause that begins `that`, `this` or
/// `which` is a connective, not a clause.
fn leading(chars: &[char]) -> Option<(&'static str, String, String)> {
    let word = ["before", "after"]
        .into_iter()
        .find(|word| starts_with_word(chars, 0, word))?;
    let mut i = spaces(chars, word.len())?;
    if let Some(after) = subject(chars, i)
        && let Some(spaced) = spaces(chars, after)
    {
        i = spaced;
    }
    if ["that", "this", "which"]
        .iter()
        .any(|w| starts_with_word(chars, i, w) && ends_word(chars, i + w.len()))
    {
        return None;
    }
    // `(.+?),\s*(.+)$`: the first comma with at least one character before it and one after the spaces.
    let mut comma = i + 1;
    while comma < chars.len() {
        if chars[comma] == ',' && !chars[i..comma].contains(&'\n') {
            let mut rest = comma + 1;
            while rest < chars.len() && chars[rest].is_whitespace() {
                rest += 1;
            }
            if rest < chars.len() && !chars[rest..].contains(&'\n') {
                let clause: String = chars[i..comma].iter().collect();
                let after: String = chars[rest..].iter().collect();
                return Some((word, clause, after));
            }
        }
        comma += 1;
    }
    None
}

/// «Y before you X»: the leftmost `\s+(before|after)\s+you\s+` with a character before and after.
fn trailing(chars: &[char]) -> Option<(String, &'static str, String)> {
    let mut i = 1;
    while i < chars.len() {
        if chars[i].is_whitespace() && !chars[..i].contains(&'\n') {
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if let Some(word) = ["before", "after"]
                .into_iter()
                .find(|word| starts_with_word(chars, j, word))
                && let Some(after_word) = spaces(chars, j + word.len())
                && let Some(after_subject) = subject(chars, after_word)
                && let Some(rest) = spaces(chars, after_subject)
                && rest < chars.len()
                && !chars[rest..].contains(&'\n')
            {
                let first: String = chars[..i].iter().collect();
                let clause: String = chars[rest..].iter().collect();
                return Some((first, word, clause));
            }
        }
        i += 1;
    }
    None
}

/// `you`, `we` or `i`, with `'ve`, `'d`, ` have` or ` had` when one follows: where the subject ends.
fn subject(chars: &[char], at: usize) -> Option<usize> {
    let you = YOU
        .into_iter()
        .find(|you| starts_with_word(chars, at, you))?;
    let end = at + you.len();
    let tensed = TENSE
        .into_iter()
        .find(|tense| starts_with_word(chars, end, tense))
        .map_or(end, |tense| end + tense.len());
    Some(tensed)
}

/// One or more whitespace characters from `at`: where they end.
fn spaces(chars: &[char], at: usize) -> Option<usize> {
    let mut i = at;
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    (i > at).then_some(i)
}

/// Whether `word` stands at `at`, letter for letter, case aside.
fn starts_with_word(chars: &[char], at: usize, word: &str) -> bool {
    let mut i = at;
    for w in word.chars() {
        match chars.get(i) {
            Some(c) if c.to_ascii_lowercase() == w => i += 1,
            _ => return false,
        }
    }
    true
}

/// `\b` before `at`: a word character on one side only.
fn boundary(chars: &[char], at: usize) -> bool {
    let before = at > 0 && is_word(chars[at - 1]);
    let after = chars.get(at).is_some_and(|c| is_word(*c));
    before != after
}

/// Whether a word ends at `at`: what follows is no word character.
fn ends_word(chars: &[char], at: usize) -> bool {
    !chars.get(at).is_some_and(|c| is_word(*c))
}

/// A word character as the regex engine the research ran counts one: ASCII letters, digits and `_`.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Every split point the words offer, in order of position, one per position, none inside quotes: the ordering
/// connectives first, then the coordinating ones, then a bare `;`, then — for the engine to judge only — a bare
/// comma. A candidate carries the whitespace and the punctuation around its connective, as the research counted
/// them, so its `end` is where the next part begins.
#[must_use]
pub fn splits(text: &str, commas: bool) -> Vec<Split> {
    let chars: Vec<char> = text.chars().collect();
    let quoted = quoted(&chars);
    let mut found: Vec<Split> = Vec::new();
    let mut add = |phrases: &[&str], order: Order, bare: Option<char>| {
        let mut i = 0;
        while i < chars.len() {
            let Some((end, word)) = connective(&chars, i, phrases, bare) else {
                i += 1;
                continue;
            };
            let start = i;
            i = end;
            if start == 0 || end == chars.len() {
                continue;
            }
            if quoted.iter().any(|q| start >= q.0 && start < q.1) {
                continue;
            }
            if found.iter().any(|s| start < s.end && end > s.start) {
                continue;
            }
            found.push(Split {
                start,
                end,
                word,
                order,
                p: None,
            });
        }
    };
    add(&THEN, Order::Then, None);
    add(&AND, Order::And, None);
    add(&[], Order::And, Some(';'));
    if commas {
        add(&[], Order::And, Some(','));
    }
    found.sort_by_key(|s| s.start);
    found
}

/// A connective at `i`: `\s*[,;]?\s*\b(phrase)\b,?\s*`, or a bare `\s*;\s*` and `\s*,\s*`; where it ends, and
/// its word as a person names it.
fn connective(
    chars: &[char],
    i: usize,
    phrases: &[&str],
    bare: Option<char>,
) -> Option<(usize, String)> {
    let mut j = i;
    while j < chars.len() && chars[j].is_whitespace() {
        j += 1;
    }
    if let Some(mark) = bare {
        if chars.get(j) != Some(&mark) {
            return None;
        }
        j += 1;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        return Some((j, mark.to_string()));
    }
    let mut punctuated = j;
    if matches!(chars.get(j), Some(',' | ';')) {
        punctuated = j + 1;
        while punctuated < chars.len() && chars[punctuated].is_whitespace() {
            punctuated += 1;
        }
    }
    let at = punctuated;
    if !boundary(chars, at) {
        return None;
    }
    let phrase = phrases.iter().find(|phrase| {
        starts_with_word(chars, at, phrase) && ends_word(chars, at + phrase.len())
    })?;
    let mut end = at + phrase.len();
    if chars.get(end) == Some(&',') {
        end += 1;
    }
    while end < chars.len() && chars[end].is_whitespace() {
        end += 1;
    }
    Some((end, chars[at..at + phrase.len()].iter().collect()))
}

/// The quoted regions of the text: `"…"`, `“…”` and `‘…’`.
fn quoted(chars: &[char]) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let close = match chars[i] {
            '"' => '"',
            '\u{201c}' => '\u{201d}',
            '\u{2018}' => '\u{2019}',
            _ => {
                i += 1;
                continue;
            }
        };
        match chars[i + 1..].iter().position(|c| *c == close) {
            Some(offset) => {
                regions.push((i, i + 2 + offset));
                i += 2 + offset;
            }
            None => i += 1,
        }
    }
    regions
}

/// The segments a choice of split points yields; one that begins with a negation is marked left out.
#[must_use]
pub fn segments(text: &str, taken: &[Split]) -> Vec<Segment> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut push = |start: usize, end: usize| {
        let piece: String = chars[start..end.min(chars.len())].iter().collect();
        let text = piece.trim();
        if !text.is_empty() {
            out.push(Segment {
                excluded: negated(text),
                text: text.to_owned(),
                start,
                end,
            });
        }
    };
    let mut sorted: Vec<&Split> = taken.iter().collect();
    sorted.sort_by_key(|s| s.start);
    let mut at = 0;
    for split in sorted {
        push(at, split.start);
        at = split.end;
    }
    push(at, chars.len());
    out
}

/// Whether a segment begins with a negation: left out, never decided, never run.
#[must_use]
pub fn negated(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    NEGATION
        .iter()
        .any(|word| starts_with_word(&chars, 0, word) && ends_word(&chars, word.len()))
}

/// Whether a fragment carries a pronoun — «pull up every one of them», «fax it to the warehouse»: by the words it
/// is a step of its own, whatever the engine made of the split before it.
#[must_use]
pub fn refers_back(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    (0..chars.len()).any(|i| pronoun_at(&chars, i).is_some())
}

/// A pronoun at `i`, the longest alternative first: where it ends.
fn pronoun_at(chars: &[char], i: usize) -> Option<(usize, &'static str)> {
    if !boundary(chars, i) {
        return None;
    }
    PRONOUNS
        .into_iter()
        .find(|word| starts_with_word(chars, i, word) && ends_word(chars, i + word.len()))
        .map(|word| (i + word.len(), word))
}

/// Code's reading of references in segment `k`: a pronoun names the step before, a plural one every earlier step;
/// `that <noun>` names the nearest earlier step whose words carry the noun, or whose result has a field of that
/// name — `fields[j]`, what step j's result may yield — or, a demonstrative only, whose first word shares the
/// noun's first five letters: «that summary» after «summarize it». `the <noun>` is weak: never a step by its verb.
#[must_use]
pub fn refs_by_code(segs: &[Segment], k: usize, fields: &[Vec<String>]) -> Vec<Ref> {
    let chars: Vec<char> = segs[k].text.chars().collect();
    let mut refs: Vec<Ref> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let Some((end, word)) = pronoun_at(&chars, i) else {
            i += 1;
            continue;
        };
        let many = word != "it";
        refs.push(Ref {
            span: Where {
                start: i,
                end,
                text: chars[i..end].iter().collect(),
            },
            from: if many { (0..k).collect() } else { vec![k - 1] },
            how: How::Pronoun,
            weak: false,
            noun: None,
            many,
            p: None,
        });
        i = end;
    }
    let mut i = 0;
    while i < chars.len() {
        let Some((end, det, noun)) = phrase_at(&chars, i) else {
            i += 1;
            continue;
        };
        let start = i;
        i = end;
        if STOP.contains(&noun.as_str()) {
            continue;
        }
        let stem = noun.strip_suffix('s').unwrap_or(&noun).to_owned();
        let weak = det == "the";
        let from = earlier(segs, k, fields, &stem, weak);
        if from.is_empty() {
            continue;
        }
        let many = det == "each" || det == "every" || stem != noun;
        refs.push(Ref {
            span: Where {
                start,
                end,
                text: chars[start..end].iter().collect(),
            },
            from,
            how: How::Phrase,
            weak,
            noun: Some(stem),
            many,
            p: None,
        });
    }
    dedupe(refs)
}

/// `(that|this|those|these|the|its|each|every) [same|resulting|new] <noun>` at `i`: where it ends, the
/// determiner and the noun, lowered.
fn phrase_at(chars: &[char], i: usize) -> Option<(usize, &'static str, String)> {
    if !boundary(chars, i) {
        return None;
    }
    let det = DETERMINERS
        .into_iter()
        .find(|det| starts_with_word(chars, i, det) && ends_word(chars, i + det.len()))?;
    let mut j = spaces(chars, i + det.len())?;
    if let Some(adjective) = ADJECTIVES.into_iter().find(|adjective| {
        starts_with_word(chars, j, adjective) && ends_word(chars, j + adjective.len())
    }) && let Some(after) = spaces(chars, j + adjective.len())
    {
        j = after;
    }
    let mut end = j;
    while end < chars.len() && chars[end].is_ascii_alphabetic() {
        end += 1;
    }
    if end == j || !ends_word(chars, end) {
        return None;
    }
    let noun: String = chars[j..end].iter().map(char::to_ascii_lowercase).collect();
    Some((end, det, noun))
}

/// The nearest earlier step a noun names: by its words — not their first word, its verb, for a weak determiner —
/// by a field of its result, or by the first five letters of its verb for a demonstrative.
fn earlier(
    segs: &[Segment],
    k: usize,
    fields: &[Vec<String>],
    stem: &str,
    weak: bool,
) -> Vec<usize> {
    for j in (0..k).rev() {
        let text = &segs[j].text;
        if let Some(at) = word_at(text, stem)
            && !(weak && at == 0)
        {
            return vec![j];
        }
        if fields
            .get(j)
            .is_some_and(|fields| fields.iter().any(|f| f == stem))
        {
            return vec![j];
        }
        let verb = text
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_lowercase();
        if !weak && stem.chars().count() >= 5 && verb.chars().take(5).eq(stem.chars().take(5)) {
            return vec![j];
        }
    }
    Vec::new()
}

/// `\b<stem>s?\b` in the text, case aside: where the first one starts, in characters.
fn word_at(text: &str, stem: &str) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    (0..chars.len()).find(|&i| {
        boundary(&chars, i)
            && starts_with_word(&chars, i, stem)
            && (ends_word(&chars, i + stem.len())
                || (chars
                    .get(i + stem.len())
                    .is_some_and(|c| c.eq_ignore_ascii_case(&'s'))
                    && ends_word(&chars, i + stem.len() + 1)))
    })
}

fn dedupe(refs: Vec<Ref>) -> Vec<Ref> {
    let mut seen: Vec<(usize, Vec<usize>)> = Vec::new();
    refs.into_iter()
        .filter(|r| {
            let key = (r.span.start, r.from.clone());
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        })
        .collect()
}

/// A split a negation follows is taken without asking.
pub(crate) const SURE: Prob = match Prob::new(1.0) {
    Some(p) => p,
    None => panic!("one is a probability"),
};

/// The split points the engine is asked about — those a negation does not follow — and the request that asks:
/// one yes/no per split point, the whole request as its state. None when nothing is asked.
pub(crate) fn judging(
    request: &str,
    all: &[Split],
) -> Result<Option<(Vec<usize>, Request)>, Unclean> {
    let chars: Vec<char> = request.chars().collect();
    let asked: Vec<usize> = (0..all.len())
        .filter(|&i| !negated(&chars[all[i].end..].iter().collect::<String>()))
        .collect();
    if asked.is_empty() {
        return Ok(None);
    }
    let mut questions = IndexMap::new();
    for (n, &i) in asked.iter().enumerate() {
        let split = &all[i];
        let before: String = chars[..split.start]
            .iter()
            .collect::<String>()
            .trim()
            .to_owned();
        let after: String = chars[split.end..]
            .iter()
            .collect::<String>()
            .trim()
            .to_owned();
        let at = if split.word == "," {
            "the comma".to_owned()
        } else {
            format!("«{}»", split.word)
        };
        let ask = Clean::new(&format!(
            "At {at}, does the request ask for two things to be done — «{before}», and separately «{after}»?"
        ))
        .map_err(|_| Unclean)?;
        questions.insert(
            own(&format!("split_{n}")),
            Question::YesNo {
                ask,
                yes: Text::Plain(plain(
                    "Two things to do: what follows the connective is another task, or the same task again for another item.",
                )),
                no: Text::Plain(plain(
                    "One thing to do: what follows the connective is part of the same task — a second value it takes, a detail, a name, or a time.",
                )),
            },
        );
    }
    Ok(Some((asked, request_of(request, questions)?)))
}

/// A request's words that the engine cannot be asked about: a control character among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unclean;

/// The split points with the engine's answers: a probability per asked one, `1` for the rest.
pub(crate) fn judged(
    all: &[Split],
    asked: &[usize],
    request: &Request,
    raw: Raw,
) -> Result<Vec<Split>, crate::adapter::Fault> {
    let answers = validated(request, raw)?;
    Ok(all
        .iter()
        .enumerate()
        .map(|(i, split)| {
            let p = match asked.iter().position(|&a| a == i) {
                Some(n) => answers
                    .get(&own(&format!("split_{n}")))
                    .and_then(|answer| answer.get("yes"))
                    .copied()
                    .unwrap_or(Prob::ZERO),
                None => SURE,
            };
            Split {
                p: Some(p),
                ..split.clone()
            }
        })
        .collect())
}

/// With two or more steps before it, which one a reference names is the engine's to say: one choice per
/// reference the words settled on a single step, over every earlier step. None when nothing is asked.
pub(crate) fn referring(
    request: &str,
    segs: &[Segment],
    refs: &[Vec<Ref>],
) -> Result<Option<Request>, Unclean> {
    let mut questions = IndexMap::new();
    for (k, seg) in segs.iter().enumerate().skip(2) {
        for (i, r) in refs[k].iter().enumerate() {
            if r.from.len() > 1 {
                continue;
            }
            let ask = Clean::new(&format!(
                "In «{}», which earlier step does «{}» refer to?",
                seg.text, r.span.text
            ))
            .map_err(|_| Unclean)?;
            let mut options = IndexMap::new();
            for (j, earlier) in segs.iter().take(k).enumerate() {
                let text = Clean::new(&format!("«{}»", earlier.text)).map_err(|_| Unclean)?;
                options.insert(key(&format!("step{j}")), Text::Plain(text));
            }
            let choice = Choice::new(ask, options, None).map_err(|_| Unclean)?;
            questions.insert(own(&format!("ref_{k}_{i}")), Question::Choice(choice));
        }
    }
    if questions.is_empty() {
        return Ok(None);
    }
    Ok(Some(request_of(request, questions)?))
}

/// The references with the engine's choices in place: `from` the step chosen, `how` the engine.
pub(crate) fn referred(
    refs: &mut [Vec<Ref>],
    request: &Request,
    raw: Raw,
) -> Result<(), crate::adapter::Fault> {
    let answers = validated(request, raw)?;
    for (k, step) in refs.iter_mut().enumerate().skip(2) {
        for (i, r) in step.iter_mut().enumerate() {
            let Some(answer) = answers.get(&own(&format!("ref_{k}_{i}"))) else {
                continue;
            };
            let Some((best, p)) = answer
                .iter()
                .max_by(|a, b| a.1.get().total_cmp(&b.1.get()))
                .map(|(key, p)| (key.as_str().to_owned(), *p))
            else {
                continue;
            };
            if let Some(j) = best
                .strip_prefix("step")
                .and_then(|j| j.parse::<usize>().ok())
            {
                r.from = vec![j];
                r.how = How::Engine;
                r.p = Some(p);
            }
        }
    }
    Ok(())
}

fn request_of(
    request: &str,
    questions: IndexMap<QuestionId, Question>,
) -> Result<Request, Unclean> {
    Ok(Request {
        state: State {
            request: Input::new(request).map_err(|_| Unclean)?,
        },
        questions,
        proposed: Vec::new(),
    })
}

/// `weave.<name>`: a question of the layer's own.
fn own(name: &str) -> QuestionId {
    QuestionId::Weave(WeaveName::new(name).expect("a name of this module is one"))
}

fn key(text: &str) -> Key {
    Key::new(text).expect("a key of this module has text")
}

fn plain(text: &str) -> Clean {
    Clean::new(text).expect("the texts of this module are clean")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(splits: &[Split]) -> Vec<(&str, Order)> {
        splits.iter().map(|s| (s.word.as_str(), s.order)).collect()
    }

    #[test]
    fn a_before_or_after_clause_reads_in_the_words_own_order() {
        assert_eq!(
            canonical("before you generate the sales report, check stock for widgets"),
            "check stock for widgets, then generate the sales report"
        );
        assert_eq!(
            canonical("after you render the logo, email the png to jo@example.com"),
            "render the logo, then email the png to jo@example.com"
        );
        assert_eq!(
            canonical("email the svg to jo@example.com after you render the logo"),
            "render the logo, then email the svg to jo@example.com"
        );
        assert_eq!(
            canonical("after you've rendered the logo, print the svg"),
            "rendered the logo, then print the svg"
        );
        assert_eq!(
            canonical("before you archive the poster, render the poster"),
            "render the poster, then archive the poster"
        );
        assert_eq!(
            canonical("mail the svg to ana@example.com after you've rendered the banner"),
            "rendered the banner, then mail the svg to ana@example.com"
        );
        // `after that, …` is a connective, not a clause.
        assert_eq!(
            canonical("build the inventory report for initech, after that check the gadgets stock"),
            "build the inventory report for initech, after that check the gadgets stock"
        );
        assert_eq!(canonical("look up order 4821"), "look up order 4821");
    }

    #[test]
    fn splits_stand_at_connectives_outside_quotes_and_never_at_the_ends() {
        let found = splits("check stock for widgets and look up order 4821", false);
        assert_eq!(words(&found), [("and", Order::And)]);
        assert_eq!((found[0].start, found[0].end), (23, 28));
        let found = splits("generate the sales report, then look up order 4821", false);
        assert_eq!(words(&found), [("then", Order::Then)]);
        assert_eq!((found[0].start, found[0].end), (25, 32));
        let found = splits(
            "generate acme's inventory report, next summarize it, and then send it to sam@example.com",
            false,
        );
        assert_eq!(
            words(&found),
            [("next", Order::Then), ("and then", Order::Then)]
        );
        assert_eq!(
            words(&splits(
                "look up job 12 and meanwhile check the deadline",
                false
            )),
            [("and meanwhile", Order::And)]
        );
        assert_eq!(
            words(&splits(
                "render the logo; in the meantime, look up job 12",
                false
            )),
            [("in the meantime", Order::And)]
        );
        assert!(splits("combine \"~/a.pdf and b\" now", false).is_empty());
        assert!(splits("and then what", false).is_empty());
        assert!(splits("look it up and", false).is_empty());
        assert_eq!(
            words(&splits(
                "check the deadline for the logo, the poster and the banner",
                true
            )),
            [(",", Order::And), ("and", Order::And)]
        );
        assert!(splits("brand new grandstand", false).is_empty());
    }

    #[test]
    fn segments_are_the_parts_between_and_a_negation_is_left_out() {
        let text = "check the deadline for the logo but not the poster";
        let taken = splits(text, false);
        let segs = segments(text, &taken);
        assert_eq!(
            segs.iter()
                .map(|s| (s.text.as_str(), s.excluded))
                .collect::<Vec<_>>(),
            [
                ("check the deadline for the logo", false),
                ("not the poster", true)
            ]
        );
        assert!(negated("don't archive the logo"));
        assert!(!negated("note the time"));
        assert!(refers_back("pull up every one of them"));
        assert!(!refers_back("look up order 4821"));
    }

    fn segs(texts: &[&str]) -> Vec<Segment> {
        texts
            .iter()
            .map(|text| Segment {
                text: (*text).to_owned(),
                start: 0,
                end: 0,
                excluded: false,
            })
            .collect()
    }

    #[test]
    fn a_pronoun_names_the_step_before_and_a_noun_a_field_or_a_verb() {
        let steps = segs(&[
            "generate acme's sales report",
            "email that report to ana@example.com",
        ]);
        let refs = refs_by_code(&steps, 1, &[vec!["path".to_owned()]]);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].span.text, "that report");
        assert_eq!(refs[0].from, [0]);
        assert_eq!(refs[0].noun.as_deref(), Some("report"));
        assert!(!refs[0].weak && !refs[0].many);
        let steps = segs(&["render the logo", "email the png to jo@example.com"]);
        let refs = refs_by_code(&steps, 1, &[vec!["png".to_owned(), "svg".to_owned()]]);
        assert_eq!(refs[0].span.text, "the png");
        assert!(refs[0].weak);
        let steps = segs(&["render the logo", "email the flyer to jo@example.com"]);
        assert!(refs_by_code(&steps, 1, &[vec!["png".to_owned()]]).is_empty());
        let steps = segs(&["list the overdue jobs", "email each client"]);
        let refs = refs_by_code(
            &steps,
            1,
            &[vec![
                "jobs".to_owned(),
                "number".to_owned(),
                "client".to_owned(),
            ]],
        );
        assert!(refs[0].many);
        assert_eq!(refs[0].noun.as_deref(), Some("client"));
        let steps = segs(&[
            "make the sales report",
            "summarize it",
            "send that summary to ana@example.com",
        ]);
        let refs = refs_by_code(
            &steps,
            2,
            &[vec!["path".to_owned()], vec!["file".to_owned()]],
        );
        assert_eq!(refs[0].span.text, "that summary");
        assert_eq!(refs[0].from, [1]);
        let refs = refs_by_code(&steps, 1, &[vec!["path".to_owned()]]);
        assert_eq!(refs[0].span.text, "it");
        assert_eq!(refs[0].from, [0]);
        let steps = segs(&["list the overdue jobs", "look up each of them"]);
        let refs = refs_by_code(&steps, 1, &[vec![]]);
        assert_eq!(refs[0].span.text, "each of them");
        assert!(refs[0].many);
        assert_eq!(refs[0].from, [0]);
    }

    #[test]
    fn the_judge_request_asks_one_yes_no_per_candidate_a_negation_does_not_follow() {
        let text = "check the deadline for the logo but not the poster";
        let all = splits(text, true);
        assert_eq!(judging(text, &all).unwrap(), None);
        let text = "generate the sales report and the inventory report, then combine them";
        let all = splits(text, true);
        let (asked, request) = judging(text, &all).unwrap().unwrap();
        assert_eq!(asked, [0, 1]);
        let ids: Vec<String> = request.questions.keys().map(ToString::to_string).collect();
        assert_eq!(ids, ["weave.split_0", "weave.split_1"]);
        let Some(Question::YesNo { ask, .. }) = request.questions.get(&own("split_1")) else {
            panic!("a yes/no");
        };
        assert_eq!(
            ask.as_str(),
            "At «then», does the request ask for two things to be done — «generate the sales report and the inventory report», and separately «combine them»?"
        );
        let raw: Raw = serde_json::from_value(serde_json::json!({
            "weave.split_0": { "yes": 0.9 }, "weave.split_1": { "yes": 0.95 }
        }))
        .unwrap();
        let judged = judged(&all, &asked, &request, raw).unwrap();
        assert_eq!(judged[0].p.map(Prob::get), Some(0.9));
    }

    #[test]
    fn the_refer_request_asks_which_step_over_every_earlier_one() {
        let steps = segs(&[
            "make the sales report",
            "summarize it",
            "send that summary to ana@example.com",
        ]);
        let fields = [vec!["path".to_owned()], vec!["file".to_owned()], vec![]];
        let refs: Vec<Vec<Ref>> = (0..3)
            .map(|k| {
                if k == 0 {
                    Vec::new()
                } else {
                    refs_by_code(&steps, k, &fields)
                }
            })
            .collect();
        let request = referring("x", &steps, &refs).unwrap().unwrap();
        let ids: Vec<String> = request.questions.keys().map(ToString::to_string).collect();
        assert_eq!(ids, ["weave.ref_2_0"]);
        let Some(Question::Choice(choice)) = request.questions.get(&own("ref_2_0")) else {
            panic!("a choice");
        };
        assert_eq!(
            choice.options().keys().map(Key::as_str).collect::<Vec<_>>(),
            ["step0", "step1"]
        );
        let mut refs = refs;
        let raw: Raw = serde_json::from_value(
            serde_json::json!({ "weave.ref_2_0": { "step0": 0.0, "step1": 1.0 } }),
        )
        .unwrap();
        referred(&mut refs, &request, raw).unwrap();
        assert_eq!(refs[2][0].from, [1]);
        assert_eq!(refs[2][0].how, How::Engine);
    }
}
