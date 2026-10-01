//! The weave's reading over arbitrary text: the words' own order, every place of the cut, the stretches code sets
//! apart, the segments between them, and the references code finds in each — any text reads, every place and
//! stretch lies in the text in order with none over another, and nothing panics. Seeded from every weave vector's
//! and fixture's request and the weave flows' sentences, in fuzz/seeds/reading (`mise run fuzz`).

#![no_main]

use evoke_core::weave::reading::{
    canonical, negated, places, refers_back, refs_by_code, segments, set_apart, set_aside, splits,
    stretches,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let request = canonical(text);
    let width = request.chars().count();
    let _ = splits(&request, true);
    let found = places(&request, true);
    let mut end = 0;
    for place in &found {
        assert!(
            end <= place.start && place.start <= place.end && place.end <= width,
            "the places overlap or leave the text in {request:?}: {found:?}"
        );
        end = place.end;
    }
    let apart = stretches(&request, &found);
    let mut end = 0;
    for stretch in &apart {
        assert!(
            end <= stretch.start && stretch.start < stretch.end && stretch.end <= width,
            "the stretches overlap or leave the text in {request:?}: {apart:?}"
        );
        end = stretch.end;
    }
    let all = set_apart(&request, found, &apart);
    let mut end = 0;
    for place in &all {
        assert!(
            end <= place.start && place.end <= width,
            "the places overlap or leave the text once stretches are set apart in {request:?}: {all:?}"
        );
        end = place.end;
    }
    let mut segs = segments(&request, &all);
    set_aside(&request, &mut segs, &apart);
    let fields: Vec<Vec<String>> = segs
        .iter()
        .map(|_| vec!["email".to_owned(), "people".to_owned(), "path".to_owned()])
        .collect();
    for k in 0..segs.len() {
        let _ = refs_by_code(&segs, k, &fields);
    }
    let _ = negated(&request);
    let _ = refers_back(&request);
});
