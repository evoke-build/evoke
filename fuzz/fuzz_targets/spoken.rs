//! The reader of values said aloud over arbitrary text, in each kind it reads and against example values of one
//! shape, of two, or none: every run it reads lies in the input and no two overlap; a value it proposes, and a
//! reading it offers at an ask, re-reads whole at a prompt through `picked`; a candidate it withdraws is one the
//! recognizers proposed. The first byte picks the kind and the examples. Seeded from every words.spoken vector's
//! input under each of them, in fuzz/seeds/spoken (`mise run fuzz`).

#![no_main]

use evoke_core::manifest::Recognizer;
use evoke_core::spoken::heard;
use evoke_core::{Clean, PickValue, Value, picked, propose};
use libfuzzer_sys::fuzz_target;

/// The kinds the reader reads, each with the example values of an argument.
const CASES: [(Recognizer, &[&str]); 12] = [
    (Recognizer::Number, &[]),
    (Recognizer::Number, &["311", "42", "1207"]),
    (Recognizer::Number, &["40"]),
    (Recognizer::Code, &["HS-0409", "VX-2214"]),
    (Recognizer::Code, &["HS-0409", "VX-221007"]),
    (Recognizer::Code, &["LX1830", "tp1051"]),
    (Recognizer::Code, &["4.12.0", "2.3.1"]),
    (Recognizer::Code, &["C02XK1ABJG5M", "c02g80t3md6r"]),
    (Recognizer::Email, &[]),
    (Recognizer::Url, &[]),
    (Recognizer::Date, &[]),
    (Recognizer::Duration, &[]),
];

/// A typed form read again alone, as a prompt reads an answer: whole, and as the value given where one is.
fn reread(typed: &str, recognizer: Recognizer, value: Option<&PickValue>, text: &str) {
    let width = typed.chars().count();
    match picked(typed, recognizer) {
        Some(Value::Pick {
            span, value: read, ..
        }) => {
            assert_eq!(
                (span.start(), span.end()),
                (0, width),
                "{typed:?} re-reads in part, from {text:?}"
            );
            if let Some(value) = value {
                assert_eq!(
                    read, *value,
                    "{typed:?} re-reads as another value, from {text:?}"
                );
            }
        }
        other => panic!("{typed:?} does not re-read as {recognizer:?}, from {text:?}: {other:?}"),
    }
}

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
    let (recognizer, examples) = CASES[usize::from(*first) % CASES.len()];
    let examples: Vec<Clean> = examples
        .iter()
        .map(|value| Clean::new(value).expect("an example value is clean"))
        .collect();
    let proposed = propose(&input);
    let heard = heard(&input, &recognizer.unranged(), &examples, &proposed);
    let width = text.chars().count();
    let mut runs = Vec::new();
    for said in &heard.said {
        runs.push((said.span.start(), said.span.end()));
        if said.shape.is_some() {
            reread(said.typed.as_str(), recognizer, Some(&said.value), text);
        }
    }
    for asked in &heard.spoken.asked {
        runs.push((asked.words.start(), asked.words.end()));
        for reading in &asked.readings {
            reread(reading.as_str(), recognizer, None, text);
        }
    }
    runs.sort_unstable();
    let mut end = 0;
    for (from, to) in runs {
        assert!(
            end <= from && from < to && to <= width,
            "runs overlap or leave the input in {text:?}: {heard:?}"
        );
        end = to;
    }
    for span in &heard.spoken.withdrawn {
        assert!(
            proposed.iter().any(|candidate| candidate.span == *span),
            "{span:?} is withdrawn and was never proposed in {text:?}"
        );
    }
});
