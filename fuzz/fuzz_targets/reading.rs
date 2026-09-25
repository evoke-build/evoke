//! The weave's reading over arbitrary text: the words' own order, every split point, the segments between them,
//! and the references code finds in each — any text reads, and nothing panics. Seeded from every weave vector's
//! and fixture's request and the weave flows' sentences, in fuzz/seeds/reading (`mise run fuzz`).

#![no_main]

use evoke_core::weave::reading::{canonical, negated, refers_back, refs_by_code, segments, splits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let request = canonical(text);
    let all = splits(&request, true);
    let segs = segments(&request, &all);
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
