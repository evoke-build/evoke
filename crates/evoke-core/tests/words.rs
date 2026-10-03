//! The core holds no word of any language: every word the reader proposes from is a pack's, under `data/`. This
//! test holds the reader's modules to it — no table of words, no word compared with a person's text, no test that
//! holds for one script's letters — so that a language is data and nothing else. The reader is the modules that
//! read the person's words; the rest of the core says something back, in evoke's own sentences, or names a key of
//! the wire, which no pack reads. The forms' primitives (a figure, a Latin letter as a code or an address is typed)
//! live in `text.rs`, outside the reader.

use std::fs;
use std::path::{Path, PathBuf};

/// The reader: the modules that read the person's words.
const READER: [&str; 10] = [
    "src/account.rs",
    "src/bounds.rs",
    "src/otherwise.rs",
    "src/pack.rs",
    "src/pointer.rs",
    "src/propose.rs",
    "src/spoken.rs",
    "src/words.rs",
    "src/weave/planning.rs",
    "src/weave/reading.rs",
];

/// The tests of one script's letters.
const ONE_SCRIPT: [&str; 8] = [
    "is_ascii_alphabetic",
    "is_ascii_alphanumeric",
    "is_ascii_lowercase",
    "is_ascii_uppercase",
    "is_ascii_digit",
    "to_ascii_lowercase",
    "to_ascii_uppercase",
    "eq_ignore_ascii_case",
];

/// What compares a literal with text on its line.
const COMPARES: [&str; 9] = [
    "==",
    "!=",
    "starts_with(",
    "ends_with(",
    "strip_prefix(",
    "strip_suffix(",
    "contains(",
    "matches!(",
    "=>",
];

fn sources() -> Vec<PathBuf> {
    READER
        .iter()
        .map(|file| Path::new(env!("CARGO_MANIFEST_DIR")).join(file))
        .collect()
}

/// The string literals of a line, unescaped as far as a word needs.
fn literals(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            // A char literal: skip it whole.
            let mut depth = 0;
            for d in chars.by_ref() {
                if d == '\\' {
                    depth = 1;
                    continue;
                }
                if d == '\'' && depth != 1 {
                    break;
                }
                depth = 0;
            }
            continue;
        }
        if c != '"' {
            continue;
        }
        let mut literal = String::new();
        let mut escaped = false;
        for d in chars.by_ref() {
            if escaped {
                literal.push(d);
                escaped = false;
            } else if d == '\\' {
                escaped = true;
            } else if d == '"' {
                break;
            } else {
                literal.push(d);
            }
        }
        found.push(literal);
    }
    found
}

/// Whether a literal is a word or two of letters: what a pack holds, and the core may not.
fn is_word(literal: &str) -> bool {
    let words: Vec<&str> = literal.split(' ').collect();
    !literal.is_empty()
        && words.len() <= 2
        && words.iter().all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_alphabetic() || c == '\'' || c == '\u{2019}')
        })
}

/// Whether a line opens a table of text: a `const` or `static` array or slice holding `&str`.
fn opens_table(line: &str) -> bool {
    let trimmed = line.trim_start();
    let declares = trimmed.starts_with("const ")
        || trimmed.starts_with("static ")
        || trimmed.starts_with("pub const ")
        || trimmed.starts_with("pub static ")
        || trimmed.starts_with("pub(crate) const ")
        || trimmed.starts_with("pub(crate) static ");
    declares && line.contains("&str") && line.contains(": [") || declares && line.contains(": &[")
}

#[test]
fn the_core_holds_no_word_of_any_language() {
    let mut findings: Vec<String> = Vec::new();
    for path in sources() {
        let shown = path
            .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
            .unwrap_or(&path)
            .display()
            .to_string();
        let source = fs::read_to_string(&path).expect("a source file reads");
        let mut in_table = false;
        let mut table_words = 0;
        let mut table_at = 0;
        for (n, line) in source.lines().enumerate() {
            let number = n + 1;
            let trimmed = line.trim_start();
            if trimmed.starts_with("#[cfg(test)]") {
                break;
            }
            if trimmed.starts_with("//") {
                continue;
            }
            if in_table {
                table_words += literals(line)
                    .iter()
                    .filter(|literal| is_word(literal))
                    .count();
                if line.contains("];") || line.trim_end().ends_with(']') {
                    if table_words > 0 {
                        findings.push(format!(
                            "{shown}:{table_at}: a table of {table_words} words"
                        ));
                    }
                    in_table = false;
                }
                continue;
            }
            if opens_table(line) {
                table_at = number;
                table_words = literals(line)
                    .iter()
                    .filter(|literal| is_word(literal))
                    .count();
                if line.contains("];") {
                    if table_words > 0 {
                        findings.push(format!("{shown}:{number}: a table of {table_words} words"));
                    }
                } else {
                    in_table = true;
                }
                continue;
            }
            if COMPARES.iter().any(|compares| line.contains(compares)) {
                for literal in literals(line)
                    .into_iter()
                    .filter(|literal| is_word(literal))
                {
                    findings.push(format!("{shown}:{number}: «{literal}» compared with text"));
                }
            }
            for test in ONE_SCRIPT {
                if line.contains(test) {
                    findings.push(format!("{shown}:{number}: {test}, a test of one script"));
                }
            }
        }
    }
    assert!(
        findings.is_empty(),
        "the core holds words of a language, or tests one script:\n  {}",
        findings.join("\n  ")
    );
}
