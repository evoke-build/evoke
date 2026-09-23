//! The recorded adapter: a `Recording` read from its TOML and rendered back, and its lookup. In: a recording's
//! text; a `Recording`; a `Recording` and a `Request`. Out: the `Recording`, or why the text is not one; its text,
//! as the spec writes one; the recorded answers for the questions asked, or `Fault::Unrecorded` for an utterance
//! the recording lacks.

use std::fmt::Write as _;

use evoke_core::adapter::{Declared, Fault, Raw, Request};
use evoke_core::document::Json;
use evoke_core::{Digest, Identity, identity};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Hand-writable answers keyed by utterance identity, with what the adapter declares. `plan`, when present, names
/// the plan the answers were recorded against: the host compares it with `Plan::digest` before calling `answer`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recording {
    #[serde(flatten)]
    pub declared: Declared,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<Digest>,
    pub answers: IndexMap<Identity, Raw>,
}

/// A recording from its TOML text — `id`, `gate`, `plan`, `[answers."<utterance>"]` — read as JSON and entered
/// through its type; the error is why it is not one.
pub fn recording(text: &str) -> Result<Recording, String> {
    let document =
        toml_edit::Document::parse(text.to_owned()).map_err(|error| error.message().to_owned())?;
    serde_json::from_value(json(document.as_item())).map_err(|error| error.to_string())
}

/// TOML as JSON: tables to objects, arrays to arrays, a datetime to its text.
fn json(item: &toml_edit::Item) -> Json {
    use toml_edit::Item;
    match item {
        Item::None => Json::Null,
        Item::Value(value) => self::value(value),
        Item::Table(table) => table
            .iter()
            .map(|(key, item)| (key.to_owned(), json(item)))
            .collect(),
        Item::ArrayOfTables(tables) => tables
            .iter()
            .map(|table| {
                table
                    .iter()
                    .map(|(key, item)| (key.to_owned(), json(item)))
                    .collect::<Json>()
            })
            .collect(),
    }
}

fn value(value: &toml_edit::Value) -> Json {
    use toml_edit::Value;
    match value {
        Value::String(s) => Json::String(s.value().clone()),
        Value::Integer(i) => Json::from(*i.value()),
        Value::Float(f) => Json::from(*f.value()),
        Value::Boolean(b) => Json::Bool(*b.value()),
        Value::Datetime(d) => Json::String(d.value().to_string()),
        Value::Array(items) => items.iter().map(self::value).collect(),
        Value::InlineTable(table) => table
            .iter()
            .map(|(key, value)| (key.to_owned(), self::value(value)))
            .collect(),
    }
}

/// A recording as its TOML — the declaration, then one `[answers."<utterance>"]` table per utterance, each question
/// an inline table, keys aligned per table — readable by `recording`, by a hand and by the CLI's `EVOKE_ANSWERS`.
#[must_use]
pub fn render(recording: &Recording) -> String {
    let mut text =
        String::from("# What the replay adapter answers, keyed by utterance identity.\n");
    let _ = writeln!(text, "id = {}", quoted(recording.declared.id.as_str()));
    if let Some(plan) = recording.plan {
        let _ = writeln!(text, "plan = {}", quoted(&plan.to_string()));
    }
    if let Some(limits) = &recording.declared.limits {
        let mut lines = Vec::new();
        if let Some(options) = limits.options {
            lines.push(("options".to_owned(), options.to_string()));
        }
        if let Some(tokens) = limits.tokens {
            lines.push(("tokens".to_owned(), tokens.to_string()));
        }
        table(&mut text, "[limits]", &lines);
    }
    if let Some(gate) = &recording.declared.gate {
        let mut lines = vec![("route".to_owned(), float(gate.route().get()))];
        if let Some(fits) = gate.fits() {
            lines.push(("fits".to_owned(), float(fits.get())));
        }
        lines.push(("read".to_owned(), float(gate.read().get())));
        lines.push(("write".to_owned(), float(gate.write().get())));
        table(&mut text, "[gate]", &lines);
    }
    for (identity, raw) in &recording.answers {
        let lines: Vec<(String, String)> = raw
            .0
            .iter()
            .map(|(question, answer)| {
                let inline = answer
                    .iter()
                    .map(|(name, p)| format!("{} = {}", key(name), float(*p)))
                    .collect::<Vec<_>>()
                    .join(", ");
                (self::key(question), format!("{{ {inline} }}"))
            })
            .collect();
        table(
            &mut text,
            &format!("[answers.{}]", quoted(identity.as_str())),
            &lines,
        );
    }
    text
}

/// A blank line, the header, then each line with its key padded to the widest.
fn table(text: &mut String, header: &str, lines: &[(String, String)]) {
    let width = lines.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
    let _ = writeln!(text, "\n{header}");
    for (key, value) in lines {
        let _ = writeln!(text, "{key:<width$} = {value}");
    }
}

/// A key bare where TOML allows it, else quoted.
fn key(text: &str) -> String {
    let bare = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { text.to_owned() } else { quoted(text) }
}

/// A TOML basic string.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A probability as TOML writes a float: always with a fraction.
fn float(p: f64) -> String {
    format!("{p:?}")
}

/// The recorded answers to the questions asked, a subset of what was recorded.
pub fn answer(recording: &Recording, request: &Request) -> Result<Raw, Fault> {
    let identity = identity(request.state.request.as_str());
    let recorded = recording
        .answers
        .get(&identity)
        .ok_or(Fault::Unrecorded { identity })?;
    let asked = request
        .questions
        .keys()
        .filter_map(|question| {
            recorded
                .0
                .get_key_value(&question.to_string())
                .map(|(id, answer)| (id.clone(), answer.clone()))
        })
        .collect();
    Ok(Raw(asked))
}

#[cfg(test)]
// The numbers under test are literals, compared exactly.
#[expect(clippy::float_cmp)]
mod tests {
    use super::*;

    /// `spec/transcripts/try/answers.toml` in the JSON form the host hands over.
    const TRY: &str = r#"{
      "id": "replay",
      "gate": { "route": 0.5, "fits": 0.3, "read": 0.6, "write": 0.8 },
      "answers": {
        "kill the lights": {
          "route": { "lights": 0.9, "timer": 0.02, "volume": 0.02, "none": 0.06 },
          "fits.lights": { "yes": 0.7 },
          "fits.timer": { "yes": 0.05 },
          "fits.volume": { "yes": 0.05 },
          "lights.room": { "den": 0.2, "office": 0.05, "unstated": 0.75 },
          "lights.state": { "off": 0.58, "on": 0.3, "dim": 0.1, "unstated": 0.02 }
        },
        "kill the lights in the den": {
          "route": { "lights": 0.91, "timer": 0.02, "volume": 0.01, "none": 0.06 },
          "fits.lights": { "yes": 0.7 },
          "fits.timer": { "yes": 0.05 },
          "fits.volume": { "yes": 0.05 },
          "lights.room": { "den": 0.85, "office": 0.05, "unstated": 0.1 },
          "lights.state": { "off": 0.88, "on": 0.05, "dim": 0.05, "unstated": 0.02 }
        }
      }
    }"#;

    fn request(text: &str) -> Request {
        let mut request: Request = serde_json::from_str(include_str!(
            "../../../spec/fixtures/request-kill-the-lights.json"
        ))
        .unwrap();
        request.state.request = evoke_core::Input::new(text).unwrap();
        request
    }

    #[test]
    fn answers_the_questions_asked_by_identity() {
        let recording: Recording = serde_json::from_str(TRY).unwrap();
        assert_eq!(recording.declared.id.as_str(), "replay");
        assert_eq!(recording.declared.gate.unwrap().write().get(), 0.8);
        assert_eq!(recording.plan, None);
        let raw = answer(&recording, &request("Kill the lights!")).unwrap();
        let ids: Vec<&str> = raw.0.keys().map(String::as_str).collect();
        assert_eq!(
            ids,
            [
                "route",
                "fits.lights",
                "lights.room",
                "lights.state",
                "fits.timer"
            ]
        );
        assert_eq!(raw.0["lights.room"]["unstated"], 0.75);
    }

    #[test]
    fn a_question_of_evokes_own_is_answered_from_the_recording() {
        let read = recording(
            "id = \"replay\"\n\n[answers.\"kill the lights\"]\n\"weave.split_0\" = { yes = 0.8 }\n",
        )
        .unwrap();
        let mut request = request("kill the lights");
        request.questions.clear();
        let clean = |text: &str| evoke_core::Clean::new(text).unwrap();
        request.questions.insert(
            evoke_core::QuestionId::parse("weave.split_0").unwrap(),
            evoke_core::Question::YesNo {
                ask: clean("Two things?"),
                yes: evoke_core::adapter::Text::Plain(clean("Two.")),
                no: evoke_core::adapter::Text::Plain(clean("One.")),
            },
        );
        let raw = answer(&read, &request).unwrap();
        assert_eq!(raw.0["weave.split_0"]["yes"], 0.8);
        assert_eq!(recording(&render(&read)).unwrap(), read);
    }

    #[test]
    fn an_unrecorded_utterance_is_a_fault() {
        let recording: Recording = serde_json::from_str(TRY).unwrap();
        assert_eq!(
            answer(&recording, &request("what time is it")).unwrap_err(),
            Fault::Unrecorded {
                identity: identity("what time is it")
            }
        );
    }

    #[test]
    fn a_recording_is_read_from_its_toml() {
        let text = include_str!("../../../spec/transcripts/try/answers.toml");
        let read = recording(text).unwrap();
        assert_eq!(read.declared.id.as_str(), "replay");
        assert_eq!(read.answers.len(), 3);
        assert_eq!(read.declared.gate.map(|gate| gate.route().get()), Some(0.5));
        assert!(recording("id = \"\"\n[answers]\n").is_err());
        assert!(recording("id = \n").is_err());
    }

    #[test]
    fn a_recording_renders_as_the_spec_writes_one() {
        for text in [
            include_str!("../../../spec/transcripts/use/answers.toml"),
            include_str!("../../../spec/transcripts/update/answers.toml"),
        ] {
            let read = recording(text).unwrap();
            let rendered = render(&read);
            assert_eq!(recording(&rendered).unwrap(), read);
            let body = |text: &str| text.split_once('\n').map(|(_, rest)| rest.to_owned());
            assert_eq!(body(&rendered), body(text));
        }
        let full = recording(
            "id = \"x\"\nplan = \"h1:0000000000000000000000000000000000000000000000000000000000000000\"\n\n[limits]\noptions = 255\ntokens  = 32000\n\n[answers.\"a \\\"b\\\"\"]\n\"fits.x\" = { yes = 1.0, \"two words\" = 0.0 }\n",
        )
        .unwrap();
        assert_eq!(recording(&render(&full)).unwrap(), full);
        assert!(render(&full).contains(
            "[answers.\"a \\\"b\\\"\"]\n\"fits.x\" = { yes = 1.0, \"two words\" = 0.0 }"
        ));
    }

    #[test]
    fn a_recording_enters_through_its_types() {
        assert!(serde_json::from_str::<Recording>(r#"{"id": "", "answers": {}}"#).is_err());
        assert!(
            serde_json::from_str::<Recording>(
                r#"{"id": "replay", "gate": {"route": 0.5, "read": 0.9, "write": 0.8}, "answers": {}}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<Recording>(r#"{"id": "replay", "answers": {}}"#).is_ok());
    }
}
