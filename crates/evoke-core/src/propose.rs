//! The five recognizers over an input, and masking. In: `Input`. Out: `Vec<Proposed>`, verbatim spans with typed
//! values, in order of position: quotes hide what they enclose, a duration hides its number.

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
            let Some(Found { span, after, read }) = recognize(&chars, i) else {
                i += 1;
                continue;
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
type Recognizer = fn(&[char], usize) -> Option<Found>;

/// In the spike's order: an earlier kind's match hides the candidates that start inside it.
const RECOGNIZERS: [Recognizer; 5] = [quoted, url, email, duration, number];

fn quoted(chars: &[char], i: usize) -> Option<Found> {
    let close = match chars[i] {
        '"' => '"',
        '\u{201c}' => '\u{201d}',
        '\u{2018}' => '\u{2019}',
        _ => return None,
    };
    let open = chars[i];
    let mut j = i + 1;
    while j < chars.len() && chars[j] != open && chars[j] != close {
        j += 1;
    }
    if j == i + 1 || j == chars.len() || chars[j] != close {
        return None;
    }
    Some(Found {
        span: i + 1..j,
        after: j + 1,
        read: Read::Quoted,
    })
}

fn url(chars: &[char], i: usize) -> Option<Found> {
    let scheme = ["https://", "http://"]
        .into_iter()
        .find(|scheme| starts_with(chars, i, scheme))?;
    let inner = |c: char| !(c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\''));
    let mut j = i + scheme.len();
    while j < chars.len() && inner(chars[j]) {
        j += 1;
    }
    let end = (i + scheme.len() + 2..=j)
        .rev()
        .find(|&end| !matches!(chars[end - 1], '.' | ',' | ';' | ':' | '!' | '?' | ')'))?;
    Some(Found {
        span: i..end,
        after: end,
        read: Read::Url,
    })
}

fn email(chars: &[char], i: usize) -> Option<Found> {
    let local = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-');
    let domain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-');
    let mut at = i;
    while at < chars.len() && local(chars[at]) {
        at += 1;
    }
    if at == i || chars.get(at) != Some(&'@') {
        return None;
    }
    let mut j = at + 1;
    while j < chars.len() && domain(chars[j]) {
        j += 1;
    }
    let end = (at + 2..j)
        .rev()
        .filter(|&dot| chars[dot] == '.')
        .find_map(|dot| {
            let letters = chars[dot + 1..j]
                .iter()
                .take_while(|c| c.is_ascii_alphabetic())
                .count();
            (letters >= 2).then_some(dot + 1 + letters)
        })?;
    Some(Found {
        span: i..end,
        after: end,
        read: Read::Email,
    })
}

/// Unit forms in the order the spike's expression tries them, with their seconds.
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

fn duration(chars: &[char], i: usize) -> Option<Found> {
    let (amount, after) = decimal(chars, i)?;
    let unit_at = after_space(chars, after);
    let (end, seconds_each) = UNITS
        .into_iter()
        .find_map(|(unit, seconds)| unit_end(chars, unit_at, unit).map(|end| (end, seconds)))?;
    Some(Found {
        span: i..end,
        after: end,
        read: Read::Seconds(seconds(amount * seconds_each)?),
    })
}

fn number(chars: &[char], i: usize) -> Option<Found> {
    let (value, after) = decimal(chars, i)?;
    let unit_at = after_space(chars, after);
    let end = if let Some(end) = unit_end(chars, unit_at, "percent") {
        end
    } else if chars.get(unit_at) == Some(&'%') {
        unit_at + 1
    } else {
        after
    };
    Some(Found {
        span: i..end,
        after: end,
        read: Read::Number(value),
    })
}

/// `\b\d+(\.\d+)?` at `i`: the number and where it ends; digits past what a number holds are no candidate.
fn decimal(chars: &[char], i: usize) -> Option<(f64, usize)> {
    if !chars[i].is_ascii_digit() || (i > 0 && is_word(chars[i - 1])) {
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

/// Whole seconds, to the nearest; an amount past what seconds can hold is no candidate.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn seconds(amount: f64) -> Option<u64> {
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

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn starts_with(chars: &[char], at: usize, text: &str) -> bool {
    text.chars()
        .enumerate()
        .all(|(k, c)| chars.get(at + k) == Some(&c))
}

#[cfg(test)]
// The numbers under test are literals, compared exactly.
#[allow(clippy::float_cmp)]
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
