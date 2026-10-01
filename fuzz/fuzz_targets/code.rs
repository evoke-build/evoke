//! The `code` recognizer over arbitrary text: every candidate that reads as a code re-reads whole at a prompt
//! through `picked` with the same reading, its value bounded and its wire form round-tripping, and no two
//! candidates of any kind overlap. Seeded from every propose and picked vector's input, the six problems'
//! sentences and the probe's, in fuzz/seeds/values (`mise run fuzz`).

#![no_main]

use evoke_core::manifest::Recognizer;
use evoke_core::{PickValue, Proposed, Value, picked, propose};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(input) = evoke_core::Input::new(text) else {
        return;
    };
    let found = propose(&input);
    let mut end = 0;
    for candidate in &found {
        assert!(
            candidate.span.start() >= end,
            "two candidates overlap in {text:?}: {found:?}"
        );
        end = candidate.span.end();
        if !matches!(candidate.value, PickValue::Code { .. }) {
            continue;
        }
        let again: Proposed = serde_json::from_str(
            &serde_json::to_string(candidate).expect("a candidate serializes"),
        )
        .expect("a candidate reads back");
        assert_eq!(
            again, *candidate,
            "the wire form does not round-trip for {text:?}"
        );
        let typed = candidate.span.text().as_str();
        let width = typed.chars().count();
        match picked(typed, Recognizer::Code) {
            Some(Value::Pick { span, value, .. }) => {
                assert_eq!(
                    (span.start(), span.end()),
                    (0, width),
                    "{typed:?} re-reads in part"
                );
                assert_eq!(
                    value, candidate.value,
                    "{typed:?} re-reads as another value"
                );
            }
            other => panic!("{typed:?} does not re-read as a code: {other:?}"),
        }
    }
});
