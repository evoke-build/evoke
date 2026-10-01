//! A value said aloud, read by the kind of the argument it would fill and by the shape its examples share. In: the
//! input, the argument's pick and its example values, the recognizers' candidates. Out: `Heard`: the values
//! proposed for a yes, each with how it stands to the shape; the runs asked, with the readings offered; the
//! candidates of the kind a spoken run withdraws from the argument's own question.
//!
//! A run is read only in the argument's kind: a number (figures said singly, in pairs, in hundreds, «oh», «zero»,
//! «double», «triple», a decimal with «point»), a code (letters said apart, number words, typed parts, a dash said,
//! «point» between figures), an address (a local part and a domain joined by «at» and «dot»), a link (a scheme
//! said), a day (a listed short form, a day word or an ordinal misspelt), a duration (counts summed across units).
//! It never reads part of an address, a function word as a code's letters, or a time after the value. A reading is
//! held against what every example value of the argument shows: put in their shape where every example agrees,
//! shown for a yes where they do not settle it, offered at an ask where it reads two ways or in a length they leave
//! open, and asked, nothing padded, where it fits no example. A code typed with spaces is read the same way and
//! always shown for a yes; a weekday misspelt is read by `words.rs`, and so is a code typed with spaces where no
//! run of the argument's shape is read here. Every word of English is a named list.

use serde::{Deserialize, Serialize};

use crate::calendar::{Day, Weekday, Which};
use crate::decide::proposes;
use crate::manifest::{Pick, Recognizer};
use crate::propose::{PickValue, Proposed};
use crate::text::{Clean, Input, Span};
use crate::words::{self, Form, Spelled};

/// The number words for one figure, each with it: «oh» stands for nought.
const UNITS: [(&str, u64); 11] = [
    ("zero", 0),
    ("oh", 0),
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
];

/// The number words from ten to nineteen, each with its number.
const TEENS: [(&str, u64); 10] = [
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
    ("sixteen", 16),
    ("seventeen", 17),
    ("eighteen", 18),
    ("nineteen", 19),
];

/// The tens, each with its number.
const TENS: [(&str, u64); 8] = [
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
];

/// The words for nought, which a tens word does not take as its unit: «twenty oh» is 20, then 0.
const NOUGHTS: [&str; 2] = ["zero", "oh"];

/// Nought said as a letter: a figure in a run of figures, never a number by itself.
const OH: &str = "oh";

/// The word that makes a count of hundreds, and closes figures said in pairs with two noughts: «twenty three
/// hundred» is 2300.
const HUNDRED: &str = "hundred";

/// The word that makes a count of thousands.
const THOUSAND: &str = "thousand";

/// The word that joins a count of hundreds or thousands to what follows it, and one count of a length to the next.
const AND: &str = "and";

/// The words that say the figure or the letter after them again, and how many times it stands: «double eight» is
/// 88.
const REPEAT: [(&str, usize); 2] = [("double", 2), ("triple", 3)];

/// The words that say a dash inside a code.
const DASH_SIGNS: [&str; 3] = ["dash", "hyphen", "minus"];

/// The words that say a point between figures.
const POINT_SIGNS: [&str; 2] = ["point", "dot"];

/// The words that say a sign inside an address, each with the sign.
const ADDRESS_SIGNS: [(&str, &str); 4] = [
    ("dot", "."),
    ("dash", "-"),
    ("hyphen", "-"),
    ("underscore", "_"),
];

/// The words that say a sign inside a link's host, each with the sign.
const HOST_SIGNS: [(&str, &str); 3] = [("dot", "."), ("dash", "-"), ("hyphen", "-")];

/// The words that say a sign inside a link's path, each with the sign.
const PATH_SIGNS: [(&str, &str); 5] = [
    ("dot", "."),
    ("dash", "-"),
    ("hyphen", "-"),
    ("underscore", "_"),
    ("slash", "/"),
];

/// The word that says a link's path goes on.
const SLASH: &str = "slash";

/// The words that may open a version and are no part of it: «v three point nine», «version two point one».
const VERSION_WORDS: [&str; 2] = ["v", "version"];

/// The word that joins an address's local part to its domain.
const AT: &str = "at";

/// The schemes a link said aloud opens with.
const SCHEMES: [&str; 2] = ["https", "http"];

/// What a scheme said whole is followed by: «h t t p s colon slash slash».
const SCHEME_SIGNS: [&str; 3] = ["colon", "slash", "slash"];

/// The most letters a scheme is said in, apart: «h t t p s».
const SCHEME_LETTERS: usize = 5;

/// The letters said apart that are words too, which may open a code and are no part of it: «a», «i».
const LETTER_WORDS: [&str; 2] = ["a", "i"];

/// The pronouns of two or three letters `words.rs`'s function words leave out, which are no code's letters either:
/// «owes us two pallets».
const PRONOUNS: [&str; 4] = ["us", "he", "him", "she"];

/// The letter a lone «o» is, which reads as nought as well where the shape wants a figure.
const LETTER_O: &str = "o";

/// The days of the week, in the calendar's order.
const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// The day words a misspelt one is held against beside the weekdays, each with its distance from today.
const DAY_WORDS: [(&str, i32); 4] = [
    ("today", 0),
    ("tonight", 0),
    ("tomorrow", 1),
    ("yesterday", -1),
];

/// The short forms of a weekday a person types, each with its place in the week.
const SHORT_DAYS: [(&str, usize); 11] = [
    ("mon", 0),
    ("tue", 1),
    ("tues", 1),
    ("wed", 2),
    ("weds", 2),
    ("thu", 3),
    ("thur", 3),
    ("thurs", 3),
    ("fri", 4),
    ("sat", 5),
    ("sun", 6),
];

/// The short forms of a day word a person types, each with its distance from today.
const SHORT_DAY_WORDS: [(&str, i32); 9] = [
    ("tmrw", 1),
    ("tmr", 1),
    ("tmw", 1),
    ("tmrow", 1),
    ("2moro", 1),
    ("2morrow", 1),
    ("2mrw", 1),
    ("2day", 0),
    ("tdy", 0),
];

/// The endings a day's name may carry that are no part of it: a possessive, a plural.
const DAY_ENDINGS: [&str; 3] = ["\u{2019}s", "'s", "s"];

/// The words before a weekday that say which one.
const WHICH: [(&str, Which); 3] = [
    ("this", Which::This),
    ("next", Which::Next),
    ("last", Which::Last),
];

/// The words before a day that make it a recurrence, which is no day.
const EVERY: [&str; 2] = ["every", "each"];

/// The words that make tomorrow the day after it: «day after tmrw».
const DAY_AFTER: [&str; 3] = ["the", "day", "after"];

/// The word an ordinal day follows: «the fourht».
const THE: &str = "the";

/// The ordinals of a day of the month, each with its day.
const ORDINALS: [(&str, u8); 21] = [
    ("first", 1),
    ("second", 2),
    ("third", 3),
    ("fourth", 4),
    ("fifth", 5),
    ("sixth", 6),
    ("seventh", 7),
    ("eighth", 8),
    ("ninth", 9),
    ("tenth", 10),
    ("eleventh", 11),
    ("twelfth", 12),
    ("thirteenth", 13),
    ("fourteenth", 14),
    ("fifteenth", 15),
    ("sixteenth", 16),
    ("seventeenth", 17),
    ("eighteenth", 18),
    ("nineteenth", 19),
    ("twentieth", 20),
    ("thirtieth", 30),
];

/// The endings of an ordinal day as it is written: 1st, 2nd, 3rd; every other, and eleven to thirteen, th.
const ORDINAL_ENDINGS: [&str; 4] = ["th", "st", "nd", "rd"];

/// The shortest word read as an ordinal misspelt.
const ORDINAL_LEAST: usize = 6;

/// The days as a day read aloud is written back: today, tomorrow, the day after tomorrow, yesterday.
const DAYS_SHOWN: [(i32, &str); 5] = [
    (0, "today"),
    (1, "tomorrow"),
    (2, "the day after tomorrow"),
    (-1, "yesterday"),
    (-2, "the day before yesterday"),
];

/// The units of a length of time, each in seconds.
const DURATION_UNITS: [(&str, u64); 15] = [
    ("hours", 3600),
    ("hour", 3600),
    ("hrs", 3600),
    ("hr", 3600),
    ("h", 3600),
    ("minutes", 60),
    ("minute", 60),
    ("mins", 60),
    ("min", 60),
    ("m", 60),
    ("seconds", 1),
    ("second", 1),
    ("secs", 1),
    ("sec", 1),
    ("s", 1),
];

/// The unit a bare count after a unit is read in: «one hr 30» is an hour and thirty minutes.
const NEXT_UNIT: [(u64, u64); 2] = [(3600, 60), (60, 1)];

/// The words that add half a unit after a count and its unit: «an hour and a half».
const HALF_WORDS: [&str; 3] = ["and", "a", "half"];

/// The word for half a unit: «half an hour», «a half hour».
const HALF: &str = "half";

/// The words that count one of a unit: «an hour».
const ARTICLES: [&str; 2] = ["a", "an"];

/// The unit words a length is written back in, one and more than one: «1 hour 30 minutes».
const DURATION_SHOWN: [(u64, &str, &str); 3] = [
    (3600, "hour", "hours"),
    (60, "minute", "minutes"),
    (1, "second", "seconds"),
];

/// The shortest word read as a number word misspelt, before a unit.
const MISSPELT_LEAST: usize = 5;

/// The most readings one run of a code is read as, its openings and its lone letters taken each way; a run that
/// would read as more is read as none.
const MOST_READINGS: usize = 256;

/// How a value read aloud stands to the shape its argument's examples share, which says what it waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    /// The words spell it whole: every character as they say it.
    Whole,
    /// Put in the shape every example shares: a case or a separator the words did not say.
    Completed,
    /// In a form the examples do not settle, or read by a convention or a spelling: shown for a yes.
    Open,
}

/// What the reader of values said aloud leaves beside the values it proposes, for one argument.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Spoken {
    /// The runs asked of the person, each with the readings offered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asked: Vec<Offered>,
    /// The candidates of the argument's kind a spoken run holds, which its own question no more offers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub withdrawn: Vec<Span>,
}

impl Spoken {
    /// Whether it holds nothing: no run asked, no candidate withdrawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.asked.is_empty() && self.withdrawn.is_empty()
    }
}

/// A run asked: its words, and what they read as, each a value the kind's recognizer reads whole; none where the
/// reading fits no example and nothing may be padded.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Offered {
    pub words: Span,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readings: Vec<Clean>,
}

/// What the reader hears for one argument: the values proposed for a yes, in the order of their words, and what it
/// leaves beside them.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Heard {
    pub said: Vec<Spelled>,
    #[serde(flatten)]
    pub spoken: Spoken,
}

/// What an argument's words say aloud, read in its kind and held against its example values: the values proposed,
/// the runs asked, the candidates withdrawn. A code typed with spaces and a weekday misspelt stand as `words.rs`
/// reads them where no spoken run does.
#[must_use]
pub fn heard(input: &Input, pick: &Pick, examples: &[Clean], proposed: &[Proposed]) -> Heard {
    let recognizer = pick.recognizer();
    let words = words_of(input);
    let shown = Examples::of(recognizer, examples, pick);
    let (candidates, others): (Vec<&Proposed>, Vec<&Proposed>) = proposed
        .iter()
        .partition(|candidate| proposes(recognizer, &candidate.value));
    // A reading a candidate proposes whole stays with the argument's own question; one a candidate covers and
    // reaches beyond is a part of it: «sixty» inside «sixty percent». Words another kind's candidate holds, or
    // crosses, are that value's: «fifty minutes» is no number.
    let own: Vec<Reading> = readings(&words, recognizer, &shown)
        .into_iter()
        .filter(|reading| {
            let part = candidates.iter().any(|candidate| {
                let (start, end) = (candidate.span.start(), candidate.span.end());
                let whole = start == reading.start
                    && end == reading.end
                    && reading.value.is(&candidate.value);
                let inside = start <= reading.start
                    && reading.end <= end
                    && (start, end) != (reading.start, reading.end);
                whole || inside
            });
            let other = others.iter().any(|candidate| {
                let (start, end) = (candidate.span.start(), candidate.span.end());
                let apart = end <= reading.start || reading.end <= start;
                let within = reading.start <= start
                    && end <= reading.end
                    && (start, end) != (reading.start, reading.end);
                !apart && !within
            });
            !part && !other
        })
        .collect();
    let mut decided: Vec<Decided> = runs(&own)
        .iter()
        .filter_map(|run| decide(recognizer, run, &shown))
        .collect();
    // Two runs that share words are one stretch read across its middle: the one that starts first stands.
    let mut end = 0;
    decided.retain(|d| {
        let apart = end <= d.start;
        if apart {
            end = d.end;
        }
        apart
    });
    let within = |at: usize| decided.iter().any(|d| d.start <= at && at < d.end);
    let withdrawn: Vec<Span> = candidates
        .iter()
        .filter(|candidate| within(candidate.span.start()))
        .map(|candidate| candidate.span.clone())
        .collect();
    let mut said = Vec::new();
    let mut asked = Vec::new();
    for d in &decided {
        let Some(words) = Span::of(input, d.start, d.end) else {
            continue;
        };
        match &d.outcome {
            Outcome::Take {
                value,
                typed,
                shape,
            } => {
                if let Ok(typed) = Clean::new(typed) {
                    said.push(Spelled {
                        span: words,
                        typed,
                        value: value.clone(),
                        form: d.form,
                        shape: Some(*shape),
                    });
                }
            }
            Outcome::Ask { readings } => asked.push(Offered {
                words,
                readings: readings
                    .iter()
                    .filter_map(|reading| Clean::new(reading).ok())
                    .collect(),
            }),
        }
    }
    let apart = |span: &Span| {
        decided
            .iter()
            .all(|d| span.end() <= d.start || d.end <= span.start())
    };
    said.extend(
        words::spelled(input, recognizer, proposed)
            .into_iter()
            .filter(|spelled| spelled.form != Form::Aloud && apart(&spelled.span)),
    );
    said.sort_by_key(|spelled| spelled.span.start());
    Heard {
        said,
        spoken: Spoken { asked, withdrawn },
    }
}

// ---- the words ----------------------------------------------------------------------------------------------

/// A word of the input as the reader takes it: as typed and lowered, the marks at its edges dropped but a leading
/// `#` or `@` and a closing `%`, and where it stands in characters.
#[derive(Clone, Debug)]
struct Word {
    raw: String,
    plain: String,
    start: usize,
    end: usize,
}

/// The input's words as `words.rs` cuts them, number words joined by a hyphen cut into each, «eighty-five». A word
/// of marks alone, «&», stays, empty: it joins nothing on either side of it.
fn words_of(input: &Input) -> Vec<Word> {
    let chars: Vec<char> = input.as_str().chars().collect();
    let mut out = Vec::new();
    for token in words::tokens(input.as_str()) {
        let raw: String = chars[token.from..token.to].iter().collect();
        let parts: Vec<&str> = raw.split('-').collect();
        if parts.len() > 1 && parts.iter().all(|part| number_word(&part.to_lowercase())) {
            let mut at = token.from;
            for part in parts {
                let length = part.chars().count();
                out.push(Word {
                    raw: part.to_owned(),
                    plain: part.to_lowercase(),
                    start: at,
                    end: at + length,
                });
                at += length + 1;
            }
            continue;
        }
        out.push(Word {
            plain: raw.to_lowercase(),
            raw,
            start: token.from,
            end: token.to,
        });
    }
    out
}

fn listed<T: Copy>(list: &[(&str, T)], word: &str) -> Option<T> {
    list.iter()
        .find(|(name, _)| *name == word)
        .map(|(_, value)| *value)
}

fn unit(word: &str) -> Option<u64> {
    listed(&UNITS, word)
}

fn teen(word: &str) -> Option<u64> {
    listed(&TEENS, word)
}

fn ten(word: &str) -> Option<u64> {
    listed(&TENS, word)
}

fn repeat(word: &str) -> Option<usize> {
    listed(&REPEAT, word)
}

/// Whether a word is a number word: a unit, a teen, a tens, «hundred», «thousand».
fn number_word(word: &str) -> bool {
    unit(word).is_some()
        || teen(word).is_some()
        || ten(word).is_some()
        || word == HUNDRED
        || word == THOUSAND
}

fn figures(word: &str) -> bool {
    !word.is_empty() && word.chars().all(|c| c.is_ascii_digit())
}

/// A unit that a tens word takes after it: one to nine.
fn unit_after_tens(word: Option<&&str>) -> Option<u64> {
    word.filter(|word| !NOUGHTS.contains(*word))
        .and_then(|word| unit(word))
}

/// How far a misspelling may stray from a day's name: none under four letters, one up to seven, two beyond
/// (`words.rs` `by_length`).
fn by_length(word: &str) -> usize {
    match word.chars().count() {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

// ---- what a reading carries ---------------------------------------------------------------------------------

/// What a reading carries beside its value, which the decision reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Marks(u16);

impl Marks {
    /// Figures said in pairs: «three eleven», «twenty two fourteen».
    const PAIRS: Self = Self(1);
    /// A number with a point: «twelve point five».
    const DECIMAL: Self = Self(1 << 1);
    /// A version said without «point»: «four twelve oh».
    const NO_POINT: Self = Self(1 << 2);
    /// A day word or a number word misspelt.
    const MISSPELT: Self = Self(1 << 3);
    /// A bare count after a unit read in the next unit: «one hr 30».
    const CONVENTION: Self = Self(1 << 4);
    /// Half a unit: «a half hour».
    const HALF: Self = Self(1 << 5);
    /// One unit counted by «a» or «an».
    const ARTICLE: Self = Self(1 << 6);
    /// The letters' case put as every example has it.
    const CASE_REFORMED: Self = Self(1 << 7);
    /// The examples disagree on the letters' case.
    const CASE_UNSETTLED: Self = Self(1 << 8);
    /// A dash every example has, not said.
    const DASH_ADDED: Self = Self(1 << 9);
    /// A dash said, which no example has.
    const DASH_DROPPED: Self = Self(1 << 10);
    /// The examples disagree on the dash.
    const DASH_UNSETTLED: Self = Self(1 << 11);

    const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    const fn has(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

/// A value as the reader reads it, before it is held against the examples.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Number(f64),
    Code(String),
    Email(String),
    Url(String),
    Day(Day),
    Seconds(u64),
}

impl Value {
    /// Whether a candidate reads the same value.
    fn is(&self, value: &PickValue) -> bool {
        match (self, value) {
            (Self::Number(read), PickValue::Number { value }) => (read - value).abs() < 1e-9,
            (Self::Code(read), PickValue::Code { value } | PickValue::Quoted { value })
            | (Self::Email(read), PickValue::Email { value })
            | (Self::Url(read), PickValue::Url { value }) => read == value.as_str(),
            (Self::Day(read), PickValue::Date { value }) => read == value,
            (Self::Seconds(read), PickValue::Seconds { value }) => read == value,
            _ => false,
        }
    }
}

/// One reading of words said aloud: where the words stand, the value, what it carries, and how its words give it:
/// said aloud, a day written short or misspelt, a code typed with spaces.
#[derive(Clone, Debug)]
struct Reading {
    start: usize,
    end: usize,
    value: Value,
    marks: Marks,
    form: Form,
}

fn reading(words: &[Word], from: usize, to: usize, value: Value, marks: Marks) -> Reading {
    Reading {
        start: words[from].start,
        end: words[to].end,
        value,
        marks,
        form: Form::Aloud,
    }
}

/// Every reading of the input's words in the argument's kind.
fn readings(words: &[Word], recognizer: Recognizer, shown: &Examples) -> Vec<Reading> {
    match recognizer {
        Recognizer::Number => numbers(words),
        Recognizer::Code => match shown.family {
            Some(Family::Version) => {
                let mut read = codes(words, true);
                for parts in &shown.parts {
                    read.extend(pointless(words, *parts));
                }
                read
            }
            Some(Family::Ticket | Family::Serial) => codes(words, false),
            None => Vec::new(),
        },
        Recognizer::Email => emails(words),
        Recognizer::Url => urls(words),
        Recognizer::Date => days(words),
        Recognizer::Duration => durations(words),
        Recognizer::Quoted | Recognizer::Time | Recognizer::Amount => Vec::new(),
    }
}

// ---- numbers ------------------------------------------------------------------------------------------------

/// A number under a hundred at `i`: a tens and its unit, a teen, a unit; never «oh» alone.
fn small_at(words: &[&str], i: usize) -> Option<(u64, usize)> {
    let word = *words.get(i)?;
    if let Some(tens) = ten(word) {
        return Some(match unit_after_tens(words.get(i + 1)) {
            Some(unit) => (tens + unit, i + 2),
            None => (tens, i + 1),
        });
    }
    if let Some(teen) = teen(word) {
        return Some((teen, i + 1));
    }
    if word != OH
        && let Some(unit) = unit(word)
    {
        return Some((unit, i + 1));
    }
    None
}

/// A number under a thousand at `i`: a small number, then «hundred» and what follows it.
fn below_thousand(words: &[&str], i: usize) -> Option<(u64, usize)> {
    let (mut value, mut j) = small_at(words, i)?;
    if words.get(j) == Some(&HUNDRED) {
        value *= 100;
        j += 1;
        let k = if words.get(j) == Some(&AND) { j + 1 } else { j };
        if let Some((rest, end)) = small_at(words, k) {
            value += rest;
            j = end;
        }
    }
    Some((value, j))
}

/// The number the whole run reads as by the number grammar: a small number, hundreds and what follows them, a count
/// of thousands and what follows it; none where a word is left over.
fn cardinal(words: &[&str]) -> Option<u64> {
    if words.is_empty() {
        return None;
    }
    if let Some(at) = words.iter().position(|word| *word == THOUSAND) {
        let (head, end) = below_thousand(&words[..at], 0)?;
        if end != at {
            return None;
        }
        let mut value = head * 1000;
        let j = at + 1;
        if j < words.len() {
            let k = if words[j] == AND { j + 1 } else { j };
            let (rest, end) = below_thousand(words, k)?;
            if end != words.len() {
                return None;
            }
            value += rest;
        }
        return Some(value);
    }
    let (value, end) = below_thousand(words, 0)?;
    (end == words.len()).then_some(value)
}

/// The run's figures, each word its own: a unit one figure, «oh» and «zero» 0, «double» and «triple» the figure
/// after again, figures as typed; a teen or a tens (with its unit) two figures, and a closing «hundred» after a
/// number 00, which make the reading pairs. None where a word is none of these.
fn digits(words: &[&str]) -> Option<(String, bool)> {
    let mut out = String::new();
    let mut pairs = false;
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        if let Some(times) = repeat(word) {
            let next = words.get(i + 1);
            let figure = next
                .and_then(|next| unit(next))
                .map(|figure| figure.to_string())
                .or_else(|| {
                    next.filter(|next| figures(next) && next.chars().count() == 1)
                        .map(|next| (*next).to_owned())
                });
            if let Some(figure) = figure {
                out.push_str(&figure.repeat(times));
                i += 2;
                continue;
            }
            return None;
        }
        if let Some(figure) = unit(word) {
            out.push_str(&figure.to_string());
        } else if figures(word) {
            out.push_str(word);
        } else if let Some(teen) = teen(word) {
            out.push_str(&teen.to_string());
            pairs = true;
        } else if let Some(tens) = ten(word) {
            match unit_after_tens(words.get(i + 1)) {
                Some(unit) => {
                    out.push_str(&(tens + unit).to_string());
                    i += 1;
                }
                None => out.push_str(&tens.to_string()),
            }
            pairs = true;
        } else if word == HUNDRED
            && !out.is_empty()
            && i == words.len() - 1
            && words[i - 1] != HUNDRED
        {
            out.push_str("00");
            pairs = true;
        } else {
            return None;
        }
        i += 1;
    }
    (!out.is_empty()).then_some((out, pairs))
}

/// The runs of neighbouring words that `keep` keeps, first and last place.
fn runs_of(words: &[Word], keep: impl Fn(&Word) -> bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if !keep(&words[i]) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j + 1 < words.len() && keep(&words[j + 1]) {
            j += 1;
        }
        out.push((i, j));
        i = j + 1;
    }
    out
}

/// The runs of number words, «double» and «triple» among them; «and» joins two of them only after «hundred» or
/// «thousand», «two hundred and six», so «five oh two and five oh nine» is two runs.
fn number_runs(words: &[Word]) -> Vec<(usize, usize)> {
    let counted = |at: usize| {
        words
            .get(at)
            .is_some_and(|word| number_word(&word.plain) || repeat(&word.plain).is_some())
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if !counted(i) {
            i += 1;
            continue;
        }
        let mut j = i;
        loop {
            if counted(j + 1) {
                j += 1;
            } else if words.get(j + 1).is_some_and(|word| word.plain == AND)
                && (words[j].plain == HUNDRED || words[j].plain == THOUSAND)
                && counted(j + 2)
            {
                j += 2;
            } else {
                break;
            }
        }
        out.push((i, j));
        i = j + 1;
    }
    out
}

/// The words of a version said aloud: number words joined by «point» or «dot» twice or more, «four point thirteen
/// point one». No part of one is a number.
fn versions(words: &[Word]) -> Vec<bool> {
    let mut marked = vec![false; words.len()];
    let runs = number_runs(words);
    let mut r = 0;
    while r < runs.len() {
        let mut last = r;
        while let Some(next) = runs.get(last + 1)
            && next.0 == runs[last].1 + 2
            && POINT_SIGNS.contains(&words[runs[last].1 + 1].plain.as_str())
        {
            last += 1;
        }
        if last - r >= 2 {
            marked[runs[r].0..=runs[last].1]
                .iter_mut()
                .for_each(|word| *word = true);
        }
        r = last + 1;
    }
    marked
}

/// The numbers said aloud: a run of number words read by the number grammar, else figure by figure; a decimal,
/// the run then «point» or «dot» and figures said. One number word alone is the recognizer's, which reads it or
/// hides it as part of a form it refuses, «from six to nine»; «oh» alone is no number, «oh no»; no part of a
/// version said aloud is one.
fn numbers(words: &[Word]) -> Vec<Reading> {
    let mut out = Vec::new();
    let mut consumed: Option<usize> = None;
    let versioned = versions(words);
    for (a, b) in number_runs(words) {
        if consumed.is_some_and(|until| a <= until) || versioned[a] {
            continue;
        }
        let said: Vec<&str> = words[a..=b]
            .iter()
            .map(|word| word.plain.as_str())
            .collect();
        let point = words
            .get(b + 1)
            .is_some_and(|word| POINT_SIGNS.contains(&word.plain.as_str()));
        let after = words
            .get(b + 2)
            .is_some_and(|word| unit(&word.plain).is_some());
        if point && after {
            let mut k = b + 2;
            let mut fraction = String::new();
            while let Some(figure) = words.get(k).and_then(|word| unit(&word.plain)) {
                fraction.push_str(&figure.to_string());
                k += 1;
            }
            let head = cardinal(&said)
                .or_else(|| digits(&said).and_then(|(figures, _)| figures.parse().ok()));
            if let Some(head) = head
                && let Ok(value) = format!("{head}.{fraction}").parse::<f64>()
            {
                out.push(reading(
                    words,
                    a,
                    k - 1,
                    Value::Number(value),
                    Marks::DECIMAL,
                ));
                consumed = Some(k - 1);
                continue;
            }
        }
        if a == b {
            continue;
        }
        if let Some(value) = cardinal(&said) {
            out.push(reading(
                words,
                a,
                b,
                Value::Number(float(value)),
                Marks::default(),
            ));
        } else if let Some((figures, pairs)) = digits(&said)
            && let Ok(value) = figures.parse::<u64>()
        {
            let marks = if pairs {
                Marks::PAIRS
            } else {
                Marks::default()
            };
            out.push(reading(words, a, b, Value::Number(float(value)), marks));
        }
    }
    out
}

/// A number as it is typed: whole without a point.
fn shown_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

/// A count as a number, to read it with a fraction, a unit or a range: a count said aloud is small.
#[expect(clippy::cast_precision_loss)]
fn float(count: u64) -> f64 {
    count as f64
}

/// A length summed in seconds as a whole count of them, none below nought.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole_seconds(seconds: f64) -> Option<u64> {
    (seconds >= 0.0 && seconds.is_finite()).then(|| seconds.round_ties_even() as u64)
}

// ---- codes --------------------------------------------------------------------------------------------------

/// What a word is inside a code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Piece {
    /// A number word, «double» or «triple».
    Number,
    /// Figures as typed.
    Figures,
    /// A dash said.
    Dash,
    /// A point said.
    Point,
    /// One letter.
    Letter,
    /// Letters and figures typed together: «C02».
    Typed,
    /// Two to four capitals: «HS».
    Capitals,
    /// Two or three small letters that are no function word: «lh».
    Short,
    /// «version» before a version.
    Version,
}

impl Piece {
    /// Whether a code holds it for itself, not only as an opening or a sign.
    const fn core(self) -> bool {
        matches!(
            self,
            Self::Number | Self::Figures | Self::Letter | Self::Typed | Self::Capitals
        )
    }
}

/// What a word is inside a code; none for a word no code holds, a function word among them: it opens no code.
fn piece(word: &Word, version: bool) -> Option<Piece> {
    let (plain, raw) = (word.plain.as_str(), word.raw.as_str());
    if number_word(plain) || repeat(plain).is_some() {
        return Some(Piece::Number);
    }
    if figures(plain) {
        return Some(Piece::Figures);
    }
    if DASH_SIGNS.contains(&plain) {
        return Some(Piece::Dash);
    }
    if POINT_SIGNS.contains(&plain) {
        return Some(Piece::Point);
    }
    let length = raw.chars().count();
    if length == 1 && raw.chars().all(char::is_alphabetic) {
        return Some(Piece::Letter);
    }
    let alphanumeric = raw.chars().all(|c| c.is_ascii_alphanumeric());
    if alphanumeric
        && raw.chars().any(|c| c.is_ascii_digit())
        && raw.chars().any(|c| c.is_ascii_alphabetic())
    {
        return Some(Piece::Typed);
    }
    if (2..=4).contains(&length) && raw.chars().all(|c| c.is_ascii_uppercase()) {
        return Some(Piece::Capitals);
    }
    if (2..=3).contains(&length)
        && raw.chars().all(|c| c.is_ascii_lowercase())
        && !words::function(plain)
        && !PRONOUNS.contains(&plain)
    {
        return Some(Piece::Short);
    }
    if version && length > 1 && VERSION_WORDS.contains(&plain) {
        return Some(Piece::Version);
    }
    None
}

/// Whether a piece may open a code and be no part of it: a short word, «version», a letter that is a word, and
/// «v» before a version.
fn opening(word: &Word, piece: Option<Piece>, version: bool) -> bool {
    match piece {
        Some(Piece::Short | Piece::Version) => true,
        Some(Piece::Letter) => {
            LETTER_WORDS.contains(&word.plain.as_str())
                || (version && VERSION_WORDS.contains(&word.plain.as_str()))
        }
        _ => false,
    }
}

/// The runs of a code's pieces: the first place, the first place after its optional openings, the last, and
/// whether anything of it is said: a number word, a letter said apart or a sign. A sign stands only between two
/// pieces; a run holds a figure, a number word or a typed part. A run of typed parts alone, «HS 0409», is a code
/// typed with spaces; one typed word alone, «eu261», is typed whole, which the recognizer reads or not.
fn code_runs(
    words: &[Word],
    pieces: &[Option<Piece>],
    version: bool,
) -> Vec<(usize, usize, usize, bool)> {
    let core = |at: usize| pieces.get(at).copied().flatten().is_some_and(Piece::core);
    let sign = |at: usize| {
        matches!(
            pieces.get(at).copied().flatten(),
            Some(Piece::Dash | Piece::Point)
        )
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if pieces[i].is_none() || sign(i) {
            i += 1;
            continue;
        }
        let a = i;
        let mut j = i;
        while j + 1 < words.len()
            && opening(&words[j], pieces[j], version)
            && (core(j + 1) || matches!(pieces[j + 1], Some(Piece::Dash)))
        {
            j += 1;
        }
        if !core(j) {
            i += 1;
            continue;
        }
        let start = j;
        let mut b = j;
        loop {
            if core(b + 1) {
                b += 1;
            } else if sign(b + 1) && core(b + 2) {
                b += 2;
            } else {
                break;
            }
        }
        let figured = (start..=b).any(|at| {
            matches!(
                pieces[at],
                Some(Piece::Number | Piece::Figures | Piece::Typed)
            )
        });
        let said = (start..=b).any(|at| {
            matches!(
                pieces[at],
                Some(Piece::Number | Piece::Letter | Piece::Dash | Piece::Point)
            )
        });
        if figured && (said || b > a) {
            out.push((a, start, b, said));
        }
        i = b + 1;
    }
    out
}

/// Every reading of every run of a code: letters as typed, number words by their figures (singly, in pairs, and by
/// the number grammar where that reads otherwise), «double» or «triple» before a letter the letter again, a lone
/// «o» as the letter and as nought, signs as said; each opening kept and dropped; «v» and «version» dropped before
/// a version only.
fn codes(words: &[Word], version: bool) -> Vec<Reading> {
    let pieces: Vec<Option<Piece>> = words.iter().map(|word| piece(word, version)).collect();
    let letter = |at: usize| pieces.get(at).copied().flatten() == Some(Piece::Letter);
    // «double» before a letter says the letter again, and is no number word of the figures beside it
    let doubled = |at: usize| repeat(&words[at].plain).is_some() && letter(at + 1);
    let plain = Marks::default();
    let mut out = Vec::new();
    for (a, core, b, said) in code_runs(words, &pieces, version) {
        for start in a..=core {
            let mut segments: Vec<Vec<(String, Marks)>> = Vec::new();
            let mut broken = false;
            let mut x = start;
            while x <= b {
                let word = &words[x];
                match pieces[x] {
                    Some(Piece::Number) if doubled(x) && x < b => {
                        let times = repeat(&word.plain).unwrap_or(1);
                        segments.push(vec![(words[x + 1].raw.repeat(times), plain)]);
                        x += 2;
                        continue;
                    }
                    Some(Piece::Number) => {
                        let mut y = x;
                        while y < b && pieces[y + 1] == Some(Piece::Number) && !doubled(y + 1) {
                            y += 1;
                        }
                        let said: Vec<&str> = words[x..=y]
                            .iter()
                            .map(|word| word.plain.as_str())
                            .collect();
                        let mut alternatives = Vec::new();
                        let read = digits(&said);
                        if let Some((figures, pairs)) = &read {
                            alternatives
                                .push((figures.clone(), if *pairs { Marks::PAIRS } else { plain }));
                        }
                        if let Some(value) = cardinal(&said)
                            && read
                                .as_ref()
                                .is_none_or(|(figures, _)| *figures != value.to_string())
                        {
                            alternatives.push((value.to_string(), plain));
                        }
                        if alternatives.is_empty() {
                            broken = true;
                            break;
                        }
                        segments.push(alternatives);
                        x = y + 1;
                        continue;
                    }
                    Some(Piece::Figures) => segments.push(vec![(word.raw.clone(), plain)]),
                    Some(Piece::Letter) if word.plain == LETTER_O => {
                        segments.push(vec![(word.raw.clone(), plain), ("0".to_owned(), plain)]);
                    }
                    Some(Piece::Letter)
                        if version
                            && VERSION_WORDS.contains(&word.plain.as_str())
                            && x == start
                            && x < core =>
                    {
                        segments.push(vec![(String::new(), plain)]);
                    }
                    Some(Piece::Version) => segments.push(vec![(String::new(), plain)]),
                    Some(Piece::Dash) => segments.push(vec![("-".to_owned(), plain)]),
                    Some(Piece::Point) => segments.push(vec![(".".to_owned(), plain)]),
                    Some(Piece::Letter | Piece::Typed | Piece::Capitals | Piece::Short) | None => {
                        segments.push(vec![(word.raw.clone(), plain)]);
                    }
                }
                x += 1;
            }
            let count = segments.iter().map(Vec::len).product::<usize>();
            if broken || count > MOST_READINGS {
                continue;
            }
            for (combination, marks) in product(&segments) {
                if combination.is_empty() {
                    continue;
                }
                let mut read = reading(words, start, b, Value::Code(combination), marks);
                if !said {
                    read.form = Form::Spaced;
                }
                out.push(read);
            }
        }
    }
    out
}

/// Every string one choice from each segment makes, in order, with what its choices carry.
fn product(segments: &[Vec<(String, Marks)>]) -> Vec<(String, Marks)> {
    let mut out = vec![(String::new(), Marks::default())];
    for segment in segments {
        let mut next = Vec::with_capacity(out.len() * segment.len());
        for (head, carried) in &out {
            for (piece, marks) in segment {
                next.push((format!("{head}{piece}"), carried.with(*marks)));
            }
        }
        out = next;
    }
    out
}

/// A version said without «point»: a run of number words cut into `parts` parts, each word a part and a tens word
/// joined to its unit one part: «four twelve oh» is 4.12.0.
fn pointless(words: &[Word], parts: usize) -> Vec<Reading> {
    let mut out = Vec::new();
    let runs = runs_of(words, |word| {
        unit(&word.plain).is_some() || teen(&word.plain).is_some() || ten(&word.plain).is_some()
    });
    for (a, b) in runs {
        let said: Vec<&str> = words[a..=b]
            .iter()
            .map(|word| word.plain.as_str())
            .collect();
        let mut groups: Vec<String> = Vec::new();
        let mut i = 0;
        while i < said.len() {
            if let Some(tens) = ten(said[i])
                && let Some(unit) = unit_after_tens(said.get(i + 1))
            {
                groups.push((tens + unit).to_string());
                i += 2;
                continue;
            }
            let value = unit(said[i])
                .or_else(|| teen(said[i]))
                .or_else(|| ten(said[i]));
            groups.push(value.map_or_else(String::new, |value| value.to_string()));
            i += 1;
        }
        if groups.len() == parts {
            let value = groups.join(".");
            out.push(reading(words, a, b, Value::Code(value), Marks::NO_POINT));
        }
    }
    out
}

// ---- addresses and links ------------------------------------------------------------------------------------

/// An address said aloud: a local part, then «at» and a domain said with «dot» and «dash»; or half typed, the @
/// typed and the domain said. A word standing alone before number words, or letters said apart before them, is read
/// with them and without.
fn emails(words: &[Word]) -> Vec<Reading> {
    let mut out = Vec::new();
    for (i, word) in words.iter().enumerate() {
        let typed_at =
            word.raw.contains('@') && !word.raw.starts_with('@') && !whole_address(&word.raw);
        if word.plain != AT && !typed_at {
            continue;
        }
        let mut domain: Vec<String> = Vec::new();
        let mut j = i + 1;
        let mut locals: Option<Vec<(String, usize)>> = None;
        if typed_at {
            let (local, head) = word.raw.split_once('@').unwrap_or_default();
            domain.push(head.to_lowercase());
            locals = Some(vec![(local.to_lowercase(), word.start)]);
        } else {
            let Some(next) = words.get(j).filter(|next| host_word(&next.plain, true)) else {
                continue;
            };
            domain.push(next.plain.clone());
            j += 1;
        }
        while j + 1 < words.len()
            && let Some(sign) = listed(&HOST_SIGNS, &words[j].plain)
            && host_word(&words[j + 1].plain, false)
        {
            domain.push(sign.to_owned());
            domain.push(words[j + 1].plain.clone());
            j += 2;
        }
        let domain = domain.concat();
        if !whole_domain(&domain) {
            continue;
        }
        let end = words[j - 1].end;
        let locals = locals.unwrap_or_else(|| local_parts(words, i));
        for (local, start) in locals {
            if local.is_empty() {
                continue;
            }
            out.push(Reading {
                start,
                end,
                value: Value::Email(format!("{local}@{domain}")),
                marks: Marks::default(),
                form: Form::Aloud,
            });
        }
    }
    out
}

/// Whether a word reads as an address typed whole: a local part, @, a host and a dot before two letters or more.
fn whole_address(raw: &str) -> bool {
    let Some((local, host)) = raw.split_once('@') else {
        return false;
    };
    let plain =
        |part: &str| !part.is_empty() && !part.chars().any(|c| c == '@' || c.is_whitespace());
    let Some((_, top)) = host.rsplit_once('.') else {
        return false;
    };
    plain(local)
        && plain(host)
        && top.chars().count() >= 2
        && top.chars().all(|c| c.is_ascii_alphabetic())
}

/// Whether a word may be a part of a host: small letters and figures, a dot or a dash inside where `dotted`.
fn host_word(word: &str, dotted: bool) -> bool {
    let mut chars = word.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || (dotted && c == '.')
        })
}

/// Whether a domain reads whole: labels of small letters, figures and dashes joined by dots, ending on two letters
/// or more.
fn whole_domain(domain: &str) -> bool {
    let labels: Vec<&str> = domain.split('.').collect();
    let label = |label: &&str| {
        !label.is_empty()
            && label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    labels.len() >= 2
        && labels.iter().all(label)
        && labels.last().is_some_and(|top| {
            top.chars().count() >= 2 && top.chars().all(|c| c.is_ascii_lowercase())
        })
}

/// What a word is in an address's local part.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Local {
    Number,
    Letter,
    Word,
}

fn local(word: &Word) -> Option<Local> {
    let plain = word.plain.as_str();
    if number_word(plain) {
        return Some(Local::Number);
    }
    let mut chars = plain.chars();
    if let (Some(c), None) = (chars.next(), chars.clone().next())
        && c.is_alphabetic()
    {
        return Some(Local::Letter);
    }
    let mut chars = plain.chars();
    let opens = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
    (opens && rest && !words::function(plain)).then_some(Local::Word)
}

/// The group of an address's local part that ends at `k`: letters said apart, or one word; where it begins.
fn group_ending(words: &[Word], k: usize) -> Option<usize> {
    match local(&words[k])? {
        Local::Letter => {
            let mut from = k;
            while from > 0 && local(&words[from - 1]) == Some(Local::Letter) {
                from -= 1;
            }
            Some(from)
        }
        Local::Word => Some(k),
        Local::Number => None,
    }
}

/// The local parts the words before «at» may read as, each with where it begins: groups of letters said apart or
/// words, joined by a sign said, then number words; two groups side by side end it. Where number words follow a
/// group no sign joins to another, the local part is read with the group and without it: «novak seventy two».
fn local_parts(words: &[Word], at: usize) -> Vec<(String, usize)> {
    let Some(last) = at.checked_sub(1) else {
        return Vec::new();
    };
    if local(&words[last]).is_none() {
        return Vec::new();
    }
    let mut first_number = at;
    while first_number > 0 && local(&words[first_number - 1]) == Some(Local::Number) {
        first_number -= 1;
    }
    let numbers: Vec<&str> = words[first_number..at]
        .iter()
        .map(|word| word.plain.as_str())
        .collect();
    let mut head: Option<usize> = None;
    let mut joined = false;
    if let Some(end) = first_number.checked_sub(1)
        && let Some(mut from) = group_ending(words, end)
    {
        while from >= 2
            && listed(&ADDRESS_SIGNS, &words[from - 1].plain).is_some()
            && let Some(before) = group_ending(words, from - 2)
        {
            from = before;
            joined = true;
        }
        head = Some(from);
    }
    let figures = if numbers.is_empty() {
        Some(String::new())
    } else {
        digits(&numbers)
            .map(|(figures, _)| figures)
            .or_else(|| cardinal(&numbers).map(|value| value.to_string()))
    };
    let Some(figures) = figures else {
        return Vec::new();
    };
    let mut out = Vec::new();
    match head {
        Some(from) => {
            let text: String = words[from..first_number]
                .iter()
                .map(|word| {
                    listed(&ADDRESS_SIGNS, &word.plain)
                        .unwrap_or(&word.plain)
                        .to_owned()
                })
                .collect();
            out.push((format!("{text}{figures}"), words[from].start));
            if !numbers.is_empty() && !joined {
                out.push((figures, words[first_number].start));
            }
        }
        None if !numbers.is_empty() => out.push((figures, words[first_number].start)),
        None => {}
    }
    out
}

/// A link said aloud: a scheme said or spelled, «colon slash slash» or not, a host and a path said with «dot»,
/// «slash», «dash»; letters said apart in a path joined. No scheme, no reading.
fn urls(words: &[Word]) -> Vec<Reading> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let mut scheme: Option<(String, usize)> = None;
        if SCHEMES.contains(&words[i].plain.as_str()) {
            scheme = Some((words[i].plain.clone(), i + 1));
        } else {
            let mut letters = String::new();
            let mut k = i;
            while k < words.len()
                && single_letter(&words[k].plain)
                && letters.chars().count() < SCHEME_LETTERS
            {
                letters.push_str(&words[k].plain);
                k += 1;
            }
            if SCHEMES.contains(&letters.as_str()) {
                scheme = Some((letters, k));
            }
        }
        let Some((scheme, mut j)) = scheme else {
            i += 1;
            continue;
        };
        let said = words.get(j..j + SCHEME_SIGNS.len()).is_some_and(|next| {
            next.iter()
                .zip(SCHEME_SIGNS)
                .all(|(word, sign)| word.plain == sign)
        });
        if said {
            j += SCHEME_SIGNS.len();
        }
        let Some(first) = words.get(j) else {
            i += 1;
            continue;
        };
        let mut host = first.plain.clone();
        j += 1;
        while j + 1 < words.len()
            && let Some(sign) = listed(&HOST_SIGNS, &words[j].plain)
        {
            host.push_str(sign);
            host.push_str(&words[j + 1].plain);
            j += 2;
        }
        let mut path = String::new();
        while j + 1 < words.len() && words[j].plain == SLASH {
            let mut segment = words[j + 1].plain.clone();
            j += 2;
            while j + 1 < words.len()
                && let Some(sign) = listed(&PATH_SIGNS, &words[j].plain).filter(|sign| *sign != "/")
            {
                j += 1;
                if words[j].plain == SLASH {
                    break;
                }
                let mut letters = String::new();
                while j < words.len() && single_character(&words[j].plain) {
                    letters.push_str(&words[j].plain);
                    j += 1;
                }
                if letters.is_empty() {
                    segment.push_str(sign);
                    segment.push_str(&words[j].plain);
                    j += 1;
                } else {
                    segment.push_str(sign);
                    segment.push_str(&letters);
                }
            }
            path.push('/');
            path.push_str(&segment);
        }
        let value = format!("{scheme}://{host}{path}");
        if whole_link(&host, &path) {
            out.push(Reading {
                start: words[i].start,
                end: words[j - 1].end,
                value: Value::Url(value),
                marks: Marks::default(),
                form: Form::Aloud,
            });
        }
        i = j;
    }
    out
}

fn single_letter(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next().is_some_and(char::is_alphabetic) && chars.next().is_none()
}

fn single_character(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next().is_some_and(char::is_alphanumeric) && chars.next().is_none()
}

/// Whether a link's host reads whole, labels joined by dots ending on two letters or more, and its path is text.
fn whole_link(host: &str, path: &str) -> bool {
    whole_domain(host) && !path.chars().any(char::is_whitespace)
}

// ---- days ---------------------------------------------------------------------------------------------------

/// The days written in a form no recognizer reads: a short form of a weekday or a day word, a day word misspelt,
/// with «this», «next», «last» before a weekday and «the day after» before tomorrow; an ordinal misspelt after
/// «the». A weekday misspelt is read by `words.rs` (`misspelt`), and a day after «every» or «each» is a
/// recurrence.
fn days(words: &[Word]) -> Vec<Reading> {
    let mut out = Vec::new();
    for (i, word) in words.iter().enumerate() {
        if i > 0 && EVERY.contains(&words[i - 1].plain.as_str()) {
            continue;
        }
        let plain = word.plain.as_str();
        let bare =
            if listed(&SHORT_DAY_WORDS, plain).is_some() || listed(&DAY_WORDS, plain).is_some() {
                plain
            } else {
                DAY_ENDINGS
                    .iter()
                    .find_map(|ending| plain.strip_suffix(ending))
                    .unwrap_or(plain)
            };
        let mut marks = Marks::default();
        let day = if let Some(at) = listed(&SHORT_DAYS, bare) {
            Day::Weekday {
                weekday: Weekday::ALL[at],
                which: None,
            }
        } else if let Some(days) = listed(&SHORT_DAY_WORDS, bare) {
            Day::Offset { days }
        } else if let Some(days) = misspelt_day(bare) {
            marks = Marks::MISSPELT;
            Day::Offset { days }
        } else {
            continue;
        };
        let mut start = word.start;
        let day = match day {
            Day::Weekday { weekday, .. } => match i
                .checked_sub(1)
                .and_then(|before| listed(&WHICH, &words[before].plain))
            {
                Some(which) => {
                    start = words[i - 1].start;
                    Day::Weekday {
                        weekday,
                        which: Some(which),
                    }
                }
                None => Day::Weekday {
                    weekday,
                    which: None,
                },
            },
            Day::Offset { days: 1 }
                if i >= DAY_AFTER.len()
                    && words[i - DAY_AFTER.len()..i]
                        .iter()
                        .zip(DAY_AFTER)
                        .all(|(word, said)| word.plain == said) =>
            {
                start = words[i - DAY_AFTER.len()].start;
                Day::Offset { days: 2 }
            }
            other => other,
        };
        out.push(Reading {
            start,
            end: word.end,
            value: Value::Day(day),
            marks,
            form: Form::Misspelt,
        });
    }
    for i in 1..words.len() {
        let plain = words[i].plain.as_str();
        if words[i - 1].plain != THE
            || listed(&ORDINALS, plain).is_some()
            || !plain.chars().all(char::is_alphabetic)
            || plain.chars().count() < ORDINAL_LEAST
        {
            continue;
        }
        let mut near: Vec<(usize, &str, u8)> = ORDINALS
            .iter()
            .map(|(name, day)| (words::distance(plain, name), *name, *day))
            .collect();
        near.sort_unstable();
        if let [(nearest, _, day), (next, ..), ..] = near[..]
            && nearest == 1
            && next > nearest
        {
            out.push(Reading {
                start: words[i - 1].start,
                end: words[i].end,
                value: Value::Day(Day::Nth { day }),
                marks: Marks::MISSPELT,
                form: Form::Misspelt,
            });
        }
    }
    out
}

/// The day word a word of four letters or more misspells, within the radius its length allows and nearer than
/// every other day's name; none where the nearest is a weekday, which `words.rs` reads.
fn misspelt_day(word: &str) -> Option<i32> {
    if !word.chars().all(char::is_alphabetic)
        || word.chars().count() < 4
        || words::function(word)
        || WEEKDAYS.contains(&word)
        || listed(&DAY_WORDS, word).is_some()
    {
        return None;
    }
    let mut near: Vec<(usize, &str)> = WEEKDAYS
        .iter()
        .chain(DAY_WORDS.iter().map(|(name, _)| name))
        .map(|name| (words::distance(word, name), *name))
        .collect();
    near.sort_unstable();
    let [(nearest, name), (next, _), ..] = near[..] else {
        return None;
    };
    if nearest == 0 || nearest > by_length(name) || next == nearest {
        return None;
    }
    listed(&DAY_WORDS, name)
}

/// A day as the reader writes it back: the weekday with its «this», a day word, the day of the month with its
/// ending.
fn shown_day(day: &Day) -> String {
    match day {
        Day::Weekday { weekday, which } => {
            let name = WEEKDAYS[Weekday::ALL
                .iter()
                .position(|day| day == weekday)
                .unwrap_or(0)];
            match which.and_then(|which| WHICH.iter().find(|(_, said)| *said == which)) {
                Some((word, _)) => format!("{word} {name}"),
                None => name.to_owned(),
            }
        }
        Day::Nth { day } => {
            let ending = if (11..=13).contains(&(day % 100)) {
                ORDINAL_ENDINGS[0]
            } else {
                match day % 10 {
                    1 => ORDINAL_ENDINGS[1],
                    2 => ORDINAL_ENDINGS[2],
                    3 => ORDINAL_ENDINGS[3],
                    _ => ORDINAL_ENDINGS[0],
                }
            };
            format!("{THE} {day}{ending}")
        }
        Day::Offset { days } => DAYS_SHOWN
            .iter()
            .find(|(offset, _)| offset == days)
            .map_or_else(|| days.to_string(), |(_, shown)| (*shown).to_owned()),
        Day::Calendar { .. } => String::new(),
    }
}

// ---- durations ----------------------------------------------------------------------------------------------

/// A length said in more words than a recognizer reads: counts with their units summed, «an hour and thirty
/// minutes»; a bare count after a unit read in the next unit, «one hr 30», «1h30»; a count with a point, «one point
/// five hours»; a number word misspelt before a unit.
fn durations(words: &[Word]) -> Vec<Reading> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let mut parts: Vec<f64> = Vec::new();
        let mut marks = Marks::default();
        let mut j = i;
        while let Some((count, k, counted)) = count_at(words, j) {
            let Some(unit) = words
                .get(k)
                .and_then(|word| listed(&DURATION_UNITS, &word.plain))
            else {
                break;
            };
            let mut count = count;
            let mut k = k + 1;
            if half_after(words, k) {
                count += 0.5;
                k += HALF_WORDS.len();
            }
            parts.push(count * float(unit));
            marks = marks.with(counted);
            j = k;
            let next_unit = NEXT_UNIT
                .iter()
                .find(|(from, _)| *from == unit)
                .map(|(_, to)| *to);
            if let Some(next) = next_unit
                && let Some((bare, end, flags)) = count_at(words, j)
                && words
                    .get(end)
                    .is_none_or(|word| listed(&DURATION_UNITS, &word.plain).is_none())
                && !flags.has(Marks::ARTICLE)
            {
                parts.push(bare * float(next));
                marks = marks.with(Marks::CONVENTION).with(flags);
                j = end;
                break;
            }
            if words.get(j).is_some_and(|word| word.plain == AND)
                && let Some((_, end, _)) = count_at(words, j + 1)
                && words
                    .get(end)
                    .is_some_and(|word| listed(&DURATION_UNITS, &word.plain).is_some())
            {
                j += 1;
            }
        }
        let more = parts.len() > 1;
        if !parts.is_empty()
            && (more || marks.has(Marks::DECIMAL.with(Marks::MISSPELT).with(Marks::HALF)))
        {
            if let Some(seconds) = whole_seconds(parts.iter().sum::<f64>()) {
                out.push(reading(words, i, j - 1, Value::Seconds(seconds), marks));
            }
            i = j;
            continue;
        }
        if let Some((hours, minutes, unit)) = typed_length(&words[i].plain) {
            let marks = if unit {
                Marks::default()
            } else {
                Marks::CONVENTION
            };
            out.push(reading(
                words,
                i,
                i,
                Value::Seconds(hours * 3600 + minutes * 60),
                marks,
            ));
        }
        i += 1;
    }
    out
}

/// Whether «and a half» stands at `k`.
fn half_after(words: &[Word], k: usize) -> bool {
    words.get(k..k + HALF_WORDS.len()).is_some_and(|next| {
        next.iter()
            .zip(HALF_WORDS)
            .all(|(word, said)| word.plain == said)
    })
}

/// A length typed together, `1h30` or `1h30m`: the hours, the minutes, and whether the minutes' unit is typed.
fn typed_length(word: &str) -> Option<(u64, u64, bool)> {
    let (hours, rest) = word.split_once('h')?;
    let (minutes, unit) = match rest.strip_suffix('m') {
        Some(minutes) => (minutes, true),
        None => (rest, false),
    };
    if !figures(hours) || !figures(minutes) {
        return None;
    }
    Some((hours.parse().ok()?, minutes.parse().ok()?, unit))
}

/// A count at `i`: figures, number words by the number grammar, a count with a point said, «a» or «an» before a
/// unit, «half» or «a half» before one, or a number word misspelt within a letter (five letters or more); with
/// «and a half» after it before a unit. The count, where it ends, and what it carries.
fn count_at(words: &[Word], i: usize) -> Option<(f64, usize, Marks)> {
    let word = words.get(i)?;
    let plain = word.plain.as_str();
    let unit_at = |at: usize| {
        words
            .get(at)
            .is_some_and(|word| listed(&DURATION_UNITS, &word.plain).is_some())
    };
    if let Some(value) = typed_count(plain) {
        return Some((value, i + 1, Marks::default()));
    }
    if ARTICLES.contains(&plain) && unit_at(i + 1) {
        return Some((1.0, i + 1, Marks::ARTICLE));
    }
    if plain == ARTICLES[0]
        && words.get(i + 1).is_some_and(|word| word.plain == HALF)
        && unit_at(i + 2)
    {
        return Some((0.5, i + 2, Marks::HALF));
    }
    if plain == HALF && unit_at(i + 1) {
        return Some((0.5, i + 1, Marks::HALF));
    }
    let mut j = i;
    let mut said: Vec<String> = Vec::new();
    while let Some(next) = words
        .get(j)
        .filter(|next| number_word(&next.plain) && next.plain != OH)
    {
        said.push(next.plain.clone());
        j += 1;
    }
    let mut marks = Marks::default();
    if said.is_empty()
        && plain.chars().all(char::is_alphabetic)
        && plain.chars().count() >= MISSPELT_LEAST
    {
        let mut near: Vec<(usize, &str)> = TENS
            .iter()
            .chain(&TEENS)
            .chain(&UNITS)
            .map(|(name, _)| (words::distance(plain, name), *name))
            .collect();
        near.sort_unstable();
        if let [(nearest, name), (next, _), ..] = near[..]
            && nearest == 1
            && next > nearest
        {
            said.push(name.to_owned());
            j = i + 1;
            marks = Marks::MISSPELT;
            while let Some(next) = words.get(j).filter(|next| unit(&next.plain).is_some()) {
                said.push(next.plain.clone());
                j += 1;
            }
        }
    }
    if said.is_empty() {
        return None;
    }
    let said: Vec<&str> = said.iter().map(String::as_str).collect();
    let whole = cardinal(&said)?;
    let mut value = float(whole);
    let point = words
        .get(j)
        .is_some_and(|word| POINT_SIGNS.contains(&word.plain.as_str()));
    if point
        && words
            .get(j + 1)
            .is_some_and(|word| unit(&word.plain).is_some())
    {
        let mut k = j + 1;
        let mut fraction = String::new();
        while let Some(figure) = words.get(k).and_then(|word| unit(&word.plain)) {
            fraction.push_str(&figure.to_string());
            k += 1;
        }
        value = format!("{whole}.{fraction}").parse().ok()?;
        j = k;
        marks = marks.with(Marks::DECIMAL);
    }
    if half_after(words, j) && unit_at(j + HALF_WORDS.len()) {
        value += 0.5;
        j += HALF_WORDS.len();
    }
    Some((value, j, marks))
}

/// Figures typed as a count, a point among them or not.
fn typed_count(word: &str) -> Option<f64> {
    let (whole, fraction) = word.split_once('.').unwrap_or((word, "0"));
    (figures(whole) && figures(fraction))
        .then(|| word.parse().ok())
        .flatten()
}

/// A length as it is written back: «1 hour 30 minutes»; none at all in the smallest unit, «0 seconds».
fn shown_length(seconds: u64) -> String {
    if seconds == 0 {
        let (_, _, more) = DURATION_SHOWN[DURATION_SHOWN.len() - 1];
        return format!("0 {more}");
    }
    let mut rest = seconds;
    let mut parts = Vec::new();
    for (unit, one, more) in DURATION_SHOWN {
        let count = rest / unit;
        rest %= unit;
        if count > 0 {
            parts.push(format!("{count} {}", if count == 1 { one } else { more }));
        }
    }
    parts.join(" ")
}

// ---- the shape the examples share -------------------------------------------------------------------------

/// The family of a code, by what every example of it is: a version, figures joined by dots; a ticket, letters then
/// figures, a dash between or not; a serial, any other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Version,
    Ticket,
    Serial,
}

/// The case of an example's letters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Upper,
    Lower,
    Mixed,
}

/// What every example value of an argument shows: the code's family, the counts of letters and figures, the dash,
/// the letters' case, the length, the parts of a version, the count of a number's figures and whether each is
/// whole; and the argument's range.
#[derive(Debug, Default)]
struct Examples {
    family: Option<Family>,
    letters: Vec<usize>,
    figures: Vec<usize>,
    dashed: Vec<bool>,
    cases: Vec<Case>,
    lengths: Vec<usize>,
    parts: Vec<usize>,
    whole: bool,
    range: Option<(f64, f64)>,
}

fn once<T: PartialEq>(list: &mut Vec<T>, value: T) {
    if !list.contains(&value) {
        list.push(value);
    }
}

/// A ticket's letters, whether a dash follows them, and its figures: `HS-0409`.
fn ticket(value: &str) -> Option<(&str, bool, &str)> {
    let letters = value.chars().take_while(char::is_ascii_alphabetic).count();
    let (head, rest) = value.split_at(letters);
    let (dashed, tail) = match rest.strip_prefix('-') {
        Some(tail) => (true, tail),
        None => (false, rest),
    };
    (letters > 0 && figures(tail)).then_some((head, dashed, tail))
}

/// Whether a value is a version: figures joined by dots.
fn version(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() >= 2 && parts.iter().all(|part| figures(part))
}

fn case_of(value: &str) -> Option<Case> {
    let letters: Vec<char> = value.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        None
    } else if letters.iter().all(|c| c.is_uppercase()) {
        Some(Case::Upper)
    } else if letters.iter().all(|c| c.is_lowercase()) {
        Some(Case::Lower)
    } else {
        Some(Case::Mixed)
    }
}

impl Examples {
    fn of(recognizer: Recognizer, examples: &[Clean], pick: &Pick) -> Self {
        let values: Vec<&str> = examples.iter().map(Clean::as_str).collect();
        let mut shown = Self {
            whole: true,
            range: match pick {
                Pick::Number(Some(range)) => Some((range.min(), range.max())),
                _ => None,
            },
            ..Self::default()
        };
        if recognizer == Recognizer::Code && !values.is_empty() {
            shown.family = Some(if values.iter().all(|value| version(value)) {
                Family::Version
            } else if values.iter().all(|value| ticket(value).is_some()) {
                Family::Ticket
            } else {
                Family::Serial
            });
        }
        for value in values {
            if let Some(case) = case_of(value) {
                once(&mut shown.cases, case);
            }
            match (recognizer, shown.family) {
                (Recognizer::Code, Some(Family::Ticket)) => {
                    if let Some((letters, dashed, figures)) = ticket(value) {
                        once(&mut shown.letters, letters.chars().count());
                        once(&mut shown.dashed, dashed);
                        once(&mut shown.figures, figures.chars().count());
                    }
                }
                (Recognizer::Code, Some(Family::Serial)) => {
                    once(&mut shown.lengths, value.chars().count());
                    once(&mut shown.dashed, value.contains('-'));
                }
                (Recognizer::Code, Some(Family::Version)) => {
                    once(&mut shown.parts, value.split('.').count());
                }
                (Recognizer::Number, _) => {
                    let lead: String = value.chars().take_while(char::is_ascii_digit).collect();
                    let count = if lead.is_empty() {
                        let said: Vec<&str> = value
                            .split_whitespace()
                            .filter(|word| number_word(word))
                            .collect();
                        cardinal(&said).map(|number| number.to_string().chars().count())
                    } else {
                        Some(lead.chars().count())
                    };
                    if let Some(count) = count {
                        once(&mut shown.figures, count);
                    }
                    if value.contains('.') {
                        shown.whole = false;
                    }
                }
                _ => {}
            }
        }
        shown
    }
}

// ---- the decision ----------------------------------------------------------------------------------------------

/// How a code reading fits the examples: whole, in a length no example has (asked, nothing padded), or not of their
/// shape at all.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fit {
    Fits,
    Length,
    None,
}

/// A code reading held against the examples: the value, put in their case and with their dash where every example
/// agrees; what it carries; how it fits.
fn code_fit(value: &str, marks: Marks, shown: &Examples) -> (String, Marks, Fit) {
    match shown.family {
        Some(Family::Version) => (value.to_owned(), marks, version_fit(value, marks, shown)),
        Some(Family::Serial) => serial_fit(value, marks, shown),
        Some(Family::Ticket) => ticket_fit(value, marks, shown),
        None => (value.to_owned(), marks, Fit::None),
    }
}

/// How a version reading fits: figures joined by dots, in a count of parts an example has; said without «point», it
/// fits only so, and is none otherwise.
fn version_fit(value: &str, marks: Marks, shown: &Examples) -> Fit {
    if value.chars().any(char::is_alphabetic) || value.contains('-') {
        return Fit::None;
    }
    let counted = shown.parts.contains(&value.split('.').count());
    if marks.has(Marks::NO_POINT) {
        return if counted { Fit::Fits } else { Fit::None };
    }
    if !value.contains('.') {
        Fit::None
    } else if counted {
        Fit::Fits
    } else {
        Fit::Length
    }
}

/// How a serial reading fits: letters and figures, of a length an example has, two off asked; without a dash no
/// example has, in the examples' case.
fn serial_fit(value: &str, marks: Marks, shown: &Examples) -> (String, Marks, Fit) {
    let core: String = value.chars().filter(|c| *c != '-').collect();
    let letters = core.chars().any(|c| c.is_ascii_alphabetic());
    let digits = core.chars().any(|c| c.is_ascii_digit());
    if !core.chars().all(|c| c.is_ascii_alphanumeric()) || !letters || !digits {
        return (value.to_owned(), marks, Fit::None);
    }
    let length = core.chars().count();
    if !shown.lengths.contains(&length) {
        let longest = shown.lengths.iter().max().copied().unwrap_or(0);
        let fit = if length.abs_diff(longest) <= 2 {
            Fit::Length
        } else {
            Fit::None
        };
        return (value.to_owned(), marks, fit);
    }
    let marks = if value.contains('-') {
        marks.with(Marks::DASH_DROPPED)
    } else {
        marks
    };
    let (core, marks) = recased(&core, shown, marks);
    (core, marks, Fit::Fits)
}

/// How a ticket reading fits: letters in a count an example has, then figures, in a count an example has or asked;
/// in the examples' case and with their dash, where every example agrees.
fn ticket_fit(value: &str, marks: Marks, shown: &Examples) -> (String, Marks, Fit) {
    let Some((letters, said_dash, digits)) = ticket(value) else {
        return (value.to_owned(), marks, Fit::None);
    };
    if !shown.letters.contains(&letters.chars().count()) {
        return (value.to_owned(), marks, Fit::None);
    }
    let fit = if shown.figures.contains(&digits.chars().count()) {
        Fit::Fits
    } else {
        Fit::Length
    };
    let (letters, mut marks) = recased(letters, shown, marks);
    let dash = if let [dashed] = shown.dashed.as_slice() {
        if *dashed && !said_dash {
            marks = marks.with(Marks::DASH_ADDED);
        }
        if !*dashed && said_dash {
            marks = marks.with(Marks::DASH_DROPPED);
        }
        *dashed
    } else {
        marks = marks.with(Marks::DASH_UNSETTLED);
        said_dash
    };
    let joined = if dash { "-" } else { "" };
    (format!("{letters}{joined}{digits}"), marks, fit)
}

/// Letters put in the case every example shares, or kept as said where the examples disagree; what that carries.
fn recased(letters: &str, shown: &Examples, marks: Marks) -> (String, Marks) {
    if let [case] = shown.cases.as_slice() {
        let put = cased(letters, *case);
        let marks = if put == letters {
            marks
        } else {
            marks.with(Marks::CASE_REFORMED)
        };
        (put, marks)
    } else {
        (letters.to_owned(), marks.with(Marks::CASE_UNSETTLED))
    }
}

fn cased(text: &str, case: Case) -> String {
    match case {
        Case::Upper => text.to_uppercase(),
        Case::Lower => text.to_lowercase(),
        Case::Mixed => text.to_owned(),
    }
}

/// Whether the examples fix the length of a value said in pairs: one count of figures, and the value's among it; a
/// serial of one length.
fn fixed_length(recognizer: Recognizer, shown: &Examples, value: &Value) -> bool {
    match (recognizer, value) {
        (Recognizer::Number, Value::Number(number)) => {
            shown.figures.len() == 1
                && shown
                    .figures
                    .contains(&shown_number(number.trunc()).chars().count())
        }
        (Recognizer::Code, Value::Code(code)) => match shown.family {
            Some(Family::Ticket) => {
                let count = code.chars().filter(char::is_ascii_digit).count();
                shown.figures.len() == 1 && shown.figures.contains(&count)
            }
            Some(Family::Serial) => shown.lengths.len() == 1,
            _ => true,
        },
        _ => true,
    }
}

/// A run: the readings of words that nest, one inside another, as an opening kept or dropped makes them.
struct Run {
    readings: Vec<Reading>,
}

/// The readings gathered into runs, the widest first where two nest.
fn runs(readings: &[Reading]) -> Vec<Run> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for reading in readings {
        if !spans.contains(&(reading.start, reading.end)) {
            spans.push((reading.start, reading.end));
        }
    }
    spans.sort_by_key(|(start, end)| (*start, std::cmp::Reverse(end - start)));
    let mut runs: Vec<(usize, usize, Vec<Reading>)> = Vec::new();
    for (start, end) in spans {
        let of_span: Vec<Reading> = readings
            .iter()
            .filter(|reading| (reading.start, reading.end) == (start, end))
            .cloned()
            .collect();
        let nests = |(from, to, _): &&mut (usize, usize, Vec<Reading>)| {
            (*from <= start && end <= *to) || (start <= *from && *to <= end)
        };
        match runs.iter_mut().find(nests) {
            Some(run) => {
                run.2.extend(of_span);
                run.0 = run.0.min(start);
                run.1 = run.1.max(end);
            }
            None => runs.push((start, end, of_span)),
        }
    }
    runs.into_iter()
        .map(|(_, _, readings)| Run { readings })
        .collect()
}

/// What the rule does with a run.
enum Outcome {
    /// Proposed for the core's own yes, standing as `shape` says.
    Take {
        value: PickValue,
        typed: String,
        shape: Shape,
    },
    /// Asked, the readings offered.
    Ask { readings: Vec<String> },
}

/// A run decided: the words of the reading it rests on, how they give it, and the outcome.
struct Decided {
    start: usize,
    end: usize,
    form: Form,
    outcome: Outcome,
}

/// A reading held as a value of the kind: the value and its typed form, where the kind's recognizer reads that form
/// whole (a code, an address, a link) or the value is read directly (a number, a day, a length).
fn standing(recognizer: Recognizer, value: &Value) -> Option<(PickValue, String)> {
    match value {
        Value::Number(number) => {
            Some((PickValue::Number { value: *number }, shown_number(*number)))
        }
        Value::Code(text) | Value::Email(text) | Value::Url(text) => {
            let read = words::read_whole(text, recognizer)?;
            Some((read, text.clone()))
        }
        Value::Day(day) => Some((PickValue::Date { value: day.clone() }, shown_day(day))),
        Value::Seconds(seconds) => Some((
            PickValue::Seconds { value: *seconds },
            shown_length(*seconds),
        )),
    }
}

/// One run's readings held against the examples: read whole, shown for a yes, asked with its readings offered, or
/// none. Two readings that both fit, figures in pairs whose length the examples leave open, a version said without
/// «point» and a number word misspelt in a length are asked; a case or a dash the examples do not settle, a decimal
/// where every example is whole, a day misspelt and a count read by convention are shown; a reading of no example's
/// length is asked with nothing offered.
fn decide(recognizer: Recognizer, run: &Run, shown: &Examples) -> Option<Decided> {
    let mut fitting: Vec<(Value, Marks, &Reading, PickValue, String)> = Vec::new();
    let mut off: Option<&Reading> = None;
    for reading in &run.readings {
        let (value, marks, fit) = match &reading.value {
            Value::Code(code) => {
                let (value, marks, fit) = code_fit(code, reading.marks, shown);
                (Value::Code(value), marks, fit)
            }
            Value::Number(number) => {
                let within = shown
                    .range
                    .is_none_or(|(least, most)| least <= *number && *number <= most);
                (
                    reading.value.clone(),
                    reading.marks,
                    if within { Fit::Fits } else { Fit::None },
                )
            }
            other => (other.clone(), reading.marks, Fit::Fits),
        };
        match fit {
            Fit::Fits => {
                let Some((read, typed)) = standing(recognizer, &value) else {
                    continue;
                };
                if !fitting.iter().any(|(held, ..)| *held == value) {
                    fitting.push((value, marks, reading, read, typed));
                }
            }
            Fit::Length => {
                off.get_or_insert(reading);
            }
            Fit::None => {}
        }
    }
    let decided = |reading: &Reading, outcome: Outcome| Decided {
        start: reading.start,
        end: reading.end,
        form: reading.form,
        outcome,
    };
    let Some((value, marks, reading, read, typed)) = fitting.first() else {
        return off.map(|reading| {
            decided(
                reading,
                Outcome::Ask {
                    readings: Vec::new(),
                },
            )
        });
    };
    if fitting.len() >= 2 {
        let readings = fitting.iter().map(|(.., typed)| typed.clone()).collect();
        return Some(decided(reading, Outcome::Ask { readings }));
    }
    let offer = || Outcome::Ask {
        readings: vec![typed.clone()],
    };
    if marks.has(Marks::NO_POINT) {
        return Some(decided(reading, offer()));
    }
    let paired =
        matches!(recognizer, Recognizer::Number | Recognizer::Code) && marks.has(Marks::PAIRS);
    if paired && !fixed_length(recognizer, shown, value) {
        return Some(decided(reading, offer()));
    }
    if recognizer == Recognizer::Duration && marks.has(Marks::MISSPELT) {
        return Some(decided(reading, offer()));
    }
    let unsettled = Marks::CASE_UNSETTLED
        .with(Marks::DASH_UNSETTLED)
        .with(Marks::CONVENTION)
        .with(Marks::MISSPELT);
    let decimal = recognizer == Recognizer::Number && marks.has(Marks::DECIMAL) && shown.whole;
    // A code typed with spaces waits for a yes whatever its shape.
    let spaced = reading.form == Form::Spaced;
    let shape = if marks.has(unsettled) || decimal || spaced {
        Shape::Open
    } else if marks.has(
        Marks::CASE_REFORMED
            .with(Marks::DASH_ADDED)
            .with(Marks::DASH_DROPPED),
    ) {
        Shape::Completed
    } else {
        Shape::Whole
    };
    Some(decided(
        reading,
        Outcome::Take {
            value: read.clone(),
            typed: typed.clone(),
            shape,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Range;
    use crate::propose::propose;

    fn examples(values: &[&str]) -> Vec<Clean> {
        values
            .iter()
            .map(|value| Clean::new(value).unwrap())
            .collect()
    }

    /// What the rule does with an argument's words, as `(words, typed, shape)` for a value taken, `(words, "?", …)`
    /// with the readings joined for a run asked.
    fn read(text: &str, pick: &Pick, values: &[&str]) -> Vec<String> {
        let input = Input::new(text).unwrap();
        let heard = heard(&input, pick, &examples(values), &propose(&input));
        let mut out: Vec<String> = heard
            .said
            .iter()
            .map(|said| format!("{} = {} ({:?})", said.span.text(), said.typed, said.shape))
            .collect();
        out.extend(heard.spoken.asked.iter().map(|asked| {
            let readings: Vec<&str> = asked.readings.iter().map(Clean::as_str).collect();
            format!("{} ? {}", asked.words.text(), readings.join(" | "))
        }));
        out
    }

    const SKUS: [&str; 4] = ["HS-0409", "VX-2214", "VX-221007", "VX-2210"];
    const FLIGHTS: [&str; 3] = ["LX1830", "FR8342", "tp1051"];
    const SERIALS: [&str; 2] = ["C02XK1ABJG5M", "c02g80t3md6r"];
    const RELEASES: [&str; 2] = ["4.12.0", "2.3.1"];
    const INCIDENTS: [&str; 4] = ["311", "42", "1207", "88"];

    #[test]
    fn a_code_takes_the_shape_every_example_shares() {
        assert_eq!(
            read("put K D oh three one five aside", &Pick::Code, &SKUS),
            ["K D oh three one five = KD-0315 (Some(Completed))"]
        );
        assert_eq!(
            read("put k d oh three one five aside", &Pick::Code, &SKUS),
            ["k d oh three one five = KD-0315 (Some(Completed))"]
        );
        // The words spell it whole: every character as they say it.
        assert_eq!(
            read("put R T dash oh two seven oh aside", &Pick::Code, &SKUS),
            ["R T dash oh two seven oh = RT-0270 (Some(Whole))"]
        );
        // Pairs where the examples show four figures and six: asked, the reading offered.
        assert_eq!(
            read("put B N forty one seventeen aside", &Pick::Code, &SKUS),
            ["B N forty one seventeen ? BN-4117"]
        );
        // The examples disagree on the case: shown for a yes.
        assert_eq!(
            read("is Q R nineteen forty on time", &Pick::Code, &FLIGHTS),
            ["Q R nineteen forty = QR1940 (Some(Open))"]
        );
    }

    #[test]
    fn no_function_word_opens_a_code_and_a_time_after_it_is_none_of_it() {
        // «the» opens no code: the702 never; three parts without «point» are asked, the version offered.
        assert_eq!(
            read("bring back the seven oh two", &Pick::Code, &RELEASES),
            ["seven oh two ? 7.0.2"]
        );
        assert_eq!(
            read("bring back four twelve oh", &Pick::Code, &RELEASES),
            ["four twelve oh ? 4.12.0"]
        );
        assert_eq!(
            read(
                "bring back v four point twelve point oh",
                &Pick::Code,
                &RELEASES
            ),
            ["v four point twelve point oh = 4.12.0 (Some(Whole))"]
        );
        assert_eq!(
            read(
                "change Q R nineteen forty at nine forty five",
                &Pick::Code,
                &FLIGHTS
            ),
            ["Q R nineteen forty = QR1940 (Some(Open))"]
        );
    }

    #[test]
    fn a_serial_typed_apart_is_read_whole() {
        assert_eq!(
            read("lock F 7 1 Q M 4 Z T 9 K 2 D", &Pick::Code, &SERIALS),
            ["F 7 1 Q M 4 Z T 9 K 2 D = F71QM4ZT9K2D (Some(Open))"]
        );
        assert_eq!(
            read(
                "lock C zero two double L seven K W H V two R",
                &Pick::Code,
                &SERIALS
            ),
            ["C zero two double L seven K W H V two R = C02LL7KWHV2R (Some(Open))"]
        );
        // A lone O where letters and figures both stand: both offered.
        assert_eq!(
            read(
                "lock F O one Q M four Z T nine K two D",
                &Pick::Code,
                &SERIALS
            ),
            ["F O one Q M four Z T nine K two D ? FO1QM4ZT9K2D | F01QM4ZT9K2D"]
        );
    }

    #[test]
    fn an_address_is_never_read_in_part() {
        let people = ["lee@example.com", "sam@example.org"];
        assert_eq!(
            read(
                "write to rivera forty one at example dot com",
                &Pick::Email,
                &people
            ),
            ["rivera forty one at example dot com ? rivera41@example.com | 41@example.com"]
        );
        assert_eq!(
            read(
                "send it to carlos dot m at rocha dot com dot br",
                &Pick::Email,
                &people
            ),
            ["carlos dot m at rocha dot com dot br = carlos.m@rocha.com.br (Some(Whole))"]
        );
        assert_eq!(
            read(
                "send it to m dot o s e i at harbourline dot com",
                &Pick::Email,
                &people
            ),
            ["m dot o s e i at harbourline dot com = m.osei@harbourline.com (Some(Whole))"]
        );
        assert_eq!(
            read(
                "mail k w seventy two at ostrand dot co",
                &Pick::Email,
                &people
            ),
            ["k w seventy two at ostrand dot co ? kw72@ostrand.co | 72@ostrand.co"]
        );
        assert_eq!(
            read(
                "email dana at example dot com at five",
                &Pick::Email,
                &people
            ),
            ["dana at example dot com = dana@example.com (Some(Whole))"]
        );
    }

    #[test]
    fn a_candidate_inside_a_spoken_run_is_withdrawn() {
        let level = Pick::Number(Range::new(0.0, 100.0));
        let input = Input::new("set the volume to seven oh").unwrap();
        let proposed = propose(&input);
        let heard = heard(&input, &level, &examples(&["60%", "40"]), &proposed);
        assert_eq!(heard.said.len(), 1);
        assert_eq!(heard.said[0].typed.as_str(), "70");
        let withdrawn: Vec<&str> = heard
            .spoken
            .withdrawn
            .iter()
            .map(|span| span.text().as_str())
            .collect();
        assert_eq!(withdrawn, ["seven"], "«seven oh» is not 7");
        assert_eq!(
            read("set the volume to double eight", &level, &["60%", "40"]),
            ["double eight = 88 (Some(Whole))"]
        );
        // Out of the range: none.
        assert!(read("set the volume to three eleven", &level, &["60%", "40"]).is_empty());
    }

    #[test]
    fn figures_in_pairs_are_asked_where_the_examples_leave_the_length_open() {
        assert_eq!(
            read(
                "open incident three eleven",
                &Pick::Number(None),
                &INCIDENTS
            ),
            ["three eleven ? 311"]
        );
        assert_eq!(
            read(
                "open incident four oh seven",
                &Pick::Number(None),
                &INCIDENTS
            ),
            ["four oh seven = 407 (Some(Whole))"]
        );
        // A cardinal the recognizer reads is its own: the reader leaves it to the question.
        assert!(
            read(
                "open incident three hundred eleven",
                &Pick::Number(None),
                &INCIDENTS
            )
            .is_empty()
        );
    }

    #[test]
    fn days_and_lengths_in_more_words() {
        let days = ["thursday", "tomorrow"];
        assert_eq!(
            read("what is on tmrw", &Pick::Date, &days),
            ["tmrw = tomorrow (Some(Whole))"]
        );
        assert_eq!(
            read("what is on next fri", &Pick::Date, &days),
            ["next fri = next friday (Some(Whole))"]
        );
        assert_eq!(
            read("what is on tomorow", &Pick::Date, &days),
            ["tomorow = tomorrow (Some(Open))"]
        );
        assert!(
            read("stand up every fri", &Pick::Date, &days).is_empty(),
            "a recurrence"
        );
        let lengths = ["2 hours", "30 minutes"];
        assert_eq!(
            read(
                "stay awake for an hour and twenty minutes",
                &Pick::Duration(None),
                &lengths
            ),
            ["an hour and twenty minutes = 1 hour 20 minutes (Some(Whole))"]
        );
        assert_eq!(
            read("stay awake for one hr 30", &Pick::Duration(None), &lengths),
            ["one hr 30 = 1 hour 30 minutes (Some(Open))"]
        );
    }

    #[test]
    fn words_that_are_another_value_or_no_value_are_never_read() {
        let level = Pick::Number(Range::new(0.0, 100.0));
        assert!(
            read(
                "Oh dear, the baby is asleep upstairs",
                &level,
                &["60%", "40"]
            )
            .is_empty()
        );
        // Words a length holds are the length's.
        assert!(
            read(
                "fifteen minutes on the clock for the rice",
                &level,
                &["60%", "40"]
            )
            .is_empty()
        );
        assert!(
            read(
                "up for twenty hours, find me a cab",
                &Pick::Number(None),
                &INCIDENTS
            )
            .is_empty()
        );
        // A version said aloud holds no number.
        assert!(
            read(
                "bring back four point thirteen point one",
                &Pick::Number(None),
                &INCIDENTS
            )
            .is_empty()
        );
        // «and» joins only after «hundred»; a word of marks alone joins nothing.
        assert_eq!(
            read(
                "open incidents five oh two and five oh nine",
                &Pick::Number(None),
                &INCIDENTS
            ),
            [
                "five oh two = 502 (Some(Whole))",
                "five oh nine = 509 (Some(Whole))"
            ]
        );
        assert_eq!(
            read(
                "open incs four ninety one & five oh three",
                &Pick::Number(None),
                &INCIDENTS
            ),
            ["five oh three = 503 (Some(Whole))", "four ninety one ? 491"]
        );
        // One word typed whole is the recognizer's to read; a pronoun is no code's letters.
        assert!(read("what does eu261 give me for tp1276", &Pick::Code, &FLIGHTS).is_empty());
        assert!(
            read(
                "that supplier still owes us two pallets",
                &Pick::Code,
                &SKUS
            )
            .is_empty()
        );
    }

    #[test]
    fn a_link_needs_its_scheme() {
        let links = ["https://example.com/files/budget.xlsx"];
        assert_eq!(
            read(
                "get h t t p s colon slash slash example dot com slash menu dot pdf",
                &Pick::Url,
                &links
            ),
            [
                "h t t p s colon slash slash example dot com slash menu dot pdf = https://example.com/menu.pdf (Some(Whole))"
            ]
        );
        assert!(read("get example dot com slash menu dot pdf", &Pick::Url, &links).is_empty());
    }
}
