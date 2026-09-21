//! `evoke new <name>`: a working reflex under `./<name>/` from the template — `reflex.toml`, `<name>.mts` and
//! `reflex.d.ts` — so `check`, `try` and a run all work at birth. In: the name. Out: `Exit`, a
//! `+` line per file. No session opens; nothing but the new directory is touched.

use std::path::Path;

use evoke_core::document::Text;
use evoke_core::name::LocalName;
use evoke_core::{Document, File, Fix, manifest, reflex_dts};

use super::{Exit, human};
use crate::args::Command;
use crate::hosts::{files, terminal};
use crate::report;

/// The template: a greeting with one quoted pick, `{name}` standing for the reflex's name in `run`.
const MANIFEST: &str = include_str!("../../template/reflex.toml");
const BODY: &str = include_str!("../../template/reflex.mts");

pub fn run(command: &Command, name: &LocalName) -> Exit {
    let invoked = command.invoked("");
    match made(name) {
        Ok(()) => Exit::Ran,
        Err(exit) => {
            if let Some(line) = report::exit(&exit, &invoked, None) {
                terminal::note(&line);
            }
            exit
        }
    }
}

/// The three files written under a directory that did not exist, each printed as it lands.
fn made(name: &LocalName) -> Result<(), Exit> {
    let dir = Path::new(name.as_str());
    if dir.exists() {
        return Err(human(format!("{name} already exists"), Fix::New));
    }
    let text = MANIFEST.replace("{name}", name.as_str());
    let parsed = manifest(Document {
        file: File::Manifest { name: name.clone() },
        text: Text::Toml(&text),
    })
    .expect("the template is a valid manifest");
    let files = [
        ("reflex.toml".to_owned(), text),
        (format!("{name}.mts"), BODY.to_owned()),
        ("reflex.d.ts".to_owned(), reflex_dts(&parsed)),
    ];
    for (file, content) in files {
        files::write(&dir.join(&file), &content).map_err(Exit::Failed)?;
        terminal::note(&report::created(&format!("{name}/{file}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use evoke_core::lint;

    use super::*;

    #[test]
    fn the_template_is_a_clean_reflex_named_for_its_directory() {
        let name = LocalName::new("hello").unwrap();
        let text = MANIFEST.replace("{name}", name.as_str());
        let parsed = manifest(Document {
            file: File::Manifest { name },
            text: Text::Toml(&text),
        })
        .unwrap();
        assert!(lint(&parsed).is_empty());
        assert_eq!(serde_json::to_value(&parsed.run).unwrap(), "hello.mts");
        assert!(BODY.contains("./reflex.d.ts"));
        assert!(reflex_dts(&parsed).contains("who: string"));
    }
}
