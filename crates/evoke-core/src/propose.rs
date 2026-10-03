//! The nine recognizers over an input, and masking. In: `Input`. Out: `Vec<Proposed>`, verbatim spans with typed
//! values, in order of position, no two overlapping: quotes hide what they enclose; a URL, an address, an amount,
//! a code, a date, a time and a duration hide what they cover; a number stands last. A form of a kind refused
//! whole — a slashed date either way round, `5:30` with no half of the day, `every monday` — hides its parts, so
//! no part of it is a candidate; a run of number words that reads as nothing is passed over and hides nothing.
//! A number and a duration read spelled out as they read in digits, the hundreds with what follows them; a date
//! reads relative, resolved at the body's door; a bare hour is a number, since a half of the day would be
//! invented.

use std::cmp::Reverse;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::calendar::{Clock, Day, days_in};
use crate::pack::{self, Lexicon, Pack};
use crate::text::{self, Clean, Input, Span, fold_char};

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
        if code.chars().count() == 3 && code.chars().all(text::capital) {
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
    let lexicon = pack::lexicon(input.as_str());
    let mut hidden: Vec<Range<usize>> = Vec::new();
    let mut found = Vec::new();
    for recognize in RECOGNIZERS {
        let mut i = 0;
        while i < chars.len() {
            let Found { span, after, read } = match recognize(&lexicon, &chars, i) {
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

/// What one kind finds at a position of the input, the words of the lexicon in hand.
type Recognizer = fn(&Lexicon, &[char], usize) -> Scan;

/// In this order: an earlier kind's match hides the candidates that reach into it. An amount stands before a
/// code, so `500USD` is an amount; a code before a date and a number, so `4.12.0` is no decimal; a date before a
/// time, so `tonight` is a date; a time before a duration and a number, so `5pm` hides its 5.
const RECOGNIZERS: [Recognizer; 9] = [
    quoted, url, email, amount, code, date, time, duration, number,
];

fn quoted(lexicon: &Lexicon, chars: &[char], i: usize) -> Scan {
    let open = chars[i];
    let Some((_, close)) = lexicon
        .pairs(|pack| &pack.quotes.value)
        .into_iter()
        .find(|(opens, _)| *opens == open)
    else {
        return Scan::Nothing;
    };
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

fn url(lexicon: &Lexicon, chars: &[char], i: usize) -> Scan {
    let Some(scheme) = lexicon
        .phrases(|pack| &pack.spoken.schemes)
        .into_iter()
        .map(|scheme| format!("{scheme}://"))
        .find(|scheme| starts_with(chars, i, scheme))
    else {
        return Scan::Nothing;
    };
    let inner = |c: char| !(c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\''));
    let mut j = i + scheme.chars().count();
    while j < chars.len() && inner(chars[j]) {
        j += 1;
    }
    let Some(end) = (i + scheme.chars().count() + 2..=j)
        .rev()
        .find(|&end| !matches!(chars[end - 1], '.' | ',' | ';' | ':' | '!' | '?' | ')'))
    else {
        return Scan::Nothing;
    };
    found(i, end, Read::Url)
}

fn email(_: &Lexicon, chars: &[char], i: usize) -> Scan {
    let local = |c: char| text::latin_or_figure(c) || matches!(c, '.' | '_' | '%' | '+' | '-');
    let domain = |c: char| text::latin_or_figure(c) || matches!(c, '.' | '-');
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
                .take_while(|c| text::latin(**c))
                .count();
            (letters >= 2).then_some(dot + 1 + letters)
        })
    else {
        return Scan::Nothing;
    };
    found(i, end, Read::Email)
}

// ---- an amount in a currency -------------------------------------------------------------------------------

/// An amount: a symbol before a figure, `€1,200`; a figure, number words, `a` or `an` before a currency word,
/// `50 dollars`, `twelve hundred euros`, `a dollar`; or a figure before a code in capitals, `1000 USD`, `500USD`.
/// A sign before the figure is the number's; `$5k` reads nothing; a symbol before a dotted thousands figure,
/// `€1.200`, is two readings, refused whole. A currency's singular word is a currency after `1`, `one` or `a`
/// only, so `3 pound beef` is a weight.
fn amount(lexicon: &Lexicon, chars: &[char], i: usize) -> Scan {
    if i > 0 && chars[i - 1] == '-' && (i == 1 || !is_word(chars[i - 2])) {
        return Scan::Nothing;
    }
    let symbol = lexicon
        .packs()
        .iter()
        .flat_map(|pack| pack.amounts.symbols.iter())
        .find(|(symbol, _)| symbol.chars().eq(std::iter::once(chars[i])));
    if let Some((_, currency)) = symbol {
        if !boundary(chars, i) {
            return Scan::Nothing;
        }
        let j = skip_space(chars, i + 1);
        let Some(figure) = figure(lexicon, chars, j) else {
            return Scan::Nothing;
        };
        let scaled = chars
            .get(figure.end)
            .is_some_and(|c| matches!(c, 'k' | 'K' | 'm' | 'M'))
            && ends_word(chars, figure.end + 1);
        if scaled {
            return Scan::Nothing;
        }
        if figure.dotted {
            return Scan::Hide(figure.end);
        }
        return moneyed(i, figure.end, figure.value, currency);
    }
    let (value, end, article) = if let Some(figure) = figure(lexicon, chars, i) {
        (figure.value, figure.end, false)
    } else if let Some((tokens, end)) = run(lexicon, chars, i) {
        // A run of number words reads whole or not at all: one that fits no form is passed over.
        match figure_value(&tokens, lexicon.teens_after_tens()) {
            Some(value) => (value, end, false),
            None => return Scan::Skip(end),
        }
    } else if let Some(end) = phrase_in(chars, i, &lexicon.phrases(|pack| &pack.numbers.article)) {
        (1.0, end, true)
    } else {
        return Scan::Nothing;
    };
    let j = skip_space(chars, end);
    // A symbol after the figure, «12 €», where a pack that reads the text writes it so.
    if lexicon.symbol_after()
        && !article
        && let Some(after) = chars.get(j)
        && let Some((_, currency)) = lexicon
            .packs()
            .iter()
            .flat_map(|pack| pack.amounts.symbols.iter())
            .find(|(symbol, _)| symbol.chars().eq(std::iter::once(*after)))
        && ends_word(chars, j + 1)
    {
        return moneyed(i, j + 1, value, currency);
    }
    let Some((word, wend)) = word_at(chars, j) else {
        return Scan::Nothing;
    };
    if let Some(currency) = lexicon.form_of(|pack| &pack.amounts.words, &word) {
        if lexicon.grammar_holds(|pack| &pack.amounts.singular, &word)
            && (value - 1.0).abs() > f64::EPSILON
        {
            return Scan::Nothing;
        }
        return moneyed(i, wend, value, currency);
    }
    let code: String = chars[j..wend].iter().collect();
    if !article && lexicon.typed(|pack| &pack.amounts.codes, &code) {
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

/// A figure at `i` with thousands groups and a fraction, the marks as the packs that read the text write them:
/// `1,200`, `19.99`, `1,234.56`; `1.200`, `19,99` where the fraction's mark is the comma.
fn figure(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Figure> {
    if !boundary(chars, i) {
        return None;
    }
    let decimal = lexicon.decimal_mark().unwrap_or('.');
    let group = if decimal == '.' { ',' } else { '.' };
    let digits = digit_run(chars, i)?;
    let mut j = digits;
    while chars.get(j) == Some(&group)
        && two_or_more_digits(chars, j + 1, 3)
        && !chars.get(j + 4).is_some_and(|c| text::figure(*c))
    {
        j += 4;
    }
    let grouped = j > digits;
    let mut text: String = chars[i..j].iter().filter(|c| **c != group).collect();
    let mut end = j;
    let mut fraction = 0;
    if chars.get(j) == Some(&decimal) && chars.get(j + 1).is_some_and(|c| text::figure(*c)) {
        end = digit_run(chars, j + 1)?;
        fraction = end - j - 1;
        text.push('.');
        text.extend(&chars[j + 1..end]);
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
    (0..count).all(|k| chars.get(at + k).is_some_and(|c| text::figure(*c)))
}

/// A figure in words, bounded: `<small>`, `a hundred [and <small>]`, `<small> hundred [and <small>]`, and with
/// `thousand` before any of those: `twelve hundred`, `one thousand two hundred and fifty`, `two thousand and
/// seventeen`, `a thousand`. Read only before a currency; a run that fits no form is nothing.
fn figure_value(tokens: &[Token], teens: bool) -> Option<f64> {
    let (thousands, rest) = match tokens.iter().position(|token| *token == Token::Thousand) {
        Some(at) => (
            Some(thousands_head(&tokens[..at], teens)?),
            &tokens[at + 1..],
        ),
        None => (None, tokens),
    };
    let rest = match (rest, thousands) {
        ([], Some(_)) => 0.0,
        ([Token::And, small @ ..], Some(_)) => small_value(small, teens)?,
        (rest, _) => hundreds_or_small(rest, teens)?,
    };
    Some(thousands.unwrap_or(0.0) * 1000.0 + rest)
}

/// What stands before `thousand`: `a`, a small number, or hundreds.
fn thousands_head(tokens: &[Token], teens: bool) -> Option<f64> {
    match tokens {
        [Token::A] => Some(1.0),
        [Token::Ones(_) | Token::Tens(_) | Token::Fused(_), ..] => hundreds_or_small(tokens, teens),
        _ => None,
    }
}

/// `<small>`, `a hundred [[and] <small>]` or `<small> hundred [[and] <small>]`.
fn hundreds_or_small(tokens: &[Token], teens: bool) -> Option<f64> {
    if let [Token::A, Token::Hundred, rest @ ..] = tokens {
        return Some(100.0 + after_hundred(rest, teens)?);
    }
    if let [Token::Hundred, rest @ ..] = tokens {
        return Some(100.0 + after_hundred(rest, teens)?);
    }
    let (small, rest) = small_prefix(tokens, teens)?;
    match rest {
        [] => Some(small),
        [Token::Hundred, rest @ ..] => Some(small * 100.0 + after_hundred(rest, teens)?),
        _ => None,
    }
}

/// What follows «hundred»: nothing, or a small number, with «and» before it or without.
fn after_hundred(tokens: &[Token], teens: bool) -> Option<f64> {
    match tokens {
        [] => Some(0.0),
        [Token::And, small @ ..] | small => small_value(small, teens),
    }
}

/// A small number alone: one word to ninety, or a tens word joined to a word from one to nine.
fn small_value(tokens: &[Token], teens: bool) -> Option<f64> {
    let (small, rest) = small_prefix(tokens, teens)?;
    rest.is_empty().then_some(small)
}

/// A small number at the head of the run: a tens word with a ones word after it — joined by the pack's joiner,
/// «vingt et un», or a teen where the pack adds one, «soixante-douze» — else one word of the tables.
#[expect(clippy::cast_precision_loss)] // a number said in words is small
fn small_prefix(tokens: &[Token], with_teens: bool) -> Option<(f64, &[Token])> {
    let adds = |ones: &u32| (1..=9).contains(ones) || (with_teens && (10..=19).contains(ones));
    match tokens {
        [Token::Tens(tens), Token::Ones(ones), rest @ ..]
        | [Token::Tens(tens), Token::And, Token::Ones(ones), rest @ ..]
            if adds(ones) =>
        {
            Some((f64::from(tens + ones), rest))
        }
        [Token::Tens(value) | Token::Ones(value), rest @ ..] => Some((f64::from(*value), rest)),
        [Token::Fused(value), rest @ ..] => Some((*value as f64, rest)),
        _ => None,
    }
}

// ---- a code ------------------------------------------------------------------------------------------------

/// An identifier as typed: a version, `4.12.0`, two dots or more, digits only; a ticket, `INC-311`, two to six
/// letters, a dash, one to six digits; a serial or a compact flight, `TP1043`, `C02XK1ABJG5M`, six or more
/// letters and digits with at least one letter and two digits; a letter in either case, so `inc-311` and
/// `tp1043` read as they are typed, and `10mins` stays a duration. At a word boundary, never after `-` or `.`,
/// ending at one; `4.12` is a decimal, `27.03.2017` a dotted date, `2.0.0-rc.1` nothing.
fn code(_: &Lexicon, chars: &[char], i: usize) -> Scan {
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
    let letters = chars[i..].iter().take_while(|c| text::latin(**c)).count();
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
        .take_while(|c| text::latin_or_figure(**c))
        .count();
    let run = &chars[i..i + length];
    let letters = run.iter().any(|c| text::latin(*c));
    let digits = run.iter().filter(|c| text::figure(**c)).count();
    let quantity =
        run.first().is_some_and(|c| text::figure(*c)) && run.iter().any(|c| text::small(*c));
    (length >= 6 && letters && digits >= 2 && !quantity).then_some(i + length)
}

// ---- a date ------------------------------------------------------------------------------------------------

/// A month and a day as they are typed in the lexicon's first pack, with the year where the reading holds one:
/// «october 5th», «october 5th 2026», a form the `date` recognizer reads back whole. None for any other reading.
#[must_use]
pub(crate) fn typed_calendar(lexicon: &Lexicon, day: &Day) -> Option<Clean> {
    let Day::Calendar { year, month, day } = day else {
        return None;
    };
    let pack = lexicon.first();
    let name = pack.months.shown(&month.to_string())?;
    let ending = pack.ordinals.suffixes.of(*day);
    let typed = match year {
        Some(year) => format!("{name} {day}{ending} {year}"),
        None => format!("{name} {day}{ending}"),
    };
    Clean::new(&typed).ok()
}

/// A date: a day word, a weekday with the word before it, a weekday joined to a day, a day or weeks ahead, an
/// ISO or slashed date, a month and a day with a year or not, a day of the month. A recurrence, a plural
/// weekday, a period, an ambiguous slashed date and a day no calendar has are refused whole.
fn date(lexicon: &Lexicon, chars: &[char], i: usize) -> Scan {
    if !boundary(chars, i) {
        return Scan::Nothing;
    }
    day_word(lexicon, chars, i)
        .or_else(|| recurrence(lexicon, chars, i))
        .or_else(|| weekday(lexicon, chars, i))
        .or_else(|| ahead(lexicon, chars, i))
        .or_else(|| numeric(chars, i))
        .or_else(|| month_first(lexicon, chars, i))
        .or_else(|| day_first(lexicon, chars, i))
        .unwrap_or(Scan::Nothing)
}

/// A candidate of a reading; nothing when the calendar refuses it.
fn dated(start: usize, end: usize, day: Option<Day>) -> Scan {
    day.map_or(Scan::Nothing, |day| found(start, end, Read::Date(day)))
}

/// A relative day, `today`, `tonight`, `tomorrow`, `yesterday`, `the day after tomorrow`, a plural's ending
/// read with it and a possessive left outside the span.
fn day_word(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    // After some words the day's word is a plain noun: «guten Morgen».
    if lexicon.holds(|pack| &pack.days.noun_after, &before_word(chars, i)) {
        return None;
    }
    let plurals = lexicon.phrases(|pack| &pack.endings.plural);
    for (word, form) in lexicon.named(|pack| &pack.days.relative) {
        let days: i32 = form.parse().ok()?;
        let end = phrase(chars, i, word).or_else(|| {
            plurals
                .iter()
                .find_map(|ending| phrase(chars, i, &format!("{word}{ending}")))
        });
        if let Some(end) = end {
            // A word that names a half of the day, «morgens», is no day.
            let whole: String = chars[i..end].iter().flat_map(|c| fold_char(*c)).collect();
            if lexicon.holds(|pack| &pack.times.am, &whole)
                || lexicon.holds(|pack| &pack.times.pm, &whole)
            {
                return None;
            }
            return Some(dated(i, end, Day::offset(days)));
        }
    }
    None
}

/// The word before `i`, as `fold` writes it, the spaces between skipped; empty where none stands there.
fn before_word(chars: &[char], i: usize) -> String {
    let mut end = i;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && chars[start - 1].is_alphabetic() {
        start -= 1;
    }
    chars[start..end]
        .iter()
        .flat_map(|c| fold_char(*c))
        .collect()
}

/// `every monday`, `each sunday`, `everyday`: passed over whole, hiding what they name.
fn recurrence(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let plurals = lexicon.phrases(|pack| &pack.endings.plural);
    let weekday = |word: &str| lexicon.form_of(|pack| &pack.days.weekdays, word).is_some();
    for each in lexicon.phrases(|pack| &pack.days.every) {
        if let Some(end) = phrase(chars, i, each)
            && chars.get(end) == Some(&' ')
            && let Some((word, wend)) = word_at(chars, end + 1)
        {
            let named = weekday(&word)
                || plurals
                    .iter()
                    .any(|ending| word.strip_suffix(ending).is_some_and(weekday))
                || lexicon.holds(|pack| &pack.days.recurring, &word)
                || lexicon.form_of(|pack| &pack.days.relative, &word).is_some()
                || ordinal_at(lexicon, chars, end + 1).is_some();
            if named {
                return Some(Scan::Hide(wend));
            }
        }
    }
    lexicon
        .phrases(|pack| &pack.days.recurrences)
        .into_iter()
        .find_map(|word| phrase(chars, i, word).map(Scan::Hide))
}

/// `friday`, `next monday`, `this wednesday`, `last tuesday`; a plural, `mondays`, and a period, `next month`,
/// read as nothing, whole. A bare weekday may join a day: `friday the 14th`, `tuesday 21 march 2017`.
fn weekday(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let mut which = None;
    let mut j = i;
    for (word, form) in lexicon.named(|pack| &pack.days.which) {
        if let Some(end) = phrase(chars, i, word)
            && chars.get(end) == Some(&' ')
        {
            which = pack::which(form);
            j = end + 1;
            break;
        }
    }
    if !boundary(chars, j) {
        return None;
    }
    let (word, end) = word_at(chars, j)?;
    let named = |word: &str| lexicon.form_of(|pack| &pack.days.weekdays, word);
    if let Some(weekday) = named(&word).and_then(pack::weekday) {
        if which.is_none()
            && let Some(joined) = weekday_day(lexicon, chars, i, end)
        {
            return Some(joined);
        }
        return Some(found(i, end, Read::Date(Day::Weekday { weekday, which })));
    }
    let plural = lexicon
        .phrases(|pack| &pack.endings.plural)
        .iter()
        .any(|ending| {
            word.strip_suffix(ending)
                .is_some_and(|day| named(day).is_some())
        });
    if plural || (which.is_some() && lexicon.holds(|pack| &pack.days.periods, &word)) {
        return Some(Scan::Hide(end));
    }
    None
}

/// The day a weekday joins, one candidate: `friday the 14th`, `tuesday 21 march 2017`.
fn weekday_day(lexicon: &Lexicon, chars: &[char], i: usize, after: usize) -> Option<Scan> {
    let k = skip_space(chars, after);
    let the = phrase_in(chars, k, &lexicon.phrases(|pack| &pack.days.the));
    let k = the.map_or(k, |end| skip_space(chars, end));
    if the.is_none() && digit_run(chars, k).is_none() && ordinal_at(lexicon, chars, k).is_none() {
        return None;
    }
    let (day, dend) = day_number_at(lexicon, chars, k)?;
    if let Some(scan) = named_month_after(lexicon, chars, i, day, dend) {
        return Some(scan);
    }
    (ordinal_at(lexicon, chars, k).is_some() && followed_plainly(lexicon, chars, dend))
        .then(|| dated(i, dend, Day::nth(day)))
}

/// A month after a day, `21 march 2017`, `the 14th of march`: the calendar day, or nothing when none follows.
fn named_month_after(
    lexicon: &Lexicon,
    chars: &[char],
    i: usize,
    day: u8,
    dend: usize,
) -> Option<Scan> {
    let k = skip_space(chars, dend);
    let of = phrase_in(chars, k, &lexicon.phrases(|pack| &pack.days.of));
    let k = of.map_or(k, |end| skip_space(chars, end));
    let (month, mend) = month_at(lexicon, chars, k)?;
    if day > days_in(month, None) {
        return None;
    }
    Some(match year_at(lexicon, chars, mend) {
        Some((year, yend)) => dated(i, yend, Day::calendar(Some(year), month, day)),
        None => dated(i, mend, Day::calendar(None, month, day)),
    })
}

/// `in three days`, `in 2 weeks`, `three days from now`, `a week from today`: read as days ahead.
fn ahead(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let units = lexicon.named(|pack| &pack.days.units);
    let unit_at = |k: usize| {
        units.iter().find_map(|(unit, form)| {
            let end = phrase(chars, k, unit)?;
            Some((end, form.parse::<i32>().ok()?))
        })
    };
    if let Some(end) = phrase_in(chars, i, &lexicon.phrases(|pack| &pack.days.ahead))
        && chars.get(end) == Some(&' ')
        && let Some((count, cend)) = count_at(lexicon, chars, end + 1)
        && let Some((e, days)) = unit_at(skip_space(chars, cend))
    {
        return Some(dated(i, e, days_ahead(count, days)));
    }
    let (count, cend) = count_at(lexicon, chars, i)?;
    let (e, days) = unit_at(skip_space(chars, cend))?;
    let from_now = lexicon.phrases(|pack| &pack.days.from_now);
    let t = phrase_in(chars, skip_space(chars, e), &from_now)?;
    Some(dated(i, t, days_ahead(count, days)))
}

fn days_ahead(count: u64, each: i32) -> Option<Day> {
    let days = i32::try_from(count).ok()?.checked_mul(each)?;
    Day::offset(days)
}

/// A count: the number words the product reads, digits, or an article, `a`, `an`.
fn count_at(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(u64, usize)> {
    if let Some(Words { end, value }) = words(lexicon, chars, i) {
        return value.and_then(|value| whole(value).map(|count| (count, end)));
    }
    if let Some(found) = digits_at(chars, i) {
        return Some(found);
    }
    phrase_in(chars, i, &lexicon.phrases(|pack| &pack.numbers.article)).map(|end| (1, end))
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
fn month_first(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let (month, mend) = month_at(lexicon, chars, i)?;
    let mut k = skip_space(chars, mend);
    if let Some(end) = phrase_in(chars, k, &lexicon.phrases(|pack| &pack.days.the)) {
        k = skip_space(chars, end);
    }
    let Some((day, dend)) = day_number_at(lexicon, chars, k) else {
        return Some(Scan::Nothing);
    };
    let (year, end) = match year_at(lexicon, chars, dend) {
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
fn day_first(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let the = phrase_in(chars, i, &lexicon.phrases(|pack| &pack.days.the));
    let k = the.map_or(i, |end| skip_space(chars, end));
    let (day, dend) = day_number_at(lexicon, chars, k)?;
    if let Some(scan) = named_month_after(lexicon, chars, i, day, dend) {
        return Some(scan);
    }
    let plainly = ordinal_at(lexicon, chars, k).is_some()
        && followed_plainly(lexicon, chars, dend)
        && (the.is_some() || led_by_time(lexicon, chars, i));
    plainly.then(|| dated(i, dend, Day::nth(day)))
}

/// A day as an ordinal, a bare figure one to thirty-one that no colon or point continues, or a number word one
/// to thirty-one.
fn day_number_at(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(u8, usize)> {
    if let Some(ordinal) = ordinal_at(lexicon, chars, i) {
        return Some(ordinal);
    }
    if let Some((day, end)) = digits_at(chars, i)
        && (1..=31).contains(&day)
        && !(matches!(chars.get(end), Some(':' | '.'))
            && chars.get(end + 1).is_some_and(|c| text::figure(*c)))
    {
        return u8::try_from(day).ok().map(|day| (day, end));
    }
    small_words(lexicon, chars, i).filter(|(day, _)| (1..=31).contains(day))
}

/// `14th`, `1st`, `fourteenth`, `twenty-first`, `twenty first`: the day and where it ends.
fn ordinal_at(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(u8, usize)> {
    if let Some((day, end)) = digits_at(chars, i) {
        let mut suffixes: Vec<&str> = lexicon
            .packs()
            .iter()
            .flat_map(|pack| pack.ordinals.suffixes.all())
            .collect();
        suffixes.sort_by_key(|suffix| Reverse(suffix.chars().count()));
        // An ending of marks alone, «5.», is an ordinal's only where a word follows it.
        let after = suffixes.iter().find_map(|suffix| {
            lowered_end(chars, end, suffix).filter(|&at| {
                ends_word(chars, at)
                    && (suffix.chars().any(char::is_alphabetic)
                        || chars
                            .get(skip_space(chars, at))
                            .is_some_and(|c| c.is_alphanumeric()))
            })
        })?;
        return (1..=31)
            .contains(&day)
            .then(|| u8::try_from(day).ok().map(|day| (day, after)))
            .flatten();
    }
    let (word, end) = word_at(chars, i)?;
    let rank = |word: &str| {
        lexicon
            .value(|pack| &pack.ordinals.words, word)
            .and_then(|rank| u8::try_from(rank).ok())
    };
    if let Some(day) = rank(&word)
        && boundary(chars, i)
    {
        return Some((day, end));
    }
    if let Some(tens) = tens(lexicon, &word)
        && tens <= 30
        && matches!(chars.get(end), Some(' ' | '-'))
        && let Some((next, nend)) = word_at(chars, end + 1)
        && let Some(ones) = rank(&next)
        && (1..=9).contains(&ones)
    {
        return Some((tens + ones, nend));
    }
    None
}

/// A month word at `i`, whole, or a short form with an optional dot: `march`, `mar.`, `Sept`.
fn month_at(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(u8, usize)> {
    if !boundary(chars, i) {
        return None;
    }
    let (word, end) = word_at(chars, i)?;
    let (form, pack) = lexicon
        .packs()
        .iter()
        .find_map(|pack| pack.months.form_of(&word).map(|form| (form, *pack)))?;
    let month: u8 = form.parse().ok()?;
    let short = pack
        .months
        .shown(form)
        .is_none_or(|shown| text::fold(shown) != word);
    let end = if short && chars.get(end) == Some(&'.') {
        end + 1
    } else {
        end
    };
    Some((month, end))
}

/// A year after a day, joined by a comma or a space: `, 2020`, ` 2020`, ` twenty seventeen`, ` two thousand and
/// seventeen`; 1900 to 2100 in figures.
fn year_at(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(i32, usize)> {
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
    let head = lexicon
        .value(|pack| &pack.numbers.ones, &word)
        .or_else(|| lexicon.value(|pack| &pack.numbers.tens, &word));
    // A year said in two pairs, «twenty seventeen», «nineteen oh five», where the pack says years so.
    let pairs = lexicon
        .packs()
        .iter()
        .any(|pack| pack.numbers.years_in_pairs);
    if pairs
        && let Some(head @ (19 | 20)) = head
        && chars.get(wend) == Some(&' ')
    {
        let base = i32::try_from(head * 100).ok()?;
        let mut k = wend + 1;
        let oh = phrase_in(chars, k, &lexicon.phrases(|pack| &pack.numbers.oh));
        if let Some(end) = oh {
            k = skip_space(chars, end);
        }
        match small_words(lexicon, chars, k) {
            Some((rest, end)) if oh.is_none() || rest < 10 => {
                return Some((base + i32::from(rest), end));
            }
            None => {
                if let Some(end) = scale_at(lexicon, chars, k, 100) {
                    return Some((base, end));
                }
            }
            Some(_) => {}
        }
    }
    if head == Some(2)
        && let Some(thousand) = scale_at(lexicon, chars, skip_space(chars, wend), 1000)
    {
        let k = skip_space(chars, thousand);
        let and = phrase_in(chars, k, &lexicon.phrases(|pack| &pack.numbers.and));
        let k = and.map_or(k, |end| skip_space(chars, end));
        return Some(
            small_words(lexicon, chars, k).map_or((2000, thousand), |(rest, end)| {
                (2000 + i32::from(rest), end)
            }),
        );
    }
    None
}

/// Whether what follows `j` is the end, punctuation, or a word that is not a noun's place: `the 14th`, `the 14th
/// at three`, `the 14th is what day`; never `the first alarm`.
fn followed_plainly(lexicon: &Lexicon, chars: &[char], j: usize) -> bool {
    let k = skip_space(chars, j);
    match chars.get(k) {
        None => true,
        Some(c) if !c.is_alphabetic() => true,
        Some(_) => word_at(chars, k)
            .is_some_and(|(word, _)| lexicon.holds(|pack| &pack.days.followers, &word)),
    }
}

/// Whether the word before `i` is a preposition of time or a weekday: `on the 14th`, `friday the 14th`, never `the
/// 13th president`.
fn led_by_time(lexicon: &Lexicon, chars: &[char], i: usize) -> bool {
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
        .flat_map(|c| fold_char(*c))
        .collect();
    lexicon.holds(|pack| &pack.days.leads, &word)
        || lexicon.form_of(|pack| &pack.days.weekdays, &word).is_some()
}

// ---- a time ------------------------------------------------------------------------------------------------

/// The half of the day a clock time names.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Meridiem {
    Am,
    Pm,
}

/// A clock time with its half of the day, `5pm`, `5:30 pm`, `seven thirty am`, `six in the morning`, `5 o'clock
/// in the afternoon`, `noon`; or a two-digit hour on the 24-hour clock, `13:00`, `09:30`. A bare hour is a
/// number; `5:30`, `4 o'clock` and `half past five` with no half, and a time with seconds, are refused whole.
fn time(lexicon: &Lexicon, chars: &[char], i: usize) -> Scan {
    if !boundary(chars, i) {
        return Scan::Nothing;
    }
    named_time(lexicon, chars, i)
        .or_else(|| half_to(lexicon, chars, i))
        .or_else(|| past_or_to(lexicon, chars, i))
        .or_else(|| minutes_past_or_to(lexicon, chars, i))
        .or_else(|| digital(lexicon, chars, i))
        .or_else(|| spoken(lexicon, chars, i))
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

/// A time named, `noon`, `midnight`, as the pack lists it with its hour.
fn named_time(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    lexicon
        .named(|pack| &pack.times.named)
        .into_iter()
        .find_map(|(words, form)| {
            let end = phrase(chars, i, words)?;
            let clock: Clock = form.parse().ok()?;
            Some(timed(i, end, clock.hour(), clock.minute(), None))
        })
}

/// `half past five pm`, `a quarter to six am`; with no half, hidden.
fn past_or_to(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let past = lexicon
        .named(|pack| &pack.times.minutes_past)
        .into_iter()
        .map(|(lead, form)| (lead, form, true));
    let to = lexicon
        .named(|pack| &pack.times.minutes_to)
        .into_iter()
        .map(|(lead, form)| (lead, form, false));
    for (lead, form, past) in past.chain(to) {
        let (Some(end), Ok(minutes)) = (phrase(chars, i, lead), form.parse::<u8>()) else {
            continue;
        };
        let k = skip_space(chars, end);
        let Some((hour, hend)) = hour_word(lexicon, chars, k).or_else(|| {
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
        return Some(past_or_to_read(
            lexicon, chars, i, hour, hend, past, minutes,
        ));
    }
    None
}

/// The hour before or after the minutes named, with its half of the day; hidden whole when none follows.
fn past_or_to_read(
    lexicon: &Lexicon,
    chars: &[char],
    i: usize,
    hour: u8,
    hend: usize,
    past: bool,
    minutes: u8,
) -> Scan {
    let Some((meridiem, mend)) = meridiem_at(lexicon, chars, hend) else {
        return Scan::Hide(hend);
    };
    let (hour, minute) = if past {
        (hour, minutes)
    } else {
        (if hour > 1 { hour - 1 } else { 12 }, 60 - minutes)
    };
    timed(i, mend, hour, minute, Some(meridiem))
}

/// `ten past nine pm`, `twenty five to six am`: the minutes, the word that says past or to between two spaces,
/// the hour.
fn minutes_past_or_to(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let Words {
        end: mend,
        value: Some(minutes),
    } = words(lexicon, chars, i)?
    else {
        return None;
    };
    let minutes = whole(minutes).filter(|minutes| (1..=59).contains(minutes))?;
    if chars.get(mend) != Some(&' ') {
        return None;
    }
    for (leads, past) in [
        (lexicon.phrases(|pack| &pack.times.past), true),
        (lexicon.phrases(|pack| &pack.times.to), false),
    ] {
        let Some(after) = leads
            .iter()
            .find_map(|lead| lowered_end(chars, mend + 1, lead))
            .filter(|&after| chars.get(after) == Some(&' '))
        else {
            continue;
        };
        let Some((hour, hend)) = hour_word(lexicon, chars, after + 1) else {
            continue;
        };
        return Some(past_or_to_read(
            lexicon,
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
fn digital(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
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
            || meridiem_at(lexicon, chars, j + 3).is_some()
            || oclock_at(lexicon, chars, j + 3).is_some())
    {
        minute = Some(read);
        j += 3;
        if chars.get(j) == Some(&':') && digits_follow(chars, j + 1) {
            return Some(Scan::Hide((j + 3).min(chars.len())));
        }
    }
    let mut meridiem = meridiem_at(lexicon, chars, j);
    // After the minutes, each half's first letter alone says it: «5:30a», «5:30p».
    if meridiem.is_none()
        && minute.is_some()
        && let Some(c) = chars.get(j)
        && let Some((a, p)) = half_letters(lexicon)
        && let Some(half) = c.to_lowercase().next().and_then(|c| {
            (c == a)
                .then_some(Meridiem::Am)
                .or((c == p).then_some(Meridiem::Pm))
        })
        && ends_word(chars, j + 1)
    {
        meridiem = Some((half, j + 1));
    }
    if let Some((half, mend)) = meridiem
        && hour <= 12
        && minute.is_none_or(|minute| minute < 60)
    {
        return Some(timed(i, mend, hour, minute.unwrap_or(0), Some(half)));
    }
    if let Some(oc) = oclock_at(lexicon, chars, j)
        && minute.is_none()
    {
        if let Some((half, mend)) = meridiem_at(lexicon, chars, oc)
            && hour <= 12
        {
            return Some(timed(i, mend, hour, 0, Some(half)));
        }
        // An o'clock word the pack also lists as a unit of time, «heures», «h», is the clock's only where the
        // form is the clock's for sure: the word attached to the figure, «9h», an hour past twelve, or a word
        // for «at» before the hour; else it is the unit's, and the figure a duration.
        let attached = skip_space(chars, j) == j;
        if units_too(lexicon, chars, j, oc)
            && !attached
            && hour < 13
            && !at_before(lexicon, chars, i)
        {
            return None;
        }
        if (two || lexicon.hour_cycle() == 24) && hour <= 23 {
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
    if !two_or_more_digits(chars, at, 2) || chars.get(at + 2).is_some_and(|c| text::figure(*c)) {
        return None;
    }
    chars[at..at + 2].iter().collect::<String>().parse().ok()
}

/// Whether what stands at `at` up to two characters on is digits: a seconds part, `:17`, refused with its time.
fn digits_follow(chars: &[char], at: usize) -> bool {
    match (chars.get(at), chars.get(at + 1)) {
        (Some(a), Some(b)) => text::figure(*a) && text::figure(*b),
        (Some(a), None) => text::figure(*a),
        _ => false,
    }
}

/// `five pm`, `seven thirty am`, `six oh five pm`, `eight hundred am`, `six o'clock in the evening`, `three in
/// the afternoon`.
fn spoken(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let (hour, j) = hour_word(lexicon, chars, i)?;
    if hour < 1 {
        return None;
    }
    let mut minute = 0;
    let mut k = j;
    if matches!(chars.get(k), Some(' ' | '-')) {
        if let Some(hundred) = scale_at(lexicon, chars, k + 1, 100) {
            return Some(match meridiem_at(lexicon, chars, hundred) {
                Some((half, mend)) => timed(i, mend, hour, 0, Some(half)),
                None => Scan::Nothing,
            });
        }
        if let Some((read, mend)) = minute_words(lexicon, chars, k + 1)
            && read < 60
        {
            minute = read;
            k = mend;
        }
    }
    if let Some((half, mend)) = meridiem_at(lexicon, chars, k) {
        return Some(timed(i, mend, hour, minute, Some(half)));
    }
    let oc = oclock_at(lexicon, chars, k)?;
    // «deux heures» is two hours, not two o'clock, unless a word for «at» stands before it.
    if units_too(lexicon, chars, k, oc) && hour < 13 && !at_before(lexicon, chars, i) {
        return None;
    }
    Some(match meridiem_at(lexicon, chars, oc) {
        Some((half, mend)) => timed(i, mend, hour, minute, Some(half)),
        None if lexicon.hour_cycle() == 24 => timed(i, oc, hour, minute, None),
        None => Scan::Nothing,
    })
}

/// «halb drei», half an hour before the hour named, where the pack says it so: on the 24-hour clock the hour as
/// said, else hidden whole, since its half of the day is not said.
fn half_to(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Scan> {
    let end = phrase_in(chars, i, &lexicon.phrases(|pack| &pack.times.half_to))?;
    let k = skip_space(chars, end);
    let (hour, hend) = hour_word(lexicon, chars, k).or_else(|| {
        digits_at(chars, k).and_then(|(hour, end)| Some((u8::try_from(hour).ok()?, end)))
    })?;
    if !(1..=12).contains(&hour) {
        return None;
    }
    if lexicon.hour_cycle() == 24 {
        return Some(timed(i, hend, hour - 1, 30, None));
    }
    Some(Scan::Hide(hend))
}

/// A half of the day as the packs write it, in any case, after one space at most: `am`, `pm`, `a.m.`, `p.m.`,
/// `in the morning`, `at night`; the longest form first, so `a.m.` keeps its last point.
fn meridiem_at(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(Meridiem, usize)> {
    let j = skip_space(chars, i);
    let am = lexicon
        .phrases(|pack| &pack.times.am)
        .into_iter()
        .map(|form| (form, Meridiem::Am));
    let pm = lexicon
        .phrases(|pack| &pack.times.pm)
        .into_iter()
        .map(|form| (form, Meridiem::Pm));
    let mut forms: Vec<(&str, Meridiem)> = am.chain(pm).collect();
    forms.sort_by_key(|(form, _)| Reverse(form.chars().count()));
    forms
        .into_iter()
        .find_map(|(form, half)| phrase_end(chars, j, form).map(|end| (half, end)))
}

/// The letter each half of the day is written as alone after the minutes: the first letter of its shortest word.
fn half_letters(lexicon: &Lexicon) -> Option<(char, char)> {
    let first = |table: fn(&'static Pack) -> &'static pack::Phrases| {
        lexicon.phrases(table).last()?.chars().next()
    };
    Some((first(|pack| &pack.times.am)?, first(|pack| &pack.times.pm)?))
}

/// `o'clock` in its spellings, after one space at most.
fn oclock_at(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<usize> {
    let j = skip_space(chars, i);
    phrase_in(chars, j, &lexicon.phrases(|pack| &pack.times.oclock))
}

/// Whether the o'clock word found from `i` to `end` is also a unit of time the packs list, «heures», «h».
fn units_too(lexicon: &Lexicon, chars: &[char], i: usize, end: usize) -> bool {
    let word: String = chars[skip_space(chars, i)..end]
        .iter()
        .flat_map(|c| fold_char(*c))
        .collect();
    lexicon
        .form_of(|pack| &pack.durations.units, &word)
        .is_some()
}

/// Whether a word for «at» stands right before `i`, «à 9 heures», «a las 2 horas»: the clock's, not a length.
fn at_before(lexicon: &Lexicon, chars: &[char], i: usize) -> bool {
    let mut end = i;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return false;
    }
    let before: String = chars[..end].iter().flat_map(|c| fold_char(*c)).collect();
    lexicon
        .phrases(|pack| &pack.times.at)
        .into_iter()
        .any(|at| {
            before
                .strip_suffix(at)
                .is_some_and(|head| head.chars().next_back().is_none_or(|c| !is_word(c)))
        })
}

/// An hour word, `one` to `twelve`, at a word boundary.
fn hour_word(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(u8, usize)> {
    let (word, end) = word_at(chars, i)?;
    let hour = ones(lexicon, &word)?;
    (hour <= 12 && boundary(chars, i)).then_some((hour, end))
}

/// `thirty`, `fifteen`, `forty five`, `oh five`: minutes after an hour word.
fn minute_words(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(u8, usize)> {
    let (word, end) = word_at(chars, i)?;
    if lexicon.holds(|pack| &pack.numbers.oh, &word)
        && let Some((next, nend)) = word_at(chars, skip_space(chars, end))
        && let Some(minute) = ones(lexicon, &next)
        && minute < 10
    {
        return Some((minute, nend));
    }
    if let Some(tens) = tens(lexicon, &word) {
        if matches!(chars.get(end), Some(' ' | '-'))
            && let Some((next, nend)) = word_at(chars, end + 1)
            && let Some(ones) = ones(lexicon, &next)
            && (1..=9).contains(&ones)
        {
            return Some((tens + ones, nend));
        }
        return Some((tens, end));
    }
    ones(lexicon, &word)
        .filter(|minute| (10..=19).contains(minute))
        .map(|minute| (minute, end))
}

/// At most a tens word and a ones word, `twenty four`, `seven`: as a day or a year reads them, never a longer run.
fn small_words(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(u8, usize)> {
    if !boundary(chars, i) {
        return None;
    }
    let (token, end) = token_at(lexicon, chars, i)?;
    let small = |value: u32, end: usize| u8::try_from(value).ok().map(|value| (value, end));
    match token {
        Token::Tens(tens) => {
            // «twenty one», «vingt-deux», «vingt et un»; «soixante-douze» where the pack adds a teen.
            let adds = |ones: u32| {
                (1..=9).contains(&ones) || (lexicon.teens_after_tens() && (10..=19).contains(&ones))
            };
            if matches!(chars.get(end), Some(' ' | '-'))
                && let Some((next, nend)) = token_at(lexicon, chars, end + 1)
            {
                match next {
                    Token::Ones(ones) if adds(ones) => return small(tens + ones, nend),
                    Token::And if chars.get(nend) == Some(&' ') => {
                        if let Some((Token::Ones(ones), oend)) = token_at(lexicon, chars, nend + 1)
                            && (1..=9).contains(&ones)
                        {
                            return small(tens + ones, oend);
                        }
                    }
                    _ => {}
                }
            }
            small(tens, end)
        }
        Token::Ones(ones) => small(ones, end),
        Token::Fused(value) if value < 100 => u8::try_from(value).ok().map(|value| (value, end)),
        _ => None,
    }
}

/// A number word to nineteen, as a small number reads it.
fn ones(lexicon: &Lexicon, word: &str) -> Option<u8> {
    lexicon
        .value(|pack| &pack.numbers.ones, word)
        .and_then(|value| u8::try_from(value).ok())
}

/// A tens word, as a small number reads it.
fn tens(lexicon: &Lexicon, word: &str) -> Option<u8> {
    lexicon
        .value(|pack| &pack.numbers.tens, word)
        .and_then(|value| u8::try_from(value).ok())
}

/// A scale word of `value`, «hundred», «thousand», whole at `at`: where it ends.
fn scale_at(lexicon: &Lexicon, chars: &[char], at: usize, value: i64) -> Option<usize> {
    lexicon
        .packs()
        .iter()
        .flat_map(|pack| pack.numbers.scale.iter())
        .filter(|(_, scale)| *scale == value)
        .find_map(|(word, _)| phrase(chars, at, word))
}

/// The first of the phrases that stands whole at `at`, at a word boundary: where it ends.
fn phrase_in(chars: &[char], at: usize, phrases: &[&str]) -> Option<usize> {
    phrases.iter().find_map(|words| phrase(chars, at, words))
}

// ---- a duration and a number, as before -------------------------------------------------------------------

/// What stands before a unit: the count, where it ends, and whether a one-letter unit — `10m` — may follow it,
/// which it may after a number and never after an article; a run of number words to pass over; or nothing.
enum Before {
    Read(f64, usize, bool),
    Skip(usize),
    Nothing,
}

/// The count at `i`: a number in words, an article's — «an hour», «half a minute» — or a number in digits.
fn before_unit(lexicon: &Lexicon, chars: &[char], i: usize) -> Before {
    match words(lexicon, chars, i) {
        Some(Words {
            end,
            value: Some(value),
        }) => return Before::Read(value, end, true),
        Some(Words { end, value: None }) => return Before::Skip(end),
        None => {}
    }
    if let Some((value, end)) = article(lexicon, chars, i) {
        return Before::Read(value, end, false);
    }
    // A minus the number's own — `-5 minutes` — makes no duration: the number stands alone, negative.
    if i > 0 && chars[i - 1] == '-' && (i == 1 || !is_word(chars[i - 2])) {
        return Before::Nothing;
    }
    match decimal(lexicon, chars, i) {
        Some((value, end)) => Before::Read(value, end, true),
        None => Before::Nothing,
    }
}

fn duration(lexicon: &Lexicon, chars: &[char], i: usize) -> Scan {
    let (count, after, letters) = match before_unit(lexicon, chars, i) {
        Before::Read(value, end, letters) => (value, end, letters),
        Before::Skip(end) => return Scan::Skip(end),
        Before::Nothing => return Scan::Nothing,
    };
    // «and a half» adds a half, after the count — «two and a half hours» — or after the unit — «an hour and a
    // half».
    let (count, after, halved) = match half_after(lexicon, chars, after) {
        Some(end) => (count + 0.5, end, true),
        None => (count, after, false),
    };
    let Some((end, seconds_each)) = unit(lexicon, chars, after_space(chars, after), letters) else {
        return Scan::Nothing;
    };
    let (count, end) = match half_after(lexicon, chars, end) {
        Some(end) if !halved => (count + 0.5, end),
        _ => (count, end),
    };
    let Some(seconds) = seconds(count * seconds_each) else {
        return Scan::Nothing;
    };
    found(i, end, Read::Seconds(seconds))
}

fn number(lexicon: &Lexicon, chars: &[char], i: usize) -> Scan {
    let (value, after) = match words(lexicon, chars, i) {
        Some(Words {
            end,
            value: Some(value),
        }) => (value, end),
        Some(Words { end, value: None }) => return Scan::Skip(end),
        None => {
            // A minus is the number's when nothing wordlike stands before it and a digit follows: `-5`, never `5-10`.
            let signed = chars[i] == '-'
                && chars.get(i + 1).is_some_and(|c| text::figure(*c))
                && (i == 0 || !is_word(chars[i - 1]));
            let Some((value, after)) = decimal(lexicon, chars, if signed { i + 1 } else { i })
            else {
                return Scan::Nothing;
            };
            (if signed { -value } else { value }, after)
        }
    };
    let unit_at = after_space(chars, after);
    let percent = lexicon
        .phrases(|pack| &pack.numbers.percent)
        .into_iter()
        .find_map(|word| unit_end(chars, unit_at, word));
    let end = if let Some(end) = percent {
        end
    } else if chars.get(unit_at) == Some(&'%') {
        unit_at + 1
    } else {
        after
    };
    found(i, end, Read::Number(value))
}

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
    /// A number said as one word that is no single word of the tables, as a pack that writes a number's words as
    /// one has it: «einundzwanzig», «zweihundert».
    Fused(u64),
    /// A hundreds word of its own, «doscientos»: a count before «hundred» in one word.
    Hundreds(u32),
}

/// A word as a token of a run of number words, as the packs list the words: the ones, the tens, the scale words
/// — a hundred, a thousand, and the words beyond them — the article, and the «and».
fn token(lexicon: &Lexicon, word: &str) -> Option<Token> {
    if let Some(value) = lexicon.value(|pack| &pack.numbers.ones, word) {
        return u32::try_from(value).ok().map(Token::Ones);
    }
    if let Some(value) = lexicon.value(|pack| &pack.numbers.tens, word) {
        return u32::try_from(value).ok().map(Token::Tens);
    }
    match lexicon.value(|pack| &pack.numbers.scale, word) {
        Some(100) => return Some(Token::Hundred),
        Some(1000) => return Some(Token::Thousand),
        Some(value) if (200..=900).contains(&value) && value % 100 == 0 => {
            return u32::try_from(value / 100).ok().map(Token::Hundreds);
        }
        Some(_) => return Some(Token::Beyond),
        None => {}
    }
    if lexicon.holds(|pack| &pack.numbers.beyond, word) {
        return Some(Token::Beyond);
    }
    if lexicon.holds(|pack| &pack.numbers.article, word) {
        return Some(Token::A);
    }
    if lexicon.holds(|pack| &pack.numbers.and, word) {
        return Some(Token::And);
    }
    lexicon.number(word).map(Token::Fused)
}

/// The word at `at`, in any letter case, as a token, with where it ends: a number word the tables write with a
/// hyphen or a space inside, «soixante-dix», «dix-sept», whole and the longest first, else the one word there.
fn token_at(lexicon: &Lexicon, chars: &[char], at: usize) -> Option<(Token, usize)> {
    let mut joined: Vec<&str> = lexicon
        .packs()
        .iter()
        .flat_map(|pack| {
            pack.numbers
                .ones
                .iter()
                .chain(pack.numbers.tens.iter())
                .chain(pack.numbers.scale.iter())
                .map(|(word, _)| word)
        })
        .filter(|word| word.contains(['-', ' ']))
        .collect();
    joined.sort_by_key(|word| std::cmp::Reverse(word.chars().count()));
    for word in joined {
        if let Some(end) = phrase(chars, at, word)
            && !chars.get(end).is_some_and(|&c| is_word(c))
            && let Some(token) = token(lexicon, word)
        {
            return Some((token, end));
        }
    }
    let (word, end) = word_at(chars, at)?;
    token(lexicon, &word).map(|token| (token, end))
}

/// Whether one space or one hyphen at `at` is followed by a word that `fits`.
fn follows(lexicon: &Lexicon, chars: &[char], at: usize, fits: impl Fn(Token) -> bool) -> bool {
    matches!(chars.get(at), Some(' ' | '-'))
        && token_at(lexicon, chars, at + 1).is_some_and(|(token, _)| fits(token))
}

/// A run of number words: where it ends, and its value when it is a form that is read.
struct Words {
    end: usize,
    value: Option<f64>,
}

/// The run of number words at `i` — the words to nineteen, the tens, «hundred» and the words beyond it, «a»
/// before those and «and» after them — joined by one space or one hyphen, with a word boundary at each end:
/// «tenant» and «one-off» hold none. It reads as one word to ninety, a tens word joined to a word from one to
/// nine, and hundreds with what follows them, «a hundred», «two hundred ninety four», «a hundred and fifty»;
/// any other run — «seven thirty», «two thousand» — reads as nothing, whole, so no part of it is a candidate.
fn words(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<Words> {
    let (tokens, end) = run(lexicon, chars, i)?;
    Some(Words {
        end,
        value: hundreds_or_small(&tokens, lexicon.teens_after_tens()),
    })
}

/// The run's tokens and where it ends, whatever they add up to.
fn run(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(Vec<Token>, usize)> {
    if i > 0 && (is_word(chars[i - 1]) || i >= 2 && hyphen_binds(chars, i - 1, i - 2)) {
        return None;
    }
    let mut tokens: Vec<Token> = Vec::new();
    let mut end = i;
    let mut at = i;
    while let Some((token, word_end)) = token_at(lexicon, chars, at) {
        let fits = match token {
            Token::A => {
                tokens.is_empty()
                    && follows(lexicon, chars, word_end, |next| {
                        matches!(next, Token::Hundred | Token::Thousand | Token::Beyond)
                    })
            }
            Token::And => {
                // «a hundred and fifty»; and the joiner between a tens word and a ones word where a pack writes
                // the two apart, «vingt et un», «treinta y uno».
                (matches!(
                    tokens.last(),
                    Some(Token::Hundred | Token::Thousand | Token::Beyond)
                ) && follows(lexicon, chars, word_end, |next| {
                    matches!(next, Token::Ones(_) | Token::Tens(_))
                })) || (matches!(tokens.last(), Some(Token::Tens(_)))
                    && follows(lexicon, chars, word_end, |next| {
                        matches!(next, Token::Ones(1..=9))
                    }))
            }
            _ => true,
        };
        if !fits {
            break;
        }
        match token {
            Token::Hundreds(count) => tokens.extend([Token::Ones(count), Token::Hundred]),
            _ => tokens.push(token),
        }
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

/// An article's count at `i`, at a word boundary and in any letter case, as the pack lists the words that stand
/// for a count before a unit — «an», «half an», «a quarter of an»: what it stands for, and where it ends.
fn article(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(f64, usize)> {
    if i > 0 && is_word(chars[i - 1]) {
        return None;
    }
    lexicon
        .named(|pack| &pack.durations.articles)
        .into_iter()
        .find_map(|(words, form)| {
            let end = phrase_end(chars, i, words)?;
            Some((form.parse::<f64>().ok()?, end))
        })
}

/// Where «and a half» ends when it follows what ends at `at`.
fn half_after(lexicon: &Lexicon, chars: &[char], at: usize) -> Option<usize> {
    let after = after_space(chars, at);
    lexicon
        .phrases(|pack| &pack.numbers.and_a_half)
        .into_iter()
        .find_map(|words| phrase_end(chars, after, words))
}

/// The unit at `at` and its seconds, the longest form first, in any letter case; a one-letter form, `10m`, only
/// after a number and as typed, since a capital letter is a code's, «2 H K L».
fn unit(lexicon: &Lexicon, chars: &[char], at: usize, letters: bool) -> Option<(usize, f64)> {
    lexicon
        .named(|pack| &pack.durations.units)
        .into_iter()
        .filter(|(unit, _)| letters || unit.chars().count() > 1)
        .find_map(|(unit, form)| {
            let end = if unit.chars().count() == 1 {
                let end = at + 1;
                (starts_with(chars, at, unit) && !chars.get(end).is_some_and(|&c| is_word(c)))
                    .then_some(end)?
            } else {
                unit_end(chars, at, unit)?
            };
            Some((end, form.parse::<f64>().ok()?))
        })
}

/// `\b\d+(\.\d+)?` at `i`: the number and where it ends; digits past what a number holds are no candidate, and
/// neither are the digits after a digit and a comma or a point, `000` in `1,000` and `4` in `0.4`.
fn decimal(lexicon: &Lexicon, chars: &[char], i: usize) -> Option<(f64, usize)> {
    let inside = i >= 2 && matches!(chars[i - 1], ',' | '.') && text::figure(chars[i - 2]);
    if !text::figure(chars[i]) || (i > 0 && is_word(chars[i - 1])) || inside {
        return None;
    }
    let digits = |from: usize| {
        from + chars[from..]
            .iter()
            .take_while(|c| text::figure(**c))
            .count()
    };
    let mark = lexicon.decimal_mark().unwrap_or('.');
    let mut end = digits(i);
    let mut text: String = chars[i..end].iter().collect();
    if chars.get(end) == Some(&mark) && chars.get(end + 1).is_some_and(|c| text::figure(*c)) {
        let fraction = digits(end + 1);
        text.push('.');
        text.extend(&chars[end + 1..fraction]);
        end = fraction;
    }
    let value: f64 = text.parse().ok()?;
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

/// Where `unit`, lower case, ends when it stands at `at` in any letter case, and no word continues it.
fn unit_end(chars: &[char], at: usize, unit: &str) -> Option<usize> {
    lowered_end(chars, at, unit).filter(|&end| !chars.get(end).is_some_and(|&c| is_word(c)))
}

/// Where `text`, words in any letter case, ends when it stands at `at` and no word continues it.
fn phrase_end(chars: &[char], at: usize, text: &str) -> Option<usize> {
    lowered_end(chars, at, text).filter(|&end| !chars.get(end).is_some_and(|&c| is_word(c)))
}

/// Where `text` ends when it stands at `at` at a word boundary, in any letter case, and no word continues it.
fn phrase(chars: &[char], at: usize, text: &str) -> Option<usize> {
    if !boundary(chars, at) {
        return None;
    }
    phrase_end(chars, at, text)
}

/// Where `text`, lower case, ends when it stands at `at`, the input's letters folded as they are read
/// (`text::fold`); none where it does not stand there.
fn lowered_end(chars: &[char], at: usize, text: &str) -> Option<usize> {
    let mut want = text.chars().peekable();
    let mut i = at;
    while want.peek().is_some() {
        let c = *chars.get(i)?;
        for folded in fold_char(c) {
            if want.next() != Some(folded) {
                return None;
            }
        }
        i += 1;
    }
    Some(i)
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
            chars[i..end].iter().flat_map(|c| fold_char(*c)).collect(),
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
            .take_while(|c| text::figure(**c))
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
    use crate::calendar::{Weekday, Which};

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
        // A unit reads in any letter case.
        assert_eq!(spans("One Hundred Percent"), [(0, 19, number(100.0))]);
        assert_eq!(spans("one hundred percent"), [(0, 19, number(100.0))]);
        assert_eq!(
            spans("two hundred, a hundred and fifty or three hundred forty seven"),
            [
                (0, 11, number(200.0)),
                (13, 32, number(150.0)),
                (36, 61, number(347.0))
            ]
        );
        assert!(spans("seven thirty, a thousand and two million").is_empty());
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
        none("at six thirty, 4 o'clock and half past five");
        assert_eq!(kinds("at eight hundred"), ["number eight hundred"]);
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
