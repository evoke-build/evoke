//! `evoke new [--playbook] <name>`: a working reflex under `./<name>/` from the template — `reflex.toml`,
//! `<name>.mts` and `reflex.d.ts` — so `check`, `try` and a run all work at birth; or a playbook, `reflex.toml`
//! alone, its plan of steps in place of a body. Both keep every rule lint holds. In: the name, and whether a
//! playbook. Out: `Exit`, a `+` line per file. No session opens; nothing but the new directory is touched.

use std::path::Path;

use evoke_core::document::Text;
use evoke_core::manifest::Run;
use evoke_core::name::LocalName;
use evoke_core::{Document, File, Fix, manifest, reflex_dts};

use super::{Exit, human};
use crate::args::Command;
use crate::hosts::{files, terminal};
use crate::report;

/// The templates: a greeting with one quoted pick, and a playbook of three steps over a service and its region;
/// `{name}` stands for the reflex's name, in `run`, and for the playbook's, in its tag and its confirm.
const MANIFEST: &str = include_str!("../../template/reflex.toml");
const BODY: &str = include_str!("../../template/reflex.mts");
const PLAYBOOK: &str = include_str!("../../template/playbook.toml");

pub fn run(command: &Command, name: &LocalName, playbook: bool) -> Exit {
    let invoked = command.invoked("");
    match made(name, playbook) {
        Ok(()) => Exit::Ran,
        Err(exit) => {
            if let Some(line) = report::exit(&exit, &invoked, None) {
                terminal::note(&line);
            }
            exit
        }
    }
}

/// The files written under a directory that did not exist, each printed as it lands: the manifest, and for a
/// reflex with a body the body and its types.
fn made(name: &LocalName, playbook: bool) -> Result<(), Exit> {
    let dir = Path::new(name.as_str());
    if dir.exists() {
        return Err(human(format!("{name} already exists"), Fix::New));
    }
    let template = if playbook { PLAYBOOK } else { MANIFEST };
    let text = template.replace("{name}", name.as_str());
    let parsed = manifest(Document {
        file: File::Manifest { name: name.clone() },
        text: Text::Toml(&text),
    })
    .expect("the template is a valid manifest");
    let mut files = vec![("reflex.toml".to_owned(), text)];
    if matches!(parsed.run, Run::File(_)) {
        files.push((format!("{name}.mts"), BODY.to_owned()));
        files.push(("reflex.d.ts".to_owned(), reflex_dts(&parsed)));
    }
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

    fn read(template: &str, name: &str) -> evoke_core::Manifest {
        let name = LocalName::new(name).unwrap();
        let text = template.replace("{name}", name.as_str());
        manifest(Document {
            file: File::Manifest { name },
            text: Text::Toml(&text),
        })
        .unwrap()
    }

    #[test]
    fn the_template_is_a_clean_reflex_named_for_its_directory() {
        let parsed = read(MANIFEST, "hello");
        assert_eq!(lint(&parsed), []);
        assert!(parsed.unknown.is_empty());
        assert_eq!(serde_json::to_value(&parsed.run).unwrap(), "hello.mts");
        assert!(BODY.contains("./reflex.d.ts"));
        assert!(reflex_dts(&parsed).contains("who: string"));
    }

    #[test]
    fn the_playbook_template_is_a_clean_plan_tagged_with_its_name() {
        let parsed = read(PLAYBOOK, "outage");
        assert_eq!(lint(&parsed), []);
        assert!(parsed.unknown.is_empty());
        assert_eq!(parsed.steps.len(), 3);
        assert_eq!(parsed.tags[0].as_str(), "outage");
        assert_eq!(
            parsed.confirm.to_string(),
            "Run the outage plan for {service}?"
        );
    }
}
