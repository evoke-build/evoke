//! `evoke teach ["<utterance>"] <call> | not <name>`: the overlay line for an utterance. In: the utterance as
//! given — said, left out for the last input decided, or a first word that is the utterance unless it names an
//! installed reflex; what it teaches; the environment. Out: `Exit`. The last input of a weave is the step the
//! lesson's reflex decided; when none or several did, the steps are named for the person to choose. The lesson is
//! typed against the plan and held to the utterance by the core, then lands in `overlays/<name>.toml` when the
//! file still reads.

use evoke_core::manifest::Record;
use evoke_core::{Diagnostic, Fix, Lesson, Utterance, teach};

use super::Exit;
use super::session::{self, Opening, Session};
use crate::args::{Command, Spoken, Taught};
use crate::hosts::Environment;
use crate::report::{self, Line};

pub fn run(command: &Command, spoken: &Spoken, lesson: &Taught, environment: &Environment) -> Exit {
    // The command as settled, for the lines that repeat it: declared here to outlive the session.
    let settled: Command;
    let mut session = match session::open(command, false, environment, Opening::Tuning) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let spoken = match spoken {
        Spoken::Given(text) => Ok((text.clone(), lesson.clone())),
        // The word names an installed reflex: the call begins with it, and the utterance is the last input.
        Spoken::Either { word, whole } if session.installed.reflexes.contains_key(word) => {
            let lesson = Taught::Call(whole.clone());
            last_input(&session, &lesson).map(|text| (text, lesson))
        }
        Spoken::Either { word, .. } => Ok((word.to_string(), lesson.clone())),
        Spoken::Last => last_input(&session, lesson).map(|text| (text, lesson.clone())),
    };
    let (text, lesson) = match spoken {
        Ok(spoken) => spoken,
        // No last input to teach: the line repeats the command with the utterance still to name.
        Err(exit) => {
            settled = Command::Teach {
                spoken: Spoken::Given(UTTERANCE.to_owned()),
                lesson: lesson.clone(),
            };
            session.reporter.command = &settled;
            return session.reporter.exit(UTTERANCE, exit);
        }
    };
    settled = Command::Teach {
        spoken: Spoken::Given(text.clone()),
        lesson: lesson.clone(),
    };
    session.reporter.command = &settled;
    let exit = taught(&mut session, &text, &lesson);
    session.reporter.exit(&text, exit)
}

/// The utterance's place in a fix line, when there is none to show.
const UTTERANCE: &str = "<utterance>";

/// The last input decided, from the log: one decision's; of a weave, the step the lesson's reflex decided,
/// when one did — else the steps, for the person to name one.
fn last_input(session: &Session<'_>, lesson: &Taught) -> Result<String, Exit> {
    let lines = session.state.tail().map_err(Exit::Failed)?;
    let lines: Vec<Line> = lines
        .iter()
        .map(|line| Line::parse(line))
        .collect::<Result<_, _>>()
        .map_err(|why| human(format!("the log's last line does not read: {why}")))?;
    let reflex = match lesson {
        Taught::Call(written) => &written.reflex,
        Taught::Not(name) => name,
    };
    let mut named = lines
        .iter()
        .filter(|line| line.step.is_none() || line.reflex() == Some(reflex));
    match (named.next(), named.next()) {
        (Some(line), None) => Ok(line.input.as_str().to_owned()),
        _ if lines.is_empty() => Err(human(
            "nothing has been decided yet; name the utterance".to_owned(),
        )),
        _ => {
            let steps: Vec<String> = lines
                .iter()
                .map(|line| report::quoted(line.input.as_str()))
                .collect();
            Err(human(format!(
                "the last input read as {} steps: {}; name the one to teach",
                steps.len(),
                steps.join(", ")
            )))
        }
    }
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
