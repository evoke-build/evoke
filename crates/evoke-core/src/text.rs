//! Strings the design constrains. In: `&str`. Out: `Clean`, `Input`, `Identity`, `Utterance`, `Span`, `NonEmpty`.

use std::fmt;

use serde::{Deserialize, Serialize, Serializer};
use unicode_normalization::UnicodeNormalization;

/// Text that cannot repaint a terminal: no C0 or C1 control but the line feed, no bidi control.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
