//! The words of a request that point at a value said elsewhere — «it», «that order», «my last order», «the last
//! 2 orders» — and the words that count one, «2 orders». In: the request's words, what code proposed in them,
//! and the names a value goes by. Out: where the words stand, how many values they name, and whether they
//! point at something.

use crate::pack::{self, Lexicon};
use crate::propose::{PickValue, Proposed};
use crate::text::{Input, Span, fold};
use crate::words::{self, Token};

/// Words that name a value said elsewhere, or count one: where they stand, how many values they name, and
/// whether they point back at what was said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pointer {
    pub words: Span,
    pub count: usize,
    /// Whether the words point at what was said before — a pronoun, «that», «the last» — rather than count
    /// alone, «2 orders».
    pub points: bool,
}

/// The first words of the input that point at a value one of `names` goes by, or count such values: «it»;
/// «that order», «this one»; «the last order», «my latest one», «the most recent order»; «the last 2 orders»,
/// «those two orders», «both orders»; and a bare count, «2 orders», which points at nothing. A name is matched
/// as itself, its plural by its stem. None where the words hold none.
pub(crate) fn pointed(input: &Input, proposed: &[Proposed], names: &[&str]) -> Option<Pointer> {
    let tokens = words::tokens(input.as_str());
    let folded: Vec<String> = tokens.iter().map(|token| fold(&token.plain)).collect();
    let lexicon = pack::lexicon(input.as_str());
    (0..tokens.len()).find_map(|i| at(input, &tokens, &folded, proposed, names, &lexicon, i))
}

/// The spans of the numbers that count a value one of `names` goes by: a number right before the plural of a
/// name, «2 orders». Such a number is no value of that argument.
pub(crate) fn counts(input: &Input, proposed: &[Proposed], names: &[&str]) -> Vec<Span> {
    let tokens = words::tokens(input.as_str());
    tokens
        .windows(2)
        .filter(|pair| plural(&pair[1].plain, names) && number(proposed, &pair[0]).is_some())
        .filter_map(|pair| Span::of(input, pair[0].from, pair[0].to))
        .collect()
}

/// The pointer that begins at token `i`, if one does.
fn at(
    input: &Input,
    tokens: &[Token],
    folded: &[String],
    proposed: &[Proposed],
    names: &[&str],
    lexicon: &Lexicon,
    i: usize,
) -> Option<Pointer> {
    let plain = |j: usize| tokens.get(j).map(|token| token.plain.as_str());
    let word = |j: usize| folded.get(j).map(String::as_str);
    let span = |from: usize, to: usize| Span::of(input, tokens[from].from, tokens[to].to);
    let first = word(i)?;
    // A pronoun, or «both orders».
    if lexicon.holds(|pack| &pack.recalled.it, first) {
        return Some(Pointer {
            words: span(i, i)?,
            count: 1,
            points: true,
        });
    }
    if lexicon.holds(|pack| &pack.recalled.both, first)
        && plain(i + 1).is_some_and(|word| plural(word, names))
    {
        return Some(Pointer {
            words: span(i, i + 1)?,
            count: 2,
            points: true,
        });
    }
    // A determiner, then a word that orders, then a count, then the name — each but the name optional, and the
    // name a plural where a count stands before it.
    let that = lexicon.holds(|pack| &pack.recalled.that, first);
    let several = lexicon.holds(|pack| &pack.recalled.several, first);
    let determiner = that || several || lexicon.holds(|pack| &pack.recalled.owns, first);
    let mut j = if determiner { i + 1 } else { i };
    let mut ordered = false;
    if let Some(taken) = words::phrase_at(folded, j, &lexicon.phrases(|pack| &pack.recalled.orders))
    {
        ordered = true;
        j += taken;
    }
    let count = tokens.get(j).and_then(|token| number(proposed, token));
    if count.is_some() {
        j += 1;
    }
    let head = plain(j)?;
    if let Some(count) = count {
        if !plural(head, names) {
            return None;
        }
        let points = ordered || several;
        return Some(Pointer {
            words: span(i, j)?,
            count,
            points,
        });
    }
    let one = word(j).is_some_and(|word| lexicon.holds(|pack| &pack.recalled.one, word));
    let is_name = names.contains(&head) || (one && (determiner || ordered));
    let points = that || (ordered && (determiner || i + 1 == j));
    if !(is_name && points) {
        return None;
    }
    Some(Pointer {
        words: span(i, j)?,
        count: 1,
        points: true,
    })
}

/// Whether a word is the plural of one of the names: its stem is a name, and it is not the name itself.
fn plural(word: &str, names: &[&str]) -> bool {
    let stem = words::stem_of(word);
    word != stem && names.contains(&stem.as_str())
}

/// The most things a count may name: a plan's own cap on its steps.
const MOST_NAMED: u8 = 24;

/// A whole number from two up to `MOST` a token spells, as code proposed it: «2», «two».
fn number(proposed: &[Proposed], token: &Token) -> Option<usize> {
    proposed.iter().find_map(|candidate| match candidate.value {
        PickValue::Number { value }
            if candidate.span.start() == token.from && candidate.span.end() == token.to =>
        {
            (2..=MOST_NAMED)
                .find(|n| (f64::from(*n) - value).abs() < f64::EPSILON)
                .map(usize::from)
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::propose::propose;

    fn found(text: &str, names: &[&str]) -> Option<(String, usize, bool)> {
        let input = Input::new(text).unwrap();
        let proposed = propose(&input);
        pointed(&input, &proposed, names).map(|pointer| {
            (
                pointer.words.text().as_str().to_owned(),
                pointer.count,
                pointer.points,
            )
        })
    }

    fn counted(text: &str, names: &[&str]) -> Vec<String> {
        let input = Input::new(text).unwrap();
        let proposed = propose(&input);
        counts(&input, &proposed, names)
            .iter()
            .map(|span| span.text().as_str().to_owned())
            .collect()
    }

    #[test]
    fn a_pronoun_and_a_pointing_determiner_name_the_newest() {
        assert_eq!(found("buy it", &["product"]), Some(("it".into(), 1, true)));
        assert_eq!(
            found("pay that order", &["order"]),
            Some(("that order".into(), 1, true))
        );
        assert_eq!(
            found("show this one", &["order"]),
            Some(("this one".into(), 1, true))
        );
        assert_eq!(
            found("pay my last order", &["order"]),
            Some(("my last order".into(), 1, true))
        );
        assert_eq!(
            found("pay the most recent order", &["order"]),
            Some(("the most recent order".into(), 1, true))
        );
        assert_eq!(
            found("refund the latest one", &["order"]),
            Some(("the latest one".into(), 1, true))
        );
    }

    #[test]
    fn a_plain_noun_and_a_name_of_another_thing_point_at_nothing() {
        assert_eq!(found("pay the order", &["order"]), None);
        assert_eq!(found("pay order 1001", &["order"]), None);
        assert_eq!(found("roll back the last deploy", &["release"]), None);
        assert_eq!(found("post in #it", &["channel"]), None);
        assert_eq!(found("pay them", &["order"]), None);
    }

    #[test]
    fn a_count_before_the_plural_names_that_many() {
        assert_eq!(
            found("pay the last 2 orders", &["order"]),
            Some(("the last 2 orders".into(), 2, true))
        );
        assert_eq!(
            found("pay those two orders", &["order"]),
            Some(("those two orders".into(), 2, true))
        );
        assert_eq!(
            found("refund both orders", &["order"]),
            Some(("both orders".into(), 2, true))
        );
        assert_eq!(
            found("pay 3 orders", &["order"]),
            Some(("3 orders".into(), 3, false))
        );
        assert_eq!(found("pay 2 of them", &["order"]), None);
        assert_eq!(counted("pay the last 2 orders", &["order"]), ["2"]);
        assert_eq!(
            counted("pay 2 of OUT-503", &["order"]),
            Vec::<String>::new()
        );
        assert_eq!(counted("order 1001", &["order"]), Vec::<String>::new());
    }
}
