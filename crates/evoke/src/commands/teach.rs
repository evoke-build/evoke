//! `evoke teach ["<utterance>"] <call> | not <name>`: the overlay line for an utterance. In: the utterance, or none
//! for the last input decided; what it teaches; the environment. Out: `Exit`. The lesson is typed against the plan
//! and held to the utterance by the core, then lands in `overlays/<name>.toml` when the file still reads.

use evoke_core::manifest::Record;
use evoke_core::{Diagnostic, Fix, Lesson, Utterance, teach};

use super::Exit;
use super::session::{self, Opening, Session};
use crate::args::{Command, Taught};
use crate::hosts::Environment;
use crate::report::Line;

pub fn run(
    command: &Command,
    utterance: Option<&str>,
    lesson: &Taught,
    environment: &Environment,
) -> Exit {
    let session = match session::open(command, false, environment, Opening::Tuning) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let text = match utterance {
        Some(text) => text.to_owned(),
        None => match last_input(&session) {
            Ok(text) => text,
            Err(exit) => return session.reporter.exit(&command.stand_in(), exit),
        },
    };
    let exit = taught(&session, &text, lesson);
    session.reporter.exit(&text, exit)
}

/// The last input decided, from the log.
fn last_input(session: &Session<'_>) -> Result<String, Exit> {
    let last = session.state.last().map_err(Exit::Failed)?;
    let line = last
        .as_deref()
        .map(Line::parse)
        .transpose()
        .map_err(|why| human(format!("the log's last line does not read: {why}")))?;
    line.map(|line| line.input.as_str().to_owned())
        .ok_or_else(|| human("nothing has been decided yet; name the utterance".to_owned()))
}

fn taught(session: &Session<'_>, text: &str, lesson: &Taught) -> Exit {
    let utterance = match Utterance::new(text) {
        Ok(utterance) => utterance,
        Err(why) => return human(format!("\"{text}\" {why}")),
    };
    let lesson = match lesson {
        Taught::Call(written) => match Lesson::typed(written.clone(), &session.plan) {
            Ok(lesson) => lesson,
            Err(refused) => return Exit::Human(refused),
        },
        Taught::Not(name) => Lesson {
            reflex: name.clone(),
            record: Record::Never,
        },
    };
    let edit = match teach(&utterance, lesson, &session.plan) {
        Ok(edit) => edit,
        Err(refused) => return Exit::Human(refused),
    };
    match session.apply(text, &edit) {
        Ok(_) => Exit::Ran,
        Err(problem) => problem,
    }
}

fn human(message: String) -> Exit {
    Exit::Human(Diagnostic {
        reflex: None,
        at: None,
        message,
        fix: Fix::Rerun,
    })
}
