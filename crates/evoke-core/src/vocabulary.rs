//! `vocab/<name>.toml` as a type. In: a `Document`. Out: `Vocabulary`, words unique under identity, each with its
//! meaning; or diagnostics.

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::diagnostic::Diagnostic;
use crate::document::{self, Diagnostics, Document, Json, Node, Value};
use crate::name::Word;
use crate::text::{Clean, identity};

/// Your closed set for the arguments that name it; may be empty, which `compile` reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vocabulary(IndexMap<Word, Meaning>);

/// What a word means, and what the body receives instead of it when set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meaning {
    pub what: Clean,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl std::ops::Deref for Vocabulary {
    type Target = IndexMap<Word, Meaning>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for Vocabulary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Vocabulary {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let json = Json::deserialize(deserializer)?;
        let mut d = Diagnostics::new(None);
        let value = read(&mut d, document::wire(&json));
        d.finish(value)
            .map_err(|errors| D::Error::custom(document::summary(&errors)))
    }
}

/// A vocabulary from its file, or every line to fix.
pub fn vocabulary(doc: Document<'_>) -> Result<Vocabulary, Vec<Diagnostic>> {
    let mut d = Diagnostics::new(Some(&doc.file));
    let value = read(&mut d, document::root(doc)?);
    d.finish(value)
}

fn read(d: &mut Diagnostics, root: Node) -> Option<Vocabulary> {
    let table = d.table(root)?;
    let mut words = IndexMap::new();
    let mut seen: IndexMap<_, Word> = IndexMap::new();
    for (key, node) in table.entries() {
        let at = node.at.clone();
        let word = match Word::new(&key) {
            Ok(word) => word,
            Err(why) => {
                d.fail(at.as_ref(), format!("word {why}"));
                continue;
            }
        };
        let id = identity(word.as_str());
        if let Some(earlier) = seen.get(&id) {
            d.fail(at.as_ref(), format!("\"{word}\" repeats \"{earlier}\""));
            continue;
        }
        if let Some(meaning) = meaning(d, node) {
            seen.insert(id, word.clone());
            words.insert(word, meaning);
        }
    }
    Some(Vocabulary(words))
}

fn meaning(d: &mut Diagnostics, node: Node) -> Option<Meaning> {
    match node.value {
        Value::Str(_) => d.line(&node).map(|what| Meaning { what, value: None }),
        Value::Table(_) => {
            let mut table = d.table(node)?;
            let what = if let Some(node) = table.take("what") {
                d.line(&node)
            } else {
                d.fail(
                    table.at.as_ref(),
                    format!("{}.what is required", table.path),
                );
                None
            };
            let value = match table.take("value") {
                Some(node) => Some(d.line(&node)?.as_str().to_owned()),
                None => None,
            };
            for (_, node) in table.entries() {
                d.fail(
                    node.at.as_ref(),
                    format!("{} is unknown; a meaning has what and value", node.path),
                );
            }
            Some(Meaning { what: what?, value })
        }
        _ => {
            d.fail(
                node.at.as_ref(),
                format!(
                    "{} must be a meaning: \"what it is\" or {{ what, value }}",
                    node.name()
                ),
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::File;
    use crate::document::Text;
    use crate::name::VocabName;

    fn parse(text: &str) -> Result<Vocabulary, Vec<Diagnostic>> {
        let file = File::Vocab {
            name: VocabName::new("rooms").unwrap(),
        };
        vocabulary(Document {
            file,
            text: Text::Toml(text),
        })
    }

    fn messages(text: &str) -> Vec<(u32, String)> {
        parse(text)
            .unwrap_err()
            .into_iter()
            .map(|e| (e.at.map_or(0, |at| at.line), e.message))
            .collect()
    }

    #[test]
    fn reads_both_meaning_shapes() {
        let rooms = parse(
            "den    = \"The TV room downstairs; also 'the snug'.\"\noffice = { what = \"The upstairs study.\", value = \"group-7\" }\n",
        )
        .unwrap();
        assert_eq!(rooms.len(), 2);
        assert_eq!(rooms.get("den").unwrap().value, None);
        assert_eq!(
            rooms.get("office").unwrap().value.as_deref(),
            Some("group-7")
        );
        assert_eq!(
            serde_json::to_value(&rooms).unwrap(),
            serde_json::json!({
                "den": { "what": "The TV room downstairs; also 'the snug'." },
                "office": { "what": "The upstairs study.", "value": "group-7" }
            })
        );
    }

    #[test]
    fn an_empty_file_is_an_empty_vocabulary() {
        assert!(parse("").unwrap().is_empty());
    }

    #[test]
    fn words_hold_their_grammar_and_are_unique_under_identity() {
        assert_eq!(
            messages(
                "none = \"Nothing.\"\nden = \"The den.\"\nDen = \"Again.\"\n\"the snug\" = \"Also.\"\n"
            ),
            [
                (1, "word \"none\" is reserved".to_owned()),
                (3, "\"Den\" repeats \"den\"".to_owned())
            ]
        );
    }

    #[test]
    fn a_meaning_is_a_line_or_a_table_with_what() {
        assert_eq!(
            messages(
                "den = 3\noffice = { value = \"x\" }\nhall = { what = \"The hall.\", also = \"no\" }\n"
            ),
            [
                (
                    1,
                    "den must be a meaning: \"what it is\" or { what, value }".to_owned()
                ),
                (2, "office.what is required".to_owned()),
                (
                    3,
                    "hall.also is unknown; a meaning has what and value".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn the_wire_form_enters_through_the_same_reader() {
        let json = "{\"den\": {\"what\": \"The den.\"}, \"none\": {\"what\": \"x\"}}";
        assert!(serde_json::from_str::<Vocabulary>(json).is_err());
        let json = "{\"den\": {\"what\": \"The den.\"}}";
        assert_eq!(serde_json::from_str::<Vocabulary>(json).unwrap().len(), 1);
    }
}
