//! Strings the design constrains. In: `&str`. Out: `Clean`, `Input`, `Identity`, `Utterance`, `Span`, `NonEmpty`.

use std::fmt;

use serde::{Deserialize, Serialize, Serializer};
use unicode_normalization::UnicodeNormalization;

/// Text that cannot repaint a terminal: no C0 or C1 control but the line feed, no bidi control.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Clean(String);

/// The character that made a string unclean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unclean(pub char);

impl Clean {
    pub fn new(text: &str) -> Result<Self, Unclean> {
        match text.chars().find(|&c| is_control(c) || is_bidi(c)) {
            Some(c) => Err(Unclean(c)),
            None => Ok(Self(text.to_owned())),
        }
    }

    /// One non-empty clean line; the error is a fragment to follow the subject: `is empty`, `must be one line`,
    /// `contains …`.
    pub fn line(text: &str) -> Result<Self, String> {
        if text.is_empty() {
            return Err("is empty".to_owned());
        }
        if text.contains('\n') {
            return Err("must be one line".to_owned());
        }
        Self::new(text).map_err(|why| why.to_string())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_control(c: char) -> bool {
    c != '\n' && c.is_control()
}

fn is_bidi(c: char) -> bool {
    matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

impl TryFrom<String> for Clean {
    type Error = Unclean;

    fn try_from(text: String) -> Result<Self, Unclean> {
        Self::new(&text)
    }
}

impl fmt::Display for Clean {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for Unclean {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == '\r' {
            return f.write_str(
                "contains a carriage return (U+000D); save the file with LF line endings",
            );
        }
        let kind = if is_bidi(self.0) {
            "a bidi control"
        } else {
            "a control character"
        };
        write!(f, "contains {kind} (U+{:04X})", u32::from(self.0))
    }
}

/// What a person typed: NFC, at most 2 000 characters, spelling kept. Untrusted, never cleaned; only its spans are.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Input(String);

/// An input over the cap, with its length in characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TooLong(pub usize);

impl Input {
    pub const CAP: usize = 2_000;

    pub fn new(text: &str) -> Result<Self, TooLong> {
        let text: String = text.nfc().collect();
        let chars = text.chars().count();
        if chars > Self::CAP {
            Err(TooLong(chars))
        } else {
            Ok(Self(text))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Input {
    type Error = TooLong;

    fn try_from(text: String) -> Result<Self, TooLong> {
        Self::new(&text)
    }
}

impl fmt::Display for TooLong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the input is {} characters long; {} is the cap",
            self.0,
            Input::CAP
        )
    }
}

/// What keys a record: NFC, lower-cased, whitespace collapsed and trimmed, terminal punctuation dropped.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "String")]
pub struct Identity(String);

/// The identity of an utterance, however it was spelled.
#[must_use]
pub fn identity(text: &str) -> Identity {
    let lowered = text.nfc().collect::<String>().to_lowercase();
    let collapsed = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
    Identity(
        collapsed
            .trim_end_matches(['.', '!', '?', '…'])
            .trim_end()
            .to_owned(),
    )
}

impl Identity {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Identity {
    fn from(text: String) -> Self {
        identity(&text)
    }
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A word as it is compared with a listed word or a word the reader knows: lowered by Unicode's rules, the letters
/// a full case fold writes as two written so («ß» as «ss», «ﬁ» as «fi»), and any script's decimal digit as its
/// figure. Only ever a comparison's form: what a person typed stays as typed, and no value is made from it.
#[must_use]
pub fn fold(text: &str) -> String {
    text.chars().flat_map(fold_char).collect()
}

/// One character as `fold` writes it.
pub(crate) fn fold_char(c: char) -> impl Iterator<Item = char> {
    let folded: &str = match c {
        'ß' | 'ẞ' => "ss",
        'ﬀ' => "ff",
        'ﬁ' => "fi",
        'ﬂ' => "fl",
        'ﬃ' => "ffi",
        'ﬄ' => "ffl",
        'ﬅ' | 'ﬆ' => "st",
        'ς' => "σ",
        _ => "",
    };
    let digit = figure_of(c);
    let lowered: Vec<char> = if !folded.is_empty() {
        folded.chars().collect()
    } else if let Some(digit) = digit {
        vec![digit]
    } else {
        c.to_lowercase().collect()
    };
    lowered.into_iter()
}

/// The zero of every run of ten decimal digits Unicode 16 has, in order: a digit's figure is its distance from the
/// zero before it.
const ZEROS: [u32; 76] = [
    0x30, 0x660, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xA66, 0xAE6, 0xB66, 0xBE6, 0xC66, 0xCE6, 0xD66,
    0xDE6, 0xE50, 0xED0, 0xF20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90,
    0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0, 0xFF10,
    0x104A0, 0x10D30, 0x10D40, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450, 0x114D0,
    0x11650, 0x116C0, 0x116D0, 0x116DA, 0x11730, 0x118E0, 0x11950, 0x11BF0, 0x11C50, 0x11D50,
    0x11DA0, 0x11F50, 0x16130, 0x16A60, 0x16AC0, 0x16B50, 0x16D70, 0x1CCF0, 0x1D7CE, 0x1D7D8,
    0x1D7E2, 0x1D7EC, 0x1D7F6, 0x1E140, 0x1E2F0, 0x1E4F0, 0x1E5F1, 0x1E950, 0x1FBF0,
];

/// A decimal digit of any script as its figure, `0` to `9`; none for any other character.
fn figure_of(c: char) -> Option<char> {
    if !c.is_numeric() {
        return None;
    }
    let point = u32::from(c);
    let zero = ZEROS
        .iter()
        .rev()
        .find(|zero| **zero <= point)
        .filter(|zero| point - **zero < 10)?;
    char::from_digit(point - zero, 10)
}

/// A figure as a value is typed and a form is read: `0` to `9`. The forms the recognizers read — a figure, a code,
/// an address, a link, a currency's code — are written in these characters whatever the language, so a test of one
/// of them is a test of the form, not of a word.
#[must_use]
pub(crate) fn figure(c: char) -> bool {
    c.is_ascii_digit()
}

/// A letter as a code, an address or a link is typed: the Latin alphabet, either case.
#[must_use]
pub(crate) fn latin(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// A letter or a figure as a code is typed.
#[must_use]
pub(crate) fn latin_or_figure(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

/// A capital as a code or a currency's code is typed.
#[must_use]
pub(crate) fn capital(c: char) -> bool {
    c.is_ascii_uppercase()
}

/// A small letter as a code, an address or a link is typed.
#[must_use]
pub(crate) fn small(c: char) -> bool {
    c.is_ascii_lowercase()
}

/// An utterance as written, with its identity: the text is what the classifier reads, the id what keys it. One
/// non-empty line, as a record's key is; the error is a fragment to follow the text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawUtterance")]
pub struct Utterance {
    text: Clean,
    id: Identity,
}

#[derive(Deserialize)]
struct RawUtterance {
    text: String,
    id: Identity,
}

impl Utterance {
    /// NFC, as an input is, so a span an input produced is found in the utterance it came from.
    pub fn new(text: &str) -> Result<Self, String> {
        let text: String = text.nfc().collect();
        let text = Clean::line(&text)?;
        let id = identity(text.as_str());
        Ok(Self { text, id })
    }

    /// A record's utterance, from the text and the identity `Records` keys it by.
    pub(crate) fn of(text: &Clean, id: &Identity) -> Self {
        Self {
            text: text.clone(),
            id: id.clone(),
        }
    }

    #[must_use]
    pub fn text(&self) -> &Clean {
        &self.text
    }

    #[must_use]
    pub fn id(&self) -> &Identity {
        &self.id
    }
}

impl TryFrom<RawUtterance> for Utterance {
    type Error = String;

    fn try_from(raw: RawUtterance) -> Result<Self, String> {
        let utterance = Self::new(&raw.text).map_err(|why| format!("\"{}\" {why}", raw.text))?;
        if utterance.id == raw.id {
            Ok(utterance)
        } else {
            Err(format!(
                "\"{}\" is not the identity of \"{}\"",
                raw.id, raw.text
            ))
        }
    }
}

/// A verbatim piece of one input: character offsets, `start < end`, and the text between them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawSpan")]
pub struct Span {
    start: usize,
    end: usize,
    text: Clean,
}

#[derive(Deserialize)]
struct RawSpan {
    start: usize,
    end: usize,
    text: Clean,
}

impl Span {
    /// The characters `start..end` of `input`, when they exist, are not empty and are clean.
    #[must_use]
    pub fn of(input: &Input, start: usize, end: usize) -> Option<Self> {
        if start >= end {
            return None;
        }
        let text: String = input
            .as_str()
            .chars()
            .skip(start)
            .take(end - start)
            .collect();
        if text.chars().count() < end - start {
            return None;
        }
        Clean::new(&text).ok().map(|text| Self { start, end, text })
    }

    #[must_use]
    pub fn start(&self) -> usize {
        self.start
    }

    #[must_use]
    pub fn end(&self) -> usize {
        self.end
    }

    #[must_use]
    pub fn text(&self) -> &Clean {
        &self.text
    }
}

impl TryFrom<RawSpan> for Span {
    type Error = String;

    fn try_from(raw: RawSpan) -> Result<Self, String> {
        if raw.start < raw.end && raw.text.as_str().chars().count() == raw.end - raw.start {
            Ok(Self {
                start: raw.start,
                end: raw.end,
                text: raw.text,
            })
        } else {
            Err(format!(
                "a span of {}..{} does not hold \"{}\"",
                raw.start, raw.end, raw.text
            ))
        }
    }
}

/// A list that refuses to be empty; on the wire, a plain array.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Vec<T>")]
pub struct NonEmpty<T>(T, Vec<T>);

impl<T> NonEmpty<T> {
    pub fn new(first: T, rest: Vec<T>) -> Self {
        Self(first, rest)
    }

    pub fn first(&self) -> &T {
        &self.0
    }

    /// One more at the end: a list that was not empty stays so.
    pub fn push(&mut self, item: T) {
        self.1.push(item);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.0).chain(&self.1)
    }
}

impl<T> IntoIterator for NonEmpty<T> {
    type Item = T;
    type IntoIter = std::iter::Chain<std::iter::Once<T>, std::vec::IntoIter<T>>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.0).chain(self.1)
    }
}

impl<T> TryFrom<Vec<T>> for NonEmpty<T> {
    type Error = String;

    fn try_from(list: Vec<T>) -> Result<Self, String> {
        let mut items = list.into_iter();
        let first = items.next().ok_or_else(|| "the list is empty".to_owned())?;
        Ok(Self(first, items.collect()))
    }
}

impl<T: Serialize> Serialize for NonEmpty<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_refuses_controls_and_bidi_but_keeps_line_feeds() {
        assert!(Clean::new("two\nlines").is_ok());
        assert_eq!(Clean::line("two\nlines").unwrap_err(), "must be one line");
        assert_eq!(Clean::line("").unwrap_err(), "is empty");
        assert!(Clean::line("one line").is_ok());
        assert_eq!(
            Clean::new("a\tb").unwrap_err().to_string(),
            "contains a control character (U+0009)"
        );
        assert_eq!(
            Clean::new("Switch \u{202e}on.").unwrap_err().to_string(),
            "contains a bidi control (U+202E)"
        );
        assert_eq!(Clean::new("\u{85}").unwrap_err(), Unclean('\u{85}'));
    }

    #[test]
    fn an_utterance_is_one_line() {
        assert_eq!(Utterance::new("").unwrap_err(), "is empty");
        assert_eq!(
            Utterance::new("two\nlines").unwrap_err(),
            "must be one line"
        );
        assert_eq!(
            Utterance::new("Kill the lights!").unwrap().id().as_str(),
            "kill the lights"
        );
    }

    #[test]
    fn input_is_nfc_and_capped() {
        assert_eq!(Input::new("cafe\u{301}").unwrap().as_str(), "café");
        assert_eq!(Input::new(&"é".repeat(2_001)).unwrap_err(), TooLong(2_001));
        assert!(Input::new(&"é".repeat(2_000)).is_ok());
    }

    #[test]
    fn identity_normalizes() {
        assert_eq!(
            identity("  Kill   the Lights!?. ").as_str(),
            "kill the lights"
        );
        assert_eq!(identity("cafe\u{301}"), identity("café"));
        assert_eq!(identity("Straße").as_str(), "straße");
    }

    #[test]
    fn span_is_verbatim() {
        let input = Input::new("dim to 30 percent").unwrap();
        assert_eq!(
            Span::of(&input, 7, 17).unwrap().text().as_str(),
            "30 percent"
        );
        assert!(Span::of(&input, 7, 7).is_none());
        assert!(Span::of(&input, 7, 18).is_none());
    }

    #[test]
    fn non_empty_round_trips() {
        let list: NonEmpty<u8> = serde_json::from_str("[1, 2]").unwrap();
        assert_eq!(list.iter().count(), 2);
        assert_eq!(serde_json::to_string(&list).unwrap(), "[1,2]");
        assert!(serde_json::from_str::<NonEmpty<u8>>("[]").is_err());
    }
}
