//! A language as data: the words each stage of the reader proposes from, one pack a language, built in from
//! `data/<tag>/pack.toml`. In: a text. Out: its `Lexicon`, the packs that read it with their tables joined — a
//! phrase list as a matcher tries it, longest first; a word's neutral form by the word; a number word's value —
//! and the built-in `Pack`s themselves, each with the digest of its file. The core holds no word of any language:
//! what it holds is how a table is read. A pack proposes; the classifier decides.

use std::cmp::Reverse;
use std::sync::OnceLock;

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value as Json;

use crate::calendar::{Weekday, Which};
use crate::digest::Digest;
use crate::text::fold;

/// The packs built in, in their order: the author's language first.
const BUILT_IN: [&str; 1] = [include_str!("../data/en/pack.toml")];

/// The built-in packs, read once.
///
/// # Panics
///
/// A built-in pack that does not read is a bug in evoke: the test suite reads every one.
pub fn packs() -> &'static [Pack] {
    static PACKS: OnceLock<Vec<Pack>> = OnceLock::new();
    PACKS.get_or_init(|| {
        BUILT_IN
            .iter()
            .map(|text| Pack::read(text).expect("a built-in pack reads"))
            .collect()
    })
}

/// A list of phrases as a matcher tries them: the longest first, so «and then» wins over «and»; each as `fold`
/// writes it.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(from = "Vec<String>")]
pub struct Phrases(Vec<String>);

impl From<Vec<String>> for Phrases {
    fn from(phrases: Vec<String>) -> Self {
        let mut folded: Vec<String> = phrases.iter().map(|phrase| fold(phrase)).collect();
        folded.sort_by_key(|phrase| Reverse(phrase.chars().count()));
        Self(folded)
    }
}

impl Phrases {
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// Whether a word, as `fold` writes it, is one of the phrases.
    #[must_use]
    pub fn holds(&self, word: &str) -> bool {
        self.0.iter().any(|phrase| phrase == word)
    }
}

/// Words as they are typed, compared without a fold: the signs that stand for «and», a currency's code.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct Typed(Vec<String>);

impl Typed {
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// Whether a word, as typed, is listed.
    #[must_use]
    pub fn holds(&self, word: &str) -> bool {
        self.0.iter().any(|listed| listed == word)
    }
}

/// Words listed under the neutral form each stands for, the form shown first: `monday = ["monday", "mon"]`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(from = "IndexMap<String, Vec<String>>")]
pub struct Named {
    /// Each form with its words as written, the shown one first.
    forms: IndexMap<String, Vec<String>>,
    /// Every word as `fold` writes it with the form it stands for, the longest word first.
    words: Vec<(String, String)>,
}

impl From<IndexMap<String, Vec<String>>> for Named {
    fn from(forms: IndexMap<String, Vec<String>>) -> Self {
        let mut words: Vec<(String, String)> = forms
            .iter()
            .flat_map(|(form, words)| words.iter().map(move |word| (fold(word), form.clone())))
            .collect();
        words.sort_by_key(|(word, _)| Reverse(word.chars().count()));
        Self { forms, words }
    }
}

impl Named {
    /// The form a word stands for, the word as `fold` writes it.
    #[must_use]
    pub fn form_of(&self, word: &str) -> Option<&str> {
        self.words
            .iter()
            .find(|(listed, _)| listed == word)
            .map(|(_, form)| form.as_str())
    }

    /// The word a form is shown as: the first listed under it.
    #[must_use]
    pub fn shown(&self, form: &str) -> Option<&str> {
        self.forms.get(form)?.first().map(String::as_str)
    }

    /// The words listed under a form, as written, the shown one first.
    #[must_use]
    pub fn words_of(&self, form: &str) -> &[String] {
        self.forms.get(form).map_or(&[], Vec::as_slice)
    }

    /// Every word with its form, the longest word first.
    pub fn words(&self) -> impl Iterator<Item = (&str, &str)> {
        self.words
            .iter()
            .map(|(word, form)| (word.as_str(), form.as_str()))
    }

    /// The forms, in the file's order.
    pub fn forms(&self) -> impl Iterator<Item = &str> {
        self.forms.keys().map(String::as_str)
    }

    /// Whether a word, as `fold` writes it, stands for some form.
    #[must_use]
    pub fn holds(&self, word: &str) -> bool {
        self.form_of(word).is_some()
    }
}

/// Words each with a number: a number word's value, an ordinal's rank, a repeat's count.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(from = "IndexMap<String, i64>")]
pub struct Numbered(IndexMap<String, i64>);

impl From<IndexMap<String, i64>> for Numbered {
    fn from(words: IndexMap<String, i64>) -> Self {
        Self(
            words
                .into_iter()
                .map(|(word, value)| (fold(&word), value))
                .collect(),
        )
    }
}

impl Numbered {
    /// The number a word stands for, the word as `fold` writes it.
    #[must_use]
    pub fn value(&self, word: &str) -> Option<i64> {
        self.0.get(word).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, i64)> {
        self.0.iter().map(|(word, value)| (word.as_str(), *value))
    }

    /// The first word listed for a value: how it is written back.
    #[must_use]
    pub fn word_of(&self, value: i64) -> Option<&str> {
        self.0
            .iter()
            .find(|(_, held)| **held == value)
            .map(|(word, _)| word.as_str())
    }
}

/// Where a request may be cut, and the words that set a part apart.
#[derive(Clone, Debug, Deserialize)]
pub struct Cut {
    pub then: Phrases,
    pub and: Phrases,
    /// The signs and letters that stand for «and» alone between two words, as typed.
    pub signs: Typed,
    /// The connectives after which a clause leads.
    pub leads: Phrases,
    pub negation: Phrases,
    pub condition: Phrases,
    pub contrast: Phrases,
    /// The word that may end a contrast's «not X».
    pub but: Phrases,
    /// The words before which a comma is no cut.
    pub subordinators: Phrases,
}

/// A clause before or after another, in the words' own order, and a clause of the person's own action.
#[derive(Clone, Debug, Deserialize)]
pub struct Clause {
    pub before: Phrases,
    pub after: Phrases,
    /// The connective the two clauses are written in order with.
    pub joiner: Phrases,
    pub subjects: Phrases,
    pub tenses: Phrases,
    pub own: Phrases,
    pub own_tenses: Phrases,
    pub conjunctions: Phrases,
    /// The head of a request's own clause, the person its subject.
    pub heads: Phrases,
}

/// Politeness, which asks for nothing.
#[derive(Clone, Debug, Deserialize)]
pub struct Courtesy {
    pub words: Phrases,
    pub phrases: Phrases,
}

/// Words that point at an earlier step, or say one is done again.
#[derive(Clone, Debug, Deserialize)]
pub struct Refer {
    pub one: Phrases,
    pub many: Phrases,
    pub determiners: Phrases,
    pub weak: Phrases,
    pub each: Phrases,
    pub adjectives: Phrases,
    pub stop: Phrases,
    pub points: Phrases,
    pub repeats: Phrases,
    pub once_more: Phrases,
    pub again: Phrases,
}

/// Words that point at a value this session returned, or count such values.
#[derive(Clone, Debug, Deserialize)]
pub struct Recalled {
    pub it: Phrases,
    pub one: Phrases,
    pub that: Phrases,
    pub owns: Phrases,
    pub several: Phrases,
    pub orders: Phrases,
    pub both: Phrases,
}

/// Words that carry no value by themselves, and words that introduce one.
#[derive(Clone, Debug, Deserialize)]
pub struct Words {
    pub function: Phrases,
    pub kind: Phrases,
    pub introduces: Phrases,
    pub articles: Phrases,
    pub after_name: Phrases,
    pub to_self: Phrases,
}

/// The endings a word takes: what comes off before it is compared with a listed word.
#[derive(Clone, Debug, Deserialize)]
pub struct Endings {
    /// Each ending with what it leaves, the longest first: `["ies", "y"]`.
    pub stem: Vec<(String, String)>,
    pub plural: Phrases,
    pub possessive: Phrases,
    pub tense: Phrases,
}

/// How a tens word and a ones word compose.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Compose {
    TensOnes,
    OnesTens,
}

/// A number said in words.
#[derive(Clone, Debug, Deserialize)]
pub struct Numbers {
    pub ones: Numbered,
    pub tens: Numbered,
    pub scale: Numbered,
    /// Past what a number reads: a run holding one reads as nothing.
    pub beyond: Phrases,
    /// A count of one before a unit, a currency or a scale word.
    pub article: Phrases,
    pub and: Phrases,
    pub noughts: Phrases,
    /// Nought said as a letter.
    pub oh: Phrases,
    pub repeat: Numbered,
    pub half: Phrases,
    pub and_a_half: Phrases,
    pub compose: Compose,
    /// The word a fused or spaced number joins its parts with, if any.
    pub joiner: String,
    pub years_in_pairs: bool,
    pub decimal_mark: char,
    pub group_mark: char,
    pub percent: Phrases,
}

/// An ordinal: a day of the month, a place among several.
#[derive(Clone, Debug, Deserialize)]
pub struct Ordinals {
    pub words: Numbered,
    pub suffixes: Suffixes,
    pub picks: Numbered,
}

/// The ending an ordinal takes as it is written: by the number's last two figures, then its last figure, then any
/// other number.
#[derive(Clone, Debug, Deserialize)]
pub struct Suffixes {
    pub other: String,
    #[serde(flatten)]
    pub by_number: IndexMap<String, String>,
}

impl Suffixes {
    /// The ending a number takes.
    #[must_use]
    pub fn of(&self, number: u8) -> &str {
        self.by_number
            .get(&(number % 100).to_string())
            .or_else(|| self.by_number.get(&(number % 10).to_string()))
            .unwrap_or(&self.other)
    }

    /// Every ending, the longest first.
    pub fn all(&self) -> Vec<&str> {
        let mut all: Vec<&str> = self
            .by_number
            .values()
            .chain(std::iter::once(&self.other))
            .map(String::as_str)
            .collect();
        all.sort_by_key(|suffix| Reverse(suffix.chars().count()));
        all.dedup();
        all
    }
}

/// A day as the words say it.
#[derive(Clone, Debug, Deserialize)]
pub struct Days {
    pub every: Phrases,
    pub recurring: Phrases,
    pub recurrences: Phrases,
    pub periods: Phrases,
    pub ahead: Phrases,
    pub from_now: Phrases,
    pub the: Phrases,
    pub of: Phrases,
    pub day_after: Phrases,
    pub followers: Phrases,
    pub leads: Phrases,
    pub which: Named,
    pub weekdays: Named,
    pub short: Named,
    pub relative: Named,
    pub relative_short: Named,
    pub units: Named,
}

/// A clock time.
#[derive(Clone, Debug, Deserialize)]
pub struct Times {
    pub oclock: Phrases,
    pub am: Phrases,
    pub pm: Phrases,
    pub past: Phrases,
    pub to: Phrases,
    pub named: Named,
    pub minutes_past: Named,
    pub minutes_to: Named,
}

/// A length of time.
#[derive(Clone, Debug, Deserialize)]
pub struct Durations {
    pub units: Named,
    pub articles: Named,
}

/// An amount in a currency.
#[derive(Clone, Debug, Deserialize)]
pub struct Amounts {
    pub symbols: IndexMap<String, String>,
    pub singular: Phrases,
    /// The currencies' codes, as typed: `USD`.
    pub codes: Typed,
    pub words: Named,
}

/// A value said aloud.
#[derive(Clone, Debug, Deserialize)]
pub struct Spoken {
    pub version: Phrases,
    pub schemes: Phrases,
    pub letters: Phrases,
    pub pronouns: Phrases,
    pub signs: Named,
}

/// The marks around a value in quotes, and around a text in quotes.
#[derive(Clone, Debug, Deserialize)]
pub struct Quotes {
    pub value: Vec<(char, char)>,
    pub text: Vec<(char, char)>,
}

/// The whole words a prompt takes for yes and for no.
#[derive(Clone, Debug, Deserialize)]
pub struct Prompt {
    pub yes: Phrases,
    pub no: Phrases,
}

/// The pack's file, as it is written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    tag: String,
    cut: Cut,
    clause: Clause,
    courtesy: Courtesy,
    refer: Refer,
    recalled: Recalled,
    words: Words,
    endings: Endings,
    numbers: Numbers,
    ordinals: Ordinals,
    days: Days,
    months: Named,
    times: Times,
    durations: Durations,
    amounts: Amounts,
    spoken: Spoken,
    quotes: Quotes,
    prompt: Prompt,
}

/// One language's words, built in.
#[derive(Clone, Debug)]
pub struct Pack {
    /// The language's tag: `en`, `de`.
    pub tag: String,
    /// The digest of the pack's file, which a plan's digest carries.
    pub digest: Digest,
    pub cut: Cut,
    pub clause: Clause,
    pub courtesy: Courtesy,
    pub refer: Refer,
    pub recalled: Recalled,
    pub words: Words,
    pub endings: Endings,
    pub numbers: Numbers,
    pub ordinals: Ordinals,
    pub days: Days,
    pub months: Named,
    pub times: Times,
    pub durations: Durations,
    pub amounts: Amounts,
    pub spoken: Spoken,
    pub quotes: Quotes,
    pub prompt: Prompt,
}

impl Pack {
    /// A pack read from its file's text.
    pub fn read(text: &str) -> Result<Self, String> {
        let parsed = toml_edit::Document::parse(text.to_owned())
            .map_err(|error| error.message().to_owned())?;
        let raw: Raw =
            serde_json::from_value(json_of(parsed.as_item())).map_err(|error| error.to_string())?;
        Ok(Self {
            tag: raw.tag,
            digest: Digest::of(text.as_bytes()),
            cut: raw.cut,
            clause: raw.clause,
            courtesy: raw.courtesy,
            refer: raw.refer,
            recalled: raw.recalled,
            words: raw.words,
            endings: raw.endings,
            numbers: raw.numbers,
            ordinals: raw.ordinals,
            days: raw.days,
            months: raw.months,
            times: raw.times,
            durations: raw.durations,
            amounts: raw.amounts,
            spoken: raw.spoken,
            quotes: raw.quotes,
            prompt: raw.prompt,
        })
    }
}

/// A TOML item as JSON, for serde to read the pack's tables from.
fn json_of(item: &toml_edit::Item) -> Json {
    use toml_edit::{Item, Value};
    match item {
        Item::None => Json::Null,
        Item::Value(value) => match value {
            Value::String(text) => Json::String(text.value().clone()),
            Value::Integer(number) => Json::from(*number.value()),
            Value::Float(number) => Json::from(*number.value()),
            Value::Boolean(flag) => Json::Bool(*flag.value()),
            Value::Datetime(when) => Json::String(when.value().to_string()),
            Value::Array(items) => Json::Array(
                items
                    .iter()
                    .map(|value| json_of(&Item::Value(value.clone())))
                    .collect(),
            ),
            Value::InlineTable(table) => table
                .iter()
                .map(|(key, value)| (key.to_owned(), json_of(&Item::Value(value.clone()))))
                .collect(),
        },
        Item::Table(table) => table
            .iter()
            .map(|(key, item)| (key.to_owned(), json_of(item)))
            .collect(),
        Item::ArrayOfTables(tables) => Json::Array(
            tables
                .iter()
                .map(|table| json_of(&Item::Table(table.clone())))
                .collect(),
        ),
    }
}

/// A weekday by its neutral form, as the wire names it.
#[must_use]
pub fn weekday(form: &str) -> Option<Weekday> {
    serde_json::from_value(Json::String(form.to_owned())).ok()
}

/// The word before a weekday by its neutral form, as the wire names it: next, this, last.
#[must_use]
pub fn which(form: &str) -> Option<Which> {
    serde_json::from_value(Json::String(form.to_owned())).ok()
}

/// A weekday's neutral form, as the wire names it.
#[must_use]
pub fn weekday_form(weekday: Weekday) -> String {
    serde_json::to_value(weekday)
        .ok()
        .and_then(|json| json.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// A which-word's neutral form, as the wire names it.
#[must_use]
pub fn which_form(which: Which) -> String {
    serde_json::to_value(which)
        .ok()
        .and_then(|json| json.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The words the reader knows for one text: the packs that read it, in order, their tables joined.
#[derive(Clone, Debug)]
pub struct Lexicon {
    packs: Vec<&'static Pack>,
}

/// The lexicon of a text: every built-in pack reads it.
#[must_use]
pub fn lexicon(_text: &str) -> Lexicon {
    Lexicon {
        packs: packs().iter().collect(),
    }
}

impl Lexicon {
    /// The packs that read the text, in order.
    #[must_use]
    pub fn packs(&self) -> &[&'static Pack] {
        &self.packs
    }

    /// One phrase list of every pack, joined, the longest phrase first.
    pub fn phrases(&self, table: impl Fn(&'static Pack) -> &'static Phrases) -> Vec<&'static str> {
        let mut phrases: Vec<&'static str> = self
            .packs
            .iter()
            .flat_map(|pack| table(pack).iter())
            .collect();
        if self.packs.len() > 1 {
            phrases.sort_by_key(|phrase| Reverse(phrase.chars().count()));
        }
        phrases
    }

    /// Whether a word, as `fold` writes it, is in one phrase list of some pack.
    pub fn holds(&self, table: impl Fn(&'static Pack) -> &'static Phrases, word: &str) -> bool {
        self.packs.iter().any(|pack| table(pack).holds(word))
    }

    /// Whether a word, as typed, is in one typed list of some pack.
    pub fn typed(&self, table: impl Fn(&'static Pack) -> &'static Typed, word: &str) -> bool {
        self.packs.iter().any(|pack| table(pack).holds(word))
    }

    /// The form a word stands for in one named table of some pack, the first pack that holds it.
    pub fn form_of(
        &self,
        table: impl Fn(&'static Pack) -> &'static Named,
        word: &str,
    ) -> Option<&'static str> {
        self.packs.iter().find_map(|pack| table(pack).form_of(word))
    }

    /// Every word of one named table of every pack with its form, the longest word first.
    pub fn named(
        &self,
        table: impl Fn(&'static Pack) -> &'static Named,
    ) -> Vec<(&'static str, &'static str)> {
        let mut words: Vec<(&'static str, &'static str)> = self
            .packs
            .iter()
            .flat_map(|pack| table(pack).words())
            .collect();
        if self.packs.len() > 1 {
            words.sort_by_key(|(word, _)| Reverse(word.chars().count()));
        }
        words
    }

    /// The number a word stands for in one numbered table of some pack.
    pub fn value(
        &self,
        table: impl Fn(&'static Pack) -> &'static Numbered,
        word: &str,
    ) -> Option<i64> {
        self.packs.iter().find_map(|pack| table(pack).value(word))
    }

    /// Every ending a stem comes off with and what it leaves, of every pack, the longest ending first.
    #[must_use]
    pub fn stems(&self) -> Vec<(&'static str, &'static str)> {
        let mut stems: Vec<(&'static str, &'static str)> = self
            .packs
            .iter()
            .flat_map(|pack| {
                pack.endings
                    .stem
                    .iter()
                    .map(|(ending, leaves)| (ending.as_str(), leaves.as_str()))
            })
            .collect();
        stems.sort_by_key(|(ending, _)| Reverse(ending.chars().count()));
        stems
    }

    /// The pairs of marks of one table of every pack, each once.
    pub fn pairs(
        &self,
        table: impl Fn(&'static Pack) -> &'static [(char, char)],
    ) -> Vec<(char, char)> {
        let mut pairs: Vec<(char, char)> = Vec::new();
        for pair in self.packs.iter().flat_map(|pack| table(pack).iter()) {
            if !pairs.contains(pair) {
                pairs.push(*pair);
            }
        }
        pairs
    }

    /// The first pack: how a form is written back.
    #[must_use]
    pub fn first(&self) -> &'static Pack {
        self.packs.first().copied().unwrap_or_else(|| &packs()[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_pack_reads_and_is_whole() {
        let packs = packs();
        assert_eq!(packs.len(), BUILT_IN.len());
        for pack in packs {
            assert!(!pack.tag.is_empty());
            assert!(pack.cut.then.iter().count() > 0, "{}: no joiner", pack.tag);
            assert!(pack.numbers.ones.value("x").is_none());
            for form in pack.days.weekdays.forms() {
                assert!(
                    weekday(form).is_some(),
                    "{}: {form} is no weekday",
                    pack.tag
                );
            }
            for form in pack.months.forms() {
                let month: u8 = form.parse().unwrap_or(0);
                assert!(
                    (1..=12).contains(&month),
                    "{}: {form} is no month",
                    pack.tag
                );
            }
        }
    }

    #[test]
    fn phrases_are_tried_longest_first_and_named_words_find_their_form() {
        let phrases = Phrases::from(vec!["and".to_owned(), "and then".to_owned()]);
        assert_eq!(phrases.iter().collect::<Vec<_>>(), ["and then", "and"]);
        let named = Named::from(IndexMap::from([(
            "monday".to_owned(),
            vec!["Montag".to_owned(), "mo".to_owned()],
        )]));
        assert_eq!(named.form_of("montag"), Some("monday"));
        assert_eq!(named.shown("monday"), Some("Montag"));
        assert_eq!(named.words().next(), Some(("montag", "monday")));
    }
}
