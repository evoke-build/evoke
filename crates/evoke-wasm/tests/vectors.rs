//! Runs `spec/vectors/<family>/` through the op table: each case's `input` goes in by name, and the reply must
//! equal `expect` — wrapped in `ok` for a function that cannot fail — numbers as one kind, key order free. A new
//! family is one line in `RESULTS` when its function returns a `Result`, and one test below.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The families whose function returns a `Result`, so their `expect` is already `{ ok } | { err }`.
const RESULTS: [&str; 13] = [
    "manifest",
    "overlay",
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

#[test]
fn identity_vectors() {
    family("identity");
}

#[test]
fn fault_vectors() {
    family("fault");
}

#[test]
fn manifest_vectors() {
    family("manifest");
}

#[test]
fn overlay_vectors() {
    family("overlay");
}

#[test]
fn effective_vectors() {
    family("effective");
}

#[test]
fn report_vectors() {
    family("report");
}

#[test]
fn compile_vectors() {
    family("compile");
}

#[test]
fn propose_vectors() {
    family("propose");
}

#[test]
fn request_vectors() {
    family("request");
}

#[test]
fn read_vectors() {
    family("read");
}

#[test]
fn gate_vectors() {
    family("gate");
}

#[test]
fn fill_vectors() {
    family("fill");
}

#[test]
fn diff_vectors() {
    family("diff");
}

#[test]
fn consent_vectors() {
    family("consent");
}

#[test]
fn teach_vectors() {
    family("teach");
}

#[test]
fn envelope_vectors() {
    family("envelope");
}

#[test]
fn argv_vectors() {
    family("argv");
}

#[test]
fn call_vectors() {
    family("call");
}

#[test]
fn by_name_vectors() {
    family("by_name");
}

#[test]
fn set_config_vectors() {
    family("set_config");
}

#[test]
fn vocab_edit_vectors() {
    family("vocab_edit");
}

#[test]
fn reference_vectors() {
    family("reference");
}

#[test]
fn lock_vectors() {
    family("lock");
}

#[test]
fn render_lock_vectors() {
    family("render_lock");
}

#[test]
fn add_entry_vectors() {
    family("add_entry");
}

#[test]
fn remove_entry_vectors() {
    family("remove_entry");
}

#[test]
fn lint_vectors() {
    family("lint");
}

#[test]
fn cases_vectors() {
    family("cases");
}

#[test]
fn thieves_vectors() {
    family("thieves");
}

#[test]
fn judge_vectors() {
    family("judge");
}

#[test]
fn regressions_vectors() {
    family("regressions");
}

#[test]
fn weave_plan_vectors() {
    family("weave.plan");
}

#[test]
fn weave_execute_vectors() {
    family("weave.execute");
}

#[test]
fn baseline_vectors() {
    family("baseline");
}

#[test]
fn reflex_dts_vectors() {
    family("reflex_dts");
}

#[test]
fn project_dts_vectors() {
    family("project_dts");
}

#[test]
fn digest_vectors() {
    family("digest");
}

#[test]
fn fix_vectors() {
    family("fix");
}

#[test]
fn values_vectors() {
    family("values");
}

#[test]
fn picked_vectors() {
    family("picked");
}
