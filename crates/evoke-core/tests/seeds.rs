//! Every real owned file under `reflexes/` and `spec/transcripts/` parses: manifests, projects, locks, vocabularies,
//! and overlays against the local reflex beside them; and every manifest lints clean. One manifest must not read —
//! the tree the update transcript skips — and this checks that it does not.

use std::fs;
use std::path::{Path, PathBuf};

use evoke_core::document::Text;
use evoke_core::name::{LocalName, VocabName};
use evoke_core::{
    Diagnostic, Document, File, Finding, Fix, lint, lock, manifest, overlay, project, vocabulary,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The one manifest that must not read: what `evoke update` reports and skips in its transcript.
const UNREADABLE: &str = "spec/transcripts/update/remote/radhi/tools/1.1.0/clock/reflex.toml";

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            walk(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

fn dir_name(path: &Path) -> String {
    path.parent()
        .and_then(Path::file_name)
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The diagnostics a file yields, or `None` when it is not one the core reads.
fn parse(path: &Path) -> Option<Vec<Diagnostic>> {
    let text = read(path);
    let parent = dir_name(path);
    let errors = match path.file_name().and_then(|name| name.to_str()) {
        Some("reflex.toml") => {
            // A reflex at a repository's root sits under the tag's directory; any name serves the file's role.
            let name = LocalName::new(&parent)
                .or_else(|_| LocalName::new("reflex"))
                .expect("a name");
            match manifest(Document {
                file: File::Manifest { name: name.clone() },
                text: Text::Toml(&text),
            }) {
                Ok(parsed) => Some(findings(&name, &lint(&parsed))),
                Err(errors) => Some(errors),
            }
        }
        Some("evoke.toml") => project(Document {
            file: File::Project,
            text: Text::Toml(&text),
        })
        .err(),
        Some("evoke.lock") => lock(Document {
            file: File::Lock,
            text: Text::Toml(&text),
        })
        .err(),
        Some(_) if parent == "vocab" => {
            let name = VocabName::new(&stem(path))
                .unwrap_or_else(|why| panic!("{}: {why}", path.display()));
            vocabulary(Document {
                file: File::Vocab { name },
                text: Text::Toml(&text),
            })
            .err()
        }
        Some(_) if parent == "overlays" => {
            let name = LocalName::new(&stem(path))
                .unwrap_or_else(|why| panic!("{}: {why}", path.display()));
            let shipped = path
                .parent()?
                .parent()?
                .join(name.as_str())
                .join("reflex.toml");
            if !shipped.exists() {
                return None;
            }
            let shipped = read(&shipped);
            let of = manifest(Document {
                file: File::Manifest { name: name.clone() },
                text: Text::Toml(&shipped),
            })
            .unwrap_or_else(|errors| panic!("{}: {errors:#?}", path.display()));
            overlay(
                Document {
                    file: File::Overlay { name },
                    text: Text::Toml(&text),
                },
                &of,
            )
            .err()
        }
        _ => return None,
    };
    Some(errors.unwrap_or_default())
}

/// Lint findings as diagnostics, so a seed that passes a cap or addresses the model fails like a parse error.
fn findings(name: &LocalName, findings: &[Finding]) -> Vec<Diagnostic> {
    findings
        .iter()
        .map(|finding| Diagnostic {
            reflex: Some(name.clone()),
            at: None,
            message: format!("lint: {}", finding.message),
            fix: Fix::Check,
        })
        .collect()
}

#[test]
fn every_seed_and_transcript_file_parses() {
    let mut files = Vec::new();
    walk(&root().join("reflexes"), &mut files);
    walk(&root().join("spec/transcripts"), &mut files);
    files.sort();
    let mut parsed = 0;
    let mut failures = Vec::new();
    for path in &files {
        let Some(errors) = parse(path) else { continue };
        parsed += 1;
        if path.ends_with(UNREADABLE) {
            assert!(!errors.is_empty(), "{UNREADABLE} reads; it must not");
            continue;
        }
        for error in errors {
            failures.push(format!(
                "{}: {} → {}",
                path.display(),
                error.message,
                error.fix.command("")
            ));
        }
    }
    assert!(
        parsed >= 20,
        "only {parsed} files parsed; the seeds and transcripts hold more"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
