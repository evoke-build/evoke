//! The nine recognizers over an input, and masking. In: `Input`. Out: `Vec<Proposed>`, verbatim spans with typed
//! values, in order of position, no two overlapping: quotes hide what they enclose; a URL, an address, an amount,
//! a code, a date, a time and a duration hide what they cover; a number stands last. A form of a kind refused
//! whole — a slashed date either way round, `5:30` with no half of the day, `every monday` — hides its parts, so
//! no part of it is a candidate; a run of number words that reads as nothing is passed over and hides nothing.
//! A number and a duration read spelled out as they read in digits; a date reads relative, resolved at the body's
//! door; a bare hour is a number, since a half of the day would be invented.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::calendar::{Clock, Day, Weekday, Which, days_in};
use crate::text::{Clean, Input, Span};

/// A candidate for a pick: where it is in the input, and what its recognizer read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proposed {
    pub span: Span,
    pub value: PickValue,
}

/// What a pick hands the body: the number, the seconds, the text, the day as read, the clock time, the amount
/// in its currency, the code.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PickValue {
    Number {
        value: f64,
    },
    #[serde(rename = "duration")]
    Seconds {
        value: u64,
    },
    Email {
        value: Clean,
    },
    Url {
        value: Clean,
    },
    Quoted {
        value: Clean,
    },
    Date {
        value: Day,
    },
    Time {
        value: Clock,
    },
    Amount {
        value: Amount,
    },
    Code {
        value: Clean,
    },
}

impl PickValue {
    /// `number` is bare: any typed span covering it hides it, and it never caps the outcome.
    #[must_use]
    pub fn is_typed(&self) -> bool {
        !matches!(self, Self::Number { .. })
    }
}

/// An amount in a currency, the one value that is not a scalar: `{ amount, currency }`, the figure never below
/// zero, the currency a three-letter code.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawAmount")]
pub struct Amount {
    amount: f64,
    currency: Currency,
}

#[derive(Deserialize)]
struct RawAmount {
    amount: f64,
    currency: Currency,
}

impl Amount {
    #[must_use]
    pub fn new(amount: f64, currency: Currency) -> Option<Self> {
        (amount.is_finite() && amount >= 0.0).then_some(Self { amount, currency })
    }

    #[must_use]
    pub fn amount(&self) -> f64 {
        self.amount
    }

    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }
}

impl TryFrom<RawAmount> for Amount {
    type Error = String;

    fn try_from(raw: RawAmount) -> Result<Self, String> {
        Self::new(raw.amount, raw.currency)
            .ok_or_else(|| "an amount is a figure of zero or more".to_owned())
    }
}

/// A currency's code: three capital letters, `EUR`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Currency(String);

impl Currency {
    pub fn new(code: &str) -> Result<Self, String> {
        if code.len() == 3 && code.bytes().all(|b| b.is_ascii_uppercase()) {
            Ok(Self(code.to_owned()))
        } else {
            Err(format!(
                "\"{code}\" is not a currency code: three capital letters"
            ))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Currency {
    type Error = String;

    fn try_from(code: String) -> Result<Self, String> {
        Self::new(&code)
    }
}

/// Every candidate span of the input, by position.
#[must_use]
pub fn propose(input: &Input) -> Vec<Proposed> {
    let chars: Vec<char> = input.as_str().chars().collect();
    let mut hidden: Vec<Range<usize>> = Vec::new();
    let mut found = Vec::new();
    for recognize in RECOGNIZERS {
        let mut i = 0;
        while i < chars.len() {
            let Found { span, after, read } = match recognize(&chars, i) {
                Scan::Found(found) => found,
                Scan::Skip(after) => {
                    i = after.max(i + 1);
                    continue;
                }
                // A form of a kind refused whole: none of its parts is a candidate of any kind.
                Scan::Hide(after) => {
                    if !overlaps(&hidden, &(i..after)) {
                        hidden.push(i..after);
                    }
                    i = after.max(i + 1);
                    continue;
                }
                Scan::Nothing => {
                    i += 1;
                    continue;
                }
            };
            // No two candidates overlap: a match whose extent reaches into a hidden range is dropped. A typed
            // match hides what it covers whether or not its text can be a span.
            if !overlaps(&hidden, &(i..after)) {
                if !matches!(read, Read::Number(_)) {
                    hidden.push(i..after);
                }
                if let Some(span) = Span::of(input, span.start, span.end) {
                    let value = read.value(span.text());
                    found.push(Proposed { span, value });
                }
            }
            i = after.max(i + 1);
        }
    }
    found.sort_by_key(|proposed| proposed.span.start());
    found
}

fn overlaps(hidden: &[Range<usize>], extent: &Range<usize>) -> bool {
    hidden
        .iter()
        .any(|range| range.start < extent.end && extent.start < range.end)
}

/// What a recognizer finds at one position: a candidate; a run of number words it reads as nothing, passed over
/// whole so that no part of it is a candidate; a form of its kind refused whole, hidden so that no part of it is
/// a candidate of any kind; or nothing.
enum Scan {
    Found(Found),
    Skip(usize),
    Hide(usize),
    Nothing,
}

/// A recognizer's match at one position: the candidate inside it, where scanning resumes, and what it reads as.
struct Found {
    span: Range<usize>,
    after: usize,
    read: Read,
}

fn found(start: usize, end: usize, read: Read) -> Scan {
    Scan::Found(Found {
        span: start..end,
        after: end,
        read,
    })
}

/// What a candidate reads as; the text kinds take the span itself.
enum Read {
    Number(f64),
    Seconds(u64),
    Email,
    Url,
    Quoted,
    Date(Day),
    Time(Clock),
    Amount(Amount),
    Code,
}

impl Read {
    fn value(self, text: &Clean) -> PickValue {
        match self {
            Self::Number(value) => PickValue::Number { value },
            Self::Seconds(value) => PickValue::Seconds { value },
            Self::Email => PickValue::Email {
                value: text.clone(),
            },
            Self::Url => PickValue::Url {
                value: text.clone(),
            },
            Self::Quoted => PickValue::Quoted {
                value: text.clone(),
            },
            Self::Date(value) => PickValue::Date { value },
            Self::Time(value) => PickValue::Time { value },
            Self::Amount(value) => PickValue::Amount { value },
            Self::Code => PickValue::Code {
                value: text.clone(),
            },
        }
    }
}

/// What one kind finds at a position of the input.
type Recognizer = fn(&[char], usize) -> Scan;

/// In this order: an earlier kind's match hides the candidates that reach into it. An amount stands before a
/// code, so `500USD` is an amount; a code before a date and a number, so `4.12.0` is no decimal; a date before a
/// time, so `tonight` is a date; a time before a duration and a number, so `5pm` hides its 5.
const RECOGNIZERS: [Recognizer; 9] = [
    quoted, url, email, amount, code, date, time, duration, number,
];

fn quoted(chars: &[char], i: usize) -> Scan {
    let close = match chars[i] {
        '"' => '"',
        '\u{201c}' => '\u{201d}',
        '\u{2018}' => '\u{2019}',
        _ => return Scan::Nothing,
    };
    let open = chars[i];
    let mut j = i + 1;
    while j < chars.len() && chars[j] != open && chars[j] != close {
        j += 1;
    }
    if j == i + 1 || j == chars.len() || chars[j] != close {
        return Scan::Nothing;
    }
    Scan::Found(Found {
        span: i + 1..j,
        after: j + 1,
        read: Read::Quoted,
    })
}

fn url(chars: &[char], i: usize) -> Scan {
    let Some(scheme) = ["https://", "http://"]
        .into_iter()
        .find(|scheme| starts_with(chars, i, scheme))
    else {
        return Scan::Nothing;
    };
    let inner = |c: char| !(c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\''));
    let mut j = i + scheme.len();
    while j < chars.len() && inner(chars[j]) {
        j += 1;
    }
    let Some(end) = (i + scheme.len() + 2..=j)
        .rev()
        .find(|&end| !matches!(chars[end - 1], '.' | ',' | ';' | ':' | '!' | '?' | ')'))
    else {
        return Scan::Nothing;
    };
    found(i, end, Read::Url)
}

fn email(chars: &[char], i: usize) -> Scan {
    let local = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-');
    let domain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-');
    let mut at = i;
    while at < chars.len() && local(chars[at]) {
        at += 1;
    }
    if at == i || chars.get(at) != Some(&'@') {
        return Scan::Nothing;
    }
    let mut j = at + 1;
    while j < chars.len() && domain(chars[j]) {
        j += 1;
    }
    let Some(end) = (at + 2..j)
        .rev()
        .filter(|&dot| chars[dot] == '.')
        .find_map(|dot| {
            let letters = chars[dot + 1..j]
                .iter()
                .take_while(|c| c.is_ascii_alphabetic())
                .count();
            (letters >= 2).then_some(dot + 1 + letters)
        })
    else {
        return Scan::Nothing;
    };
    found(i, end, Read::Email)
}

// ---- an amount in a currency -------------------------------------------------------------------------------

/// The symbols read before a figure, and what each stands for: `$` is the US dollar.
const SYMBOLS: [(char, &str); 3] = [('$', "USD"), ('€', "EUR"), ('£', "GBP")];

/// The currency words read after a figure or number words.
const CURRENCY_WORDS: [(&str, &str); 9] = [
    ("dollar", "USD"),
    ("dollars", "USD"),
    ("euro", "EUR"),
    ("euros", "EUR"),
    ("pound", "GBP"),
    ("pounds", "GBP"),
    ("yen", "JPY"),
    ("rupee", "INR"),
    ("rupees", "INR"),
];

/// The singular words: a currency after `1`, `one` or `a` only, so `3 pound beef` is a weight.
const SINGULAR: [&str; 4] = ["dollar", "euro", "pound", "rupee"];

/// The ISO codes read as themselves, in capitals, after a figure or number words.
const CODES: [&str; 22] = [
    "USD", "EUR", "GBP", "JPY", "INR", "CAD", "AUD", "CHF", "CNY", "SEK", "NOK", "DKK", "NZD",
    "MXN", "BRL", "ZAR", "SGD", "HKD", "KRW", "PLN", "CZK", "TRY",
];

/// An amount: a symbol before a figure, `€1,200`; a figure, number words, `a` or `an` before a currency word,
/// `50 dollars`, `twelve hundred euros`, `a dollar`; or a figure before a code in capitals, `1000 USD`, `500USD`.
/// A sign before the figure is the number's; `$5k` reads nothing; a symbol before a dotted thousands figure,
/// `€1.200`, is two readings, refused whole.
fn amount(chars: &[char], i: usize) -> Scan {
    if i > 0 && chars[i - 1] == '-' && (i == 1 || !is_word(chars[i - 2])) {
        return Scan::Nothing;
    }
    if let Some((_, currency)) = SYMBOLS.into_iter().find(|(symbol, _)| *symbol == chars[i]) {
        if !boundary(chars, i) {
            return Scan::Nothing;
        }
        let j = skip_space(chars, i + 1);
        let Some(figure) = figure(chars, j) else {
            return Scan::Nothing;
        };
        let scaled = chars
            .get(figure.end)
            .is_some_and(|c| matches!(c.to_ascii_lowercase(), 'k' | 'm'))
            && ends_word(chars, figure.end + 1);
        if scaled {
            return Scan::Nothing;
        }
        if figure.dotted {
            return Scan::Hide(figure.end);
        }
        return moneyed(i, figure.end, figure.value, currency);
    }
    let (value, end, article) = if let Some(figure) = figure(chars, i) {
        (figure.value, figure.end, false)
    } else if let Some((tokens, end)) = run(chars, i) {
        // A run of number words reads whole or not at all: one that fits no form is passed over.
        match figure_value(&tokens) {
            Some(value) => (value, end, false),
            None => return Scan::Skip(end),
        }
    } else if let Some(end) = phrase(chars, i, "a").or_else(|| phrase(chars, i, "an")) {
        (1.0, end, true)
    } else {
        return Scan::Nothing;
    };
    let j = skip_space(chars, end);
    let Some((word, wend)) = word_at(chars, j) else {
        return Scan::Nothing;
    };
    if let Some((_, currency)) = CURRENCY_WORDS.into_iter().find(|(w, _)| *w == word) {
        if SINGULAR.contains(&word.as_str()) && (value - 1.0).abs() > f64::EPSILON {
            return Scan::Nothing;
        }
        return moneyed(i, wend, value, currency);
    }
    let code: String = chars[j..wend].iter().collect();
    if !article && CODES.contains(&code.as_str()) {
        return moneyed(i, wend, value, &code);
    }
    Scan::Nothing
}

/// A candidate of an amount; nothing when the figure is no amount.
fn moneyed(start: usize, end: usize, value: f64, currency: &str) -> Scan {
    Currency::new(currency)
        .ok()
        .and_then(|currency| Amount::new(value, currency))
        .map_or(Scan::Nothing, |amount| {
            found(start, end, Read::Amount(amount))
        })
}

/// A figure in digits: where it ends, its value, and whether it is a dotted thousands figure, `1.200`.
struct Figure {
    value: f64,
    end: usize,
    dotted: bool,
}

/// A figure at `i` with thousands groups and a fraction: `1,200`, `19.99`, `1,234.56`.
fn figure(chars: &[char], i: usize) -> Option<Figure> {
    if !boundary(chars, i) {
        return None;
    }
    let digits = digit_run(chars, i)?;
    let mut j = digits;
    while chars.get(j) == Some(&',')
        && two_or_more_digits(chars, j + 1, 3)
        && !chars.get(j + 4).is_some_and(char::is_ascii_digit)
    {
        j += 4;
    }
    let grouped = j > digits;
    let mut text: String = chars[i..j].iter().filter(|c| **c != ',').collect();
    let mut end = j;
    let mut fraction = 0;
    if chars.get(j) == Some(&'.') && chars.get(j + 1).is_some_and(char::is_ascii_digit) {
        end = digit_run(chars, j + 1)?;
        fraction = end - j - 1;
        text.extend(&chars[j..end]);
    }
    let value: f64 = text.parse().ok()?;
    value.is_finite().then_some(Figure {
        value,
        end,
        dotted: !grouped && digits - i <= 3 && fraction == 3,
    })
}

/// Whether exactly `count` digits stand at `at`.
fn two_or_more_digits(chars: &[char], at: usize, count: usize) -> bool {
    (0..count).all(|k| chars.get(at + k).is_some_and(char::is_ascii_digit))
}

/// A figure in words, bounded: `<small>`, `a hundred [and <small>]`, `<small> hundred [and <small>]`, and with
/// `thousand` before any of those: `twelve hundred`, `one thousand two hundred and fifty`, `two thousand and
/// seventeen`, `a thousand`. Read only before a currency; a run that fits no form is nothing.
fn figure_value(tokens: &[Token]) -> Option<f64> {
    let (thousands, rest) = match tokens.iter().position(|token| *token == Token::Thousand) {
        Some(at) => (Some(thousands_head(&tokens[..at])?), &tokens[at + 1..]),
        None => (None, tokens),
    };
    let rest = match (rest, thousands) {
        ([], Some(_)) => 0.0,
        ([Token::And, small @ ..], Some(_)) => small_value(small)?,
        (rest, _) => hundreds_or_small(rest)?,
    };
    Some(thousands.unwrap_or(0.0) * 1000.0 + rest)
}

/// What stands before `thousand`: `a`, a small number, or hundreds.
fn thousands_head(tokens: &[Token]) -> Option<f64> {
    match tokens {
        [Token::A] => Some(1.0),
        [Token::Ones(_) | Token::Tens(_), ..] => hundreds_or_small(tokens),
        _ => None,
    }
}

/// `<small>`, `a hundred [and <small>]` or `<small> hundred [and <small>]`.
fn hundreds_or_small(tokens: &[Token]) -> Option<f64> {
    match tokens {
        [Token::A, Token::Hundred] => Some(100.0),
        [Token::A, Token::Hundred, Token::And, small @ ..] => Some(100.0 + small_value(small)?),
        _ => {
            let (small, rest) = small_prefix(tokens)?;
            match rest {
                [] => Some(small),
                [Token::Hundred] => Some(small * 100.0),
                [Token::Hundred, Token::And, more @ ..] => Some(small * 100.0 + small_value(more)?),
                _ => None,
            }
        }
    }
}

/// A small number alone: one word to ninety, or a tens word joined to a word from one to nine.
fn small_value(tokens: &[Token]) -> Option<f64> {
    let (small, rest) = small_prefix(tokens)?;
    rest.is_empty().then_some(small)
}

fn small_prefix(tokens: &[Token]) -> Option<(f64, &[Token])> {
    match tokens {
        [Token::Tens(tens), Token::Ones(ones), rest @ ..] if (1..=9).contains(ones) => {
            Some((f64::from(tens + ones), rest))
        }
        [Token::Tens(value) | Token::Ones(value), rest @ ..] => Some((f64::from(*value), rest)),
        _ => None,
    }
}

// ---- a code ------------------------------------------------------------------------------------------------

/// An identifier as typed: a version, `4.12.0`, two dots or more, digits only; a ticket, `INC-311`, two to six
/// letters, a dash, one to six digits; a serial or a compact flight, `TP1043`, `C02XK1ABJG5M`, six or more
/// letters and digits with at least one letter and two digits; a letter in either case, so `inc-311` and
/// `tp1043` read as they are typed, and `10mins` stays a duration. At a word boundary, never after `-` or `.`,
/// ending at one; `4.12` is a decimal, `27.03.2017` a dotted date, `2.0.0-rc.1` nothing.
fn code(chars: &[char], i: usize) -> Scan {
    if !boundary(chars, i) || (i > 0 && matches!(chars[i - 1], '-' | '.')) {
        return Scan::Nothing;
    }
    for end in [version(chars, i), ticket(chars, i), serial(chars, i)]
        .into_iter()
        .flatten()
    {
        let joined = matches!(chars.get(end), Some('.' | '-'))
            && chars.get(end + 1).is_some_and(|c| c.is_alphanumeric());
        if ends_word(chars, end) && !joined {
            return found(i, end, Read::Code);
        }
    }
    Scan::Nothing
}

/// `\d+(\.\d+){2,}` at `i`, unless it is a dotted date: three parts, the last four digits, the first at most
/// thirty-one and the second at most twelve.
fn version(chars: &[char], i: usize) -> Option<usize> {
    let mut parts: Vec<(usize, usize)> = vec![(i, digit_run(chars, i)?)];
    loop {
        let end = parts.last().map_or(i, |(_, end)| *end);
        if chars.get(end) != Some(&'.') {
            break;
        }
        let Some(next) = digit_run(chars, end + 1) else {
            break;
        };
        parts.push((end + 1, next));
    }
    if parts.len() < 3 {
        return None;
    }
    let value = |(start, end): (usize, usize)| -> u64 {
        chars[start..end]
            .iter()
            .collect::<String>()
            .parse()
            .unwrap_or(u64::MAX)
    };
    let dotted_date = parts.len() == 3
        && parts[2].1 - parts[2].0 == 4
        && value(parts[0]) <= 31
        && value(parts[1]) <= 12;
    (!dotted_date).then(|| parts.last().map_or(i, |(_, end)| *end))
}

/// `[A-Za-z]{2,6}-\d{1,6}` at `i`.
fn ticket(chars: &[char], i: usize) -> Option<usize> {
    let letters = chars[i..]
        .iter()
        .take_while(|c| c.is_ascii_alphabetic())
        .count();
    if !(2..=6).contains(&letters) || chars.get(i + letters) != Some(&'-') {
        return None;
    }
    let end = digit_run(chars, i + letters + 1)?;
    (end - (i + letters + 1) <= 6).then_some(end)
}

/// `[A-Za-z0-9]{6,}` at `i` with a letter and two digits among them; a figure before small letters is a quantity
/// with its unit, `10mins`, so a run that holds a small letter starts with a letter.
fn serial(chars: &[char], i: usize) -> Option<usize> {
    let length = chars[i..]
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric())
        .count();
    let run = &chars[i..i + length];
    let letters = run.iter().any(char::is_ascii_alphabetic);
    let digits = run.iter().filter(|c| c.is_ascii_digit()).count();
    let quantity =
        run.first().is_some_and(char::is_ascii_digit) && run.iter().any(char::is_ascii_lowercase);
    (length >= 6 && letters && digits >= 2 && !quantity).then_some(i + length)
}

// ---- a date ------------------------------------------------------------------------------------------------

const DAY_WORDS: [(&str, i32); 4] = [
    ("today", 0),
    ("tonight", 0),
    ("tomorrow", 1),
    ("yesterday", -1),
];
const DAY_PHRASES: [(&str, i32); 3] = [
    ("the day after tomorrow", 2),
    ("day after tomorrow", 2),
    ("the day before yesterday", -2),
];
const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];
const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];
const MONTHS_SHORT: [(&str, u8); 12] = [
    ("jan", 1),
    ("feb", 2),
    ("mar", 3),
    ("apr", 4),
    ("jun", 6),
    ("jul", 7),
    ("aug", 8),
    ("sep", 9),
    ("sept", 9),
    ("oct", 10),
    ("nov", 11),
    ("dec", 12),
];
const ORDINALS: [(&str, u8); 22] = [
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
    ("twelveth", 12),
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
/// What `every` or `each` may name: a recurrence, passed over whole.
const RECURRING: [&str; 9] = [
    "day", "week", "month", "year", "morning", "night", "evening", "weekday", "weekend",
];
/// A period after `this`, `next` or `last`: not a day.
const PERIODS: [&str; 8] = [
    "week",
    "weeks",
    "month",
    "months",
    "year",
    "years",
    "weekend",
    "fortnight",
];
const DAY_UNITS: [(&str, i32); 5] = [
    ("days", 1),
    ("day", 1),
    ("weeks", 7),
    ("week", 7),
    ("fortnight", 14),
];
/// What may follow a day of the month for it to read as one: nothing, punctuation, or one of these words.
const PLAIN_FOLLOWERS: [&str; 17] = [
    "at", "is", "or", "and", "this", "next", "until", "till", "from", "to", "on", "in", "fall",
    "falls", "come", "comes", "already",
];
/// A preposition of time before a day of the month, `on the 14th`, `by the twenty second`.
const TIME_LEADS: [&str; 10] = [
    "on", "for", "by", "until", "till", "from", "before", "after", "since", "of",
];

/// A date: a day word, a weekday with the word before it, a weekday joined to a day, a day or weeks ahead, an
/// ISO or slashed date, a month and a day with a year or not, a day of the month. A recurrence, a plural
/// weekday, a period, an ambiguous slashed date and a day no calendar has are refused whole.
fn date(chars: &[char], i: usize) -> Scan {
    if !boundary(chars, i) {
        return Scan::Nothing;
    }
    day_word(chars, i)
        .or_else(|| recurrence(chars, i))
        .or_else(|| weekday(chars, i))
        .or_else(|| ahead(chars, i))
        .or_else(|| numeric(chars, i))
        .or_else(|| month_first(chars, i))
        .or_else(|| day_first(chars, i))
        .unwrap_or(Scan::Nothing)
}

/// A candidate of a reading; nothing when the calendar refuses it.
fn dated(start: usize, end: usize, day: Option<Day>) -> Scan {
    day.map_or(Scan::Nothing, |day| found(start, end, Read::Date(day)))
}

/// `today`, `tonight`, `tomorrow`, `yesterday`, a possessive left outside the span; `the day after tomorrow`.
fn day_word(chars: &[char], i: usize) -> Option<Scan> {
    for (word, days) in DAY_WORDS {
        let plural = format!("{word}s");
        if let Some(end) = phrase(chars, i, word).or_else(|| phrase(chars, i, &plural)) {
            return Some(dated(i, end, Day::offset(days)));
        }
    }
    DAY_PHRASES.into_iter().find_map(|(words, days)| {
        phrase(chars, i, words).map(|end| dated(i, end, Day::offset(days)))
    })
}

/// `every monday`, `each sunday`, `everyday`: passed over whole, hiding what they name.
fn recurrence(chars: &[char], i: usize) -> Option<Scan> {
    for each in ["every", "each"] {
        if let Some(end) = phrase(chars, i, each)
            && chars.get(end) == Some(&' ')
            && let Some((word, wend)) = word_at(chars, end + 1)
        {
            let named = WEEKDAYS.contains(&word.as_str())
                || word
                    .strip_suffix('s')
                    .is_some_and(|day| WEEKDAYS.contains(&day))
                || RECURRING.contains(&word.as_str())
                || DAY_WORDS.iter().any(|(day, _)| *day == word)
                || ordinal_at(chars, end + 1).is_some();
            if named {
                return Some(Scan::Hide(wend));
            }
        }
    }
    ["everyday", "weekly", "daily", "monthly"]
        .into_iter()
        .find_map(|word| phrase(chars, i, word).map(Scan::Hide))
}

/// `friday`, `next monday`, `this wednesday`, `last tuesday`; a plural, `mondays`, and a period, `next month`,
/// read as nothing, whole. A bare weekday may join a day: `friday the 14th`, `tuesday 21 march 2017`.
fn weekday(chars: &[char], i: usize) -> Option<Scan> {
    let mut which = None;
    let mut j = i;
    for (word, said) in [
        ("next", Which::Next),
        ("this", Which::This),
        ("last", Which::Last),
    ] {
        if let Some(end) = phrase(chars, i, word)
            && chars.get(end) == Some(&' ')
        {
            which = Some(said);
            j = end + 1;
            break;
        }
    }
    if !boundary(chars, j) {
        return None;
    }
    let (word, end) = word_at(chars, j)?;
    if let Some(at) = WEEKDAYS.iter().position(|day| *day == word) {
        if which.is_none()
            && let Some(joined) = weekday_day(chars, i, end)
        {
            return Some(joined);
        }
        let weekday = Weekday::ALL[at];
        return Some(found(i, end, Read::Date(Day::Weekday { weekday, which })));
    }
    if word
        .strip_suffix('s')
        .is_some_and(|day| WEEKDAYS.contains(&day))
        || (which.is_some() && PERIODS.contains(&word.as_str()))
    {
        return Some(Scan::Hide(end));
    }
    None
}

/// The day a weekday joins, one candidate: `friday the 14th`, `tuesday 21 march 2017`.
fn weekday_day(chars: &[char], i: usize, after: usize) -> Option<Scan> {
    let k = skip_space(chars, after);
    let the = phrase(chars, k, "the");
    let k = the.map_or(k, |end| skip_space(chars, end));
    if the.is_none() && digit_run(chars, k).is_none() && ordinal_at(chars, k).is_none() {
        return None;
    }
    let (day, dend) = day_number_at(chars, k)?;
    if let Some(scan) = named_month_after(chars, i, day, dend) {
        return Some(scan);
    }
    (ordinal_at(chars, k).is_some() && followed_plainly(chars, dend))
        .then(|| dated(i, dend, Day::nth(day)))
}

/// A month after a day, `21 march 2017`, `the 14th of march`: the calendar day, or nothing when none follows.
fn named_month_after(chars: &[char], i: usize, day: u8, dend: usize) -> Option<Scan> {
    let k = skip_space(chars, dend);
    let of = phrase(chars, k, "of");
    let k = of.map_or(k, |end| skip_space(chars, end));
    let (month, mend) = month_at(chars, k)?;
    if day > days_in(month, None) {
        return None;
    }
    Some(match year_at(chars, mend) {
        Some((year, yend)) => dated(i, yend, Day::calendar(Some(year), month, day)),
        None => dated(i, mend, Day::calendar(None, month, day)),
    })
}

/// `in three days`, `in 2 weeks`, `three days from now`, `a week from today`: read as days ahead.
fn ahead(chars: &[char], i: usize) -> Option<Scan> {
    if let Some(end) = phrase(chars, i, "in")
        && chars.get(end) == Some(&' ')
        && let Some((count, cend)) = count_at(chars, end + 1)
    {
        let k = skip_space(chars, cend);
        for (unit, days) in DAY_UNITS {
            if let Some(e) = phrase(chars, k, unit) {
                return Some(dated(i, e, days_ahead(count, days)));
            }
        }
    }
    let (count, cend) = count_at(chars, i)?;
    let k = skip_space(chars, cend);
    for (unit, days) in &DAY_UNITS[..4] {
        if let Some(e) = phrase(chars, k, unit) {
            for tail in ["from now", "from today"] {
                if let Some(t) = phrase(chars, skip_space(chars, e), tail) {
                    return Some(dated(i, t, days_ahead(count, *days)));
                }
            }
        }
    }
    None
}

fn days_ahead(count: u64, each: i32) -> Option<Day> {
    let days = i32::try_from(count).ok()?.checked_mul(each)?;
    Day::offset(days)
}

/// A count: the number words the product reads, digits, or `a` and `an`.
fn count_at(chars: &[char], i: usize) -> Option<(u64, usize)> {
    if let Some(Words { end, value }) = words(chars, i) {
        return value.and_then(|value| whole(value).map(|count| (count, end)));
    }
    if let Some(found) = digits_at(chars, i) {
        return Some(found);
    }
    phrase(chars, i, "a")
        .or_else(|| phrase(chars, i, "an"))
        .map(|end| (1, end))
}

/// A whole number as its count.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole(value: f64) -> Option<u64> {
    (value >= 0.0 && value.fract() == 0.0 && value < 1e15).then_some(value as u64)
}

/// `2026-05-05`; `27/03/2017`, `3/21/2017`, `27.03.2017` when one order alone is possible: a slashed date with
/// both parts under thirteen is refused whole, and so is a day no calendar has.
fn numeric(chars: &[char], i: usize) -> Option<Scan> {
    if let Some((year, month, day, end)) = iso_at(chars, i) {
        return Some(
            Day::calendar(Some(year), month, day)
                .map_or(Scan::Hide(end), |day| found(i, end, Read::Date(day))),
        );
    }
    let (a, b, year, end) = slashed_at(chars, i)?;
    if a > 12 && (1..=12).contains(&b) && a <= days_in(b, year) {
        return Some(dated(i, end, Day::calendar(year, b, a)));
    }
    if b > 12 && (1..=12).contains(&a) && b <= days_in(a, year) {
        return Some(dated(i, end, Day::calendar(year, a, b)));
    }
    ((1..=12).contains(&a) && (1..=12).contains(&b)).then_some(Scan::Hide(end))
}

/// `dddd-dd-dd` at `i`, at a word boundary after.
fn iso_at(chars: &[char], i: usize) -> Option<(i32, u8, u8, usize)> {
    let (year, yend) = short_digits(chars, i, 4, 4)?;
    if chars.get(yend) != Some(&'-') {
        return None;
    }
    let (month, mend) = short_digits(chars, yend + 1, 2, 2)?;
    if chars.get(mend) != Some(&'-') {
        return None;
    }
    let (day, end) = short_digits(chars, mend + 1, 2, 2)?;
    if !ends_word(chars, end) {
        return None;
    }
    Some((
        i32::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
        end,
    ))
}

/// `d/m`, `d/m/yy`, `d/m/yyyy` or `d.m.yyyy` at `i`: the two parts, the year and where it ends.
fn slashed_at(chars: &[char], i: usize) -> Option<(u8, u8, Option<i32>, usize)> {
    let (a, aend) = short_digits(chars, i, 1, 2)?;
    let (b, bend, year, end) = match chars.get(aend) {
        Some('/') => {
            let (b, bend) = short_digits(chars, aend + 1, 1, 2)?;
            let (year, end) = if chars.get(bend) == Some(&'/') {
                let (year, yend) = short_digits(chars, bend + 1, 2, 4)?;
                let year = if year < 100 { year + 2000 } else { year };
                (Some(i32::try_from(year).ok()?), yend)
            } else {
                (None, bend)
            };
            if chars.get(end) == Some(&'/') {
                return None;
            }
            (b, bend, year, end)
        }
        Some('.') => {
            let (b, bend) = short_digits(chars, aend + 1, 1, 2)?;
            if chars.get(bend) != Some(&'.') {
                return None;
            }
            let (year, end) = short_digits(chars, bend + 1, 4, 4)?;
            (b, bend, Some(i32::try_from(year).ok()?), end)
        }
        _ => return None,
    };
    let _ = bend;
    if !ends_word(chars, end) {
        return None;
    }
    Some((u8::try_from(a).ok()?, u8::try_from(b).ok()?, year, end))
}

/// A run of `least` to `most` digits at `at`, exactly: its value and where it ends.
fn short_digits(chars: &[char], at: usize, least: usize, most: usize) -> Option<(u32, usize)> {
    let end = digit_run(chars, at)?;
    if !(least..=most).contains(&(end - at)) {
        return None;
    }
    let value: u32 = chars[at..end].iter().collect::<String>().parse().ok()?;
    Some((value, end))
}

/// `march 7`, `March 6th`, `may fifth`, `mar. 4th, 2020`, `march the seventh`; a day the month lacks, `february
/// 30`, is refused whole; a month alone is nothing.
fn month_first(chars: &[char], i: usize) -> Option<Scan> {
    let (month, mend) = month_at(chars, i)?;
    let mut k = skip_space(chars, mend);
    if let Some(end) = phrase(chars, k, "the") {
        k = skip_space(chars, end);
    }
    let Some((day, dend)) = day_number_at(chars, k) else {
        return Some(Scan::Nothing);
    };
    let (year, end) = match year_at(chars, dend) {
        Some((year, yend)) => (Some(year), yend),
        None => (None, dend),
    };
    if day > days_in(month, year) {
        return Some(Scan::Hide(end));
    }
    Some(dated(i, end, Day::calendar(year, month, day)))
}

/// `7th march`, `the 7th of march`, `14 march 2020`; a day of the month, `on the 14th`, `by the twenty second`,
/// `the 15th` followed plainly, never `the first alarm` or `on 6th ave`.
fn day_first(chars: &[char], i: usize) -> Option<Scan> {
    let the = phrase(chars, i, "the");
    let k = the.map_or(i, |end| skip_space(chars, end));
    let (day, dend) = day_number_at(chars, k)?;
    if let Some(scan) = named_month_after(chars, i, day, dend) {
        return Some(scan);
    }
    let plainly = ordinal_at(chars, k).is_some()
        && followed_plainly(chars, dend)
        && (the.is_some() || led_by_time(chars, i));
    plainly.then(|| dated(i, dend, Day::nth(day)))
}

/// A day as an ordinal, a bare figure one to thirty-one that no colon or point continues, or a number word one
/// to thirty-one.
fn day_number_at(chars: &[char], i: usize) -> Option<(u8, usize)> {
    if let Some(ordinal) = ordinal_at(chars, i) {
        return Some(ordinal);
    }
    if let Some((day, end)) = digits_at(chars, i)
        && (1..=31).contains(&day)
        && !(matches!(chars.get(end), Some(':' | '.'))
            && chars.get(end + 1).is_some_and(char::is_ascii_digit))
    {
        return u8::try_from(day).ok().map(|day| (day, end));
    }
    small_words(chars, i).filter(|(day, _)| (1..=31).contains(day))
}

/// `14th`, `1st`, `fourteenth`, `twenty-first`, `twenty first`: the day and where it ends.
fn ordinal_at(chars: &[char], i: usize) -> Option<(u8, usize)> {
    if let Some((day, end)) = digits_at(chars, i) {
        let suffix: String = chars
            .get(end..end + 2)?
            .iter()
            .map(char::to_ascii_lowercase)
            .collect();
        return (matches!(suffix.as_str(), "st" | "nd" | "rd" | "th")
            && ends_word(chars, end + 2)
            && (1..=31).contains(&day))
        .then(|| u8::try_from(day).ok().map(|day| (day, end + 2)))
        .flatten();
    }
    let (word, end) = word_at(chars, i)?;
    if let Some((_, day)) = ORDINALS.into_iter().find(|(ordinal, _)| *ordinal == word)
        && boundary(chars, i)
    {
        return Some((day, end));
    }
    if let Some(tens) = tens(&word)
        && tens <= 30
        && matches!(chars.get(end), Some(' ' | '-'))
        && let Some((next, nend)) = word_at(chars, end + 1)
        && let Some((_, ones)) = ORDINALS.into_iter().find(|(ordinal, _)| *ordinal == next)
        && (1..=9).contains(&ones)
    {
        return Some((tens + ones, nend));
    }
    None
}

/// A month word at `i`, whole or short with an optional dot: `march`, `mar.`, `Sept`.
fn month_at(chars: &[char], i: usize) -> Option<(u8, usize)> {
    if !boundary(chars, i) {
        return None;
    }
    let (word, end) = word_at(chars, i)?;
    if let Some(at) = MONTHS.iter().position(|month| *month == word) {
        return u8::try_from(at + 1).ok().map(|month| (month, end));
    }
    let (_, month) = MONTHS_SHORT.into_iter().find(|(short, _)| *short == word)?;
    let end = if chars.get(end) == Some(&'.') {
        end + 1
    } else {
        end
    };
    Some((month, end))
}

/// A year after a day, joined by a comma or a space: `, 2020`, ` 2020`, ` twenty seventeen`, ` two thousand and
/// seventeen`; 1900 to 2100 in figures.
fn year_at(chars: &[char], i: usize) -> Option<(i32, usize)> {
    let mut j = i;
    if chars.get(j) == Some(&',') {
        j += 1;
    }
    j = skip_space(chars, j);
    if j == i {
        return None;
    }
    if let Some((year, end)) = digits_at(chars, j)
        && (1900..=2100).contains(&year)
        && ends_word(chars, end)
    {
        return i32::try_from(year).ok().map(|year| (year, end));
    }
    let (word, wend) = word_at(chars, j)?;
    if (word == "nineteen" || word == "twenty") && chars.get(wend) == Some(&' ') {
        let base = if word == "nineteen" { 1900 } else { 2000 };
        let mut k = wend + 1;
        let oh = phrase(chars, k, "oh");
        if let Some(end) = oh {
            k = skip_space(chars, end);
        }
        match small_words(chars, k) {
            Some((rest, end)) if oh.is_none() || rest < 10 => {
                return Some((base + i32::from(rest), end));
            }
            None => {
                if let Some(end) = phrase(chars, k, "hundred") {
                    return Some((base, end));
                }
            }
            Some(_) => {}
        }
    }
    if word == "two"
        && let Some(thousand) = phrase(chars, skip_space(chars, wend), "thousand")
    {
        let k = skip_space(chars, thousand);
        let and = phrase(chars, k, "and");
        let k = and.map_or(k, |end| skip_space(chars, end));
        return Some(
            small_words(chars, k).map_or((2000, thousand), |(rest, end)| {
                (2000 + i32::from(rest), end)
            }),
        );
    }
    None
}

/// Whether what follows `j` is the end, punctuation, or a word that is not a noun's place: `the 14th`, `the 14th
/// at three`, `the 14th is what day`; never `the first alarm`.
fn followed_plainly(chars: &[char], j: usize) -> bool {
    let k = skip_space(chars, j);
    match chars.get(k) {
        None => true,
        Some(c) if !c.is_alphabetic() => true,
        Some(_) => {
            word_at(chars, k).is_some_and(|(word, _)| PLAIN_FOLLOWERS.contains(&word.as_str()))
        }
    }
}

/// Whether the word before `i` is a preposition of time or a weekday: `on the 14th`, `friday the 14th`, never `the
/// 13th president`.
fn led_by_time(chars: &[char], i: usize) -> bool {
    let mut end = i;
    while end > 0 && chars[end - 1] == ' ' {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && chars[start - 1].is_alphabetic() {
        start -= 1;
    }
    let word: String = chars[start..end]
        .iter()
        .flat_map(|c| c.to_lowercase())
        .collect();
    TIME_LEADS.contains(&word.as_str()) || WEEKDAYS.contains(&word.as_str())
}

// ---- a time ------------------------------------------------------------------------------------------------

/// The half of the day a clock time names.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Meridiem {
    Am,
    Pm,
}

const NAMED_TIMES: [(&str, u8); 6] = [
    ("noontime", 12),
    ("noon", 12),
    ("midday", 12),
    ("midnight", 0),
    ("twelve noon", 12),
    ("twelve midnight", 0),
];
/// `half past five pm`, `quarter to six am`: the minutes each names, and whether they are past the hour.
const PAST_OR_TO: [(&str, bool, u8); 5] = [
    ("half past", true, 30),
    ("quarter past", true, 15),
    ("a quarter past", true, 15),
    ("quarter to", false, 15),
    ("a quarter to", false, 15),
];

/// A clock time with its half of the day, `5pm`, `5:30 pm`, `seven thirty am`, `six in the morning`, `5 o'clock
/// in the afternoon`, `noon`; or a two-digit hour on the 24-hour clock, `13:00`, `09:30`. A bare hour is a
/// number; `5:30`, `4 o'clock` and `half past five` with no half, and a time with seconds, are refused whole.
fn time(chars: &[char], i: usize) -> Scan {
    if !boundary(chars, i) {
        return Scan::Nothing;
    }
    named_time(chars, i)
        .or_else(|| past_or_to(chars, i))
        .or_else(|| minutes_past_or_to(chars, i))
        .or_else(|| digital(chars, i))
        .or_else(|| spoken(chars, i))
        .unwrap_or(Scan::Nothing)
}

/// A candidate of a clock time on the 24-hour clock; nothing when the clock refuses it.
fn timed(start: usize, end: usize, hour: u8, minute: u8, meridiem: Option<Meridiem>) -> Scan {
    let hour = match meridiem {
        Some(Meridiem::Am) => hour % 12,
        Some(Meridiem::Pm) => hour % 12 + 12,
        None => hour,
    };
    Clock::new(hour, minute).map_or(Scan::Nothing, |clock| found(start, end, Read::Time(clock)))
}

fn named_time(chars: &[char], i: usize) -> Option<Scan> {
    NAMED_TIMES
        .into_iter()
        .find_map(|(words, hour)| phrase(chars, i, words).map(|end| timed(i, end, hour, 0, None)))
}

/// `half past five pm`, `a quarter to six am`; with no half, hidden.
fn past_or_to(chars: &[char], i: usize) -> Option<Scan> {
    for (lead, past, minutes) in PAST_OR_TO {
        let Some(end) = phrase(chars, i, lead) else {
            continue;
        };
        let k = skip_space(chars, end);
        let Some((hour, hend)) = hour_word(chars, k).or_else(|| {
            digits_at(chars, k).and_then(|(hour, end)| {
                Some((
                    u8::try_from(hour)
                        .ok()
                        .filter(|hour| (1..=12).contains(hour))?,
                    end,
                ))
            })
        }) else {
            continue;
        };
        return Some(past_or_to_read(chars, i, hour, hend, past, minutes));
    }
    None
}

/// The hour before or after the minutes named, with its half of the day; hidden whole when none follows.
fn past_or_to_read(
    chars: &[char],
    i: usize,
    hour: u8,
    hend: usize,
    past: bool,
    minutes: u8,
) -> Scan {
    let Some((meridiem, mend)) = meridiem_at(chars, hend) else {
        return Scan::Hide(hend);
    };
    let (hour, minute) = if past {
        (hour, minutes)
    } else {
        (if hour > 1 { hour - 1 } else { 12 }, 60 - minutes)
    };
    timed(i, mend, hour, minute, Some(meridiem))
}

/// `ten past nine pm`, `twenty five to six am`.
fn minutes_past_or_to(chars: &[char], i: usize) -> Option<Scan> {
    let Words {
        end: mend,
        value: Some(minutes),
    } = words(chars, i)?
    else {
        return None;
    };
    let minutes = whole(minutes).filter(|minutes| (1..=59).contains(minutes))?;
    for (lead, past) in [(" past ", true), (" to ", false)] {
        if !phrase_lower(chars, mend, lead) {
            continue;
        }
        let Some((hour, hend)) = hour_word(chars, mend + lead.len()) else {
            continue;
        };
        return Some(past_or_to_read(
            chars,
            i,
            hour,
            hend,
            past,
            u8::try_from(minutes).ok()?,
        ));
    }
    None
}

/// `5pm`, `5:30 pm`, `07:03 PM`, `17:30`, `13 o'clock`, `5 o'clock in the afternoon`.
fn digital(chars: &[char], i: usize) -> Option<Scan> {
    let (hour, mut j) = digits_at(chars, i)?;
    if hour > 24 || j - i > 2 {
        return None;
    }
    let hour = u8::try_from(hour).ok()?;
    let two = j - i == 2;
    let mut minute: Option<u8> = None;
    // A dot joins the minutes only where a half of the day or `o'clock` follows, so the form is a clock time
    // for sure; that half must then apply, or the figures hide.
    let dotted = chars.get(j) == Some(&'.');
    if matches!(chars.get(j), Some(':' | '.'))
        && let Some(read) = two_digit_minutes(chars, j + 1)
        && (chars[j] == ':'
            || meridiem_at(chars, j + 3).is_some()
            || oclock_at(chars, j + 3).is_some())
    {
        minute = Some(read);
        j += 3;
        if chars.get(j) == Some(&':') && digits_follow(chars, j + 1) {
            return Some(Scan::Hide((j + 3).min(chars.len())));
        }
    }
    let mut meridiem = meridiem_at(chars, j);
    if meridiem.is_none()
        && minute.is_some()
        && let Some(c) = chars.get(j)
        && matches!(c.to_ascii_lowercase(), 'a' | 'p')
        && ends_word(chars, j + 1)
    {
        let half = if c.eq_ignore_ascii_case(&'a') {
            Meridiem::Am
        } else {
            Meridiem::Pm
        };
        meridiem = Some((half, j + 1));
    }
    if let Some((half, mend)) = meridiem
        && hour <= 12
        && minute.is_none_or(|minute| minute < 60)
    {
        return Some(timed(i, mend, hour, minute.unwrap_or(0), Some(half)));
    }
    if let Some(oc) = oclock_at(chars, j)
        && minute.is_none()
    {
        if let Some((half, mend)) = meridiem_at(chars, oc)
            && hour <= 12
        {
            return Some(timed(i, mend, hour, 0, Some(half)));
        }
        if two && hour <= 23 {
            return Some(timed(i, oc, hour, 0, None));
        }
        return Some(Scan::Hide(oc));
    }
    if let Some(minute) = minute
        && ends_word(chars, j)
    {
        if two && hour <= 23 && minute < 60 && !dotted {
            return Some(timed(i, j, hour, minute, None));
        }
        return Some(Scan::Hide(j));
    }
    Some(Scan::Nothing)
}

/// Exactly two digits at `at`, a third not following: the minutes.
fn two_digit_minutes(chars: &[char], at: usize) -> Option<u8> {
    if !two_or_more_digits(chars, at, 2) || chars.get(at + 2).is_some_and(char::is_ascii_digit) {
        return None;
    }
    chars[at..at + 2].iter().collect::<String>().parse().ok()
}

/// Whether what stands at `at` up to two characters on is digits: a seconds part, `:17`, refused with its time.
fn digits_follow(chars: &[char], at: usize) -> bool {
    match (chars.get(at), chars.get(at + 1)) {
        (Some(a), Some(b)) => a.is_ascii_digit() && b.is_ascii_digit(),
        (Some(a), None) => a.is_ascii_digit(),
        _ => false,
    }
}

/// `five pm`, `seven thirty am`, `six oh five pm`, `eight hundred am`, `six o'clock in the evening`, `three in
/// the afternoon`.
fn spoken(chars: &[char], i: usize) -> Option<Scan> {
    let (hour, j) = hour_word(chars, i)?;
    if hour < 1 {
        return None;
    }
    let mut minute = 0;
    let mut k = j;
    if matches!(chars.get(k), Some(' ' | '-')) {
        if let Some(hundred) = phrase(chars, k + 1, "hundred") {
            return Some(match meridiem_at(chars, hundred) {
                Some((half, mend)) => timed(i, mend, hour, 0, Some(half)),
                None => Scan::Nothing,
            });
        }
        if let Some((read, mend)) = minute_words(chars, k + 1)
            && read < 60
        {
            minute = read;
            k = mend;
        }
    }
    if let Some((half, mend)) = meridiem_at(chars, k) {
        return Some(timed(i, mend, hour, minute, Some(half)));
    }
    let oc = oclock_at(chars, k)?;
    Some(match meridiem_at(chars, oc) {
        Some((half, mend)) => timed(i, mend, hour, minute, Some(half)),
        None => Scan::Nothing,
    })
}

/// `am`, `pm`, `a.m.`, `p.m.` in any case after one space at most; `in the morning`, `in the afternoon`, `in
/// the evening`, `at night`.
fn meridiem_at(chars: &[char], i: usize) -> Option<(Meridiem, usize)> {
    let j = skip_space(chars, i);
    for (form, half) in [
        ("a.m.", Meridiem::Am),
        ("p.m.", Meridiem::Pm),
        ("a.m", Meridiem::Am),
        ("p.m", Meridiem::Pm),
        ("am", Meridiem::Am),
        ("pm", Meridiem::Pm),
    ] {
        if let Some(end) = phrase_end(chars, j, form) {
            return Some((half, end));
        }
    }
    for (form, half) in [
        ("in the morning", Meridiem::Am),
        ("in the afternoon", Meridiem::Pm),
        ("in the evening", Meridiem::Pm),
        ("at night", Meridiem::Pm),
    ] {
        if let Some(end) = phrase(chars, j, form) {
            return Some((half, end));
        }
    }
    None
}

/// `o'clock` in its spellings, after one space at most.
fn oclock_at(chars: &[char], i: usize) -> Option<usize> {
    let j = skip_space(chars, i);
    ["o'clock", "o’clock", "oclock", "o clock"]
        .into_iter()
        .find_map(|form| phrase(chars, j, form))
}

/// An hour word, `one` to `twelve`, at a word boundary.
fn hour_word(chars: &[char], i: usize) -> Option<(u8, usize)> {
    let (word, end) = word_at(chars, i)?;
    let hour = ones(&word)?;
    (hour <= 12 && boundary(chars, i)).then_some((hour, end))
}

/// `thirty`, `fifteen`, `forty five`, `oh five`: minutes after an hour word.
fn minute_words(chars: &[char], i: usize) -> Option<(u8, usize)> {
    let (word, end) = word_at(chars, i)?;
    if (word == "oh" || word == "o")
        && let Some((next, nend)) = word_at(chars, skip_space(chars, end))
        && let Some(minute) = ones(&next)
        && minute < 10
    {
        return Some((minute, nend));
    }
    if let Some(tens) = tens(&word) {
        if matches!(chars.get(end), Some(' ' | '-'))
            && let Some((next, nend)) = word_at(chars, end + 1)
            && let Some(ones) = ones(&next)
            && (1..=9).contains(&ones)
        {
            return Some((tens + ones, nend));
        }
        return Some((tens, end));
    }
    ones(&word)
        .filter(|minute| (10..=19).contains(minute))
        .map(|minute| (minute, end))
}

/// At most a tens word and a ones word, `twenty four`, `seven`: as a day or a year reads them, never a longer run.
fn small_words(chars: &[char], i: usize) -> Option<(u8, usize)> {
    if !boundary(chars, i) {
        return None;
    }
    let (word, end) = word_at(chars, i)?;
    if let Some(tens) = tens(&word) {
        if matches!(chars.get(end), Some(' ' | '-'))
            && let Some((next, nend)) = word_at(chars, end + 1)
            && let Some(ones) = ones(&next)
            && (1..=9).contains(&ones)
        {
            return Some((tens + ones, nend));
        }
        return Some((tens, end));
    }
    ones(&word).map(|ones| (ones, end))
}

fn ones(word: &str) -> Option<u8> {
    ONES.iter()
        .find(|(ones, _)| *ones == word)
        .and_then(|(_, value)| u8::try_from(*value).ok())
}

fn tens(word: &str) -> Option<u8> {
    TENS.iter()
        .find(|(tens, _)| *tens == word)
        .and_then(|(_, value)| u8::try_from(*value).ok())
}

// ---- a duration and a number, as before -------------------------------------------------------------------

/// Unit forms in the order they are tried, with their seconds.
const UNITS: [(&str, f64); 15] = [
    ("seconds", 1.0),
    ("second", 1.0),
    ("secs", 1.0),
    ("sec", 1.0),
    ("minutes", 60.0),
    ("minute", 60.0),
    ("mins", 60.0),
    ("min", 60.0),
    ("hours", 3600.0),
    ("hour", 3600.0),
    ("hrs", 3600.0),
    ("hr", 3600.0),
    ("s", 1.0),
    ("m", 60.0),
    ("h", 3600.0),
];

/// The words that stand for an amount before a unit written out, and what they stand for.
const ARTICLES: [(&str, f64); 6] = [
    ("a quarter of an", 0.25),
    ("a quarter of a", 0.25),
    ("half an", 0.5),
    ("half a", 0.5),
    ("an", 1.0),
    ("a", 1.0),
];

/// What stands before a unit: the count, where it ends, and whether a one-letter unit — `10m` — may follow it,
/// which it may after a number and never after an article; a run of number words to pass over; or nothing.
enum Before {
    Read(f64, usize, bool),
    Skip(usize),
    Nothing,
}

/// The count at `i`: a number in words, an article's — «an hour», «half a minute» — or a number in digits.
fn before_unit(chars: &[char], i: usize) -> Before {
    match words(chars, i) {
        Some(Words {
            end,
            value: Some(value),
        }) => return Before::Read(value, end, true),
        Some(Words { end, value: None }) => return Before::Skip(end),
        None => {}
    }
    if let Some((value, end)) = article(chars, i) {
        return Before::Read(value, end, false);
    }
    // A minus the number's own — `-5 minutes` — makes no duration: the number stands alone, negative.
    if i > 0 && chars[i - 1] == '-' && (i == 1 || !is_word(chars[i - 2])) {
        return Before::Nothing;
    }
    match decimal(chars, i) {
        Some((value, end)) => Before::Read(value, end, true),
        None => Before::Nothing,
    }
}

fn duration(chars: &[char], i: usize) -> Scan {
    let (count, after, letters) = match before_unit(chars, i) {
        Before::Read(value, end, letters) => (value, end, letters),
        Before::Skip(end) => return Scan::Skip(end),
        Before::Nothing => return Scan::Nothing,
    };
    // «and a half» adds a half, after the count — «two and a half hours» — or after the unit — «an hour and a
    // half».
    let (count, after, halved) = match half_after(chars, after) {
        Some(end) => (count + 0.5, end, true),
        None => (count, after, false),
    };
    let Some((end, seconds_each)) = unit(chars, after_space(chars, after), letters) else {
        return Scan::Nothing;
    };
    let (count, end) = match half_after(chars, end) {
        Some(end) if !halved => (count + 0.5, end),
        _ => (count, end),
    };
    let Some(seconds) = seconds(count * seconds_each) else {
        return Scan::Nothing;
    };
    found(i, end, Read::Seconds(seconds))
}

fn number(chars: &[char], i: usize) -> Scan {
    let (value, after) = match words(chars, i) {
        Some(Words {
            end,
            value: Some(value),
        }) => (value, end),
        Some(Words { end, value: None }) => return Scan::Skip(end),
        None => {
            // A minus is the number's when nothing wordlike stands before it and a digit follows: `-5`, never `5-10`.
            let signed = chars[i] == '-'
                && chars.get(i + 1).is_some_and(char::is_ascii_digit)
                && (i == 0 || !is_word(chars[i - 1]));
            let Some((value, after)) = decimal(chars, if signed { i + 1 } else { i }) else {
                return Scan::Nothing;
            };
            (if signed { -value } else { value }, after)
        }
    };
    let unit_at = after_space(chars, after);
    let end = if let Some(end) = unit_end(chars, unit_at, "percent") {
        end
    } else if chars.get(unit_at) == Some(&'%') {
        unit_at + 1
    } else {
        after
    };
    found(i, end, Read::Number(value))
}

/// The number words read, with their values: the ones to nineteen, then the tens.
const ONES: [(&str, u32); 20] = [
    ("zero", 0),
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
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
const TENS: [(&str, u32); 8] = [
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
];

/// A word of a run of number words.
#[derive(Clone, Copy, PartialEq)]
enum Token {
    Ones(u32),
    Tens(u32),
    Hundred,
    /// «thousand»: past what a number reads, and what a figure in words may hold once.
    Thousand,
    /// «million», «billion»: past what is read, so a run holding one reads as nothing.
    Beyond,
    /// «a», opening a run before «hundred» or a word beyond it: «a hundred».
    A,
    /// «and», after «hundred» or a word beyond it and before a number word: «a hundred and fifty».
    And,
}

fn token(word: &str) -> Option<Token> {
    if let Some((_, value)) = ONES.iter().find(|(ones, _)| *ones == word) {
        return Some(Token::Ones(*value));
    }
    if let Some((_, value)) = TENS.iter().find(|(tens, _)| *tens == word) {
        return Some(Token::Tens(*value));
    }
    match word {
        "hundred" => Some(Token::Hundred),
        "thousand" => Some(Token::Thousand),
        "million" | "billion" => Some(Token::Beyond),
        "a" => Some(Token::A),
        "and" => Some(Token::And),
        _ => None,
    }
}

/// The word at `at`, in any letter case, as a token, with where it ends.
fn token_at(chars: &[char], at: usize) -> Option<(Token, usize)> {
    let (word, end) = word_at(chars, at)?;
    token(&word).map(|token| (token, end))
}

/// Whether one space or one hyphen at `at` is followed by a word that `fits`.
fn follows(chars: &[char], at: usize, fits: impl Fn(Token) -> bool) -> bool {
    matches!(chars.get(at), Some(' ' | '-'))
        && token_at(chars, at + 1).is_some_and(|(token, _)| fits(token))
}

/// A run of number words: where it ends, and its value when it is a form that is read.
struct Words {
    end: usize,
    value: Option<f64>,
}

/// The run of number words at `i` — the words to nineteen, the tens, «hundred» and the words beyond it, «a»
/// before those and «and» after them — joined by one space or one hyphen, with a word boundary at each end:
/// «tenant» and «one-off» hold none. It reads as one word to ninety, a tens word joined to a word from one to
/// nine, «a hundred» or «one hundred»; a longer run — «two hundred», «a hundred and fifty», «seven thirty» —
/// reads as nothing, whole, so no part of it is a candidate.
fn words(chars: &[char], i: usize) -> Option<Words> {
    let (tokens, end) = run(chars, i)?;
    let value = match tokens[..] {
        [Token::Ones(value) | Token::Tens(value)] => Some(value),
        [Token::Tens(tens), Token::Ones(ones)] if (1..=9).contains(&ones) => Some(tens + ones),
        [Token::A | Token::Ones(1), Token::Hundred] => Some(100),
        _ => None,
    };
    Some(Words {
        end,
        value: value.map(f64::from),
    })
}

/// The run's tokens and where it ends, whatever they add up to.
fn run(chars: &[char], i: usize) -> Option<(Vec<Token>, usize)> {
    if i > 0 && (is_word(chars[i - 1]) || i >= 2 && hyphen_binds(chars, i - 1, i - 2)) {
        return None;
    }
    let mut tokens: Vec<Token> = Vec::new();
    let mut end = i;
    let mut at = i;
    while let Some((token, word_end)) = token_at(chars, at) {
        let fits = match token {
            Token::A => {
                tokens.is_empty()
                    && follows(chars, word_end, |next| {
                        matches!(next, Token::Hundred | Token::Thousand | Token::Beyond)
                    })
            }
            Token::And => {
                matches!(
                    tokens.last(),
                    Some(Token::Hundred | Token::Thousand | Token::Beyond)
                ) && follows(chars, word_end, |next| {
                    matches!(next, Token::Ones(_) | Token::Tens(_))
                })
            }
            _ => true,
        };
        if !fits {
            break;
        }
        tokens.push(token);
        end = word_end;
        if !matches!(chars.get(end), Some(' ' | '-')) {
            break;
        }
        at = end + 1;
    }
    if tokens.is_empty()
        || chars.get(end).is_some_and(|&c| is_word(c))
        || hyphen_binds(chars, end, end + 1)
    {
        return None;
    }
    Some((tokens, end))
}

/// Whether a hyphen at `at` binds the word beside it to a word at `other`, as in «one-off».
fn hyphen_binds(chars: &[char], at: usize, other: usize) -> bool {
    chars.get(at) == Some(&'-') && chars.get(other).is_some_and(|c| c.is_alphabetic())
}

/// An article's count at `i`, at a word boundary and in any letter case: what it stands for, and where it ends.
fn article(chars: &[char], i: usize) -> Option<(f64, usize)> {
    if i > 0 && is_word(chars[i - 1]) {
        return None;
    }
    ARTICLES
        .into_iter()
        .find_map(|(words, value)| phrase_end(chars, i, words).map(|end| (value, end)))
}

/// Where «and a half» ends when it follows what ends at `at`.
fn half_after(chars: &[char], at: usize) -> Option<usize> {
    phrase_end(chars, after_space(chars, at), "and a half")
}

/// The unit at `at` and its seconds; a one-letter form, `10m`, only after a number.
fn unit(chars: &[char], at: usize, letters: bool) -> Option<(usize, f64)> {
    UNITS
        .into_iter()
        .filter(|(unit, _)| letters || unit.len() > 1)
        .find_map(|(unit, seconds)| unit_end(chars, at, unit).map(|end| (end, seconds)))
}

/// `\b\d+(\.\d+)?` at `i`: the number and where it ends; digits past what a number holds are no candidate, and
/// neither are the digits after a digit and a comma or a point, `000` in `1,000` and `4` in `0.4`.
fn decimal(chars: &[char], i: usize) -> Option<(f64, usize)> {
    let inside = i >= 2 && matches!(chars[i - 1], ',' | '.') && chars[i - 2].is_ascii_digit();
    if !chars[i].is_ascii_digit() || (i > 0 && is_word(chars[i - 1])) || inside {
        return None;
    }
    let digits = |from: usize| {
        from + chars[from..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count()
    };
    let mut end = digits(i);
    if chars.get(end) == Some(&'.') && chars.get(end + 1).is_some_and(char::is_ascii_digit) {
        end = digits(end + 1);
    }
    let value: f64 = chars[i..end].iter().collect::<String>().parse().ok()?;
    value.is_finite().then_some((value, end))
}

/// Whole seconds, to the nearest; a count past what seconds can hold is no candidate, and neither is one that
/// rounds to nothing, `0.4 seconds`; a `0` said outright stays.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn seconds(count: f64) -> Option<u64> {
    if count > 0.0 && count < 0.5 {
        return None;
    }
    let whole = count.round();
    (whole < u64::MAX as f64).then_some(whole as u64)
}

// ---- the characters ----------------------------------------------------------------------------------------

/// One space may separate a count from its unit.
fn after_space(chars: &[char], at: usize) -> usize {
    if chars.get(at).is_some_and(|c| c.is_whitespace()) {
        at + 1
    } else {
        at
    }
}

/// One space at `at`, exactly, is passed over.
fn skip_space(chars: &[char], at: usize) -> usize {
    if chars.get(at) == Some(&' ') {
        at + 1
    } else {
        at
    }
}

/// Where `unit` ends when it stands at `at` and no word continues it.
fn unit_end(chars: &[char], at: usize, unit: &str) -> Option<usize> {
    let end = at + unit.len();
    (starts_with(chars, at, unit) && !chars.get(end).is_some_and(|&c| is_word(c))).then_some(end)
}

/// Where `text`, words in any letter case, ends when it stands at `at` and no word continues it.
fn phrase_end(chars: &[char], at: usize, text: &str) -> Option<usize> {
    let end = at + text.chars().count();
    (phrase_lower(chars, at, text) && !chars.get(end).is_some_and(|&c| is_word(c))).then_some(end)
}

/// Where `text` ends when it stands at `at` at a word boundary, in any letter case, and no word continues it.
fn phrase(chars: &[char], at: usize, text: &str) -> Option<usize> {
    if !boundary(chars, at) {
        return None;
    }
    phrase_end(chars, at, text)
}

/// Whether `text`, in any letter case, stands at `at`.
fn phrase_lower(chars: &[char], at: usize, text: &str) -> bool {
    text.chars().enumerate().all(|(k, c)| {
        chars
            .get(at + k)
            .is_some_and(|d| d.to_ascii_lowercase() == c)
    })
}

/// The alphabetic word at `i`, lowered, and where it ends; none when no letter stands there.
fn word_at(chars: &[char], i: usize) -> Option<(String, usize)> {
    let end = i + chars
        .get(i..)?
        .iter()
        .take_while(|c| c.is_alphabetic())
        .count();
    (end > i).then(|| {
        (
            chars[i..end]
                .iter()
                .flat_map(|c| c.to_lowercase())
                .collect(),
            end,
        )
    })
}

/// A run of digits at `i` with a word boundary before it: its value and where it ends; more digits than a
/// count holds is none.
fn digits_at(chars: &[char], i: usize) -> Option<(u64, usize)> {
    if !boundary(chars, i) {
        return None;
    }
    let end = digit_run(chars, i)?;
    chars[i..end]
        .iter()
        .collect::<String>()
        .parse()
        .ok()
        .map(|value| (value, end))
}

/// Where the digits at `at` end, when at least one stands there.
fn digit_run(chars: &[char], at: usize) -> Option<usize> {
    let end = at
        + chars
            .get(at..)?
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
    (end > at).then_some(end)
}

/// Whether a word may begin at `i`: the start, or no word character before it.
fn boundary(chars: &[char], i: usize) -> bool {
    i == 0 || !is_word(chars[i - 1])
}

/// Whether a word may end at `j`: the end, or no word character there.
fn ends_word(chars: &[char], j: usize) -> bool {
    !chars.get(j).is_some_and(|&c| is_word(c))
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn starts_with(chars: &[char], at: usize, text: &str) -> bool {
    text.chars()
        .enumerate()
        .all(|(k, c)| chars.get(at + k) == Some(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str) -> Vec<(usize, usize, PickValue)> {
        propose(&Input::new(text).unwrap())
            .into_iter()
            .map(|p| (p.span.start(), p.span.end(), p.value))
            .collect()
    }

    /// Each candidate as `<kind> <text>`, the value aside.
    fn kinds(text: &str) -> Vec<String> {
        propose(&Input::new(text).unwrap())
            .into_iter()
            .map(|p| {
                let kind = serde_json::to_value(&p.value).unwrap()["type"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                format!("{kind} {}", p.span.text())
            })
            .collect()
    }

    /// No candidate at all.
    fn none(text: &str) {
        assert!(kinds(text).is_empty(), "{text}: {:?}", kinds(text));
    }

    fn number(value: f64) -> PickValue {
        PickValue::Number { value }
    }

    fn seconds(value: u64) -> PickValue {
        PickValue::Seconds { value }
    }

    fn quoted(text: &str) -> PickValue {
        PickValue::Quoted {
            value: Clean::new(text).unwrap(),
        }
    }

    #[test]
    fn units_take_their_forms_and_need_a_boundary() {
        assert_eq!(spans("10mins")[0].2, seconds(600));
        assert_eq!(spans("2 hr")[0].2, seconds(7200));
        assert_eq!(spans("10 ms")[0].2, number(10.0));
        assert_eq!(spans("3secondsx")[0].2, number(3.0));
        assert!(spans("x10 minutes").is_empty());
        assert_eq!(spans("30 percentile")[0], (0, 2, number(30.0)));
        assert_eq!(spans("30 percent.")[0], (0, 10, number(30.0)));
    }

    #[test]
    fn digits_past_what_a_number_or_seconds_hold_are_no_candidate() {
        let long = "9".repeat(400);
        assert!(spans(&format!("dim to {long}")).is_empty());
        assert!(spans(&format!("wait {long} hours")).is_empty());
        assert_eq!(
            spans("wait 99999999999999999999 hours"),
            [(5, 25, number(1e20))]
        );
        assert_eq!(spans("wait 1e3 hours")[0].2, number(1.0));
    }

    #[test]
    fn number_words_read_whole_or_not_at_all() {
        assert_eq!(
            spans("twenty five or twenty-five"),
            [(0, 11, number(25.0)), (15, 26, number(25.0))]
        );
        assert_eq!(spans("One Hundred Percent"), [(0, 11, number(100.0))]);
        assert_eq!(spans("one hundred percent"), [(0, 19, number(100.0))]);
        assert!(spans("two hundred, a hundred and fifty, seven thirty, a thousand").is_empty());
        assert!(spans("tenant one-off fifty5 twenty-fiveish").is_empty());
        assert_eq!(spans("5fifty"), [(0, 1, number(5.0))]);
        assert_eq!(
            spans("lamp one and two"),
            [(5, 8, number(1.0)), (13, 16, number(2.0))]
        );
        assert_eq!(spans("twenty five mins"), [(0, 16, seconds(1500))]);
    }

    #[test]
    fn articles_and_halves_make_durations() {
        assert_eq!(spans("in an hour's time"), [(3, 10, seconds(3600))]);
        assert_eq!(spans("half a minute")[0].2, seconds(30));
        assert_eq!(spans("a quarter of an hour and a half")[0].2, seconds(2700));
        assert_eq!(spans("2 and a half hours")[0].2, seconds(9000));
        assert_eq!(spans("2 hours and a half")[0].2, seconds(9000));
        assert_eq!(spans("five m")[0].2, seconds(300));
        assert_eq!(spans("at seven a m"), [(3, 8, number(7.0))]);
        assert!(spans("a timer and an alarm").is_empty());
    }

    #[test]
    fn urls_and_emails_drop_trailing_punctuation_and_hide_their_insides() {
        assert_eq!(spans("see https://a.io/x)?")[0].1, 18);
        assert!(spans("https://x").is_empty());
        assert_eq!(
            spans("mail a@b.co.uk.")[0],
            (
                5,
                14,
                PickValue::Email {
                    value: Clean::new("a@b.co.uk").unwrap()
                }
            )
        );
        assert!(spans("a@b.c").is_empty());
        assert_eq!(spans("at https://x.org/a@b.com/3").len(), 1);
    }

    #[test]
    fn a_dropped_match_still_hides_nothing_of_its_own_kind() {
        assert_eq!(
            spans("\"a 5\" 6"),
            [(1, 4, quoted("a 5")), (6, 7, number(6.0))]
        );
        assert_eq!(spans("say \"\" now"), []);
        assert_eq!(spans("“a “b” c”")[0].2, quoted("b"));
    }

    #[test]
    fn a_match_hides_its_inside_even_when_its_text_cannot_be_a_span() {
        assert_eq!(spans("say \"a\t5\" 6"), [(10, 11, number(6.0))]);
        assert_eq!(spans("https://a.io/\u{7}5 7"), [(16, 17, number(7.0))]);
    }

    #[test]
    fn a_date_reads_relative_and_hides_its_figures() {
        assert_eq!(
            kinds("move it to tomorrow at three"),
            ["date tomorrow", "number three"]
        );
        assert_eq!(
            kinds("send tomorrow's agenda and todays numbers"),
            ["date tomorrow", "date todays"]
        );
        assert_eq!(
            kinds("next monday, this friday or last tuesday"),
            ["date next monday", "date this friday", "date last tuesday"]
        );
        assert_eq!(kinds("remind me on mondays at 9am"), ["time 9am"]);
        assert_eq!(
            kinds("every monday at nine and everyday at noon"),
            ["number nine", "time noon"]
        );
        assert_eq!(
            kinds("on may fifth, the 14th of march, march 7 and 7th march"),
            [
                "date may fifth",
                "date the 14th of march",
                "date march 7",
                "date 7th march"
            ]
        );
        assert_eq!(
            kinds("see me friday the 14th and tuesday 21 march 2017"),
            ["date friday the 14th", "date tuesday 21 march 2017"]
        );
        assert_eq!(
            kinds("pay it on the 14th and remind me by the twenty second"),
            ["date the 14th", "date the twenty second"]
        );
        none("remove the first alarm and play the second song");
        assert_eq!(
            kinds("take out from the deli on 6th ave and pay on the 6th"),
            ["number 6", "date the 6th"]
        );
        assert_eq!(
            kinds("in three days, in two weeks and a week from today"),
            [
                "date in three days",
                "date in two weeks",
                "date a week from today"
            ]
        );
        none("on february 30, april 31 and 2026-02-29");
        assert_eq!(
            kinds("on 27/03/2017, on 5/17/2037 and on 12/03/2019"),
            ["date 27/03/2017", "date 5/17/2037"]
        );
        assert_eq!(
            kinds("on 27.03.2017 but not 7.30"),
            ["date 27.03.2017", "number 7.30"]
        );
        none("this week, next month and this weekend");
        assert_eq!(kinds("what day is the 15th"), ["date the 15th"]);
        assert_eq!(
            kinds("december 4, 1982 and march fourth two thousand and seventeen"),
            [
                "date december 4, 1982",
                "date march fourth two thousand and seventeen"
            ]
        );
    }

    #[test]
    fn a_date_carries_its_reading() {
        let day = |text: &str| match &spans(text)[0].2 {
            PickValue::Date { value } => value.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(day("tomorrow"), Day::Offset { days: 1 });
        assert_eq!(day("in 2 weeks"), Day::Offset { days: 14 });
        assert_eq!(
            day("next monday"),
            Day::Weekday {
                weekday: Weekday::Monday,
                which: Some(Which::Next)
            }
        );
        assert_eq!(day("the 14th"), Day::Nth { day: 14 });
        assert_eq!(
            day("may fifth"),
            Day::Calendar {
                year: None,
                month: 5,
                day: 5
            }
        );
        assert_eq!(
            day("2026-05-05"),
            Day::Calendar {
                year: Some(2026),
                month: 5,
                day: 5
            }
        );
        assert_eq!(
            day("3/21/2017"),
            Day::Calendar {
                year: Some(2017),
                month: 3,
                day: 21
            }
        );
        assert!(
            spans("in 5000 days")
                .iter()
                .all(|(_, _, value)| matches!(value, PickValue::Number { .. }))
        );
    }

    #[test]
    fn a_time_needs_its_half_of_the_day_or_two_digits() {
        let clock = |text: &str| match &spans(text)[0].2 {
            PickValue::Time { value } => value.to_string(),
            other => panic!("{other:?}"),
        };
        assert_eq!(clock("5am"), "05:00");
        assert_eq!(clock("6:30 pm"), "18:30");
        assert_eq!(clock("7 a.m."), "07:00");
        assert_eq!(clock("12 AM"), "00:00");
        assert_eq!(clock("12 pm"), "12:00");
        assert_eq!(clock("five pm"), "17:00");
        assert_eq!(clock("seven thirty am"), "07:30");
        assert_eq!(clock("eight at night"), "20:00");
        assert_eq!(clock("15:00"), "15:00");
        assert_eq!(clock("09:30"), "09:30");
        assert_eq!(clock("five o'clock in the afternoon"), "17:00");
        assert_eq!(clock("half past five pm"), "17:30");
        assert_eq!(clock("a quarter to six am"), "05:45");
        assert_eq!(clock("ten past nine pm"), "21:10");
        assert_eq!(clock("noon"), "12:00");
        assert_eq!(clock("midnight"), "00:00");
        assert_eq!(clock("six in the morning"), "06:00");
        assert_eq!(clock("13 o'clock"), "13:00");
        assert_eq!(
            kinds("move the meeting to tomorrow at three"),
            ["date tomorrow", "number three"]
        );
        assert_eq!(
            kinds("wake me at 5:30 and set the volume to 10"),
            ["number 10"]
        );
        none("at six thirty, 4 o'clock, half past five and eight hundred");
        none("the deploy at 21:05:17 or 24:00");
        assert_eq!(kinds("at seven a m"), ["number seven"]);
        assert_eq!(
            kinds("this morning, tonight's plan and this afternoon"),
            ["date tonight"]
        );
        assert_eq!(
            kinds("remind me in ten minutes and one hour from now"),
            ["duration ten minutes", "duration one hour"]
        );
        assert_eq!(kinds("wake me up at 5am this week"), ["time 5am"]);
    }

    #[test]
    fn an_amount_reads_with_its_currency() {
        let money = |text: &str| match &spans(text)[0].2 {
            PickValue::Amount { value } => {
                format!("{} {}", value.amount(), value.currency().as_str())
            }
            other => panic!("{other:?}"),
        };
        assert_eq!(money("pay €1,200"), "1200 EUR");
        assert_eq!(money("$50"), "50 USD");
        assert_eq!(money("£19.99"), "19.99 GBP");
        assert_eq!(money("€ 1,200"), "1200 EUR");
        assert_eq!(money("50 dollars"), "50 USD");
        assert_eq!(money("twelve hundred dollars"), "1200 USD");
        assert_eq!(money("a hundred and fifty euros"), "150 EUR");
        assert_eq!(
            money("one thousand two hundred and fifty euros"),
            "1250 EUR"
        );
        assert_eq!(money("3000 yen"), "3000 JPY");
        assert_eq!(money("a dollar"), "1 USD");
        assert_eq!(money("1000 USD"), "1000 USD");
        assert_eq!(money("500USD"), "500 USD");
        assert_eq!(money("it costs $1,234.56"), "1234.56 USD");
        assert_eq!(kinds("pay €1,200 to sam"), ["amount €1,200"]);
        assert_eq!(kinds("send 50 to sam"), ["number 50"]);
        none("the $ sign");
        assert_eq!(
            kinds("a fifteen pound turkey, 3 pound beef, one pound and a dollar"),
            [
                "number fifteen",
                "number 3",
                "amount one pound",
                "amount a dollar"
            ]
        );
        assert_eq!(kinds("-€5 and -5 dollars"), ["number 5", "number -5"]);
        assert_eq!(kinds("USD 50 and try 3 times"), ["number 50", "number 3"]);
        assert_eq!(kinds("1gbp"), ["number 1"]);
        assert_eq!(kinds("pay €1.200 or €1,200"), ["amount €1,200"]);
        assert_eq!(kinds("$5k"), ["number 5"]);
        none("seven thirty dollars");
    }

    #[test]
    fn a_code_reads_as_typed() {
        assert_eq!(
            kinds("roll back 4.12.0 and 1.2.3.4"),
            ["code 4.12.0", "code 1.2.3.4"]
        );
        none("v2.3.1 and v2");
        assert_eq!(
            kinds("from 4.12.0 to 4.11.3"),
            ["code 4.12.0", "code 4.11.3"]
        );
        assert_eq!(kinds("dim to 4.12"), ["number 4.12"]);
        assert_eq!(
            kinds("look up INC-311 and reserve PC-2210"),
            ["code INC-311", "code PC-2210"]
        );
        assert_eq!(
            kinds("wait 5-10 minutes for INC-311"),
            ["number 5", "duration 10 minutes", "code INC-311"]
        );
        assert_eq!(
            kinds("book TP1043 and LH 1234"),
            ["code TP1043", "number 1234"]
        );
        assert_eq!(
            kinds("wipe C02XK1ABJG5M and FVFH3KLMN2Q7"),
            ["code C02XK1ABJG5M", "code FVFH3KLMN2Q7"]
        );
        none("convert to MP3 and H264 for WIN10");
        assert_eq!(
            kinds("look up inc-311 and book tp1043"),
            ["code inc-311", "code tp1043"]
        );
        assert_eq!(
            kinds("wipe c02xk1abjg5m and Fvfh3klmn2q7"),
            ["code c02xk1abjg5m", "code Fvfh3klmn2q7"]
        );
        assert_eq!(kinds("revert a1b2c3d4e5f"), ["code a1b2c3d4e5f"]);
        none("convert to mp3 and h264 for win10");
        assert_eq!(
            kinds("wait 10mins then send 1200gbp"),
            ["duration 10mins", "number 1200"]
        );
        assert_eq!(kinds("book 3U8888"), ["code 3U8888"]);
        assert_eq!(kinds("a-1 and lh 1234"), ["number 1", "number 1234"]);
        assert_eq!(kinds("roll back \"4.12.0\""), ["quoted 4.12.0"]);
        assert_eq!(
            kinds("look up incident 311 and order 4821"),
            ["number 311", "number 4821"]
        );
        assert_eq!(kinds("2.0.0-rc.1"), ["number 2.0", "number 1"]);
        assert_eq!(
            kinds("see https://example.com/v/4.12.0"),
            ["url https://example.com/v/4.12.0"]
        );
        assert_eq!(
            kinds("mail sam2026@example.com"),
            ["email sam2026@example.com"]
        );
        assert_eq!(
            kinds("note \"tomorrow at three for €5\""),
            ["quoted tomorrow at three for €5"]
        );
        assert_eq!(
            kinds("timer for 3 minutes called \"eggs\" at 30 percent and -5"),
            [
                "duration 3 minutes",
                "quoted eggs",
                "number 30 percent",
                "number -5"
            ]
        );
    }

    #[test]
    fn no_two_candidates_overlap() {
        assert_eq!(kinds("pay may 5 dollars"), ["amount 5 dollars"]);
        assert_eq!(
            kinds("on march 7 at the 14th"),
            ["date march 7", "date the 14th"]
        );
        assert_eq!(kinds("8 tonight"), ["number 8", "date tonight"]);
        for text in [
            "pay may 5 dollars",
            "wake me at 6:30 tomorrow and again at 7",
            "on 12/03/2019 at 5:30 for €1.200",
        ] {
            let found = spans(text);
            for (a, b) in found.iter().zip(found.iter().skip(1)) {
                assert!(a.1 <= b.0, "{text}: {a:?} and {b:?}");
            }
        }
    }
}
