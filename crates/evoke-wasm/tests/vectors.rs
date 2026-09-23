//! Runs `spec/vectors/<family>/` through the op table: each case's `input` goes in by name, and the reply must
//! equal `expect` — wrapped in `ok` for a function that cannot fail — numbers as one kind, key order free. A new
//! family is one line in `families!`, and one in `RESULTS` when its function returns a `Result`; a directory the
//! list lacks fails the last test.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The families whose function returns a `Result`, so their `expect` is already `{ ok } | { err }`.
const RESULTS: [&str; 20] = [
    "manifest",
    "overlay",
    "vocabulary",
    "project",
    "name",
    "compile",
    "request",
    "read",
    "teach",
    "argv",
    "call",
    "by_name",
    "set_config",
    "reference",
    "lock",
    "weave.plan",
    "jev.settings",
    "jev.answers",
    "replay.recording",
    "replay.answer",
];

fn spec() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// A JSON file under `spec/`, its `{ "$ref": … }` objects replaced by what they name.
fn load(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let value =
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    resolve(value)
}

fn resolve(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            if let (1, Some(Value::String(path))) = (map.len(), map.get("$ref")) {
                return load(&spec().join(path));
            }
            map.into_iter()
                .map(|(key, value)| (key, resolve(value)))
                .collect()
        }
        Value::Array(items) => items.into_iter().map(resolve).collect(),
        other => other,
    }
}

/// What the op table must answer: the case's `expect`, wrapped in `ok` unless the function itself returns a `Result`.
fn expected(family: &str, expect: Value) -> Value {
    if RESULTS.contains(&family) {
        expect
    } else {
        json!({ "ok": expect })
    }
}

/// Every number as an `f64`, so `40` and `40.0` are one number.
fn normalized(value: Value) -> Value {
    match value {
        Value::Number(n) => n.as_f64().map_or(Value::Null, Value::from),
        Value::Array(items) => items.into_iter().map(normalized).collect(),
        Value::Object(map) => map
            .into_iter()
            .map(|(key, value)| (key, normalized(value)))
            .collect(),
        other => other,
    }
}

fn family(name: &str) {
    let dir = spec().join("vectors").join(name);
    let mut cases: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no cases under {}", dir.display());
    let mut failures = Vec::new();
    for path in &cases {
        let file = path
            .file_name()
            .map_or_else(String::new, |file| file.to_string_lossy().into_owned());
        let case = load(path);
        let expected = normalized(expected(name, case["expect"].clone()));
        let actual = normalized(evoke_wasm::call(name, &case["input"]));
        if expected != actual {
            failures.push(format!(
                "{name}/{file}\n  expected: {expected:#}\n  actual:   {actual:#}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} {name} cases failed\n\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n\n")
    );
}

/// One test per family, and the list the directory must match: a family added under `spec/vectors/` and not
/// named here fails the check below, so nothing runs nowhere. `version` alone has no family: it answers the
/// build's own number.
macro_rules! families {
    ($($test:ident => $family:literal),* $(,)?) => {
        const FAMILIES: &[&str] = &[$($family),*];
        $(
            #[test]
            fn $test() {
                family($family);
            }
        )*
    };
}

families! {
    identity_vectors => "identity",
    fault_vectors => "fault",
    manifest_vectors => "manifest",
    overlay_vectors => "overlay",
    effective_vectors => "effective",
    report_vectors => "report",
    compile_vectors => "compile",
    propose_vectors => "propose",
    request_vectors => "request",
    read_vectors => "read",
    gate_vectors => "gate",
    fill_vectors => "fill",
    diff_vectors => "diff",
    consent_vectors => "consent",
    teach_vectors => "teach",
    envelope_vectors => "envelope",
    argv_vectors => "argv",
    call_vectors => "call",
    by_name_vectors => "by_name",
    set_config_vectors => "set_config",
    vocab_edit_vectors => "vocab_edit",
    reference_vectors => "reference",
    lock_vectors => "lock",
    render_lock_vectors => "render_lock",
    add_entry_vectors => "add_entry",
    remove_entry_vectors => "remove_entry",
    lint_vectors => "lint",
    cases_vectors => "cases",
    thieves_vectors => "thieves",
    judge_vectors => "judge",
    regressions_vectors => "regressions",
    weave_plan_vectors => "weave.plan",
    weave_execute_vectors => "weave.execute",
    baseline_vectors => "baseline",
    reflex_dts_vectors => "reflex_dts",
    project_dts_vectors => "project_dts",
    digest_vectors => "digest",
    fix_vectors => "fix",
    values_vectors => "values",
    picked_vectors => "picked",
    vocabulary_vectors => "vocabulary",
    project_vectors => "project",
    name_vectors => "name",
    location_vectors => "location",
    jev_settings_vectors => "jev.settings",
    jev_request_vectors => "jev.request",
    jev_answers_vectors => "jev.answers",
    replay_recording_vectors => "replay.recording",
    replay_render_vectors => "replay.render",
    replay_answer_vectors => "replay.answer",
}

#[test]
fn every_family_in_the_directory_has_a_test() {
    let dir = spec().join("vectors");
    let mut found: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    found.sort();
    let mut listed: Vec<&str> = FAMILIES.to_vec();
    listed.sort_unstable();
    assert_eq!(
        found, listed,
        "spec/vectors/ and the families tested here differ"
    );
}
