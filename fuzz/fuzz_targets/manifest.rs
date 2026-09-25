//! The manifest reader over arbitrary bytes: any text reads to a manifest or to its lines to fix, and never
//! panics. Seeded from every manifest under spec/ and reflexes/, in fuzz/seeds (`mise run fuzz`).

#![no_main]

use evoke_core::document::Text;
use evoke_core::name::LocalName;
use evoke_core::{Document, File, manifest};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = manifest(Document {
        file: File::Manifest {
            name: LocalName::new("fuzzed").expect("a name"),
        },
        text: Text::Toml(text),
    });
});
