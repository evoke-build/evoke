//! Reading a request: the words' own order, where it may split, which parts are left out, and which part refers to
//! an earlier one — code alone from the literal words, and the two questions the engine settles: whether a split
//! point separates two things, and which earlier step a reference names. In: the request, and segments. Out:
//! splits, segments, references, and the two requests with what their answers say.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::{Choice, Key, Prob, Question, QuestionId, Raw, Request, Scope, State, Text};
use crate::decide::validated;
use crate::name::{ArgName, LocalName, WeaveName};
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
    /// Whether the plan cuts the request here: no step, and no part out of the plan, holds both sides.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
}

/// A segment of the request: its text and where it sits, in characters; one that begins with a negation is left
/// out — never decided, never run — and one that begins with a condition refuses the whole request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub left: Option<Left>,
}

/// Why a segment is no step: it says what not to do, or opens with a condition no step can judge; or code set it
/// apart by its words as the person's own action or a courtesy: set aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Left {
    Negated,
    Conditional,
    Aside(Stretch),
}

/// What a stretch of the request code sets apart by its words alone is: the person's own action, «before I call
/// him back»; a courtesy, «if you would»; a contrast's «not X» before the Y it keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stretch {
    Own,
    Courtesy,
    Contrast,
}

/// A stretch set apart, where it stands in characters: from its first word to the end of its last, its marks
/// outside. Its two ends are places of the cut taken without asking, as a negation's is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Apart {
    pub start: usize,
    pub end: usize,
    pub what: Stretch,
}

impl Segment {
    /// A segment left out: it begins with a negation.
    #[must_use]
    pub fn excluded(&self) -> bool {
        self.left == Some(Left::Negated)
    }
}

/// The reference word in a step's text, in characters of that text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Where {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// How a reference was found: a pronoun, a determiner and a noun, the engine choosing among steps, or a second
/// verb before the object of the first, whose step takes from the first's as «it» would — code's, and certain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum How {
    Pronoun,
    Phrase,
    Engine,
    Shared,
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
/// A segment that begins so is a condition, which no step can judge: the whole request is refused.
const CONDITION: [&str; 3] = ["if", "unless", "in case"];
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
/// A noun after a determiner that names no thing — and the words that follow `that` or `this` when it is a
/// conjunction, «make sure that the timer is off», not a reference.
const STOP: [&str; 34] = [
    "one", "same", "other", "way", "time", "first", "second", "last", "next", "rest", "the", "a",
    "an", "it", "is", "was", "are", "were", "will", "would", "can", "could", "should", "there",
    "they", "we", "you", "i", "no", "not", "all", "any", "some", "of",
];

/// The words a subject may be written as before a `before` or `after` clause: `you`, `we`, `i`, with a tense.
const YOU: [&str; 3] = ["you", "we", "i"];
const TENSE: [&str; 4] = ["'ve", "'d", " have", " had"];

/// The tokens that stand for «and» alone between two words, each a place of the cut, shown as typed.
const JOINERS: [&str; 5] = ["+", "&", "n", "nd", "adn"];
/// Number words: a joiner beside one joins a number said aloud or a spelled code, «six n v», and is no place.
const NUMBER_WORDS: [&str; 31] = [
    "zero",
    "oh",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fifty",
    "sixty",
    "seventy",
    "eighty",
    "ninety",
    "hundred",
    "thousand",
];
/// The words that open a clause of the person's own action, with a subject of `OWN`.
const OWN_OPENS: [&str; 2] = ["before", "after"];
/// The subjects of a clause that states the person's own action, «before I call him back».
const OWN: [&str; 2] = ["i", "we"];
/// A tense after such a subject, «before I've checked it».
const OWN_TENSES: [&str; 6] = ["'ve", "'d", "'m", "'ll", " have", " had"];
/// The head of a request's own clause, the person its subject: where a clause that leads without a mark ends,
/// «Before I promise anyone a date I need to know …».
const REQUEST_HEADS: [&str; 14] = [
    "i need",
    "i want",
    "i'd like",
    "i would like",
    "i'd love",
    "i would love",
    "i must",
    "i have to",
    "we need",
    "we want",
    "we'd like",
    "we would like",
    "we must",
    "we have to",
];
/// The connectives after which a clause leads, «…, and before I decide anything I'd like …», «and not the Pro, …».
const LEADS: [&str; 7] = ["and", "but", "or", "then", "also", "so", "plus"];
/// Courtesies that open with «if» and state no condition.
const COURTESIES: [&str; 7] = [
    "if it's not too much trouble",
    "if it is not too much trouble",
    "if you don't mind",
    "if you do not mind",
    "if you would",
    "if possible",
    "if so",
];
/// The word that opens a contrast, «not X, Y».
const CONTRAST: &str = "not";
/// The word that may end a contrast's X, «not X but Y».
const CONTRAST_BUT: &str = "but";
/// The marks that close a clause or a sentence.
const MARKS: &str = ",;:.?!";
/// The marks a word may carry that say nothing of it: stripped before a joiner's neighbour is read.
const AROUND: &str = ",;:.?!\"'()";
/// The marks that end a sentence.
const ENDS: [char; 3] = ['.', '?', '!'];

/// A `before` or `after` clause, leading or trailing, in the words' own order: «after you X, Y» and «Y after you X»
/// both read «X, then Y»; «before you X, Y» and «Y before you X» read «Y, then X». Every word stays the person's;
/// only the order and one connective change, so the rest of the reading needs no second path. A clause whose
/// subject is the person, «before I forget», is their own action: set apart where it stands, never reordered.
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
    if own_subject(chars, i).is_some() {
        return None;
    }
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
                && own_subject(chars, after_word).is_none()
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

/// `i` or `we` as a whole word at `at`, with a tense when one follows as a whole word: where the subject ends.
fn own_subject(chars: &[char], at: usize) -> Option<usize> {
    let own = OWN
        .into_iter()
        .find(|own| starts_with_word(chars, at, own) && ends_word(chars, at + own.len()))?;
    let end = at + own.len();
    Some(
        OWN_TENSES
            .into_iter()
            .find(|tense| {
                starts_with_word(chars, end, tense) && ends_word(chars, end + tense.len())
            })
            .map_or(end, |tense| end + tense.len()),
    )
}

/// One or more whitespace characters from `at`: where they end.
fn spaces(chars: &[char], at: usize) -> Option<usize> {
    let mut i = at;
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    (i > at).then_some(i)
}

/// Whether `word` stands at `at`, letter for letter, case aside; a curly apostrophe reads as the straight one, so
/// «don’t», as a Mac types it, is «don't».
fn starts_with_word(chars: &[char], at: usize, word: &str) -> bool {
    let mut i = at;
    for w in word.chars() {
        match chars.get(i) {
            Some(c) if c.to_ascii_lowercase() == w || (*c == '\u{2019}' && w == '\'') => i += 1,
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
                cut: false,
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

/// The places the planner cuts at: `splits`, and each joiner that stands for «and» between two words, every one
/// asked of the engine as a place is.
#[must_use]
pub fn places(text: &str, commas: bool) -> Vec<Split> {
    let mut found = splits(text, commas);
    let chars: Vec<char> = text.chars().collect();
    let quoted = quoted(&chars);
    for split in joiners(&chars, &quoted) {
        if !found
            .iter()
            .any(|s| split.start < s.end && split.end > s.start)
        {
            found.push(split);
        }
    }
    found.sort_by_key(|s| s.start);
    found
}

/// Whether a place's word is a sign or a letter typed for «and».
#[must_use]
pub fn joins(word: &str) -> bool {
    JOINERS.contains(&word)
}

/// Every joiner standing alone between two words, outside quotes, as a place: from the end of the word before to
/// the start of the word after, its word as typed. Never after a mark («sam, n ana»), beside a number or a number
/// word («2 + 2», «six n v»), beside a single letter («R & D», «with one n»), nor between two capitalised words,
/// a name («Hartwell & Sons»).
fn joiners(chars: &[char], quoted: &[(usize, usize)]) -> Vec<Split> {
    let tokens = words_of(chars);
    let mut found = Vec::new();
    for k in 1..tokens.len().saturating_sub(1) {
        let (start, end) = tokens[k];
        let token: String = chars[start..end].iter().collect();
        if !JOINERS.contains(&token.as_str()) || quoted.iter().any(|q| start >= q.0 && start < q.1)
        {
            continue;
        }
        let (before, after) = (tokens[k - 1], tokens[k + 1]);
        if MARKS.contains(chars[before.1 - 1]) {
            continue;
        }
        let neighbours = [
            bare_word(&chars[before.0..before.1]),
            bare_word(&chars[after.0..after.1]),
        ];
        let number = |word: &str| {
            word.starts_with(char::is_numeric)
                || NUMBER_WORDS.contains(&word.to_lowercase().as_str())
        };
        let capital = |word: &str| word.starts_with(char::is_uppercase);
        if neighbours
            .iter()
            .any(|word| word.chars().count() < 2 || number(word))
            || neighbours.iter().all(|word| capital(word))
        {
            continue;
        }
        found.push(Split {
            start: before.1,
            end: after.0,
            word: token,
            order: Order::And,
            p: None,
            cut: false,
        });
    }
    found
}

/// The words of a text as runs of characters between whitespace, where each starts and ends.
fn words_of(chars: &[char]) -> Vec<(usize, usize)> {
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        words.push((start, i));
    }
    words
}

/// A word without the marks around it that say nothing of it.
fn bare_word(chars: &[char]) -> String {
    let word: String = chars.iter().collect();
    word.trim_matches(|c: char| AROUND.contains(c)).to_owned()
}

/// Every stretch code sets apart by its words, in order and never two over one another: a clause of the person's
/// own action, a courtesy heading or ending a part by the places found, a contrast's «not X» before the more
/// words it keeps.
#[must_use]
pub fn stretches(text: &str, places: &[Split]) -> Vec<Apart> {
    let chars: Vec<char> = text.chars().collect();
    let quoted = quoted(&chars);
    let mut found = own_clauses(&chars, &quoted);
    found.extend(courtesies(&chars, &quoted, places));
    found.extend(contrasts(&chars, &quoted));
    let mut kept: Vec<Apart> = Vec::new();
    for apart in found {
        if !kept
            .iter()
            .any(|k| apart.start < k.end && apart.end > k.start)
        {
            kept.push(apart);
        }
    }
    kept.sort_by_key(|apart| apart.start);
    kept
}

/// The text before `at`, its trailing whitespace aside: where it ends.
fn trimmed_before(chars: &[char], at: usize) -> usize {
    let mut i = at;
    while i > 0 && chars[i - 1].is_whitespace() {
        i -= 1;
    }
    i
}

/// Whether the words before `at` end a clause or open one: nothing before, a mark («,», «;», «:», a sentence end,
/// a dash), or a connective of `LEADS`.
fn leads(chars: &[char], at: usize) -> bool {
    let end = trimmed_before(chars, at);
    end == 0
        || ".?!,;:-\u{2013}\u{2014}".contains(chars[end - 1])
        || LEADS.iter().any(|word| {
            let n = word.chars().count();
            end >= n && starts_with_word(chars, end - n, word) && boundary(chars, end - n)
        })
}

/// Whether the mark at `i` stands between two digits, inside a figure: «1:1», «10:30», «1,200». Such a mark ends
/// no clause.
fn in_figure(chars: &[char], i: usize) -> bool {
    i > 0 && chars[i - 1].is_numeric() && chars.get(i + 1).is_some_and(|next| next.is_numeric())
}

/// Where a clause that opens at `from` ends: the first «,» «;» «:», a sentence end, a spaced dash, or the head of a
/// request's own clause (`REQUEST_HEADS`); and whether that end is a sentence end or the text's own, which a
/// leading clause may not reach.
fn clause_end(chars: &[char], from: usize) -> (usize, bool) {
    let mut i = from;
    while i < chars.len() {
        let c = chars[i];
        if matches!(c, ',' | ';' | ':') && !in_figure(chars, i) {
            return (i, false);
        }
        if ENDS.contains(&c) && chars.get(i + 1).is_none_or(|next| next.is_whitespace()) {
            return (i, true);
        }
        if c.is_whitespace()
            && chars
                .get(i + 1)
                .is_some_and(|d| matches!(d, '-' | '\u{2013}' | '\u{2014}'))
            && chars.get(i + 2).is_some_and(|w| w.is_whitespace())
        {
            return (i, false);
        }
        if boundary(chars, i)
            && REQUEST_HEADS
                .iter()
                .any(|head| starts_with_word(chars, i, head) && ends_word(chars, i + head.len()))
        {
            return (i, false);
        }
        i += 1;
    }
    (chars.len(), true)
}

/// Every clause «before|after I|we …» outside quotes, from its first word to the end of its last. One that leads
/// (nothing, a mark or a connective before it) ends at a mark, a spaced dash or a request's head, and one that
/// reaches a sentence end or the text's end first is left as typed, since code cannot say where it ends; one that
/// trails ends at the first mark, a request's head, or the end.
fn own_clauses(chars: &[char], quoted: &[(usize, usize)]) -> Vec<Apart> {
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let Some(open) = OWN_OPENS.into_iter().find(|word| {
            boundary(chars, i)
                && starts_with_word(chars, i, word)
                && ends_word(chars, i + word.len())
        }) else {
            i += 1;
            continue;
        };
        let Some(head) = spaces(chars, i + open.len()).and_then(|at| own_subject(chars, at)) else {
            i += 1;
            continue;
        };
        if quoted.iter().any(|q| i >= q.0 && i < q.1) {
            i = head;
            continue;
        }
        let (end, sentence) = clause_end(chars, head);
        if leads(chars, i) && sentence {
            i = head;
            continue;
        }
        found.push(Apart {
            start: i,
            end: trimmed_before(chars, end),
            what: Stretch::Own,
        });
        i = end.max(head);
    }
    found
}

/// Every courtesy of `COURTESIES` outside quotes that heads a part — nothing before it, or a place ending where
/// it starts — or ends one — only marks after it, or a place starting where it ends.
fn courtesies(chars: &[char], quoted: &[(usize, usize)], places: &[Split]) -> Vec<Apart> {
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let Some(courtesy) = COURTESIES.into_iter().find(|phrase| {
            boundary(chars, i)
                && starts_with_word(chars, i, phrase)
                && ends_word(chars, i + phrase.len())
        }) else {
            i += 1;
            continue;
        };
        let end = i + courtesy.len();
        let heads = trimmed_before(chars, i) == 0 || places.iter().any(|p| p.end == i);
        let ends = chars[end..]
            .iter()
            .all(|c| c.is_whitespace() || MARKS.contains(*c))
            || places.iter().any(|p| p.start == end);
        if (heads || ends) && !quoted.iter().any(|q| i >= q.0 && i < q.1) {
            found.push(Apart {
                start: i,
                end,
                what: Stretch::Courtesy,
            });
        }
        i = end;
    }
    found
}

/// Every «not X» that heads a clause (nothing, a mark, a dash or a connective before it) outside quotes, X up to
/// the first «,», «:» or «but», holding no other mark, with more words after it: X is left out and what follows
/// kept.
fn contrasts(chars: &[char], quoted: &[(usize, usize)]) -> Vec<Apart> {
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !(boundary(chars, i)
            && starts_with_word(chars, i, CONTRAST)
            && ends_word(chars, i + CONTRAST.len())
            && leads(chars, i)
            && !quoted.iter().any(|q| i >= q.0 && i < q.1))
        {
            i += 1;
            continue;
        }
        let from = i + CONTRAST.len();
        let mut end = None;
        let mut j = from;
        while j < chars.len() {
            let c = chars[j];
            if matches!(c, ',' | ':') && !in_figure(chars, j) {
                end = Some((j, j + 1));
                break;
            }
            if matches!(c, ';' | '.' | '?' | '!') {
                break;
            }
            if boundary(chars, j)
                && j > from
                && starts_with_word(chars, j, CONTRAST_BUT)
                && ends_word(chars, j + CONTRAST_BUT.len())
            {
                end = Some((j, j + CONTRAST_BUT.len()));
                break;
            }
            j += 1;
        }
        i = from;
        let Some((stop, after)) = end else {
            continue;
        };
        let x_end = trimmed_before(chars, stop);
        let more = chars[after..].iter().any(|c| !c.is_whitespace());
        if x_end > from && more {
            found.push(Apart {
                start: from - CONTRAST.len(),
                end: x_end,
                what: Stretch::Contrast,
            });
            i = after;
        }
    }
    found
}

/// The places with every stretch set apart: no place inside one; at each of its ends a place taken without
/// asking (`p` at one, as a place a negation follows) — the place found there, or one made of the marks and
/// spaces between (a connective of `LEADS` before a leading stretch goes with the place); none at the text's
/// edges. A stretch with no space before it to cut at stays as typed.
#[must_use]
pub fn set_apart(text: &str, places: Vec<Split>, stretches: &[Apart]) -> Vec<Split> {
    let chars: Vec<char> = text.chars().collect();
    let first = chars.iter().position(|c| !c.is_whitespace()).unwrap_or(0);
    let mut places = places;
    for apart in stretches {
        let mut sure: Vec<Split> = Vec::new();
        if apart.start > first {
            if let Some(place) = places.iter().find(|p| p.end == apart.start) {
                sure.push(place.clone());
            } else if let Some(place) = place_before(&chars, apart.start) {
                sure.push(place);
            } else {
                continue;
            }
        }
        let rest = (apart.end..chars.len())
            .find(|&i| !chars[i].is_whitespace() && !MARKS.contains(chars[i]));
        if let Some(next) = rest {
            if let Some(place) = places.iter().find(|p| p.start == apart.end) {
                sure.push(place.clone());
            } else {
                let word: String = chars[apart.end..next].iter().collect();
                sure.push(Split {
                    start: apart.end,
                    end: next,
                    word: word.trim().to_owned(),
                    order: Order::And,
                    p: None,
                    cut: false,
                });
            }
        }
        let (from, to) = (
            sure.first()
                .map_or(apart.start, |s| s.start.min(apart.start)),
            sure.last().map_or(apart.end, |s| s.end.max(apart.end)),
        );
        places.retain(|p| !(p.start < to && p.end > from));
        places.extend(sure.into_iter().map(|place| Split {
            p: Some(SURE),
            ..place
        }));
    }
    places.sort_by_key(|p| p.start);
    places
}

/// A place made before a stretch at `at`: the spaces before it, with the mark before them when there is one, or
/// the connective of `LEADS` and the mark before it; none where no space stands before the stretch.
fn place_before(chars: &[char], at: usize) -> Option<Split> {
    let end = trimmed_before(chars, at);
    if end == at {
        return None;
    }
    let lead = LEADS.iter().find(|word| {
        let n = word.chars().count();
        end >= n && starts_with_word(chars, end - n, word) && boundary(chars, end - n)
    });
    let start = match lead {
        Some(word) => {
            let mut start = trimmed_before(chars, end - word.chars().count());
            if start > 0 && matches!(chars[start - 1], ',' | ';') {
                start = trimmed_before(chars, start - 1);
            }
            start
        }
        None => end,
    };
    let word = lead.map_or_else(
        || {
            chars[..end]
                .last()
                .filter(|c| !c.is_alphanumeric())
                .map(ToString::to_string)
                .unwrap_or_default()
        },
        |word| (*word).to_owned(),
    );
    (start > 0).then_some(Split {
        start,
        end: at,
        word,
        order: Order::And,
        p: None,
        cut: false,
    })
}

/// The segments that are a stretch of the person's own action or a courtesy, marked set aside: a segment whose
/// words, its closing marks aside, are the stretch's.
pub fn set_aside(text: &str, segments: &mut [Segment], stretches: &[Apart]) {
    let chars: Vec<char> = text.chars().collect();
    for apart in stretches {
        if apart.what == Stretch::Contrast {
            continue;
        }
        let words: String = chars[apart.start..apart.end].iter().collect();
        for seg in segments.iter_mut() {
            let own = seg
                .text
                .trim_end_matches(|c: char| c.is_whitespace() || MARKS.contains(c));
            if seg.start <= apart.start && apart.end <= seg.end && own == words {
                seg.left = Some(Left::Aside(apart.what));
            }
        }
    }
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
                left: left(text),
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
    heads(text, &NEGATION)
}

/// Whether a segment begins with a condition — `if`, `unless`, `in case` — which no step can judge.
#[must_use]
pub fn conditional(text: &str) -> bool {
    heads(text, &CONDITION)
}

/// Why a segment is no step, when it is not one.
fn left(text: &str) -> Option<Left> {
    if negated(text) {
        Some(Left::Negated)
    } else if conditional(text) && !heads(text, &COURTESIES) {
        // «if you would», «if so» state no condition: code sets them apart where they stand.
        Some(Left::Conditional)
    } else {
        None
    }
}

/// Whether a text begins with one of the words, whole.
fn heads(text: &str, words: &[&str]) -> bool {
    let chars: Vec<char> = text.chars().collect();
    words
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

/// A pronoun at `i`, the longest alternative first: where it ends. A `#` before the word makes it a name —
/// «#it» is a channel — never a pronoun.
fn pronoun_at(chars: &[char], i: usize) -> Option<(usize, &'static str)> {
    if !boundary(chars, i) || (i > 0 && chars[i - 1] == '#') {
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
    // The first segment has nothing earlier to refer to.
    if k == 0 {
        return Vec::new();
    }
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
        let stem = stem_of(&noun).to_owned();
        let weak = det == "the";
        let from = earlier(segs, k, fields, &stem, weak);
        if from.is_empty() {
            continue;
        }
        // Plural from the words: `each`, `every`, or a noun in `s` the source did not write that way — «the
        // address» beside «look up dana's address» is that one thing.
        let many = det == "each"
            || det == "every"
            || (stem != noun && !attested(segs, fields, from[0], &noun));
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
            .is_some_and(|fields| fields.iter().any(|f| stem_of(f) == stem))
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

/// A word's stem, the same on both sides of a comparison: `addresses` and `address` meet at `address`, `clients`
/// and `client` at `client`; a word in `ss` stays as it is.
pub(crate) fn stem_of(word: &str) -> &str {
    if let Some(head) = word.strip_suffix("sses") {
        &word[..head.len() + 2]
    } else if word.ends_with('s') && !word.ends_with("ss") {
        &word[..word.len() - 1]
    } else {
        word
    }
}

/// Whether a noun, as written, stands whole among step `j`'s words or names a field of its result.
fn attested(segs: &[Segment], fields: &[Vec<String>], j: usize, noun: &str) -> bool {
    let chars: Vec<char> = segs[j].text.chars().collect();
    (0..chars.len()).any(|i| {
        boundary(&chars, i)
            && starts_with_word(&chars, i, noun)
            && ends_word(&chars, i + noun.len())
    }) || fields
        .get(j)
        .is_some_and(|fields| fields.iter().any(|f| f == noun))
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
    // A split a negation or a condition follows is taken as sure: what comes after it is no step's to judge. A
    // place code took at a stretch it set apart is never asked either.
    let asked: Vec<usize> = (0..all.len())
        .filter(|&i| left(&chars[all[i].end..].iter().collect::<String>()).is_none())
        .filter(|&i| all[i].p.is_none())
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
    questions.insert(own(COUNT), counting());
    Ok(Some((asked, request_of(request, questions)?)))
}

/// The name of the question that rides the split points': how many things the request asks.
const COUNT: &str = "count";

/// How many things a request asks, and its four answers.
const HOW_MANY: &str = "How many separate things does the request ask to be done?";
const ONE: &str = "One thing.";
const TWO: &str = "Two things.";
const MORE: &str = "Three things or more.";
const NOTHING: &str = "Nothing: it asks for no action.";

/// The question of how many things the request asks.
fn counting() -> Question {
    let options = [
        ("one", ONE),
        ("two", TWO),
        ("more", MORE),
        ("none", NOTHING),
    ]
    .into_iter()
    .map(|(answer, text)| (key(answer), Text::Plain(plain(text))))
    .collect();
    Question::Choice(
        Choice::new(plain(HOW_MANY), options, None).expect("a choice without a sentinel is one"),
    )
}

/// The share of *one thing* among the answers to how many things the request asks; none where it was not
/// asked.
pub(crate) fn one_thing(raw: &Raw) -> Option<Prob> {
    let answer = raw.0.get(&own(COUNT).to_string())?;
    Prob::new(answer.get("one").copied().unwrap_or(0.0))
}

/// What a part of the request does, and its three answers.
const PART_ASKS: &str = "It asks for something to be done.";
const PART_DETAIL: &str = "It adds a detail to another part of the request.";
const PART_ASIDE: &str = "It gives a reason, a circumstance or a remark, and asks for nothing.";

/// Which reflex the words of a part ask for, read in the whole request: the words stand for `{words}`.
const NAMED: &str = "In the request, which one do the words \"{words}\" ask for?";
/// The kind of that question's name, before where the part's words stand.
const SPAN: &str = "span_";

/// The no of a value one part states, asked of another.
pub(crate) const ANOTHER: &str = "Another one, or none.";

/// The yes and the no of a switch one part sets, asked of another.
pub(crate) const THIS: &str = "The part asks for this too.";
pub(crate) const NOT_THIS: &str = "The part does not ask for this.";

/// What a part of the request does: `weave.part_<start>_<end>`, by where its words stand in the request.
pub(crate) fn part(seg: &Segment) -> Result<(QuestionId, Question), Unclean> {
    let ask = Clean::new(&format!(
        "In the request, what does the part «{}» do?",
        seg.text
    ))
    .map_err(|_| Unclean)?;
    let options = [
        ("asks", PART_ASKS),
        ("detail", PART_DETAIL),
        ("aside", PART_ASIDE),
    ]
    .into_iter()
    .map(|(answer, text)| (key(answer), Text::Plain(plain(text))))
    .collect();
    let choice = Choice::new(ask, options, None).map_err(|_| Unclean)?;
    Ok((
        own(&format!("part_{}_{}", seg.start, seg.end)),
        Question::Choice(choice),
    ))
}

/// Which reflex a part's words ask for, read in the whole request: `weave.span_<start>_<end>`, by where its words
/// stand in the request, over the route's own options as the route offers them, its ask naming the words.
pub(crate) fn span(seg: &Segment, route: &Choice) -> Result<(QuestionId, Question), Unclean> {
    let ask = Clean::new(&NAMED.replace("{words}", &seg.text)).map_err(|_| Unclean)?;
    let choice = Choice::new(ask, route.options().clone(), route.otherwise().cloned())
        .map_err(|_| Unclean)?;
    Ok((
        own(&format!("{SPAN}{}_{}", seg.start, seg.end)),
        Question::Choice(choice),
    ))
}

/// Whether a question asks which reflex a part's words ask for, read in the whole request: its judgment stands for
/// the route in a decision of the part's words.
pub(crate) fn is_span(question: &QuestionId) -> bool {
    matches!(question, QuestionId::Weave(name) if name.as_str().starts_with(SPAN))
}

/// Whether a value one part states is another part's: `weave.share_<taker>_<giver>_<reflex>__<argument>`, the
/// parts by where their words begin; the argument's own ask of the taker's words, a yes the value as shown,
/// a no as the value's kind says it.
pub(crate) fn shared(
    (taker, giver): (&Segment, &Segment),
    (reflex, arg): (&LocalName, &ArgName),
    ask: &Clean,
    (shown, no): (&str, &str),
) -> Result<(QuestionId, Question), Unclean> {
    let ask = Clean::new(&format!("For the part «{}»: {ask}", taker.text)).map_err(|_| Unclean)?;
    Ok((
        own(&format!(
            "share_{}_{}_{}",
            taker.start,
            giver.start,
            crate::pins::pair(reflex, arg)
        )),
        Question::YesNo {
            ask,
            yes: Text::Plain(Clean::new(shown).map_err(|_| Unclean)?),
            no: Text::Plain(plain(no)),
        },
    ))
}

/// A request of the plan's own about the whole request.
pub(crate) fn asking(
    request: &str,
    questions: IndexMap<QuestionId, Question>,
) -> Result<Request, Unclean> {
    request_of(request, questions)
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
            // A plural names every earlier step; a shared object names the one step code found it in.
            if r.from.len() > 1 || r.how == How::Shared {
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
            let id = own(&format!("ref_{k}_{i}"));
            let (Some(answer), Some(Question::Choice(choice))) =
                (answers.get(&id), request.questions.get(&id))
            else {
                continue;
            };
            // The first of the highest, in the request's own order, as the foundation reads a tie.
            let mut best: Option<(String, Prob)> = None;
            for key in choice.options().keys() {
                let p = answer.get(key).copied();
                if let Some(p) = p
                    && best.as_ref().is_none_or(|(_, b)| p.get() > b.get())
                {
                    best = Some((key.as_str().to_owned(), p));
                }
            }
            let Some((best, p)) = best else {
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
        scope: Scope::Full,
        named: None,
        recent: IndexMap::new(),
        listed: IndexMap::new(),
        spelled: IndexMap::new(),
        spoken: IndexMap::new(),
    })
}

/// The noun a step opening with `check that <noun>` names — a reference to an earlier step by code, `check that
/// writes land`, where `check whether` is meant — when it opens so: what a playbook's lint flags. A step that
/// refers on purpose, `roll back that release`, is not opened by a check.
#[must_use]
pub(crate) fn checked_that(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    if !starts_with_word(&chars, 0, "check") {
        return None;
    }
    let after = spaces(&chars, "check".len())?;
    match phrase_at(&chars, after) {
        Some((_, "that", noun)) if !STOP.contains(&noun.as_str()) => Some(noun),
        _ => None,
    }
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
                .map(|s| (s.text.as_str(), s.excluded()))
                .collect::<Vec<_>>(),
            [
                ("check the deadline for the logo", false),
                ("not the poster", true)
            ]
        );
        assert!(negated("don't archive the logo"));
        assert!(negated("don\u{2019}t archive the logo"));
        assert!(!negated("note the time"));
        // A condition is the negation's twin: its split is sure, and the segment is marked, never left out.
        assert!(conditional("if checkout is failing"));
        assert!(conditional("in case it rains"));
        assert!(!conditional("iffy"));
        let text = "roll it back if checkout is failing";
        let all = splits(text, true);
        assert!(all.is_empty());
        let text = "if checkout is failing in eu-west, roll it back";
        let segs = segments(text, &splits(text, true));
        assert_eq!(segs[0].left, Some(Left::Conditional));
        assert!(!segs[0].excluded());
        assert!(refers_back("pull up every one of them"));
        assert!(!refers_back("look up order 4821"));
        // A channel's name is no pronoun.
        assert!(!refers_back("tell #it"));
        assert!(refers_back("tell #it about it"));
    }

    fn segs(texts: &[&str]) -> Vec<Segment> {
        texts
            .iter()
            .map(|text| Segment {
                text: (*text).to_owned(),
                start: 0,
                end: 0,
                left: None,
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
        // `that` before a function word is a conjunction, not a reference; the first segment refers to nothing.
        let steps = segs(&["kill the lights", "make sure that the timer is off"]);
        assert!(refs_by_code(&steps, 1, &[vec![]]).is_empty());
        assert!(refs_by_code(&steps, 0, &[vec![]]).is_empty());
        // A noun in `s` the source wrote that way names that one thing; one the source did not is many.
        let steps = segs(&["look up dana's address", "print the address"]);
        let refs = refs_by_code(&steps, 1, &[vec!["address".to_owned()]]);
        assert_eq!(refs[0].noun.as_deref(), Some("address"));
        assert!(!refs[0].many);
        assert_eq!(stem_of("classes"), "class");
        assert_eq!(stem_of("clients"), "client");
        assert_eq!(stem_of("status"), "statu");
        let steps = segs(&["list the team", "email the addresses"]);
        let refs = refs_by_code(&steps, 1, &[vec!["address".to_owned()]]);
        assert_eq!(refs[0].from, [0]);
        assert!(refs[0].many);
        let steps = segs(&["check the status", "print that status"]);
        let refs = refs_by_code(&steps, 1, &[vec!["status".to_owned()]]);
        assert!(!refs[0].many);
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
        assert_eq!(ids, ["weave.split_0", "weave.split_1", "weave.count"]);
        let Some(Question::YesNo { ask, .. }) = request.questions.get(&own("split_1")) else {
            panic!("a yes/no");
        };
        assert_eq!(
            ask.as_str(),
            "At «then», does the request ask for two things to be done — «generate the sales report and the inventory report», and separately «combine them»?"
        );
        let raw: Raw = serde_json::from_value(serde_json::json!({
            "weave.split_0": { "yes": 0.9 }, "weave.split_1": { "yes": 0.95 },
            "weave.count": { "one": 0.1, "two": 0.2, "more": 0.7 }
        }))
        .unwrap();
        assert_eq!(one_thing(&raw).map(Prob::get), Some(0.1));
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

    #[test]
    fn a_part_is_asked_which_reflex_its_words_ask_for_over_the_routes_own_options() {
        let route = Choice::closed(
            plain("Which one does the request ask for?"),
            [(
                key("keys"),
                Text::Rich {
                    what: plain("List a person's API keys."),
                    not_for: vec![plain("revoking a key")],
                    examples: vec![plain("list the API keys for ana")],
                },
            )]
            .into_iter()
            .collect(),
            (key("none"), Text::Plain(plain("None of these."))),
        );
        let seg = Segment {
            text: "Sam".to_owned(),
            start: 37,
            end: 40,
            left: None,
        };
        let (id, question) = span(&seg, &route).unwrap();
        assert_eq!(id.to_string(), "weave.span_37_40");
        assert!(is_span(&id));
        assert!(!is_span(&part(&seg).unwrap().0));
        let Question::Choice(choice) = question else {
            panic!("a choice");
        };
        assert_eq!(
            choice.ask().as_str(),
            "In the request, which one do the words \"Sam\" ask for?"
        );
        assert_eq!(choice.options(), route.options());
        assert_eq!(choice.otherwise(), route.otherwise());
    }

    /// The places a text is cut at, set apart as the planner sets them, and the parts they make with what each is
    /// — `-` left out, `x` set aside, `?` a condition, else a part — where the engine answers every place it is
    /// asked about «one thing»: only the places code takes are cut.
    fn cut(text: &str) -> (Vec<Split>, Vec<(String, char)>) {
        let found = places(text, true);
        let apart = stretches(text, &found);
        let all = set_apart(text, found, &apart);
        let asked = judging(text, &all)
            .unwrap()
            .map_or_else(Vec::new, |(asked, _)| asked);
        let taken: Vec<Split> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| !asked.contains(i))
            .map(|(_, split)| split.clone())
            .collect();
        let mut segs = segments(text, &taken);
        set_aside(text, &mut segs, &stretches(text, &places(text, true)));
        let parts = segs
            .into_iter()
            .map(|seg| {
                let mark = match seg.left {
                    Some(Left::Negated) => '-',
                    Some(Left::Conditional) => '?',
                    Some(Left::Aside(_)) => 'x',
                    None => ' ',
                };
                (seg.text, mark)
            })
            .collect();
        (all, parts)
    }

    fn parts(text: &str) -> Vec<(String, char)> {
        cut(text).1
    }

    fn owned(parts: &[(&str, char)]) -> Vec<(String, char)> {
        parts.iter().map(|(t, m)| ((*t).to_owned(), *m)).collect()
    }

    /// The question the engine is asked of each place, in order.
    fn asked(text: &str, all: &[Split]) -> Vec<String> {
        judging(text, all)
            .unwrap()
            .map_or_else(Vec::new, |(_, request)| {
                request
                    .questions
                    .values()
                    .filter_map(|q| match q {
                        Question::YesNo { ask, .. } => Some(ask.as_str().to_owned()),
                        Question::Choice(_) => None,
                    })
                    .collect()
            })
    }

    #[test]
    fn a_joiner_between_two_words_is_a_place_asked_as_typed() {
        let text = "ask about stock n costs for the antwerp crates";
        assert!(splits(text, true).is_empty(), "no connective");
        let all = places(text, true);
        assert_eq!(words(&all), [("n", Order::And)]);
        assert_eq!(
            asked(text, &all),
            [
                "At «n», does the request ask for two things to be done — «ask about stock», and separately «costs for the antwerp crates»?"
            ]
        );
        for (text, word) in [
            ("find the invoice + send it on", "+"),
            ("book the room & tell lena@example.com", "&"),
            ("print the report adn file it", "adn"),
            ("call lena nd omar", "nd"),
            ("omar - title & desk?", "&"),
        ] {
            assert_eq!(words(&places(text, true)), [(word, Order::And)], "{text}");
        }
        // Beside a number or a single letter, inside a name or quotes, after a mark, or not alone: no place.
        for text in [
            "what is 3 + 4 again",
            "wipe f seven one q m four n z t nine k",
            "open incs four ninety one & five oh three",
            "remind Brightwell & Daughters about the rent",
            "the Q & A notes",
            "It is Hanna, with one n, as she spells it",
            "rename it \"x + y\" please",
            "call lena, n omar",
            "queue the 80's r&b mix",
            "+ omar",
        ] {
            assert!(
                places(text, true)
                    .iter()
                    .all(|s| !JOINERS.contains(&s.word.as_str())),
                "{text}"
            );
        }
    }

    #[test]
    fn the_persons_own_action_is_set_aside_never_reordered() {
        for text in [
            "jot this down before I forget",
            "Before I call him back, find his number",
            "before we've left, order a cab",
        ] {
            assert_eq!(canonical(text), text);
        }
        assert_eq!(
            canonical("before you archive the poster, render the poster"),
            "render the poster, then archive the poster",
            "«before you» is the order of two steps"
        );
        assert_eq!(
            parts("The plumber rang, jot it down before I forget."),
            owned(&[
                ("The plumber rang, jot it down", ' '),
                ("before I forget.", 'x')
            ])
        );
        let text = "Want to know where the parcel is before I email the client anything, track the order pls";
        let (all, cut) = cut(text);
        assert!(
            all.iter().all(|s| s.p == Some(SURE)),
            "both ends are code's"
        );
        assert!(asked(text, &all).is_empty());
        assert_eq!(
            cut,
            owned(&[
                ("Want to know where the parcel is", ' '),
                ("before I email the client anything", 'x'),
                ("track the order pls", ' ')
            ])
        );
        // A leading clause ends at a mark or at a request's own head.
        assert_eq!(
            parts("Before I tell anyone a price I need to see what we paid last year"),
            owned(&[
                ("Before I tell anyone a price", 'x'),
                ("I need to see what we paid last year", ' ')
            ])
        );
        assert_eq!(
            parts("Before I board the train: how busy is the office today?"),
            owned(&[
                ("Before I board the train", 'x'),
                ("how busy is the office today?", ' ')
            ])
        );
        // A mark inside a figure ends no clause.
        assert_eq!(
            parts("before we start the 1:1, dim the lights in the den"),
            owned(&[
                ("before we start the 1:1", 'x'),
                ("dim the lights in the den", ' ')
            ])
        );
        // One that reaches a sentence end, or no subject of its own, is left as typed.
        for text in [
            "before i order anything tell me what is left",
            "water the plants before it gets dark",
            "ring Eve after Ian arrives",
            "write \"before I forget\" on the board",
        ] {
            assert_eq!(
                parts(text).iter().filter(|p| p.1 == 'x').count(),
                0,
                "{text}"
            );
        }
    }

    #[test]
    fn a_courtesy_heading_or_ending_a_part_is_set_aside() {
        assert_eq!(
            parts("Tell me who is on call tonight, if you would."),
            owned(&[
                ("Tell me who is on call tonight", ' '),
                ("if you would.", 'x')
            ])
        );
        assert_eq!(
            parts("is the printer jammed again, and if so since when?"),
            owned(&[
                ("is the printer jammed again", ' '),
                ("if so", 'x'),
                ("since when?", ' ')
            ])
        );
        assert_eq!(
            parts("If it's not too much trouble, could you tell me which rooms are free?"),
            owned(&[
                ("If it's not too much trouble", 'x'),
                ("could you tell me which rooms are free?", ' ')
            ])
        );
        // A condition still refuses; a courtesy inside a part stays in it.
        assert_eq!(parts("if the build is red, start it again")[0].1, '?');
        assert_eq!(
            parts("Two reminders if you would: ping the whole team"),
            owned(&[("Two reminders if you would: ping the whole team", ' ')])
        );
    }

    #[test]
    fn a_contrast_leaves_its_x_out_and_keeps_what_follows() {
        let text = "Not the March figures this time, I want June's: what we shipped";
        let (all, cut) = cut(text);
        assert!(asked(text, &all).is_empty(), "the comma is code's");
        assert_eq!(
            cut,
            owned(&[
                ("Not the March figures this time", '-'),
                ("I want June's: what we shipped", ' ')
            ])
        );
        assert_eq!(
            parts("Just the lobby for now, not the garage: leave a note for the porter"),
            owned(&[
                ("Just the lobby for now", ' '),
                ("not the garage", '-'),
                ("leave a note for the porter", ' ')
            ])
        );
        assert_eq!(
            parts("look at sales not in Spain but in Portugal"),
            owned(&[("look at sales not in Spain but in Portugal", ' ')]),
            "a «not» inside a clause is no contrast"
        );
        assert_eq!(
            parts("sales for June. not in Spain but in Portugal"),
            owned(&[
                ("sales for June.", ' '),
                ("not in Spain", '-'),
                ("in Portugal", ' ')
            ])
        );
        // Nothing kept after X, or a sentence end first: no contrast.
        assert_eq!(
            parts("Since the update went out, not one parcel has been scanned. Help."),
            owned(&[
                ("Since the update went out", ' '),
                ("not one parcel has been scanned. Help.", '-')
            ])
        );
    }
}
