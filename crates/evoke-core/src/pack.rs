//! A language as data: the words each stage of the reader proposes from, one pack a language, built in from
//! `data/<tag>/pack.toml`. In: a text. Out: its `Lexicon`, the packs that read it with their tables joined — a
//! phrase list as a matcher tries it, longest first; a word's neutral form by the word; a number word's value —
//! and the built-in `Pack`s themselves, each with the digest of its file. The core holds no word of any language:
//! what it holds is how a table is read. A pack proposes; the classifier decides.

use std::cmp::Reverse;
use std::collections::HashSet;
use std::sync::OnceLock;

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value as Json;

use crate::calendar::{Weekday, Which};
use crate::digest::Digest;
use crate::text::fold;

/// The packs built in, in their order: the author's language first.
const BUILT_IN: [&str; 2] = [
    include_str!("../data/en/pack.toml"),
    include_str!("../data/de/pack.toml"),
];

/// The built-in packs, read once.
///
/// # Panics
///
/// A built-in pack that does not read is a bug in evoke: the test suite reads every one.
pub fn packs() -> &'static [Pack] {
    static PACKS: OnceLock<Vec<Pack>> = OnceLock::new();
    PACKS.get_or_init(|| {
        let mut packs: Vec<Pack> = BUILT_IN
            .iter()
            .map(|text| Pack::read(text).expect("a built-in pack reads"))
            .collect();
        // A pack's own words: three letters or more, no courtesy word, listed by no other pack.
        let all: Vec<HashSet<String>> = packs.iter().map(|pack| pack.listed.clone()).collect();
        for (at, pack) in packs.iter_mut().enumerate() {
            let courtesy: HashSet<String> = pack.courtesy.words.iter().map(str::to_owned).collect();
            pack.own = pack
                .listed
                .iter()
                .filter(|word| word.chars().count() > SHORT)
                .filter(|word| !courtesy.contains(*word))
                .filter(|word| {
                    all.iter()
                        .enumerate()
                        .all(|(other, words)| other == at || !words.contains(*word))
                })
                .cloned()
                .collect();
        }
        packs
    })
}

/// How many letters a word has at most to be short: a short word shows no pack, and a pack that the text does not
/// show holds its short words back — the two-letter forms, «so», «do», «u».
const SHORT: usize = 2;

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
    /// Whether a number's words are written as one, «einundzwanzig», «zweihundert».
    pub fused: bool,
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
    /// The words after which a day's word is a plain noun, not a day: «guten Morgen».
    pub noun_after: Phrases,
    /// Words near a day's name that are no day, however near: «Montage», «Leute».
    pub never: Phrases,
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
    /// 12, where a bare hour needs its half of the day; 24, where «9 Uhr» is nine in the morning.
    pub hour_cycle: u8,
    /// The words before an hour that mean half an hour before it: «halb drei» is half past two.
    pub half_to: Phrases,
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
    /// Whether a currency's symbol may follow the figure: «12 €».
    pub symbol_after: bool,
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
    name: String,
    capitals_mark_names: bool,
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
    /// The language's name, as evoke says it: `English`, `German`.
    pub name: String,
    /// The digest of the pack's file, which a plan's digest carries.
    pub digest: Digest,
    /// Whether a capital letter marks a name or a code.
    pub capitals_mark_names: bool,
    /// Every word the pack lists, as `fold` writes it.
    listed: HashSet<String>,
    /// The words that show the pack: four letters or more, no courtesy, listed by no other built-in pack.
    own: HashSet<String>,
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
        let json = json_of(parsed.as_item());
        let mut listed = HashSet::new();
        words_of(&json, &mut listed);
        let raw: Raw = serde_json::from_value(json).map_err(|error| error.to_string())?;
        Ok(Self {
            tag: raw.tag,
            name: raw.name,
            digest: Digest::of(text.as_bytes()),
            capitals_mark_names: raw.capitals_mark_names,
            listed,
            own: HashSet::new(),
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

impl Pack {
    /// Whether a word, as `fold` writes it, is one the pack lists anywhere.
    #[must_use]
    pub fn lists(&self, word: &str) -> bool {
        self.listed.contains(word)
    }

    /// Whether a word, as `fold` writes it, shows the pack.
    #[must_use]
    pub fn shows(&self, word: &str) -> bool {
        self.own.contains(word)
    }

    /// A number said in words as one word, as `fold` writes it: a number word of the tables, or, where the pack
    /// writes a number's words as one, their composition — a ones word, the joiner and a tens word
    /// («einundzwanzig»), a count before a scale word and what follows it («zweihundertdreißig»).
    #[must_use]
    pub fn number(&self, word: &str) -> Option<u64> {
        let numbers = &self.numbers;
        let single = numbers
            .ones
            .value(word)
            .or_else(|| numbers.tens.value(word))
            .or_else(|| numbers.scale.value(word));
        if let Some(value) = single {
            return u64::try_from(value).ok();
        }
        if !numbers.fused || word.is_empty() {
            return None;
        }
        // The largest scale first, so that «zweitausenddreihundert» splits at «tausend».
        let mut scales: Vec<_> = numbers.scale.iter().collect();
        scales.sort_by_key(|(_, scale)| Reverse(*scale));
        for (scale_word, scale) in scales {
            let Some(at) = word.find(scale_word) else {
                continue;
            };
            let (head, rest) = (&word[..at], &word[at + scale_word.len()..]);
            let count = if head.is_empty() || numbers.article.holds(head) {
                Some(1)
            } else {
                self.number(head)
            };
            let Some(count) = count else {
                continue;
            };
            let rest = if rest.is_empty() {
                Some(0)
            } else {
                self.number(rest)
            };
            if let Some(rest) = rest
                && let Ok(scale) = u64::try_from(scale)
            {
                return Some(count * scale + rest);
            }
        }
        for (tens_word, tens) in numbers.tens.iter() {
            let Some(head) = word.strip_suffix(tens_word) else {
                continue;
            };
            let Some(ones_word) = head.strip_suffix(numbers.joiner.as_str()) else {
                continue;
            };
            // «ein» before the joiner is one, as it is before a scale word: «einundzwanzig».
            let ones = numbers
                .ones
                .value(ones_word)
                .or_else(|| numbers.article.holds(ones_word).then_some(1));
            if let Some(ones) = ones
                && (1..10).contains(&ones)
                && let (Ok(ones), Ok(tens)) = (u64::try_from(ones), u64::try_from(tens))
            {
                return Some(tens + ones);
            }
        }
        None
    }
}

/// Every word a pack's file lists, as `fold` writes it: the items of its arrays, and the keys of its tables of
/// numbers. A table's keys that name a form stand for no word, and a setting is no word.
fn words_of(json: &Json, into: &mut HashSet<String>) {
    match json {
        Json::Array(items) => {
            for item in items {
                match item {
                    Json::String(word) => {
                        into.insert(fold(word));
                    }
                    other => words_of(other, into),
                }
            }
        }
        Json::Object(entries) => {
            for (key, value) in entries {
                if value.is_number() {
                    into.insert(fold(key));
                } else {
                    words_of(value, into);
                }
            }
        }
        _ => {}
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

/// A yes or a no typed at a prompt, in any built-in pack's words: `Some(true)` for a yes, `Some(false)` for a no,
/// none for any other word. The word is compared as `fold` writes it.
#[must_use]
pub fn answer(typed: &str) -> Option<bool> {
    let word = fold(typed.trim());
    let packs = packs();
    if packs.iter().any(|pack| pack.prompt.yes.holds(&word)) {
        return Some(true);
    }
    packs
        .iter()
        .any(|pack| pack.prompt.no.holds(&word))
        .then_some(false)
}

/// A pack a text shows, and the words that show it.
#[derive(Clone, Debug)]
pub struct Shown {
    pub pack: &'static Pack,
    pub words: Vec<String>,
}

/// The words the reader knows for one text: the packs that read it, in order, their tables joined. The first
/// built-in pack, the author's language, reads every text; another pack reads a text where the text holds more of
/// the pack's own words than of any other pack's, or as many as of another pack's that is not the first — a tie
/// with the first pack is the first pack's. A text that shows no pack is read by every pack, but a pack other than
/// the first holds back its short forms there, and every word the first pack lists. A pack's grammar — its endings,
/// how its numbers compose, its marks and its clock — applies only where the pack is shown, and the first pack's
/// where none is.
#[derive(Clone, Debug)]
pub struct Lexicon {
    /// The packs that read the text: the shown ones first, the first built-in pack among them, then the rest where
    /// none is shown.
    packs: Vec<&'static Pack>,
    /// The packs whose grammar applies.
    grammar: Vec<&'static Pack>,
    shown: Vec<Shown>,
    unshown: bool,
}

/// The lexicon of a text.
#[must_use]
pub fn lexicon(text: &str) -> Lexicon {
    let all = packs();
    let first = &all[0];
    // A token with a figure or an address's mark inside is a code, a URL or an address: it shows no pack.
    let tokens: Vec<String> = text
        .split_whitespace()
        .map(|token| fold(token.trim_matches(|c: char| !c.is_alphanumeric())))
        .filter(|token| {
            !token.is_empty()
                && !token
                    .chars()
                    .any(|c| c.is_numeric() || matches!(c, '@' | '/' | ':'))
        })
        .collect();
    let mut shown: Vec<Shown> = all
        .iter()
        .map(|pack| Shown {
            pack,
            words: tokens
                .iter()
                .filter(|token| pack.shows(token))
                .cloned()
                .collect(),
        })
        .collect();
    let most = shown.iter().map(|s| s.words.len()).max().unwrap_or(0);
    if most == 0 {
        return Lexicon {
            packs: all.iter().collect(),
            grammar: vec![first],
            shown: Vec::new(),
            unshown: true,
        };
    }
    shown.retain(|s| s.words.len() == most);
    // A tie with the first pack is the first pack's.
    if shown.len() > 1 && shown.iter().any(|s| s.pack.tag == first.tag) {
        shown.retain(|s| s.pack.tag == first.tag);
    }
    let mut packs: Vec<&'static Pack> = shown.iter().map(|s| s.pack).collect();
    if !packs.iter().any(|pack| pack.tag == first.tag) {
        packs.push(first);
    }
    Lexicon {
        grammar: shown.iter().map(|s| s.pack).collect(),
        packs,
        shown,
        unshown: false,
    }
}

impl Lexicon {
    /// The packs that read the text, in order.
    #[must_use]
    pub fn packs(&self) -> &[&'static Pack] {
        &self.packs
    }

    /// The packs the text shows, with the words that show each; none where it shows none.
    #[must_use]
    pub fn shown(&self) -> &[Shown] {
        &self.shown
    }

    /// Whether a pack's word may propose in the text: every word of a shown pack, and of the first pack; of the
    /// others, where no pack is shown, a word that is not short and that the first pack does not list — a word two
    /// languages share is read as the first lists it.
    fn offers(&self, pack: &Pack, word: &str) -> bool {
        let first = &packs()[0];
        !self.unshown
            || pack.tag == first.tag
            || (word.chars().count() > SHORT && !first.lists(word))
    }

    /// One phrase list of every pack, joined, the longest phrase first.
    pub fn phrases(&self, table: impl Fn(&'static Pack) -> &'static Phrases) -> Vec<&'static str> {
        let mut phrases: Vec<&'static str> = self
            .packs
            .iter()
            .flat_map(|pack| {
                table(pack)
                    .iter()
                    .filter(|phrase| self.offers(pack, phrase))
            })
            .collect();
        if self.packs.len() > 1 {
            phrases.sort_by_key(|phrase| Reverse(phrase.chars().count()));
        }
        phrases
    }

    /// Whether a word, as `fold` writes it, is in one phrase list of some pack.
    pub fn holds(&self, table: impl Fn(&'static Pack) -> &'static Phrases, word: &str) -> bool {
        self.packs
            .iter()
            .any(|pack| self.offers(pack, word) && table(pack).holds(word))
    }

    /// Whether a word, as typed, is in one typed list of some pack.
    pub fn typed(&self, table: impl Fn(&'static Pack) -> &'static Typed, word: &str) -> bool {
        self.packs
            .iter()
            .any(|pack| self.offers(pack, word) && table(pack).holds(word))
    }

    /// Whether a word, as `fold` writes it, is in one phrase list of a pack whose grammar applies.
    pub fn grammar_holds(
        &self,
        table: impl Fn(&'static Pack) -> &'static Phrases,
        word: &str,
    ) -> bool {
        self.grammar.iter().any(|pack| table(pack).holds(word))
    }

    /// The form a word stands for in one named table of some pack, the first pack that holds it.
    pub fn form_of(
        &self,
        table: impl Fn(&'static Pack) -> &'static Named,
        word: &str,
    ) -> Option<&'static str> {
        self.packs
            .iter()
            .filter(|pack| self.offers(pack, word))
            .find_map(|pack| table(pack).form_of(word))
    }

    /// Every word of one named table of every pack with its form, the longest word first.
    pub fn named(
        &self,
        table: impl Fn(&'static Pack) -> &'static Named,
    ) -> Vec<(&'static str, &'static str)> {
        let mut words: Vec<(&'static str, &'static str)> = self
            .packs
            .iter()
            .flat_map(|pack| {
                table(pack)
                    .words()
                    .filter(|(word, _)| self.offers(pack, word))
            })
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
        self.packs
            .iter()
            .filter(|pack| self.offers(pack, word))
            .find_map(|pack| table(pack).value(word))
    }

    /// A number said as one word, as `fold` writes it: a number word of some pack, or a composition a pack whose
    /// grammar applies writes as one word.
    #[must_use]
    pub fn number(&self, word: &str) -> Option<u64> {
        let single = self
            .value(|pack| &pack.numbers.ones, word)
            .or_else(|| self.value(|pack| &pack.numbers.tens, word))
            .or_else(|| self.value(|pack| &pack.numbers.scale, word))
            .and_then(|value| u64::try_from(value).ok());
        single.or_else(|| self.grammar.iter().find_map(|pack| pack.number(word)))
    }

    /// Every ending a stem comes off with and what it leaves, of the one pack the text shows most — the first
    /// where two show alike, so that one language's endings never come off another's words — the longest ending
    /// first.
    #[must_use]
    pub fn stems(&self) -> Vec<(&'static str, &'static str)> {
        let mut stems: Vec<(&'static str, &'static str)> = self
            .grammar
            .iter()
            .take(1)
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

    /// The mark every pack whose grammar applies puts before a number's fraction; none where they differ.
    #[must_use]
    pub fn decimal_mark(&self) -> Option<char> {
        let mut marks = self.grammar.iter().map(|pack| pack.numbers.decimal_mark);
        let first = marks.next()?;
        marks.all(|mark| mark == first).then_some(first)
    }

    /// The hours a clock counts: 24 where every pack whose grammar applies counts so, else 12.
    #[must_use]
    pub fn hour_cycle(&self) -> u8 {
        if self.grammar.iter().all(|pack| pack.times.hour_cycle == 24) {
            24
        } else {
            12
        }
    }

    /// Whether a capital marks a name or a code in the text: so for every pack whose grammar applies.
    #[must_use]
    pub fn capitals_mark_names(&self) -> bool {
        self.grammar.iter().all(|pack| pack.capitals_mark_names)
    }

    /// Whether a currency's symbol may follow the figure, as some pack whose grammar applies writes it.
    #[must_use]
    pub fn symbol_after(&self) -> bool {
        self.grammar.iter().any(|pack| pack.amounts.symbol_after)
    }

    /// The first pack that reads the text: how a form is written back.
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
    fn a_yes_or_a_no_is_read_from_every_pack_and_no_word_is_both() {
        for pack in packs() {
            assert!(!pack.name.is_empty(), "{}: no name", pack.tag);
            for word in pack.prompt.yes.iter() {
                assert_eq!(answer(word), Some(true), "{}: «{word}» is no yes", pack.tag);
            }
            for word in pack.prompt.no.iter() {
                assert_eq!(answer(word), Some(false), "{}: «{word}» is no no", pack.tag);
            }
        }
        let yes: Vec<&str> = packs()
            .iter()
            .flat_map(|pack| pack.prompt.yes.iter())
            .collect();
        for pack in packs() {
            for word in pack.prompt.no.iter() {
                assert!(!yes.contains(&word), "«{word}» is a yes and a no");
            }
        }
        assert_eq!(answer("Ja"), Some(true));
        assert_eq!(answer("maybe"), None);
    }

    #[test]
    fn a_text_is_read_by_the_pack_it_shows_most_and_the_first_pack_always() {
        let first = packs()[0].tag.as_str();
        let tags = |text: &str| -> Vec<String> {
            lexicon(text)
                .packs()
                .iter()
                .map(|pack| pack.tag.clone())
                .collect()
        };
        // Own words of three letters or more show a pack; a tie with the first pack is the first pack's.
        assert_eq!(tags("clear out the bin for good"), [first]);
        assert_eq!(tags("call the man"), [first]);
        assert_eq!(
            tags("schau dir Incident 318 und Incident 330 an"),
            ["de", first]
        );
        assert_eq!(tags("liste anas sessions auf"), ["de", first]);
        // A text that shows no pack is read by every pack, short words of the others held back.
        let none = lexicon("wipe C02G8TVYPQ3K asap");
        assert_eq!(none.packs().len(), packs().len());
        assert!(none.shown().is_empty());
        assert!(!none.typed(|pack| &pack.cut.signs, "u"));
        assert!(
            lexicon("liste anas sessions auf u meld sie ab").typed(|pack| &pack.cut.signs, "u")
        );
        // The first pack's grammar where none is shown; the shown pack's where one is.
        assert_eq!(
            none.decimal_mark(),
            lexicon("send it to the team").decimal_mark()
        );
        assert_eq!(lexicon("liste anas sessions auf").decimal_mark(), Some(','));
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
