//! What a body receives: the envelope a file body reads on stdin, or the argv a program is spawned with. In: the
//! chosen call, its active reflex, the input, the deadline and the home. Out: an `Envelope` — config still
//! references, which the host resolves — or an argv, where a placeholder whose argument is unstated is dropped
//! and a value starting with `-` is refused. In both, a word's value and a plain setting that name a path under
//! the home, `~/Desktop`, are expanded to it: the body, the argv and the declaration see one path.

use indexmap::IndexMap;
use serde::Serialize;

use crate::call::{Value, expand_home};
use crate::decide::Chosen;
use crate::diagnostic::{Diagnostic, Fix};
use crate::document::Json;
use crate::manifest::{Element, Run};
use crate::name::{ArgName, ConfigKey, LocalName};
use crate::plan::{Active, Millis};
use crate::project::Setting;
use crate::text::Input;

/// One JSON line on the loader's stdin. `args` carries values: an option key, a vocabulary word's `value` if set
/// else the word, a pick's value, `true` for a flag. An inline body has no `run`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Envelope {
    pub reflex: LocalName,
    #[serde(skip_serializing_if = "Run::is_inline")]
    pub run: Run,
    pub args: IndexMap<ArgName, Json>,
    pub input: Input,
    pub config: IndexMap<ConfigKey, Setting>,
    pub deadline: Millis,
}

/// The envelope of a chosen call, for a file body and an inline one alike.
#[must_use]
pub fn envelope(
    chosen: &Chosen,
    active: &Active,
    input: &Input,
    deadline: Millis,
    home: &str,
) -> Envelope {
    Envelope {
        reflex: chosen.call.reflex.clone(),
        run: active.run.clone(),
        args: chosen
            .call
            .args
            .iter()
            .map(|(name, value)| (name.clone(), value.under_home(home)))
            .collect(),
        input: input.clone(),
        config: active
            .config
            .iter()
            .map(|(key, setting)| {
                let setting = match setting {
                    Setting::Plain { value } => Setting::Plain {
                        value: expand_home(value, home),
                    },
                    Setting::Env { var } => Setting::Env { var: var.clone() },
                };
                (key.clone(), setting)
            })
            .collect(),
        deadline,
    }
}

/// The argv of a chosen call: the program, then each element as written or as its argument's value.
pub fn argv(chosen: &Chosen, active: &Active, home: &str) -> Result<Vec<String>, Diagnostic> {
    let reflex = &chosen.call.reflex;
    let Run::Argv { program, rest } = &active.run else {
        return Err(refused(
            reflex,
            format!("{reflex} runs a file, not an argv"),
        ));
    };
    let mut argv = vec![program.to_string()];
    for element in rest {
        match element {
            Element::Literal(text) => argv.push(text.to_string()),
            Element::Arg(name) => {
                let Some(value) = chosen.call.args.get(name) else {
                    continue;
                };
                let text = text(value, home);
                if text.starts_with('-') {
                    return Err(refused(
                        reflex,
                        format!(
                            "{name} is \"{text}\", which starts with -; an argv cannot carry it"
                        ),
                    ));
                }
                argv.push(text);
            }
        }
    }
    Ok(argv)
}

/// A value as one argv element: the envelope's value as a person would type it.
fn text(value: &Value, home: &str) -> String {
    match value.under_home(home) {
        Json::String(text) => text,
        Json::Number(number) => number
            .as_f64()
            .map_or_else(|| number.to_string(), |number| number.to_string()),
        other => other.to_string(),
    }
}

fn refused(reflex: &LocalName, message: String) -> Diagnostic {
    Diagnostic {
        reflex: Some(reflex.clone()),
        at: None,
        message,
        fix: Fix::Rerun,
    }
}
