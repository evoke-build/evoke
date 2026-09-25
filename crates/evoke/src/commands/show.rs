//! `evoke show [name]`: what is installed, or one reflex as it is used. In: a name or none, the environment. Out:
//! `Exit`. Without a name, every reflex on a line with what it may touch under it — the answer, on stdout — the
//! machine's status once when it does not hold a declaration whole, and each inactive one's problems with their
//! fixes, on stderr; with one, the effective manifest, each line marked shipped or yours, the lines of the overlay
//! that address nothing the reflex has, then that reflex's problems.

use evoke_core::document::Text as Source;
use evoke_core::name::LocalName;
use evoke_core::text::NonEmpty;
use evoke_core::{Diagnostic, Document, File, Fix, overlay};

use super::session::{self, Opening, Session};
use super::{Exit, nothing_installed};
use crate::args::Command;
use crate::hosts::{Environment, terminal};
use crate::report::{self, Gutter};

pub fn run(command: &Command, name: Option<&LocalName>, environment: &Environment) -> Exit {
    let session = match session::open(command, false, environment, Opening::Listing) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let exit = shown(&session, name);
    session.reporter.exit(&command.stand_in(), exit)
}

fn shown(session: &Session<'_>, name: Option<&LocalName>) -> Exit {
    let invoked = session.reporter.command.placeholder();
    let inactive = |problems: &[Diagnostic]| {
        if !problems.is_empty() {
            terminal::note(&report::inactive(
                problems,
                &invoked,
                &session.reporter.paths,
            ));
        }
    };
    let Some(name) = name else {
        if session.project.reflexes.is_empty() {
            return nothing_installed();
        }
        terminal::answer(&report::rows(
            &session.rows(session.project.reflexes.keys()),
            Gutter::Listed,
        ));
        if let Some(status) = report::status(&session.contained) {
            terminal::note(&status);
        }
        let problems: Vec<Diagnostic> = session
            .plan
            .inactive()
            .values()
            .flat_map(NonEmpty::iter)
            .cloned()
            .collect();
        inactive(&problems);
        return Exit::Ran;
    };
    let Some(item) = session.installed.reflexes.get(name) else {
        return Exit::Human(Diagnostic {
            reflex: Some(name.clone()),
            at: None,
            message: format!("{name} is not installed"),
            fix: Fix::Show { reflex: None },
        });
    };
    match &item.wording {
        Ok(effective) => {
            terminal::answer(&report::manifest(effective));
            match session.overlay_text(name) {
                Ok(Some(text)) => {
                    if let Some((shipped, _)) = session.shipped.get(name)
                        && let Ok(yours) = overlay(
                            Document {
                                file: File::Overlay { name: name.clone() },
                                text: Source::Toml(&text),
                            },
                            shipped,
                        )
                        && !yours.orphaned.is_empty()
                    {
                        terminal::note(&report::orphaned(name, &yours.orphaned));
                    }
                }
                Ok(None) => {}
                Err(exit) => return exit,
            }
            if let Some(problems) = session.plan.inactive().get(name) {
                inactive(&problems.iter().cloned().collect::<Vec<_>>());
            }
            Exit::Ran
        }
        Err(problems) => session
            .reporter
            .human(&session.reporter.command.stand_in(), problems.clone()),
    }
}
