//! The five recognizers over an input, and masking. In: `Input`. Out: `Vec<Proposed>`, verbatim spans with typed
//! values, in order of position: quotes hide what they enclose, a duration hides its number. A number and a
//! duration read spelled out as they read in digits.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::text::{Clean, Input, Span};

/// A candidate for a pick: where it is in the input, and what its recognizer read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proposed {
    pub span: Span,
    pub value: PickValue,
}

/// What a pick hands the body: the number, the seconds, or the text.
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
}

impl PickValue {
    /// `number` is bare: any typed span covering it hides it, and it never caps the outcome.
    #[must_use]
    pub fn is_typed(&self) -> bool {
        !matches!(self, Self::Number { .. })
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
                    i = after;
                    continue;
                }
                Scan::Nothing => {
                    i += 1;
                    continue;
                }
            };
            if !hidden.iter().any(|range| range.contains(&i)) {
                // A typed match hides what it covers whether or not its text can be a span.
                if !matches!(read, Read::Number(_)) {
                    hidden.push(i..after);
                }
                if let Some(span) = Span::of(input, span.start, span.end) {
                    let value = read.value(span.text());
                    found.push(Proposed { span, value });
                }
            }
            i = after;
        }
    }
    found.sort_by_key(|proposed| proposed.span.start());
    found
}

/// What a recognizer finds at one position: a candidate; a run of number words it reads as nothing, passed over
/// whole so that no part of it is a candidate; or nothing.
enum Scan {
    Found(Found),
    Skip(usize),
    Nothing,
}

/// A recognizer's match at one position: the candidate inside it, where scanning resumes, and what it reads as.
struct Found {
    span: Range<usize>,
    after: usize,
    read: Read,
}

/// What a candidate reads as; the text kinds take the span itself.
enum Read {
    Number(f64),
    Seconds(u64),
    Email,
    Url,
    Quoted,
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
        }
    }
}

/// What one kind finds at a position of the input.
type Recognizer = fn(&[char], usize) -> Scan;

/// In this order: an earlier kind's match hides the candidates that start inside it.
const RECOGNIZERS: [Recognizer; 5] = [quoted, url, email, duration, number];

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
    Scan::Found(Found {
        span: i..end,
        after: end,
        read: Read::Url,
    })
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
    Scan::Found(Found {
        span: i..end,
        after: end,
        read: Read::Email,
    })
}

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

/// What stands before a unit: the amount, where it ends, and whether a one-letter unit — `10m` — may follow it,
/// which it may after a number and never after an article; a run of number words to pass over; or nothing.
enum Amount {
    Read(f64, usize, bool),
    Skip(usize),
    Nothing,
}

/// The amount at `i`: a number in words, an article's — «an hour», «half a minute» — or a number in digits.
fn amount(chars: &[char], i: usize) -> Amount {
    match words(chars, i) {
        Some(Words {
            end,
            value: Some(value),
        }) => return Amount::Read(value, end, true),
        Some(Words { end, value: None }) => return Amount::Skip(end),
        None => {}
    }
    if let Some((value, end)) = article(chars, i) {
        return Amount::Read(value, end, false);
    }
    // A minus the number's own — `-5 minutes` — makes no duration: the number stands alone, negative.
    if i > 0 && chars[i - 1] == '-' && (i == 1 || !is_word(chars[i - 2])) {
        return Amount::Nothing;
    }
    match decimal(chars, i) {
        Some((value, end)) => Amount::Read(value, end, true),
        None => Amount::Nothing,
    }
}

fn duration(chars: &[char], i: usize) -> Scan {
    let (amount, after, letters) = match amount(chars, i) {
        Amount::Read(value, end, letters) => (value, end, letters),
        Amount::Skip(end) => return Scan::Skip(end),
        Amount::Nothing => return Scan::Nothing,
    };
    // «and a half» adds a half, after the amount — «two and a half hours» — or after the unit — «an hour and a
    // half».
    let (amount, after, halved) = match half_after(chars, after) {
        Some(end) => (amount + 0.5, end, true),
        None => (amount, after, false),
    };
    let Some((end, seconds_each)) = unit(chars, after_space(chars, after), letters) else {
        return Scan::Nothing;
    };
    let (amount, end) = match half_after(chars, end) {
        Some(end) if !halved => (amount + 0.5, end),
        _ => (amount, end),
    };
    let Some(seconds) = seconds(amount * seconds_each) else {
        return Scan::Nothing;
    };
    Scan::Found(Found {
        span: i..end,
        after: end,
        read: Read::Seconds(seconds),
    })
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
    Scan::Found(Found {
        span: i..end,
        after: end,
        read: Read::Number(value),
    })
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
    /// «thousand», «million», «billion»: past what is read, so a run holding one reads as nothing.
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
        "thousand" | "million" | "billion" => Some(Token::Beyond),
        "a" => Some(Token::A),
        "and" => Some(Token::And),
        _ => None,
    }
}

/// The word at `at`, in any letter case, as a token, with where it ends.
fn token_at(chars: &[char], at: usize) -> Option<(Token, usize)> {
    let end = at
        + chars
            .get(at..)?
            .iter()
            .take_while(|c| c.is_alphabetic())
            .count();
    let word: String = chars[at..end]
        .iter()
        .flat_map(|c| c.to_lowercase())
        .collect();
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
                        matches!(next, Token::Hundred | Token::Beyond)
                    })
            }
            Token::And => {
                matches!(tokens.last(), Some(Token::Hundred | Token::Beyond))
                    && follows(chars, word_end, |next| {
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

/// Whether a hyphen at `at` binds the word beside it to a word at `other`, as in «one-off».
fn hyphen_binds(chars: &[char], at: usize, other: usize) -> bool {
    chars.get(at) == Some(&'-') && chars.get(other).is_some_and(|c| c.is_alphabetic())
}

/// An article's amount at `i`, at a word boundary and in any letter case: what it stands for, and where it ends.
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

/// Whole seconds, to the nearest; an amount past what seconds can hold is no candidate, and neither is one that
/// rounds to nothing, `0.4 seconds`; a `0` said outright stays.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn seconds(amount: f64) -> Option<u64> {
    if amount > 0.0 && amount < 0.5 {
        return None;
    }
    let whole = amount.round();
    (whole < u64::MAX as f64).then_some(whole as u64)
}

/// One space may separate an amount from its unit.
fn after_space(chars: &[char], at: usize) -> usize {
    if chars.get(at).is_some_and(|c| c.is_whitespace()) {
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
    let stands = text.chars().enumerate().all(|(k, c)| {
        chars
            .get(at + k)
            .is_some_and(|d| d.to_ascii_lowercase() == c)
    });
    (stands && !chars.get(end).is_some_and(|&c| is_word(c))).then_some(end)
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
}
