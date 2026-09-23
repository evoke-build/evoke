//! The grammar of decision 0015, pure: a first argument exactly a command word selects the command, else the
//! arguments are the input, bare words joined by one space; `--help`, `-h`, `help` and `--version`, `-V` in the
//! first place are what they say, and `--help` or `-h` right after a command word is the help too; `--` forces
//! input; stdin lines are input. In: the arguments after the program, and whether stdin is a pipe. Out: a
//! `Command` with validated arguments — a blank input is none — or a `Diagnostic` whose fix is `evoke --help`.

use std::ffi::OsString;
use std::fmt::{self, Write as _};

use evoke_core::name::{ConfigKey, LocalName, Tag, VarName, VocabName, Word};
use evoke_core::project::{Location, Setting};
use evoke_core::vocabulary::Meaning;
use evoke_core::{Clean, Diagnostic, Fix, Input, VocabChange, Written, call, reference};

/// Every command word, so a new one never reads as input; the ones not built yet are refused by name.
const WORDS: [&str; 21] = [
    "help",
    "try",
    "why",
    "run",
    "add",
    "remove",
    "update",
    "sync",
    "trust",
    "show",
    "teach",
    "vocab",
    "config",
    "test",
    "new",
    "check",
    "edit",
    "search",
    "publish",
    "adapter",
    "calibrate",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `evoke [--json] [--tag <tag>]… ["<input>"]`: decide, gate, run.
    Use(Arguments),
    /// `evoke try [--json] [--tag <tag>]… "<input>"`: decide only.
    Try(Arguments),
    /// `evoke why`: the last decision, explained from the log.
    Why,
    /// `evoke run [--json] <call>`: no classifier; the effect policy holds.
    Run { written: Written, json: bool },
    /// `evoke teach ["<utterance>"] <call> | not <name>`: the utterance omitted means the last input.
    Teach { spoken: Spoken, lesson: Taught },
    /// `evoke show [name]`: what is installed, or one effective manifest.
    Show(Option<LocalName>),
    /// `evoke vocab <name> [add <word> "<meaning>" [--value v] | remove <word>]`: the words, or one changed.
    Vocab {
        name: VocabName,
        change: Option<VocabChange>,
    },
    /// `evoke config <name> <key> <value> | --env VAR`: one setting.
    Config {
        reflex: LocalName,
        key: ConfigKey,
        setting: Setting,
    },
    /// `evoke add <ref>… [--as name]`: fetch, lint, the thief report, then the project and the lock written.
    Add {
        refs: Vec<Ref>,
        name: Option<LocalName>,
    },
    /// `evoke remove <name>`: the entry gone from the project and the lock; your files stay.
    Remove(LocalName),
    /// `evoke update [name] [--accept name]`: every unpinned remote reflex to its newest tag, or one; a looser
    /// effect accepted.
    Update {
        reflex: Option<LocalName>,
        accept: Option<LocalName>,
    },
    /// `evoke sync`: the lock realised in the store and the runtime recorded; nothing changes.
    Sync,
    /// `evoke trust`: this project blessed at its content.
    Trust,
    /// `evoke new <name>`: a working reflex under `./<name>/` from the template.
    New(LocalName),
    /// `evoke check`: the reflex here — its lines to fix, lint, its types, its contract against the newest tag.
    Check,
    /// `evoke test [name]`: every example and test of every active reflex, or of one, judged over the whole set.
    Test(Option<LocalName>),
    /// `evoke --help`, `-h` or `help`, or `--help` after a command word: every command, and what it does.
    Help,
    /// `evoke --version` or `-V`: `evoke <version>`.
    Version,
}

/// A ref as typed at `add`, parsed: a repository at its pin or its newest tag, or a directory on this machine,
/// `.`, `./dir` or `../dir`, as typed from the working directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ref {
    pub written: String,
    pub location: Location,
}

/// What `use` and `try` take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arguments {
    pub json: bool,
    pub tags: Vec<Tag>,
    pub input: Inputs,
}

/// Where the input comes from: the argument, one per line of a piped stdin, or the terminal — the REPL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inputs {
    One(String),
    Stdin,
    Terminal,
}

/// The utterance as `teach` was given it: said, left out for the last input decided, or a first word that could
/// be either the utterance or the reflex the call names — `kill lights state=off` — which the command settles by
/// what is installed, `whole` being the call read with the word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spoken {
    Given(String),
    Last,
    Either { word: LocalName, whole: Written },
}

/// What `teach` says about the utterance: the call it makes, or that it is not this reflex's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Taught {
    Call(Written),
    Not(LocalName),
}

impl fmt::Display for Taught {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Call(written) => write!(f, "{written}"),
            Self::Not(name) => write!(f, "not {name}"),
        }
    }
}

impl Command {
    /// The literal command that decides `input` again — for `teach`, that teaches the utterance again: what
    /// `Fix::Rerun` prints.
    #[must_use]
    pub fn invoked(&self, input: &str) -> String {
        let (word, arguments) = match self {
            Self::Use(arguments) => (None, arguments),
            Self::Try(arguments) => (Some("try"), arguments),
            Self::Why => return "evoke why".to_owned(),
            Self::Run { written, json } => {
                let flag = if *json { " --json" } else { "" };
                return format!("evoke run{flag} {written}");
            }
            Self::Teach {
                spoken: Spoken::Either { word, .. },
                lesson,
            } => return format!("evoke teach {word} {lesson}"),
            Self::Teach { lesson, .. } => return format!("evoke teach {} {lesson}", quoted(input)),
            Self::Show(None) => return "evoke show".to_owned(),
            Self::Show(Some(name)) => return format!("evoke show {name}"),
            Self::Vocab { name, change } => {
                let mut line = format!("evoke vocab {name}");
                match change {
                    None => {}
                    Some(VocabChange::Add { word, meaning }) => {
                        let _ = write!(
                            line,
                            " add {} {}",
                            self::word(word.as_str()),
                            self::word(meaning.what.as_str())
                        );
                        if let Some(value) = &meaning.value {
                            let _ = write!(line, " --value {}", self::word(value));
                        }
                    }
                    Some(VocabChange::Remove { word }) => {
                        let _ = write!(line, " remove {}", self::word(word.as_str()));
                    }
                }
                return line;
            }
            Self::Config {
                reflex,
                key,
                setting,
            } => {
                let value = match setting {
                    Setting::Plain { value } => word(value),
                    Setting::Env { var } => format!("--env {var}"),
                };
                return format!("evoke config {reflex} {key} {value}");
            }
            Self::Add { refs, name } => {
                let mut line = "evoke add".to_owned();
                for r in refs {
                    let _ = write!(line, " {}", r.written);
                }
                if let Some(name) = name {
                    let _ = write!(line, " --as {name}");
                }
                return line;
            }
            Self::Remove(name) => return format!("evoke remove {name}"),
            Self::Update { reflex, accept } => {
                let mut line = "evoke update".to_owned();
                if let Some(reflex) = reflex {
                    let _ = write!(line, " {reflex}");
                }
                if let Some(accept) = accept {
                    let _ = write!(line, " --accept {accept}");
                }
                return line;
            }
            Self::Sync => return "evoke sync".to_owned(),
            Self::Trust => return "evoke trust".to_owned(),
            Self::New(name) => return format!("evoke new {name}"),
            Self::Check => return "evoke check".to_owned(),
            Self::Test(None) => return "evoke test".to_owned(),
            Self::Test(Some(name)) => return format!("evoke test {name}"),
            Self::Help => return "evoke --help".to_owned(),
            Self::Version => return "evoke --version".to_owned(),
        };
        let mut line = "evoke".to_owned();
        if let Some(word) = word {
            line.push(' ');
            line.push_str(word);
        }
        if arguments.json {
            line.push_str(" --json");
        }
        for tag in &arguments.tags {
            let _ = write!(line, " --tag {tag}");
        }
        line.push(' ');
        // An input over the cap is refused whole: the line to run is one with a shorter input.
        let shown = if input.chars().count() > Input::CAP {
            "<input>"
        } else {
            input
        };
        line.push_str(&quoted(shown));
        line
    }

    /// The input as the setup lines name it before one is read: the one given, else `<input>` — for `teach`,
    /// `<utterance>`. What a command reports its problems against until it has read an input.
    #[must_use]
    pub fn stand_in(&self) -> String {
        match self {
            Self::Use(Arguments {
                input: Inputs::One(input),
                ..
            })
            | Self::Try(Arguments {
                input: Inputs::One(input),
                ..
            }) => input.clone(),
            Self::Teach {
                spoken: Spoken::Given(utterance),
                ..
            } => utterance.clone(),
            Self::Teach { .. } => "<utterance>".to_owned(),
            _ => "<input>".to_owned(),
        }
    }

    /// The literal command as the setup lines name it: `invoked` at the stand-in input.
    #[must_use]
    pub fn placeholder(&self) -> String {
        self.invoked(&self.stand_in())
    }
}

/// A JSON string: how an input or an utterance is written back.
fn quoted(text: &str) -> String {
    serde_json::Value::String(text.to_owned()).to_string()
}

/// A word as a shell would need it: bare when it is one plain word, else a JSON string.
fn word(text: &str) -> String {
    let plain = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | ':' | '@'));
    if plain { text.to_owned() } else { quoted(text) }
}

/// The command the arguments spell, every argument validated.
pub fn parse(
    arguments: impl IntoIterator<Item = OsString>,
    stdin_is_pipe: bool,
) -> Result<Command, Diagnostic> {
    let arguments: Vec<String> = arguments
        .into_iter()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| usage("an argument is not UTF-8"))
        })
        .collect::<Result<_, _>>()?;
    let Some((word, rest)) = arguments.split_first() else {
        return deciding("evoke", &arguments, stdin_is_pipe).map(Command::Use);
    };
    // `evoke add --help` is the help, as `evoke --help` is; after `--`, or as an input, `--help` is a word.
    if WORDS.contains(&word.as_str())
        && matches!(rest.first().map(String::as_str), Some("--help" | "-h"))
    {
        return Ok(Command::Help);
    }
    match word.as_str() {
        "--help" | "-h" | "help" => Ok(Command::Help),
        "--version" | "-V" => Ok(Command::Version),
        "try" => {
            let arguments = deciding("try", rest, stdin_is_pipe)?;
            if arguments.input == Inputs::Terminal {
                return Err(usage("try needs an input"));
            }
            Ok(Command::Try(arguments))
        }
        "why" => {
            if rest.is_empty() {
                Ok(Command::Why)
            } else {
                Err(usage("why takes no arguments"))
            }
        }
        "run" => run(rest),
        "teach" => teach(rest),
        "show" => match rest {
            [] => Ok(Command::Show(None)),
            [name] => named("show", name).map(Some).map(Command::Show),
            _ => Err(usage("show takes one name")),
        },
        "vocab" => vocab(rest),
        "config" => config(rest),
        "add" => add(rest),
        "remove" => match rest {
            [name] => named("remove", name).map(Command::Remove),
            _ => Err(usage("remove takes one name: evoke remove <name>")),
        },
        "update" => update(rest),
        "sync" if rest.is_empty() => Ok(Command::Sync),
        "sync" => Err(usage("sync takes no arguments")),
        "trust" if rest.is_empty() => Ok(Command::Trust),
        "trust" => Err(usage("trust takes no arguments")),
        "new" => match rest {
            [name] => named("new", name).map(Command::New),
            _ => Err(usage("new takes one name: evoke new <name>")),
        },
        "check" if rest.is_empty() => Ok(Command::Check),
        "check" => Err(usage("check takes no arguments")),
        "test" => match rest {
            [] => Ok(Command::Test(None)),
            [name] => named("test", name).map(Some).map(Command::Test),
            _ => Err(usage("test takes one name: evoke test [<name>]")),
        },
        word if WORDS.contains(&word) => Err(usage(format!("evoke {word} is not available yet"))),
        _ => deciding("evoke", &arguments, stdin_is_pipe).map(Command::Use),
    }
}

/// `[--json] [--tag <tag>]… [--] [<input>…]`: the bare words are one input, joined by a space, so a sentence
/// needs no quotes; after `--` even a flag is a word. A command word behind a flag — `evoke --json try …` — is
/// refused rather than decided, since it was meant as the command.
fn deciding(
    word: &str,
    arguments: &[String],
    stdin_is_pipe: bool,
) -> Result<Arguments, Diagnostic> {
    let mut json = false;
    let mut tags = Vec::new();
    let mut words: Vec<&str> = Vec::new();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--json" => json = true,
            "--tag" => {
                let tag = rest.next().ok_or_else(|| usage("--tag needs a tag"))?;
                if tag.starts_with('-') {
                    return Err(usage(format!("--tag needs a tag, and {tag} is a flag")));
                }
                tags.push(Tag::new(tag).map_err(usage)?);
            }
            "--" => words.extend(rest.by_ref().map(String::as_str)),
            flag if flag.starts_with('-') && flag.len() > 1 => {
                return Err(usage(format!(
                    "{flag} is not a flag of {word}; the flags are --json and --tag <tag>, and -- ends them"
                )));
            }
            input if word == "evoke" && words.is_empty() && WORDS.contains(&input) => {
                return Err(usage(format!(
                    "{input} is a command word; write it first, or -- before it to say it"
                )));
            }
            input => words.push(input),
        }
    }
    let input = if words.is_empty() {
        if stdin_is_pipe {
            Inputs::Stdin
        } else {
            Inputs::Terminal
        }
    } else {
        let joined = words.join(" ");
        if joined.trim().is_empty() {
            return Err(usage(format!("{word} needs an input")));
        }
        Inputs::One(joined)
    };
    Ok(Arguments { json, tags, input })
}

/// A command's one name: a flag in its place is refused as a flag, not as a malformed name.
fn named(command: &str, token: &str) -> Result<LocalName, Diagnostic> {
    if token.starts_with('-') && token.len() > 1 {
        return Err(usage(format!("{token} is not a flag of {command}")));
    }
    LocalName::new(token).map_err(usage)
}

/// `[--json] <call>`: the flag first, then the call as one argument or as its words.
fn run(arguments: &[String]) -> Result<Command, Diagnostic> {
    let mut json = false;
    let mut rest = arguments;
    while let Some((flag, after)) = rest.split_first() {
        match flag.as_str() {
            "--json" => json = true,
            flag if flag.starts_with('-') && flag.len() > 1 => {
                return Err(usage(format!(
                    "{flag} is not a flag of run; the flag is --json"
                )));
            }
            _ => break,
        }
        rest = after;
    }
    if rest.is_empty() {
        return Err(usage(
            "run needs a call: evoke run [--json] <name> [<arg>=<value> | <flag>]…",
        ));
    }
    if let Some(flag) = rest.iter().find(|argument| argument.starts_with("--")) {
        return Err(usage(format!("run takes {flag} before the call")));
    }
    let written = call(&line(rest)).map_err(help)?;
    Ok(Command::Run { written, json })
}

/// `["<utterance>"] (<call> | not <name>)`: the first argument is the utterance when it could not name a reflex,
/// or when `not` follows it; `not` itself, a lone word, or a word followed by what is no call of its own begins
/// the lesson, and the utterance is the last input. A word followed by a call could be either — `kill lights
/// state=off` — and is left to the command, with both readings.
fn teach(arguments: &[String]) -> Result<Command, Diagnostic> {
    let Some((first, rest)) = arguments.split_first() else {
        return Err(usage(NEEDS));
    };
    if first.starts_with('-') && first.len() > 1 {
        return Err(usage(format!(
            "{first} is not a flag of teach; teach takes none"
        )));
    }
    if first.trim().is_empty() {
        return Err(usage("teach needs an utterance; write it in quotes"));
    }
    if first == "not" {
        return Ok(Command::Teach {
            spoken: Spoken::Last,
            lesson: lesson(arguments)?,
        });
    }
    let Ok(word) = LocalName::new(first) else {
        return Ok(Command::Teach {
            spoken: Spoken::Given(first.clone()),
            lesson: lesson(rest)?,
        });
    };
    if matches!(rest.first(), Some(not) if not == "not") {
        return Ok(Command::Teach {
            spoken: Spoken::Given(first.clone()),
            lesson: lesson(rest)?,
        });
    }
    let whole = call(&line(arguments)).map_err(help)?;
    let following = if rest.is_empty() {
        None
    } else {
        call(&line(rest)).ok()
    };
    Ok(match following {
        Some(following) => Command::Teach {
            spoken: Spoken::Either { word, whole },
            lesson: Taught::Call(following),
        },
        None => Command::Teach {
            spoken: Spoken::Last,
            lesson: Taught::Call(whole),
        },
    })
}

const NEEDS: &str = "teach needs a call or not <name>: evoke teach [\"<utterance>\"] <name> [<arg>=<value> | <flag>]… | not <name>";

/// `<call> | not <name>`.
fn lesson(tokens: &[String]) -> Result<Taught, Diagnostic> {
    match tokens {
        [] => Err(usage(NEEDS)),
        [not, name] if not == "not" => Ok(Taught::Not(LocalName::new(name).map_err(usage)?)),
        [not, ..] if not == "not" => Err(usage("not takes one name")),
        tokens => Ok(Taught::Call(call(&line(tokens)).map_err(help)?)),
    }
}

/// The call line from its argv elements: one element is the line as `evoke` printed it; several are its tokens,
/// and a value the shell already delimited — `duration="10 minutes"` reaching us as `duration=10 minutes` — is
/// quoted for the grammar.
fn line(elements: &[String]) -> String {
    match elements {
        [line] => line.clone(),
        tokens => tokens
            .iter()
            .map(|token| match token.split_once('=') {
                Some((arg, value))
                    if !value.starts_with('"')
                        && value.contains(|c: char| c.is_whitespace() || c == '"' || c == '=') =>
                {
                    format!("{arg}={}", quoted(value))
                }
                _ => token.clone(),
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// `<name> [add <word> "<meaning>" [--value v] | remove <word>]`.
fn vocab(arguments: &[String]) -> Result<Command, Diagnostic> {
    let Some((name, rest)) = arguments.split_first() else {
        return Err(usage(
            "vocab needs a name: evoke vocab <name> [add <word> \"<meaning>\" [--value <v>] | remove <word>]",
        ));
    };
    let name = VocabName::new(name).map_err(usage)?;
    let change = match rest {
        [] => None,
        [verb, rest @ ..] if verb == "add" => {
            let mut value = None;
            let mut words = Vec::new();
            let mut rest = rest.iter();
            while let Some(argument) = rest.next() {
                match argument.as_str() {
                    "--value" => {
                        value = Some(
                            rest.next()
                                .ok_or_else(|| usage("--value needs a value"))?
                                .clone(),
                        );
                    }
                    flag if flag.starts_with("--") => {
                        return Err(usage(format!(
                            "{flag} is not a flag of vocab add; the flag is --value <v>"
                        )));
                    }
                    word => words.push(word),
                }
            }
            let [word, meaning] = words.as_slice() else {
                return Err(usage(format!(
                    "vocab {name} add takes a word and its meaning: evoke vocab {name} add <word> \"<meaning>\" [--value <v>]"
                )));
            };
            let word = Word::new(word).map_err(|why| usage(format!("word {why}")))?;
            let what = Clean::line(meaning)
                .map_err(|why| usage(format!("the meaning \"{meaning}\" {why}")))?;
            Some(VocabChange::Add {
                word,
                meaning: Meaning { what, value },
            })
        }
        [verb, word] if verb == "remove" => Some(VocabChange::Remove {
            word: Word::new(word).map_err(|why| usage(format!("word {why}")))?,
        }),
        [verb, ..] if verb == "remove" => {
            return Err(usage(format!("vocab {name} remove takes one word")));
        }
        [verb, ..] => {
            return Err(usage(format!(
                "vocab {name} takes add or remove, not {verb}"
            )));
        }
    };
    Ok(Command::Vocab { name, change })
}

/// `<name> <key> <value> | --env VAR`.
fn config(arguments: &[String]) -> Result<Command, Diagnostic> {
    const NEEDS: &str =
        "config takes a reflex, a key and a value: evoke config <name> <key> <value> | --env <VAR>";
    let (reflex, key, setting) = match arguments {
        [reflex, key, env, var] if env == "--env" => (
            reflex,
            key,
            Setting::Env {
                var: VarName::new(var).map_err(usage)?,
            },
        ),
        [reflex, key, value] if !value.starts_with("--") => (
            reflex,
            key,
            Setting::Plain {
                value: value.clone(),
            },
        ),
        [_, _, env] if env == "--env" => return Err(usage("--env needs a variable")),
        _ => return Err(usage(NEEDS)),
    };
    Ok(Command::Config {
        reflex: LocalName::new(reflex).map_err(usage)?,
        key: ConfigKey::new(key).map_err(usage)?,
        setting,
    })
}

/// `<ref>… [--as <name>]`: every ref parsed; `--as` names one reflex, so it goes with one ref.
fn add(arguments: &[String]) -> Result<Command, Diagnostic> {
    let mut refs = Vec::new();
    let mut name = None;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--as" => {
                let text = rest.next().ok_or_else(|| usage("--as needs a name"))?;
                name = Some(LocalName::new(text).map_err(usage)?);
            }
            flag if flag.starts_with('-') && flag.len() > 1 => {
                return Err(usage(format!(
                    "{flag} is not a flag of add; the flag is --as <name>"
                )));
            }
            text => {
                let location = if is_local(text) {
                    Location::Local {
                        path: text.to_owned(),
                    }
                } else {
                    let (reference, pin) = reference(text).map_err(help)?;
                    Location::Remote { reference, pin }
                };
                refs.push(Ref {
                    written: text.to_owned(),
                    location,
                });
            }
        }
    }
    if refs.is_empty() {
        return Err(usage(
            "add needs a ref: evoke add <owner/repo[/dir][@tag]>… [--as <name>]",
        ));
    }
    if name.is_some() && refs.len() > 1 {
        return Err(usage("--as names one reflex; add one ref with it"));
    }
    Ok(Command::Add { refs, name })
}

/// A directory rather than a repository: what `evoke.toml` writes a local reflex as, and `.` or `..` themselves.
fn is_local(text: &str) -> bool {
    matches!(text, "." | "..") || text.starts_with("./") || text.starts_with("../")
}

/// `[<name>] [--accept <name>]`.
fn update(arguments: &[String]) -> Result<Command, Diagnostic> {
    let mut reflex = None;
    let mut accept = None;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--accept" => {
                let text = rest.next().ok_or_else(|| usage("--accept needs a name"))?;
                accept = Some(LocalName::new(text).map_err(usage)?);
            }
            flag if flag.starts_with('-') && flag.len() > 1 => {
                return Err(usage(format!(
                    "{flag} is not a flag of update; the flag is --accept <name>"
                )));
            }
            text => {
                if reflex.is_some() {
                    return Err(usage(
                        "update takes one name: evoke update [<name>] [--accept <name>]",
                    ));
                }
                reflex = Some(LocalName::new(text).map_err(usage)?);
            }
        }
    }
    Ok(Command::Update { reflex, accept })
}

/// Arguments that spell no command: what is wrong, and `evoke --help` to see every command.
fn usage(message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        reflex: None,
        at: None,
        message: message.into(),
        fix: Fix::Help,
    }
}

/// A core grammar's refusal — a call, a ref — as an argument error: the same message, `evoke --help` as its fix.
fn help(refused: Diagnostic) -> Diagnostic {
    Diagnostic {
        fix: Fix::Help,
        ..refused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(arguments: &[&str], stdin_is_pipe: bool) -> Result<Command, Diagnostic> {
        parse(arguments.iter().map(OsString::from), stdin_is_pipe)
    }

    fn arguments_of(arguments: &[&str], stdin_is_pipe: bool) -> Arguments {
        match parsed(arguments, stdin_is_pipe) {
            Ok(Command::Try(arguments) | Command::Use(arguments)) => arguments,
            Ok(_) => panic!("not a deciding command"),
            Err(error) => panic!("{}", error.message),
        }
    }

    #[test]
    fn the_first_word_selects_the_command() {
        assert!(matches!(parsed(&["try", "x"], false), Ok(Command::Try(_))));
        assert!(matches!(parsed(&["x"], false), Ok(Command::Use(_))));
        assert!(matches!(
            parsed(&["--json", "x"], false),
            Ok(Command::Use(_))
        ));
        assert_eq!(parsed(&["why"], false), Ok(Command::Why));
        assert_eq!(
            parsed(&["edit", "lights"], false).unwrap_err().message,
            "evoke edit is not available yet"
        );
        assert_eq!(
            parsed(&["why", "now"], false).unwrap_err().message,
            "why takes no arguments"
        );
    }

    #[test]
    fn help_and_version_are_flags_in_the_first_place_and_words_nowhere() {
        assert_eq!(parsed(&["--help"], false), Ok(Command::Help));
        assert_eq!(parsed(&["-h"], true), Ok(Command::Help));
        assert_eq!(parsed(&["help"], false), Ok(Command::Help));
        assert_eq!(parsed(&["help", "add"], false), Ok(Command::Help));
        assert_eq!(parsed(&["--help", "show"], false), Ok(Command::Help));
        assert_eq!(parsed(&["--version"], false), Ok(Command::Version));
        assert_eq!(parsed(&["-V"], false), Ok(Command::Version));
        assert_eq!(Command::Help.invoked(""), "evoke --help");
        assert_eq!(Command::Version.invoked(""), "evoke --version");
        // Right after a command word, `--help` is the help; anywhere else it is a word or a flag refused.
        assert_eq!(parsed(&["show", "--help"], false), Ok(Command::Help));
        assert_eq!(parsed(&["add", "-h"], false), Ok(Command::Help));
        assert_eq!(parsed(&["try", "--help"], false), Ok(Command::Help));
        assert!(parsed(&["try", "x", "--help"], false).is_err());
        assert!(parsed(&["show", "lights", "--help"], false).is_err());
        assert_eq!(
            arguments_of(&["--", "--help"], false).input,
            Inputs::One("--help".to_owned())
        );
        assert_eq!(
            arguments_of(&["--", "help"], false).input,
            Inputs::One("help".to_owned())
        );
        assert_eq!(
            arguments_of(&["try", "help"], false).input,
            Inputs::One("help".to_owned())
        );
    }

    #[test]
    fn a_command_word_behind_a_flag_is_refused_not_decided() {
        let refused = parsed(&["--json", "try", "lock it"], false).unwrap_err();
        assert_eq!(
            refused.message,
            "try is a command word; write it first, or -- before it to say it"
        );
        assert_eq!(refused.fix, Fix::Help);
        assert!(parsed(&["--tag", "home", "show"], false).is_err());
        assert_eq!(
            arguments_of(&["--json", "--", "try", "it"], false).input,
            Inputs::One("try it".to_owned())
        );
        assert_eq!(
            arguments_of(&["--json", "lock", "it"], false).input,
            Inputs::One("lock it".to_owned())
        );
    }

    #[test]
    fn the_stand_in_is_the_input_or_its_placeholder() {
        let given = parsed(&["try", "--json", "kill the lights"], false).unwrap();
        assert_eq!(given.stand_in(), "kill the lights");
        assert_eq!(given.placeholder(), "evoke try --json \"kill the lights\"");
        assert_eq!(parsed(&[], true).unwrap().stand_in(), "<input>");
        assert_eq!(
            parsed(&["teach", "lights", "state=off"], false)
                .unwrap()
                .stand_in(),
            "<utterance>"
        );
        assert_eq!(parsed(&["show"], false).unwrap().stand_in(), "<input>");
        // The line to rerun never wraps a line already made.
        let command = parsed(&[], true).unwrap();
        assert_eq!(command.invoked(&command.stand_in()), "evoke \"<input>\"");
        // An input over the cap is not repeated in the line to rerun.
        let long = "a".repeat(Input::CAP + 1);
        assert_eq!(
            parsed(&[long.as_str()], false).unwrap().invoked(&long),
            "evoke \"<input>\""
        );
        let exact = "a".repeat(Input::CAP);
        assert!(
            parsed(&[exact.as_str()], false)
                .unwrap()
                .invoked(&exact)
                .contains(&exact)
        );
    }

    #[test]
    fn the_grammar_of_the_arguments() {
        let plain = arguments_of(&["kill the lights"], false);
        assert_eq!(plain.input, Inputs::One("kill the lights".to_owned()));
        assert!(!plain.json && plain.tags.is_empty());
        let full = arguments_of(&["try", "--json", "--tag", "home", "--", "--dash"], true);
        assert!(full.json);
        assert_eq!(full.tags[0].as_str(), "home");
        assert_eq!(full.input, Inputs::One("--dash".to_owned()));
        assert_eq!(arguments_of(&["try"], true).input, Inputs::Stdin);
        assert_eq!(arguments_of(&[], true).input, Inputs::Stdin);
        assert_eq!(arguments_of(&[], false).input, Inputs::Terminal);
        assert_eq!(
            arguments_of(&["-"], false).input,
            Inputs::One("-".to_owned())
        );
    }

    #[test]
    fn bare_words_are_one_input() {
        let bare = arguments_of(&["kill", "the", "lights"], false);
        assert_eq!(bare.input, Inputs::One("kill the lights".to_owned()));
        let tagged = arguments_of(&["try", "kill", "--tag", "home", "the", "lights"], false);
        assert_eq!(tagged.input, Inputs::One("kill the lights".to_owned()));
        assert_eq!(tagged.tags[0].as_str(), "home");
        let forced = arguments_of(&["--", "test", "the", "alarm", "--json"], false);
        assert_eq!(
            forced.input,
            Inputs::One("test the alarm --json".to_owned())
        );
        assert!(!forced.json);
        assert_eq!(
            parsed(&["kill", "the", "lights"], false)
                .unwrap()
                .invoked("kill the lights"),
            "evoke \"kill the lights\""
        );
        assert_eq!(
            parsed(&["", " "], false).unwrap_err().message,
            "evoke needs an input"
        );
    }

    #[test]
    fn the_tune_commands_and_their_lines() {
        let lines = [
            (
                &["run", "lights", "room=den", "state=off"][..],
                "evoke run lights room=den state=off",
            ),
            (
                &["run", "timer duration=\"10 minutes\""],
                "evoke run timer duration=\"10 minutes\"",
            ),
            (
                &["run", "timer", "duration=10 minutes", "label=\"eggs\""],
                "evoke run timer duration=\"10 minutes\" label=eggs",
            ),
            (
                &["teach", "start one", "timer", "duration=10 minutes"],
                "evoke teach \"start one\" timer duration=\"10 minutes\"",
            ),
            (
                &["teach", "kill the lights", "lights", "state=off"],
                "evoke teach \"kill the lights\" lights state=off",
            ),
            (
                &["teach", "lights", "state=off"],
                "evoke teach \"<utterance>\" lights state=off",
            ),
            (
                &["teach", "what time is it", "not", "timer"],
                "evoke teach \"what time is it\" not timer",
            ),
            (
                &["teach", "not", "timer"],
                "evoke teach \"<utterance>\" not timer",
            ),
            (
                &["teach", "kill", "lights", "state=off"],
                "evoke teach kill lights state=off",
            ),
            (
                &["teach", "kill", "not", "timer"],
                "evoke teach \"kill\" not timer",
            ),
            (&["teach", "lights"], "evoke teach \"<utterance>\" lights"),
            (&["show"], "evoke show"),
            (&["show", "lights"], "evoke show lights"),
            (&["vocab", "rooms"], "evoke vocab rooms"),
            (
                &["vocab", "rooms", "add", "den", "The TV room."],
                "evoke vocab rooms add den \"The TV room.\"",
            ),
            (
                &[
                    "vocab", "rooms", "add", "the snug", "The den.", "--value", "group-7",
                ],
                "evoke vocab rooms add \"the snug\" \"The den.\" --value group-7",
            ),
            (
                &["vocab", "rooms", "remove", "den"],
                "evoke vocab rooms remove den",
            ),
            (
                &["config", "lights", "bridge", "10.0.0.2"],
                "evoke config lights bridge 10.0.0.2",
            ),
            (
                &["config", "lights", "token", "--env", "HUE_TOKEN"],
                "evoke config lights token --env HUE_TOKEN",
            ),
        ];
        for (arguments, line) in lines {
            let command = parsed(arguments, false)
                .unwrap_or_else(|error| panic!("{arguments:?}: {}", error.message));
            assert_eq!(command.placeholder(), line, "{arguments:?}");
        }
        assert!(matches!(
            parsed(&["teach", "kill the lights", "lights", "state=off"], false),
            Ok(Command::Teach {
                spoken: Spoken::Given(_),
                lesson: Taught::Call(_)
            })
        ));
        assert!(matches!(
            parsed(&["teach", "lights", "state=off"], false),
            Ok(Command::Teach {
                spoken: Spoken::Last,
                ..
            })
        ));
        let Ok(Command::Vocab {
            change: Some(VocabChange::Add { meaning, .. }),
            ..
        }) = parsed(
            &["vocab", "rooms", "add", "den", "The den.", "--value", "g"],
            false,
        )
        else {
            panic!("an add");
        };
        assert_eq!(meaning.value.as_deref(), Some("g"));
    }

    #[test]
    fn run_takes_its_flag_first_and_the_call_after() {
        let Ok(Command::Run { written, json }) =
            parsed(&["run", "--json", "lights", "room=den"], false)
        else {
            panic!("a run");
        };
        assert!(json);
        assert_eq!(written.to_string(), "lights room=den");
        let command = parsed(&["run", "--json", "lights", "room=den"], false).unwrap();
        assert_eq!(command.placeholder(), "evoke run --json lights room=den");
        assert!(matches!(
            parsed(&["run", "lights", "room=den"], false),
            Ok(Command::Run { json: false, .. })
        ));
        assert_eq!(
            parsed(&["run", "--loud", "lights"], false)
                .unwrap_err()
                .message,
            "--loud is not a flag of run; the flag is --json"
        );
    }

    #[test]
    fn a_first_word_that_could_be_either_is_left_to_the_command_with_both_readings() {
        let Ok(Command::Teach {
            spoken: Spoken::Either { word, whole },
            lesson: Taught::Call(following),
        }) = parsed(&["teach", "kill", "lights", "state=off"], false)
        else {
            panic!("either");
        };
        assert_eq!(word.as_str(), "kill");
        assert_eq!(whole.to_string(), "kill lights state=off");
        assert_eq!(following.to_string(), "lights state=off");
        // A word followed by what is no call of its own begins the call; a lone word is one.
        assert!(matches!(
            parsed(&["teach", "lihgts", "state=off"], false),
            Ok(Command::Teach {
                spoken: Spoken::Last,
                ..
            })
        ));
        assert!(matches!(
            parsed(&["teach", "lights"], false),
            Ok(Command::Teach {
                spoken: Spoken::Last,
                lesson: Taught::Call(_)
            })
        ));
        // `not` after the word makes it the utterance; a word that is no name always is.
        assert!(matches!(
            parsed(&["teach", "kill", "not", "timer"], false),
            Ok(Command::Teach {
                spoken: Spoken::Given(_),
                lesson: Taught::Not(_)
            })
        ));
        assert!(matches!(
            parsed(&["teach", "Kill", "lights", "state=off"], false),
            Ok(Command::Teach {
                spoken: Spoken::Given(_),
                ..
            })
        ));
        assert_eq!(
            parsed(&["teach", "kill", "room="], false).unwrap_err().fix,
            Fix::Help
        );
    }

    #[test]
    fn the_install_commands_and_their_lines() {
        let lines = [
            (
                &["add", "radhi/home/lights", "radhi/timer@1.0.1"][..],
                "evoke add radhi/home/lights radhi/timer@1.0.1",
            ),
            (
                &["add", "radhi/home/lights", "--as", "lamps"],
                "evoke add radhi/home/lights --as lamps",
            ),
            (&["remove", "lights"], "evoke remove lights"),
            (&["update"], "evoke update"),
            (&["update", "lights"], "evoke update lights"),
            (
                &["update", "--accept", "lights"],
                "evoke update --accept lights",
            ),
            (&["sync"], "evoke sync"),
            (&["trust"], "evoke trust"),
        ];
        for (arguments, line) in lines {
            let command = parsed(arguments, false)
                .unwrap_or_else(|error| panic!("{arguments:?}: {}", error.message));
            assert_eq!(command.placeholder(), line, "{arguments:?}");
        }
        let Ok(Command::Add { refs, name }) =
            parsed(&["add", "radhi/timer@1.0.1", "--as", "eggs"], false)
        else {
            panic!("an add");
        };
        assert!(matches!(
            &refs[0].location,
            Location::Remote { pin: Some(pin), .. } if pin.to_string() == "1.0.1"
        ));
        assert_eq!(name.map(|name| name.to_string()), Some("eggs".to_owned()));
        let Ok(Command::Add { refs, .. }) = parsed(&["add", "./hello", "..", "."], false) else {
            panic!("an add");
        };
        let paths: Vec<&str> = refs
            .iter()
            .map(|r| match &r.location {
                Location::Local { path } => path.as_str(),
                Location::Remote { .. } => "remote",
            })
            .collect();
        assert_eq!(paths, ["./hello", "..", "."]);
    }

    #[test]
    fn the_author_commands_and_their_lines() {
        let lines = [
            (&["new", "hello"][..], "evoke new hello"),
            (&["check"], "evoke check"),
            (&["test"], "evoke test"),
            (&["test", "timer"], "evoke test timer"),
        ];
        for (arguments, line) in lines {
            let command = parsed(arguments, false)
                .unwrap_or_else(|error| panic!("{arguments:?}: {}", error.message));
            assert_eq!(command.placeholder(), line, "{arguments:?}");
        }
        for arguments in [
            &["new"][..],
            &["new", "Hello"],
            &["new", "a", "b"],
            &["check", "lights"],
            &["test", "a", "b"],
            &["test", "none"],
        ] {
            assert_eq!(
                parsed(arguments, false).unwrap_err().fix,
                Fix::Help,
                "{arguments:?}"
            );
        }
    }

    #[test]
    fn the_invoked_line_repeats_the_command() {
        let full = parsed(&["try", "--json", "--tag", "home", "x"], false).unwrap();
        assert_eq!(
            full.invoked("say \"hi\""),
            "evoke try --json --tag home \"say \\\"hi\\\"\""
        );
        let bare = parsed(&["--tag", "home", "x"], false).unwrap();
        assert_eq!(bare.invoked("x"), "evoke --tag home \"x\"");
        assert_eq!(bare.placeholder(), "evoke --tag home \"x\"");
        assert_eq!(
            parsed(&[], true).unwrap().placeholder(),
            "evoke \"<input>\""
        );
        assert_eq!(Command::Why.invoked(""), "evoke why");
        let teach = parsed(&["teach", "lights", "state=off"], false).unwrap();
        assert_eq!(
            teach.invoked("dim the office"),
            "evoke teach \"dim the office\" lights state=off"
        );
    }

    #[test]
    fn everything_else_is_a_diagnostic_that_ends_in_help() {
        for arguments in [
            &["try"][..],
            &["try", ""],
            &["try", "  "],
            &["try", "--tag"],
            &["try", "--tag", "Home", "x"],
            &["try", "--loud", "x"],
            &[""],
            &["--tag"],
            &["run"],
            &["run", "Lights"],
            &["run", "--json"],
            &["run", "--loud", "lights"],
            &["run", "lights", "--json"],
            &["teach"],
            &["teach", "kill the lights"],
            &["teach", "kill the lights", "not"],
            &["teach", "kill the lights", "not", "a", "b"],
            &["show", "a", "b"],
            &["show", "Lights"],
            &["vocab"],
            &["vocab", "rooms", "add", "den"],
            &["vocab", "rooms", "add", "den", "x", "--value"],
            &["vocab", "rooms", "add", "none", "x"],
            &["vocab", "rooms", "remove"],
            &["vocab", "rooms", "drop", "den"],
            &["config", "lights", "bridge"],
            &["config", "lights", "token", "--env"],
            &["config", "lights", "token", "--env", "1x"],
            &["add"],
            &["add", "radhi"],
            &["add", "radhi/home", "radhi/timer", "--as", "x"],
            &["add", "radhi/home", "--as"],
            &["add", "radhi/home", "--json"],
            &["remove"],
            &["remove", "a", "b"],
            &["update", "a", "b"],
            &["update", "--accept"],
            &["sync", "now"],
            &["trust", "me"],
        ] {
            let error = parsed(arguments, false).unwrap_err();
            assert_eq!(error.fix, Fix::Help, "{arguments:?}");
        }
        assert_eq!(
            parsed(&["try"], false).unwrap_err().message,
            "try needs an input"
        );
        assert_eq!(
            parsed(&["run", "lights", "room="], false)
                .unwrap_err()
                .message,
            "room= needs a value: bare text or a JSON string"
        );
        assert_eq!(
            parsed(&["vocab", "rooms", "add", "none", "x"], false)
                .unwrap_err()
                .message,
            "word \"none\" is reserved"
        );
    }
}
