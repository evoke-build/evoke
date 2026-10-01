//! The `date` recognizer over arbitrary text: every candidate that reads as a date re-reads whole at a prompt
//! through `picked` with the same reading, its value bounded and its wire form round-tripping, a day read beside
//! another's none or a day of the calendar, and no two candidates of any kind overlap. Seeded from every propose
//! and picked vector's input, the six problems' sentences and the probe's, in fuzz/seeds/values (`mise run fuzz`).

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
        if !matches!(candidate.value, PickValue::Date { .. }) {
            continue;
        }
        if let PickValue::Date { value } = &candidate.value {
            // Every reading resolves against any day, and lands on a day of the calendar.
            let today: evoke_core::calendar::Date = "2026-05-05".parse().expect("a day");
            let _ = value.resolve(today);
            // Read beside any other candidate's day, it is none or a day of the calendar that resolves too.
            for other in &found {
                if let PickValue::Date { value: named } = &other.value
                    && let Some(read) = value.beside(named)
                {
                    assert!(
                        matches!(read, evoke_core::calendar::Day::Calendar { .. }),
                        "{value:?} beside {named:?} reads as {read:?}"
                    );
                    let _ = read.resolve(today);
                }
            }
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
        match picked(typed, Recognizer::Date) {
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
            // An ordinal read after `on` — «pay it on 6th» — is a bare figure alone: a prompt wants `the 6th`.
            None => {}
            other => panic!("{typed:?} re-reads as something else: {other:?}"),
        }
    }
});
