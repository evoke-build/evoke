//! What code knows of a request before any question is asked. For a list, the words of the input that hold a
//! listed word: the word itself, another form of it, its stem, its spelling within a radius set by its length and
//! its neighbours on the list, or a word of its meaning. For a kind, a value spelled out in words with the form it
//! stands for, a day misspelt, a code typed with spaces. Code proposes; it takes nothing. In: an `Input`; a list's
//! keys with what each means; a kind and the input's candidates. Out: `Listed`, `Spelled`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::Key;
use crate::calendar::{Day, Weekday, Which};
use crate::manifest::Recognizer;
use crate::propose::{PickValue, Proposed, propose};
use crate::spoken::Shape;
use crate::text::{self, Clean, Input, Span, fold};

/// The words that carry no value by themselves: articles, pronouns, prepositions, auxiliaries, a courtesy, a
/// greeting. A run of them alone is no run, and none of them is a listed word misspelt or a word of a meaning.
pub(crate) const FUNCTION: [&str; 83] = [
    "a", "an", "the", "this", "that", "these", "those", "my", "your", "our", "his", "her", "its",
    "their", "me", "i", "you", "we", "it", "to", "of", "in", "on", "at", "for", "from", "with",
    "by", "as", "into", "onto", "and", "or", "then", "so", "but", "if", "is", "are", "was", "be",
    "been", "am", "do", "does", "did", "can", "could", "will", "would", "shall", "should", "may",
    "might", "must", "have", "has", "had", "not", "no", "please", "pls", "plz", "thanks", "thank",
    "thx", "kindly", "hey", "hi", "hello", "ok", "okay", "just", "now", "some", "any", "there",
    "here", "up", "out", "off", "over", "about",
];

/// The words of a meaning that say what kind of thing a listed word is, or join its clauses, and so name no one
/// word of the list.
const KIND: [&str; 7] = [
    "also", "where", "team", "channel", "folder", "service", "region",
];

/// What a sign is called when a value is said aloud, and the sign it stands for.
const SIGNS: [(&str, &str); 11] = [
    ("at", "@"),
    ("dot", "."),
    ("point", "."),
    ("dash", "-"),
    ("hyphen", "-"),
    ("minus", "-"),
    ("underscore", "_"),
    ("slash", "/"),
    ("colon", ":"),
    ("hashtag", "#"),
    ("hash", "#"),
];

/// The number words a value said aloud holds, each with its figures: «oh» and the letter «o» stand for zero.
const FIGURES: [(&str, &str); 22] = [
    ("zero", "0"),
    ("oh", "0"),
    ("o", "0"),
    ("one", "1"),
    ("two", "2"),
    ("three", "3"),
    ("four", "4"),
    ("five", "5"),
    ("six", "6"),
    ("seven", "7"),
    ("eight", "8"),
    ("nine", "9"),
    ("ten", "10"),
    ("eleven", "11"),
    ("twelve", "12"),
    ("thirteen", "13"),
    ("fourteen", "14"),
    ("fifteen", "15"),
    ("sixteen", "16"),
    ("seventeen", "17"),
    ("eighteen", "18"),
    ("nineteen", "19"),
];

/// The tens a value said aloud holds, each with its first figure.
const TENS: [(&str, &str); 8] = [
    ("twenty", "2"),
    ("thirty", "3"),
    ("forty", "4"),
    ("fifty", "5"),
    ("sixty", "6"),
    ("seventy", "7"),
    ("eighty", "8"),
    ("ninety", "9"),
];

/// The word that closes a number said aloud with two noughts: «twenty three hundred» is 2300.
const HUNDRED: &str = "hundred";

/// The days of the week, as a misspelt one is held against them.
const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// The words before a day that make it a recurrence, which is no day.
const EVERY: [&str; 2] = ["every", "each"];

/// The words a courtesy is made of: a request's «please», a «thanks» at its end.
const COURTESY: [&str; 10] = [
    "please", "pls", "plz", "thanks", "thank", "you", "thx", "cheers", "ta", "kindly",
];

/// The words before a weekday that say which one: «next monday».
const WHICH: [(&str, Which); 3] = [
    ("next", Which::Next),
    ("this", Which::This),
    ("last", Which::Last),
];

/// How the input's words hold a listed word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum How {
    /// The word itself.
    Same,
    /// Another form of it: «eu west» for `eu-west`.
    Form,
    /// The same word with an ending: a plural, a possessive.
    Stem,
    /// A spelling within the word's radius.
    Spelling,
    /// A word of what the listed word means.
    Meaning,
}

impl How {
    /// Whether the words are the listed word, in some form of it: a word of its meaning alone is not.
    #[must_use]
    pub fn firm(self) -> bool {
        self != Self::Meaning
    }
}

/// A listed word the input's own words hold: the word, where they stand, and how they hold it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listed {
    pub key: Key,
    pub span: Span,
    pub how: How,
}

/// How a form that no recognizer reads as typed was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Form {
    /// Said aloud: «sam at example dot com».
    Aloud,
    /// A day's name misspelt: «monady».
    Misspelt,
    /// A code typed with spaces: «BR 1187».
    Spaced,
}

/// A value the input's words spell out in a form no recognizer reads as typed: the words, the form they stand
/// for as it would be typed, what that form reads as, and how it was read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spelled {
    pub span: Span,
    pub typed: Clean,
    pub value: PickValue,
    pub form: Form,
    /// How a value read in its argument's kind stands to the shape the argument's examples share; none where no
    /// example was held against it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
}

/// A word of the input as typed: where it stands in characters, and the word as it is compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    /// Where the word as typed begins and ends, the marks at its edges included.
    pub start: usize,
    pub end: usize,
    /// Where the word as compared begins and ends.
    pub from: usize,
    pub to: usize,
    /// Lower case, the marks at its edges dropped but a leading `#` or `@` and a closing `%`.
    pub plain: String,
}

/// Whether a word carries no value by itself.
pub(crate) fn function(word: &str) -> bool {
    FUNCTION.contains(&word)
}

/// The input's words, each a run of characters no space divides.
pub(crate) fn tokens(input: &str) -> Vec<Token> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        tokens.push(token(&chars, start, i));
    }
    tokens
}

/// One word, with the part of it that is compared: what is left once the marks at its edges are dropped.
fn token(chars: &[char], start: usize, end: usize) -> Token {
    let word = &chars[start..end];
    let opens = |c: &char| c.is_alphanumeric() || matches!(c, '#' | '@');
    let closes = |c: &char| c.is_alphanumeric() || *c == '%';
    let from = word.iter().position(opens).unwrap_or(word.len());
    let to = word
        .iter()
        .rposition(closes)
        .map_or(from, |at| (at + 1).max(from));
    Token {
        start,
        end,
        from: start + from,
        to: start + to,
        plain: word[from..to]
            .iter()
            .flat_map(|c| c.to_lowercase())
            .collect(),
    }
}

/// The listed words the input's own words hold, one finding a word at most and the best one: the word itself
/// before another form of it, a form before its stem, a stem before a spelling, the nearer spelling before the
/// farther, and a word of its meaning only when nothing else holds it.
#[must_use]
pub fn listed(input: &Input, list: &IndexMap<Key, Clean>) -> Vec<Listed> {
    let tokens = tokens(input.as_str());
    let words: Vec<String> = tokens
        .iter()
        .map(|token| named(&fold(&token.plain)))
        .collect();
    let keys: Vec<String> = list.keys().map(|key| named(&fold(key.as_str()))).collect();
    let mut found = Vec::new();
    let meanings: Vec<Vec<String>> = list.values().map(said_of).collect();
    for (at, (key, word)) in list.keys().zip(&keys).enumerate() {
        let held = holding(word, &keys, &words).or_else(|| meant(at, &meanings, &words));
        if let Some((from, to, how)) = held
            && let Some(span) = Span::of(input, tokens[from].from, tokens[to].to)
        {
            found.push(Listed {
                key: key.clone(),
                span,
                how,
            });
        }
    }
    found
}

/// A word as a list names it: a channel's `#` is no part of the name.
pub(crate) fn named(word: &str) -> String {
    word.strip_prefix('#').unwrap_or(word).to_owned()
}

/// The words that hold a listed word by what it is written as: from which to which, and how.
fn holding(key: &str, keys: &[String], words: &[String]) -> Option<(usize, usize, How)> {
    let spaced = key.replace(['-', '_'], " ");
    let joined: String = key
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | ' '))
        .collect();
    let radius = radius(key, keys);
    let mut best: Option<(usize, usize, How, usize)> = None;
    for i in 0..words.len() {
        for n in 1..=3 {
            if i + n > words.len() || words[i..i + n].iter().any(String::is_empty) {
                continue;
            }
            let run = words[i..i + n].join(" ");
            let tight: String = words[i..i + n].concat();
            let read = if run == key {
                Some((How::Same, 0))
            } else if run == spaced || tight == key || tight == joined || run == joined {
                Some((How::Form, 0))
            } else if n == 1 && stem(&run) == stem(key) {
                Some((How::Stem, 0))
            } else if n == 1
                && radius > 0
                && run.chars().all(char::is_alphabetic)
                && !function(&run)
            {
                let distance = distance(&run, key).min(distance(&stem(&run), &stem(key)));
                (distance <= radius).then_some((How::Spelling, distance))
            } else {
                None
            };
            if let Some((how, distance)) = read
                && best.is_none_or(|(_, _, held, at)| (how, distance) < (held, at))
            {
                best = Some((i, i + n - 1, how, distance));
            }
        }
    }
    best.map(|(from, to, how, _)| (from, to, how))
}

/// The words of a meaning that may name its listed word: longer than three letters, no function word and no
/// word for a kind of thing.
fn said_of(meaning: &Clean) -> Vec<String> {
    fold(meaning.as_str())
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() > 3 && !function(word) && !KIND.contains(word))
        .map(str::to_owned)
        .collect()
}

/// The word of the input that is a word of a listed word's meaning, when one is: the word itself, its stem, or,
/// for a word of six letters or more, a spelling one step away. A word that another listed word's meaning holds
/// too says what kind of thing the list holds, and names no one word of it.
fn meant(at: usize, meanings: &[Vec<String>], words: &[String]) -> Option<(usize, usize, How)> {
    let shared = |word: &String| {
        meanings
            .iter()
            .enumerate()
            .any(|(other, said)| other != at && said.contains(word))
    };
    let said: Vec<&String> = meanings[at].iter().filter(|word| !shared(word)).collect();
    words
        .iter()
        .position(|word| {
            let length = word.chars().count();
            length >= 4
                && !function(word)
                && said.iter().any(|meant| {
                    *meant == word
                        || stem(meant) == stem(word)
                        || (length >= 6 && distance(meant, word) <= 1)
                })
        })
        .map(|at| (at, at, How::Meaning))
}

/// How far a spelling may stray from a listed word and still be proposed: none under four letters, one up to
/// seven, two beyond; and never past half the way to the word's nearest neighbour on its list.
pub(crate) fn radius(word: &str, list: &[String]) -> usize {
    let by_length = by_length(word);
    let nearest = list
        .iter()
        .filter(|other| other.as_str() != word)
        .map(|other| distance(other, word))
        .min()
        .unwrap_or(99);
    by_length.min(nearest.saturating_sub(1) / 2)
}

/// The radius a word's length alone allows.
fn by_length(word: &str) -> usize {
    match word.chars().count() {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

/// A word without the ending a plural or a possessive adds.
fn stem(word: &str) -> String {
    let length = word.chars().count();
    if let Some(head) = word.strip_suffix("sses") {
        return format!("{head}ss");
    }
    if let Some(head) = word.strip_suffix("ies")
        && length > 4
    {
        return format!("{head}y");
    }
    // A possessive's ending comes off first, and what is left may be a plural: «payments's».
    if let Some(head) = word
        .strip_suffix("'s")
        .or_else(|| word.strip_suffix("\u{2019}s"))
    {
        return stem(head);
    }
    if let Some(head) = word.strip_suffix('s')
        && !word.ends_with("ss")
        && length > 3
    {
        return head.to_owned();
    }
    word.to_owned()
}

/// The distance between two spellings: a letter put in, left out or changed, and two neighbours swapped, each
/// one step.
pub(crate) fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut rows = vec![vec![0_usize; b.len() + 1]; a.len() + 1];
    for (i, row) in rows.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in rows[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut least = (rows[i - 1][j] + 1)
                .min(rows[i][j - 1] + 1)
                .min(rows[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                least = least.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = least;
        }
    }
    rows[a.len()][b.len()]
}

/// Whether words are politeness and nothing else: «thanks», «thank you», «please».
#[must_use]
pub(crate) fn courtesy(text: &str) -> bool {
    let words: Vec<String> = tokens(text).into_iter().map(|token| token.plain).collect();
    !words.is_empty()
        && words
            .iter()
            .all(|word| word.is_empty() || COURTESY.contains(&word.as_str()))
}

/// Whether the words of a finding say what to do, and so support no value: the reflex's own name in any number or
/// tense, unless they are the listed word itself as its list writes it and not the name as it is written, which
/// names the thing: «Downloads», the folder, beside `download`.
#[must_use]
pub(crate) fn says_what_to_do(held: &Listed, reflex: &str) -> bool {
    let said = held.span.text().as_str();
    names(said, reflex) && (held.how != How::Same || fold(said) == fold(reflex))
}

/// Whether a word of the request is a reflex's own name, whatever its number or tense: «downloaded» is
/// `download`, «notes» is `note`. Such a word says what to do, and supports no value.
#[must_use]
pub(crate) fn names(word: &str, reflex: &str) -> bool {
    let word = fold(word)
        .trim_matches(|c: char| ".,;:!?\"'".contains(c))
        .to_owned();
    let name = fold(reflex);
    let forms = [
        word.clone(),
        word.trim_end_matches('e').to_owned(),
        root(&word),
        root(&word).trim_end_matches('e').to_owned(),
    ];
    forms.contains(&name)
        || forms.contains(&name.trim_end_matches('e').to_owned())
        || word == root(&name)
}

/// A word without what number or tense adds: notes, noted, noting are note.
fn root(word: &str) -> String {
    let length = word.chars().count();
    ["ing", "ed", "es", "s", "d"]
        .into_iter()
        .find_map(|ending| {
            word.strip_suffix(ending)
                .filter(|_| length - ending.len() >= 3)
        })
        .unwrap_or(word)
        .to_owned()
}

/// The values of a kind the input's words spell out in a form its recognizer does not read as typed: said aloud,
/// for an address, a link, a code and a number; misspelt, for a day; typed with spaces, for a code. None stands
/// on words a candidate holds beyond it: «ninety four» is no number inside «two hundred ninety four». A
/// candidate that lies inside the words is a part of what they spell, as the figures of «BR 1187» are.
#[must_use]
pub fn spelled(input: &Input, kind: Recognizer, proposed: &[Proposed]) -> Vec<Spelled> {
    let tokens = tokens(input.as_str());
    let mut found = match kind {
        Recognizer::Email | Recognizer::Url | Recognizer::Number => aloud(input, &tokens, kind),
        Recognizer::Code => {
            let mut found = spaced(input, &tokens);
            // The same words read both ways are a code typed with spaces.
            let again: Vec<Spelled> = aloud(input, &tokens, kind)
                .into_iter()
                .filter(|aloud| !found.iter().any(|spaced| spaced.span == aloud.span))
                .collect();
            found.extend(again);
            found
        }
        Recognizer::Date => misspelt(input, &tokens),
        Recognizer::Duration | Recognizer::Quoted | Recognizer::Time | Recognizer::Amount => {
            Vec::new()
        }
    };
    found.retain(|spelled| {
        proposed.iter().all(|held| {
            let apart =
                held.span.end() <= spelled.span.start() || spelled.span.end() <= held.span.start();
            let inside = spelled.span.start() <= held.span.start()
                && held.span.end() <= spelled.span.end()
                && held.span != spelled.span;
            apart || inside
        })
    });
    found.sort_by_key(|spelled| spelled.span.start());
    found
}

/// What a word is in a form said aloud.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Said {
    /// «at», «dot».
    Sign,
    /// A number word, or figures.
    Number,
    /// One letter.
    Letter,
    /// Any other word of letters and figures.
    Word,
    /// A word that cannot stand in what is typed.
    None,
}

/// Letters and figures as an address, a link and a code are typed: the alphabet they are written in.
fn typed_word(word: &str) -> bool {
    !word.is_empty() && word.chars().all(|c| text::small(c) || text::figure(c))
}

fn sign(word: &str) -> Option<&'static str> {
    SIGNS
        .iter()
        .find(|(name, _)| *name == word)
        .map(|(_, sign)| *sign)
}

fn figures(word: &str) -> Option<&'static str> {
    FIGURES
        .iter()
        .find(|(name, _)| *name == word)
        .map(|(_, figures)| *figures)
}

fn tens(word: &str) -> Option<&'static str> {
    TENS.iter()
        .find(|(name, _)| *name == word)
        .map(|(_, figure)| *figure)
}

fn said(word: &str) -> Said {
    if sign(word).is_some() {
        Said::Sign
    } else if (figures(word).is_some() && word != "o")
        || tens(word).is_some()
        || word == HUNDRED
        || (!word.is_empty() && word.chars().all(text::figure))
    {
        Said::Number
    } else if word.chars().count() == 1 && word.chars().all(text::small) {
        Said::Letter
    } else if typed_word(word) {
        Said::Word
    } else {
        Said::None
    }
}

/// A run of words said aloud, written as it would be typed: «dana dot weiss at example dot org», «five dot oh
/// dot two», «lh eleven sixty seven», «v x dash twenty three hundred». None when the run holds nothing said
/// aloud, or a word that cannot stand in what is typed.
fn typed(words: &[&str]) -> Option<String> {
    let mut out = String::new();
    let mut spoke = false;
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        if let Some(sign) = sign(word)
            && i > 0
            && i + 1 < words.len()
        {
            out.push_str(sign);
            spoke = true;
        } else if let Some(ten) = tens(word) {
            let unit = words
                .get(i + 1)
                .filter(|next| !matches!(**next, "zero" | "oh" | "o"))
                .and_then(|next| figures(next))
                .filter(|figures| figures.len() == 1);
            out.push_str(ten);
            match unit {
                Some(unit) => {
                    out.push_str(unit);
                    i += 1;
                }
                None => out.push('0'),
            }
            spoke = true;
        } else if let Some(figures) = figures(word).filter(|_| word != "o") {
            out.push_str(figures);
            spoke = true;
        } else if word == HUNDRED {
            // Two noughts after a number, and only where the number ends there.
            let after_number =
                i > 0 && said(words[i - 1]) == Said::Number && words[i - 1] != HUNDRED;
            let ends = words
                .get(i + 1)
                .is_none_or(|next| said(next) != Said::Number);
            if !(after_number && ends) {
                return None;
            }
            out.push_str("00");
        } else if typed_word(word) {
            out.push_str(word);
        } else {
            return None;
        }
        i += 1;
    }
    spoke.then_some(out)
}

/// The runs of the input's words that read as one value said aloud: signs, numbers and letters in a row, and a
/// word only where a sign joins it to the run, so that two words side by side end a run; one number alone is
/// no run. A short word before the numbers, «lh eleven sixty seven», is proposed both with and without. Kept
/// where, written as typed, the recognizer of the kind reads it whole.
fn aloud(input: &Input, tokens: &[Token], kind: Recognizer) -> Vec<Spelled> {
    let words: Vec<&str> = tokens.iter().map(|token| token.plain.as_str()).collect();
    let kinds: Vec<Said> = words.iter().map(|word| said(word)).collect();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if matches!(kinds[i], Said::None | Said::Sign) {
            i += 1;
            continue;
        }
        let mut j = i;
        let mut spoke = kinds[i] == Said::Number;
        while j + 1 < words.len() {
            let (here, next) = (kinds[j], kinds[j + 1]);
            if next == Said::None {
                break;
            }
            if next == Said::Sign {
                // A sign joins only what follows it.
                if j + 2 >= words.len() || matches!(kinds[j + 2], Said::None | Said::Sign) {
                    break;
                }
                j += 2;
                spoke = true;
                continue;
            }
            if next == Said::Word || here == Said::Word {
                break;
            }
            j += 1;
            spoke |= next == Said::Number;
        }
        if spoke && j > i {
            runs.push((i, j));
        }
        i = j + 1;
    }
    let mut tried: Vec<(usize, usize)> = Vec::new();
    let mut found = Vec::new();
    let mut offer = |from: usize, to: usize| {
        if tried.contains(&(from, to)) {
            return;
        }
        tried.push((from, to));
        let Some(form) = typed(&words[from..=to]) else {
            return;
        };
        if let Some(value) = read_whole(&form, kind)
            && let (Some(span), Ok(typed)) = (
                Span::of(input, tokens[from].from, tokens[to].to),
                Clean::new(&form),
            )
        {
            found.push(Spelled {
                span,
                typed,
                value,
                form: Form::Aloud,
                shape: None,
            });
        }
    };
    for (from, to) in runs {
        offer(from, to);
        if from > 0
            && kinds[from] == Said::Number
            && kinds[from - 1] == Said::Word
            && words[from - 1].chars().count() <= 3
        {
            offer(from - 1, to);
        }
        if kinds[from] == Said::Word && from < to && kinds[from + 1] == Said::Number {
            offer(from + 1, to);
        }
    }
    found
}

/// What a form reads as when the kind's recognizer reads the whole of it.
pub(crate) fn read_whole(form: &str, kind: Recognizer) -> Option<PickValue> {
    let input = Input::new(form).ok()?;
    let whole = form.chars().count();
    propose(&input)
        .into_iter()
        .find(|proposed| {
            reads(kind, &proposed.value)
                && proposed.span.start() == 0
                && proposed.span.end() == whole
        })
        .map(|proposed| proposed.value)
}

/// Whether a value is of the kind.
fn reads(kind: Recognizer, value: &PickValue) -> bool {
    matches!(
        (kind, value),
        (Recognizer::Number, PickValue::Number { .. })
            | (Recognizer::Email, PickValue::Email { .. })
            | (Recognizer::Url, PickValue::Url { .. })
            | (Recognizer::Code, PickValue::Code { .. })
    )
}

/// A day's name misspelt: a word of letters within the radius its day's length allows, two neighbours swapped
/// counting as one step, and farther from every other day; «munday», as near to sunday, is none. The word before
/// it that says which, «next monady», is part of it. A day in the plural with no apostrophe, «wednesdays
/// meetings», is one step from its day and falls under the rule; a day after «every» or «each» is a recurrence.
fn misspelt(input: &Input, tokens: &[Token]) -> Vec<Spelled> {
    let mut found = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        let word = token.plain.as_str();
        let recurs = i > 0 && EVERY.contains(&tokens[i - 1].plain.as_str());
        if word.is_empty()
            || !word.chars().all(char::is_alphabetic)
            || WEEKDAYS.contains(&word)
            || recurs
        {
            continue;
        }
        let mut near: Vec<(usize, usize)> = WEEKDAYS
            .iter()
            .enumerate()
            .map(|(at, day)| (distance(word, day), at))
            .collect();
        near.sort_unstable();
        let [(nearest, at), (next, _), ..] = near[..] else {
            continue;
        };
        if nearest == 0 || nearest > by_length(WEEKDAYS[at]) || next == nearest {
            continue;
        }
        let which = i.checked_sub(1).and_then(|before| {
            WHICH
                .iter()
                .find(|(word, _)| *word == tokens[before].plain)
                .map(|(_, which)| (before, *which))
        });
        let start = which.map_or(token.from, |(before, _)| tokens[before].from);
        let which = which.map(|(_, which)| which);
        let shown = match which {
            Some(which) => format!("{} {}", said_which(which), WEEKDAYS[at]),
            None => WEEKDAYS[at].to_owned(),
        };
        if let (Some(span), Ok(typed)) = (Span::of(input, start, token.to), Clean::new(&shown)) {
            found.push(Spelled {
                span,
                typed,
                value: PickValue::Date {
                    value: Day::Weekday {
                        weekday: Weekday::ALL[at],
                        which,
                    },
                },
                form: Form::Misspelt,
                shape: None,
            });
        }
    }
    found
}

/// The word that says which weekday, as it is typed.
fn said_which(which: Which) -> &'static str {
    WHICH
        .iter()
        .find(|(_, said)| *said == which)
        .map_or("", |(word, _)| *word)
}

/// A part of a code typed with spaces, by its shape alone.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    /// Figures alone: «1187».
    Figures,
    /// Letters and figures: «C02».
    Mixed,
    /// Capital letters alone: «BR».
    Capitals,
    /// Small letters alone, three at most: «lh», «hnk».
    Small(usize),
    /// No part of a code: a function word, a longer word in small letters, a word with a mark inside it.
    None,
}

fn part(token: &Token, typed: &str) -> Part {
    let typed_whole = !typed.is_empty() && typed.chars().all(text::latin_or_figure);
    if !typed_whole || function(&token.plain) || token.from != token.start {
        return Part::None;
    }
    let letters = typed.chars().filter(|c| text::latin(*c)).count();
    let length = typed.chars().count();
    if letters == 0 {
        Part::Figures
    } else if letters < length {
        Part::Mixed
    } else if typed.chars().all(text::capital) {
        Part::Capitals
    } else if length <= 3 && typed.chars().all(text::small) {
        Part::Small(length)
    } else {
        Part::None
    }
}

/// The most parts a code typed with spaces is read over.
const MOST_PARTS: usize = 6;

/// A code typed with spaces: a run of neighbouring words, each a part of a code by its shape, that reads whole
/// in the code's grammar once joined. Letters before figures are a ticket, joined by a dash, «BR 1187»; any
/// other run is joined by nothing, «C02 YT8 HNK P3W». A run opens with capitals, figures or two small letters,
/// ends on capitals or figures, holds a figure, and stops at a mark: «BR 1187,» ends there.
fn spaced(input: &Input, tokens: &[Token]) -> Vec<Spelled> {
    let typed: Vec<String> = tokens
        .iter()
        .map(|token| chars_of(input, token.from, token.to))
        .collect();
    let parts: Vec<Part> = tokens
        .iter()
        .zip(&typed)
        .map(|(token, typed)| part(token, typed))
        .collect();
    let mut found = Vec::new();
    let mut from = 0;
    while from < tokens.len() {
        let opens = !matches!(parts[from], Part::None | Part::Small(3));
        if !opens {
            from += 1;
            continue;
        }
        let mut to = from;
        while to + 1 < tokens.len()
            && to + 1 - from < MOST_PARTS
            && parts[to + 1] != Part::None
            && tokens[to].to == tokens[to].end
        {
            to += 1;
        }
        while to > from && matches!(parts[to], Part::Small(_)) {
            to -= 1;
        }
        found.extend(joined(input, tokens, &typed, &parts, from, to));
        from = to + 1;
    }
    found
}

/// What a run of parts reads as: each ticket it holds, letters then figures, joined by a dash; else the run
/// whole, joined by nothing, when it holds a figure.
fn joined(
    input: &Input,
    tokens: &[Token],
    typed: &[String],
    parts: &[Part],
    from: usize,
    to: usize,
) -> Vec<Spelled> {
    let read = |a: usize, b: usize, form: String| {
        let value = read_whole(&form, Recognizer::Code)?;
        Some(Spelled {
            span: Span::of(input, tokens[a].from, tokens[b].to)?,
            typed: Clean::new(&form).ok()?,
            value,
            form: Form::Spaced,
            shape: None,
        })
    };
    let tickets: Vec<Spelled> = (from..to)
        .filter(|&at| {
            matches!(parts[at], Part::Capitals | Part::Small(_)) && parts[at + 1] == Part::Figures
        })
        .filter_map(|at| read(at, at + 1, format!("{}-{}", typed[at], typed[at + 1])))
        .collect();
    if !tickets.is_empty() || to == from {
        return tickets;
    }
    let figured = parts[from..=to]
        .iter()
        .any(|part| matches!(part, Part::Figures | Part::Mixed));
    if !figured {
        return Vec::new();
    }
    read(from, to, typed[from..=to].concat())
        .into_iter()
        .collect()
}

/// The characters `from..to` of an input, as typed.
fn chars_of(input: &Input, from: usize, to: usize) -> String {
    input.as_str().chars().skip(from).take(to - from).collect()
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

    fn found(text: &str, words: &[(&str, &str)]) -> Vec<(String, String, How)> {
        listed(&Input::new(text).unwrap(), &list(words))
            .into_iter()
            .map(|found| {
                (
                    found.key.to_string(),
                    found.span.text().to_string(),
                    found.how,
                )
            })
            .collect()
    }

    const REGIONS: [(&str, &str); 2] = [
        ("eu-west", "The region in Ireland."),
        ("us-east", "The region in Virginia."),
    ];

    #[test]
    fn a_listed_word_is_found_under_its_forms() {
        assert_eq!(
            found("errors in eu-west", &REGIONS),
            [("eu-west".to_owned(), "eu-west".to_owned(), How::Same)]
        );
        assert_eq!(
            found("errors in EU West, please", &REGIONS),
            [("eu-west".to_owned(), "EU West".to_owned(), How::Form)]
        );
        assert_eq!(
            found("errors in euwest", &REGIONS),
            [("eu-west".to_owned(), "euwest".to_owned(), How::Form)]
        );
        assert_eq!(
            found("what failed in ireland", &REGIONS),
            [("eu-west".to_owned(), "ireland".to_owned(), How::Meaning)]
        );
        assert!(found("what failed", &REGIONS).is_empty());
    }

    #[test]
    fn a_stem_and_a_spelling_hold_a_word() {
        let people = [("maria", "Maria."), ("sam", "Sam."), ("ana", "Ana.")];
        assert_eq!(
            found("when does Maria's laptop come", &people),
            [("maria".to_owned(), "Maria's".to_owned(), How::Stem)]
        );
        assert_eq!(
            found("look up mraia", &people),
            [("maria".to_owned(), "mraia".to_owned(), How::Spelling)]
        );
        // Under four letters a word has no radius: «anna» is no `ana`, and the person is asked.
        assert!(found("look up anna", &people).is_empty());
        // A channel's `#` is no part of its name.
        let channels = [("#incident", "Where an outage is handled.")];
        assert_eq!(
            found("tell incident we are on it", &channels),
            [("#incident".to_owned(), "incident".to_owned(), How::Same)]
        );
        assert_eq!(
            found("tell the outage people", &channels),
            [("#incident".to_owned(), "outage".to_owned(), How::Meaning)]
        );
    }

    #[test]
    fn a_radius_stops_halfway_to_the_nearest_neighbour() {
        let months = [("june", "June."), ("july", "July.")];
        // `june` and `july` sit two apart: a spelling one step from one is as near the other.
        assert!(found("the books for jule", &months).is_empty());
        let one = [("september", "September.")];
        assert_eq!(
            found("the books for setpember", &one),
            [(
                "september".to_owned(),
                "setpember".to_owned(),
                How::Spelling
            )]
        );
        assert_eq!(radius("september", &["september".to_owned()]), 2);
        assert_eq!(radius("june", &["june".to_owned(), "july".to_owned()]), 0);
        assert_eq!(distance("monady", "monday"), 1);
        assert_eq!(distance("anna", "ana"), 1);
        assert_eq!(distance("", "abc"), 3);
    }

    #[test]
    fn the_actions_own_name_is_no_value() {
        assert!(names("downloaded", "download"));
        assert!(names("Notes:", "note"));
        assert!(names("noting", "note"));
        assert!(!names("downloads", "upload"));
    }

    fn spelt(text: &str, kind: Recognizer) -> Vec<(String, String, Form)> {
        let input = Input::new(text).unwrap();
        spelled(&input, kind, &propose(&input))
            .into_iter()
            .map(|spelled| {
                (
                    spelled.span.text().to_string(),
                    spelled.typed.to_string(),
                    spelled.form,
                )
            })
            .collect()
    }

    #[test]
    fn a_value_said_aloud_is_written_as_typed() {
        assert_eq!(
            spelt(
                "write to dana dot weiss at example dot org",
                Recognizer::Email
            ),
            [(
                "dana dot weiss at example dot org".to_owned(),
                "dana.weiss@example.org".to_owned(),
                Form::Aloud
            )]
        );
        assert_eq!(
            spelt("roll back five dot oh dot two", Recognizer::Code),
            [(
                "five dot oh dot two".to_owned(),
                "5.0.2".to_owned(),
                Form::Aloud
            )]
        );
        assert_eq!(
            spelt("set aside v x dash twenty three hundred", Recognizer::Code),
            [(
                "v x dash twenty three hundred".to_owned(),
                "vx-2300".to_owned(),
                Form::Aloud
            )]
        );
        assert_eq!(
            spelt("book me on q r nineteen forty", Recognizer::Code),
            [(
                "q r nineteen forty".to_owned(),
                "qr1940".to_owned(),
                Form::Aloud
            )]
        );
        // A form the recognizer reads as typed is a candidate already, and no word of it is spelled again.
        assert!(spelt("write to dana@example.org", Recognizer::Email).is_empty());
        assert!(spelt("look up incident 311", Recognizer::Number).is_empty());
        assert!(spelt("set it to two hundred ninety four", Recognizer::Number).is_empty());
    }

    #[test]
    fn a_day_misspelt_is_one_day_and_no_other() {
        assert_eq!(
            spelt("am i in anything on mondya", Recognizer::Date),
            [("mondya".to_owned(), "monday".to_owned(), Form::Misspelt)]
        );
        assert_eq!(
            spelt("move them to next thrusday", Recognizer::Date),
            [(
                "next thrusday".to_owned(),
                "next thursday".to_owned(),
                Form::Misspelt
            )]
        );
        assert_eq!(
            spelt("show me fridays meetings", Recognizer::Date),
            [("fridays".to_owned(), "friday".to_owned(), Form::Misspelt)]
        );
        // As near to sunday as to monday: asked.
        assert!(spelt("what is on munday", Recognizer::Date).is_empty());
        assert!(spelt("what is on monday", Recognizer::Date).is_empty());
        assert!(spelt("what is on my mind", Recognizer::Date).is_empty());
    }

    #[test]
    fn a_code_typed_with_spaces_reads_once_joined() {
        assert_eq!(
            spelt("reserve all of HS 0409", Recognizer::Code),
            [("HS 0409".to_owned(), "HS-0409".to_owned(), Form::Spaced)]
        );
        assert_eq!(
            spelt("wipe C02 XK1 ABJ G5M now", Recognizer::Code),
            [(
                "C02 XK1 ABJ G5M".to_owned(),
                "C02XK1ABJG5M".to_owned(),
                Form::Spaced
            )]
        );
        assert_eq!(
            spelt("wipe c02 xk1 abj g5m too", Recognizer::Code),
            [(
                "c02 xk1 abj g5m".to_owned(),
                "c02xk1abjg5m".to_owned(),
                Form::Spaced
            )]
        );
        // A number that belongs to another argument stands outside the ticket beside it.
        assert_eq!(
            spelt("hold 50 of br 2210", Recognizer::Code),
            [("br 2210".to_owned(), "br-2210".to_owned(), Form::Spaced)]
        );
        assert_eq!(
            spelt("hold 50 BR 2210, thanks", Recognizer::Code),
            [("BR 2210".to_owned(), "BR-2210".to_owned(), Form::Spaced)]
        );
        // A code typed whole is a candidate; a word before a number is no part of a code, nor a quantity.
        assert!(spelt("reserve HS-0409", Recognizer::Code).is_empty());
        assert!(spelt("reserve 50 of them", Recognizer::Code).is_empty());
        assert!(spelt("look up incident 311", Recognizer::Code).is_empty());
        assert!(spelt("add 5 to room 12", Recognizer::Code).is_empty());
        assert!(spelt("a timer for 10 minutes", Recognizer::Code).is_empty());
        assert!(spelt("wait ab 10 minutes", Recognizer::Code).is_empty());
        // A candidate inside the words is a part of what they spell.
        assert_eq!(
            spelt("erase q r t seven w eight h two now", Recognizer::Code),
            [(
                "q r t seven w eight h two".to_owned(),
                "qrt7w8h2".to_owned(),
                Form::Aloud
            )]
        );
    }
}
