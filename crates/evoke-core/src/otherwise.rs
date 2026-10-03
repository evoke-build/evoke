//! A listed word said otherwise: what code proposes beside what `words.rs` finds, and never reads by itself. In:
//! the input, an argument's list with what each word means, what `words::listed` found. Out: `Proposal`s, each a
//! listed word, the words that propose it and how: a slip beside an apostrophe or a hyphen, a prefix of exactly one
//! word, a diminutive of exactly one listed name, a longer word the listed word begins, one edit of a word under
//! four letters. Beside them, whether words are a given name that is not a listed word (`another`), which is never
//! read as it.
//!
//! A prefix and a slip may be read where the choice anchored on the words confirms them; a longer word, a
//! diminutive and a short word's edit name the listed word's kin as readily as the word itself («Samuel», «Joe»,
//! «Anna»), and are only ever made ready at an ask. The given names and their pet forms are data, a file a
//! language under `data/`.

use std::sync::OnceLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::{Key, Prob};
use crate::text::{Clean, Input, Span, fold};
use crate::words::{self, Listed};

/// At this share the choice anchored on the words confirms a listed word code found by its spelling or its meaning,
/// or proposed as a prefix or a slip: a gate on what may be read, never a floor, since the value's number stays the
/// weaker of its two views.
pub(crate) const ANCHORED: f64 = 0.9;

/// At this share the better of an argument's two views makes its top listed word the likely one at an ask, where
/// code proposed none.
pub(crate) const LIKELY: f64 = 0.3;

/// The fewest letters a prefix is proposed from: «aug», never «au».
const PREFIX: usize = 3;

/// The fewest letters a listed word has for a prefix of it to be proposed: a prefix of a shorter one is a spelling
/// `words.rs` reads by its radius, or another word.
const PREFIXED: usize = 5;

/// A listed word under this many letters proposes its spellings one edit away, which `words.rs` never reads.
const SHORT: usize = 4;

/// The endings a possessive adds to a word, dropped before it is compared: «Maira's», «Samuel’s», «Jonas'».
const POSSESSIVE: [&str; 4] = ["'s", "\u{2019}s", "'", "\u{2019}"];

/// The marks that join a word's parts, dropped before a slip beside them is compared: «eu-wwest».
const JOINS: [char; 3] = ['-', '_', ' '];

/// The marks after which `words.rs` compares no spelling: an apostrophe, a hyphen, an underscore.
const MARKS: [char; 4] = ['\'', '\u{2019}', '-', '_'];

/// The endings that make a plural of a listed word, and so no longer word: «sams», «boxes».
const PLURAL: [&str; 2] = ["s", "es"];

/// The pet and short forms of given names, a line a full name and its forms: a diminutive proposes the one listed
/// name it shares a line with.
const DIMINUTIVES: &str = include_str!("../data/en/diminutives.tsv");

/// Given names, in small letters, sorted as text compares: a word on it that is not a listed word is never read as
/// one.
const GIVEN: &str = include_str!("../data/en/given-names.txt");

/// A data file's lines: what it holds, without the lines that say what it is.
fn lines(data: &'static str) -> impl Iterator<Item = &'static str> {
    data.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// The pet forms, read once: each full name with its forms.
fn diminutives() -> &'static [(&'static str, Vec<&'static str>)] {
    static ROWS: OnceLock<Vec<(&'static str, Vec<&'static str>)>> = OnceLock::new();
    ROWS.get_or_init(|| {
        lines(DIMINUTIVES)
            .filter_map(|line| line.split_once('\t'))
            .map(|(name, forms)| (name, forms.split(' ').collect()))
            .collect()
    })
}

/// The given names, read once, in the file's order.
fn given_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| lines(GIVEN).collect())
}

/// How code proposes a listed word it does not read, the most telling first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum By {
    /// A spelling within the listed word's radius once a possessive is dropped or a hyphen joined: «Maira's»,
    /// «eu-wwest».
    Slip,
    /// Three letters or more that begin exactly one listed word of five or more: «Sept», «aug».
    Prefix,
    /// A pet form that shares a line of `DIMINUTIVES` with exactly one listed name: «Sammy», «Mia».
    Diminutive,
    /// A longer word the listed word begins, neither its plural nor its last letter doubled: «Samuel», «augusta».
    Longer,
    /// One edit of a listed word under four letters: «Anna», «sma».
    Short,
}

impl By {
    /// Whether the choice anchored on the words may confirm the proposal: a prefix and a slip.
    #[must_use]
    pub fn anchors(self) -> bool {
        matches!(self, Self::Slip | Self::Prefix)
    }
}

/// A listed word the input's words propose, which code does not read: the word, the words, and how.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub key: Key,
    pub span: Span,
    pub by: By,
}

/// What the choice anchored on the words confirmed a listed word by: its share, and the words it was asked of.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anchored {
    pub p: Prob,
    pub words: Span,
}

/// The listed words the input's own words propose where `words::listed` found none: one proposal a word at most,
/// the most telling, from the first word of the input that makes it. A word a finding holds proposes nothing more,
/// and neither does the reflex's own name nor a word the set's reflexes say of themselves (`own`: their names, what
/// they do, their examples, their asks, as `decide.rs` gathers them), which says what to do: «check» never
/// proposes `checkout`, nor «down» `downloads`.
#[must_use]
pub fn proposed(
    input: &Input,
    list: &IndexMap<Key, Clean>,
    found: &[Listed],
    reflex: &str,
    own: &[String],
) -> Vec<Proposal> {
    let tokens = words::tokens(input.as_str());
    let keys: Vec<String> = list
        .keys()
        .map(|key| words::named(&fold(key.as_str())))
        .collect();
    let free: Vec<&words::Token> = tokens
        .iter()
        .filter(|token| {
            !token.plain.is_empty()
                && !words::function(&token.plain)
                && !words::names(&token.plain, reflex)
                && !own.contains(&token.plain)
                && !found
                    .iter()
                    .any(|held| held.span.start() <= token.from && token.to <= held.span.end())
        })
        .collect();
    let mut proposals = Vec::new();
    for (at, key) in list.keys().enumerate() {
        if found.iter().any(|held| held.key == *key) {
            continue;
        }
        let best = free
            .iter()
            .filter_map(|token| Some((by(&token.plain, at, &keys)?, token.from, token.to)))
            .min_by_key(|(by, from, _)| (*by, *from));
        if let Some((by, from, to)) = best
            && let Some(span) = Span::of(input, from, to)
        {
            proposals.push(Proposal {
                key: key.clone(),
                span,
                by,
            });
        }
    }
    proposals
}

/// How one word of the input proposes the listed word at `at`, if it does: the word as compared (`Token::plain`),
/// and the list's words as `words.rs` names them.
fn by(plain: &str, at: usize, keys: &[String]) -> Option<By> {
    let key = &keys[at];
    let word = bare(plain);
    if word.is_empty() || word == *key {
        return None;
    }
    let letters = word.chars().all(char::is_alphabetic);
    let length = word.chars().count();
    let marked = plain.chars().any(|c| MARKS.contains(&c));
    if marked {
        let joined: String = word.chars().filter(|c| !JOINS.contains(c)).collect();
        let listed: String = key.chars().filter(|c| !JOINS.contains(c)).collect();
        if !joined.is_empty()
            && joined.chars().all(char::is_alphabetic)
            && words::distance(&joined, &listed) <= words::radius(key, keys)
            && words::radius(key, keys) > 0
        {
            return Some(By::Slip);
        }
    }
    if letters
        && length >= PREFIX
        && key.chars().count() >= PREFIXED
        && key.starts_with(&word)
        && keys.iter().filter(|other| other.starts_with(&word)).count() == 1
    {
        return Some(By::Prefix);
    }
    if letters && diminutive(&word, keys) == Some(at) {
        return Some(By::Diminutive);
    }
    if letters && length > key.chars().count() && word.starts_with(key.as_str()) {
        let rest = &word[key.len()..];
        let doubled = key
            .chars()
            .last()
            .is_some_and(|last| rest == last.to_string());
        if !PLURAL.contains(&rest) && !doubled {
            return Some(By::Longer);
        }
    }
    if letters && key.chars().count() < SHORT && words::distance(&word, key) == 1 {
        return Some(By::Short);
    }
    None
}

/// The one listed word a pet form shares a line of `DIMINUTIVES` with, by its place; none where the word is no pet
/// form, or shares its lines with no listed word or with more than one. A full name is no pet form: «Samuel» is a
/// longer word, never sam's diminutive.
fn diminutive(word: &str, keys: &[String]) -> Option<usize> {
    let rows: Vec<&(&str, Vec<&str>)> = diminutives()
        .iter()
        .filter(|(_, forms)| forms.contains(&word))
        .collect();
    let mut linked = keys.iter().enumerate().filter(|(_, key)| {
        key.as_str() != word
            && rows
                .iter()
                .any(|(name, forms)| *name == key.as_str() || forms.contains(&key.as_str()))
    });
    let (at, _) = linked.next()?;
    linked.next().is_none().then_some(at)
}

/// A word as it is compared: lower case, a channel's `#` and a possessive's ending dropped.
fn bare(word: &str) -> String {
    let lowered = words::named(&fold(word));
    POSSESSIVE
        .iter()
        .find_map(|ending| lowered.strip_suffix(ending))
        .unwrap_or(&lowered)
        .to_owned()
}

/// Whether words are a given name.
#[must_use]
pub fn given(words: &str) -> bool {
    given_names().binary_search(&bare(words).as_str()).is_ok()
}

/// Whether words that hold or propose a listed word are a given name that is not the listed word, nor a word of what
/// it means: another person's name, which is never read as it.
#[must_use]
pub fn another(words: &str, key: &Key, meaning: &Clean) -> bool {
    let word = bare(words);
    let listed = words::named(&fold(key.as_str()));
    let meant = fold(meaning.as_str())
        .split(|c: char| !c.is_alphanumeric())
        .any(|said| said == word);
    given(&word) && word != listed && !meant
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(words: &[(&str, &str)]) -> IndexMap<Key, Clean> {
        words
            .iter()
            .map(|(key, meaning)| (Key::new(key).unwrap(), Clean::new(meaning).unwrap()))
            .collect()
    }

    fn proposals(text: &str, words: &[(&str, &str)]) -> Vec<(String, String, By)> {
        let input = Input::new(text).unwrap();
        let list = list(words);
        let found = words::listed(&input, &list);
        proposed(&input, &list, &found, "find", &[])
            .into_iter()
            .map(|p| (p.key.to_string(), p.span.text().to_string(), p.by))
            .collect()
    }

    const PEOPLE: [(&str, &str); 3] = [
        ("sam", "Sam, a colleague."),
        ("ana", "Ana, a colleague."),
        ("jo", "Jo, a colleague."),
    ];
    const MONTHS: [(&str, &str); 3] = [
        ("august", "August."),
        ("september", "September."),
        ("october", "October."),
    ];

    #[test]
    fn the_list_is_sorted_and_each_name_once() {
        assert!(given_names().windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            diminutives()
                .iter()
                .all(|(name, forms)| !name.is_empty() && !forms.is_empty())
        );
        assert!(given("Samuel's") && given("MARIJA") && !given("laptop"));
    }

    #[test]
    fn a_kin_name_proposes_and_is_no_finding() {
        assert_eq!(
            proposals("where is Samuel's laptop", &PEOPLE),
            [("sam".into(), "Samuel's".into(), By::Longer)]
        );
        assert_eq!(
            proposals("does Sammy still have a badge", &PEOPLE),
            [("sam".into(), "Sammy".into(), By::Diminutive)]
        );
        assert_eq!(
            proposals("does Anna still have a badge", &PEOPLE),
            [("ana".into(), "Anna".into(), By::Short)]
        );
        assert_eq!(
            proposals("is Joe signed in anywhere", &PEOPLE),
            [("jo".into(), "Joe".into(), By::Diminutive)]
        );
    }

    #[test]
    fn a_prefix_proposes_one_word_of_five_or_more() {
        assert_eq!(
            proposals("any invoices in Sept", &MONTHS),
            [("september".into(), "Sept".into(), By::Prefix)]
        );
        assert_eq!(
            proposals("the statement we got in aug", &MONTHS),
            [("august".into(), "aug".into(), By::Prefix)]
        );
        // «augusta» is august's spelling, which `words.rs` finds: no proposal.
        assert!(proposals("any invoices in augusta", &MONTHS).is_empty());
        let shipments = [
            ("rotterdam", "The container coming through Rotterdam."),
            ("hamburg", "The pallets coming through Hamburg."),
        ];
        assert_eq!(
            proposals("is the hamburger load on its way", &shipments),
            [("hamburg".into(), "hamburger".into(), By::Longer)]
        );
        // «oc» is under three letters; «sam» is the word itself.
        assert!(proposals("any invoices in oc", &MONTHS).is_empty());
        assert!(proposals("has sam's laptop turned up", &PEOPLE).is_empty());
    }

    #[test]
    fn a_slip_beside_an_apostrophe_or_a_hyphen_proposes() {
        let regions = [
            ("eu-west", "Europe, the Ireland region."),
            ("us-east", "The United States, the Virginia region."),
        ];
        assert_eq!(
            proposals("how is payments doing in eu-wwest", &regions),
            [("eu-west".into(), "eu-wwest".into(), By::Slip)]
        );
        let people = [
            ("maria", "Maria, a backend engineer."),
            ("sam", "Sam, a salesperson."),
        ];
        assert_eq!(
            proposals("when does Maira's contract start", &people),
            [("maria".into(), "Maira's".into(), By::Slip)]
        );
        // A plural and a last letter doubled are no longer word; the doubled one is a short word's edit.
        assert!(proposals("list the sams", &PEOPLE).is_empty());
        assert_eq!(
            proposals("where is samm's laptop", &PEOPLE),
            [("sam".into(), "samm's".into(), By::Short)]
        );
    }

    #[test]
    fn a_word_the_reflex_says_of_itself_proposes_nothing() {
        let input = Input::new("check the errors for the shop").unwrap();
        let services = list(&[
            ("checkout", "The checkout service."),
            ("search", "The search service."),
        ]);
        let found = words::listed(&input, &services);
        let alone = proposed(&input, &services, &found, "errors", &[]);
        assert_eq!(alone.len(), 1, "«check» begins checkout alone: {alone:?}");
        let own = ["check".to_owned(), "errors".to_owned()];
        assert!(proposed(&input, &services, &found, "errors", &own).is_empty());
    }

    #[test]
    fn another_persons_name_is_not_the_listed_word() {
        let sam = Key::new("sam").unwrap();
        let maria = Key::new("maria").unwrap();
        let meaning = Clean::new("Sam, a colleague.").unwrap();
        assert!(another("Samuel's", &sam, &meaning));
        assert!(!another("Sam's", &sam, &meaning));
        assert!(another(
            "Marija",
            &maria,
            &Clean::new("Maria, a backend engineer.").unwrap()
        ));
        // A name the listed word's own meaning holds is the author's word for it.
        assert!(!another(
            "Samuel",
            &sam,
            &Clean::new("Samuel Lee, sales.").unwrap()
        ));
        assert!(!another(
            "mraia",
            &maria,
            &Clean::new("Maria, a backend engineer.").unwrap()
        ));
    }
}
