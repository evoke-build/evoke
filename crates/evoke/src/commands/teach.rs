//! `evoke teach ["<utterance>"] <call> | not <name>`: the overlay line for an utterance. In: the utterance as
//! given — said, left out for the last input decided, or a first word that is the utterance unless it names an
//! installed reflex; what it teaches; the environment. Out: `Exit`. The lesson is typed against the plan and held
//! to the utterance by the core, then lands in `overlays/<name>.toml` when the file still reads.

use evoke_core::manifest::Record;
use evoke_core::{Diagnostic, Fix, Lesson, Utterance, teach};

use super::Exit;
use super::session::{self, Opening, Session};
use crate::args::{Command, Spoken, Taught};
use crate::hosts::Environment;
use crate::report::Line;

pub fn run(command: &Command, spoken: &Spoken, lesson: &Taught, environment: &Environment) -> Exit {
    // The command as settled, for the lines that repeat it: declared here to outlive the session.
    let settled: Command;
    let mut session = match session::open(command, false, environment, Opening::Tuning) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let (text, lesson) = match spoken {
        Spoken::Given(text) => (text.clone(), lesson.clone()),
        // The word names an installed reflex: the call begins with it, and the utterance is the last input.
        Spoken::Either { word, whole } if session.installed.reflexes.contains_key(word) => {
            match last_input(&session) {
                Ok(text) => (text, Taught::Call(whole.clone())),
                Err(exit) => return session.reporter.exit(&command.stand_in(), exit),
            }
        }
        Spoken::Either { word, .. } => (word.to_string(), lesson.clone()),
        Spoken::Last => match last_input(&session) {
            Ok(text) => (text, lesson.clone()),
            Err(exit) => return session.reporter.exit(&command.stand_in(), exit),
        },
    };
    settled = Command::Teach {
        spoken: Spoken::Given(text.clone()),
        lesson: lesson.clone(),
    };
    session.reporter.command = &settled;
    let exit = taught(&mut session, &text, &lesson);
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

fn taught(session: &mut Session<'_>, text: &str, lesson: &Taught) -> Exit {
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
