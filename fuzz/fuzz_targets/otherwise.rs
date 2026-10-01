//! The reader of listed words said otherwise over arbitrary text and a list the first byte picks: at most one
//! proposal a listed word, each for a word no finding holds, its words one word of the input and never the listed
//! word itself; whether words are another person's name is answered for any text, and nothing panics. Seeded from
//! sentences that say a listed word otherwise, in fuzz/seeds/otherwise (`mise run fuzz`).

#![no_main]

use evoke_core::otherwise::{another, proposed};
use evoke_core::{Clean, Key, listed};
use indexmap::IndexMap;
use libfuzzer_sys::fuzz_target;

/// Lists of the kinds a vocabulary holds: people, months, regions, services, channels.
const LISTS: [&[(&str, &str)]; 5] = [
    &[
        ("sam", "Sam, a colleague."),
        ("ana", "Ana, a colleague."),
        ("jo", "Jo, a colleague."),
    ],
    &[
        ("august", "August."),
        ("september", "September."),
        ("october", "October."),
    ],
    &[
        ("eu-west", "Europe, the Ireland region."),
        ("us-east", "The United States, the Virginia region."),
    ],
    &[
        ("checkout", "The checkout service."),
        ("payments", "The payments service."),
        ("search", "The search service."),
    ],
    &[
        ("#incident", "Where an outage is handled."),
        ("#ops", "The operations team's channel."),
    ],
];

fuzz_target!(|data: &[u8]| {
    let Some((first, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let Ok(input) = evoke_core::Input::new(text) else {
        return;
    };
    let list: IndexMap<Key, Clean> = LISTS[usize::from(*first) % LISTS.len()]
        .iter()
        .map(|(key, meaning)| {
            (
                Key::new(key).expect("a listed word is a key"),
                Clean::new(meaning).expect("a meaning is clean"),
            )
        })
        .collect();
    let found = listed(&input, &list);
    let proposals = proposed(&input, &list, &found, "find", &[]);
    let width = text.chars().count();
    for (at, proposal) in proposals.iter().enumerate() {
        assert!(
            list.contains_key(&proposal.key),
            "{:?} is proposed and is on no list, in {text:?}",
            proposal.key
        );
        assert!(
            proposals[..at]
                .iter()
                .all(|other| other.key != proposal.key),
            "{:?} is proposed twice in {text:?}",
            proposal.key
        );
        assert!(
            found.iter().all(|held| held.key != proposal.key),
            "{:?} is found and proposed in {text:?}",
            proposal.key
        );
        let said = proposal.span.text().as_str();
        assert!(
            proposal.span.start() < proposal.span.end()
                && proposal.span.end() <= width
                && !said.contains(char::is_whitespace),
            "{said:?} is not one word of {text:?}"
        );
        assert!(
            said.to_lowercase() != proposal.key.as_str(),
            "{said:?} is the listed word itself in {text:?}"
        );
        let _ = another(said, &proposal.key, &list[&proposal.key]);
    }
});
