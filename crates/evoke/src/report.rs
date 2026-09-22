//! Render decisions, `try`, `why`, `show`, `vocab`, `update`, `check`, `test`, the installed rows, the prompts,
//! every failure with its fix, and the one line a decision leaves — printed by `--json`, kept by the log; pure.
//! In: core values, an `Exit`, the invoked line and the owned files' paths as shown. Out: `Text` for the terminal
//! — the plain words, with the roles it may weight: the call, the effect, the weakest judgment, the arrow of a
//! fix, the sign of a write — and the prompts and the JSON line as plain strings. The blocks of `try`, `why`,
//! `use`, `show`, `add`, `update`, `check`, `test` and the JSON line are pinned by the transcripts; the failure
//! line, its JSON object and the roles by the tests here.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use evoke_core::adapter::QuestionId;
use evoke_core::contract::Change;
use evoke_core::decide::{Missing, Why};
use evoke_core::manifest::{Effect, Manifest, Run};
use evoke_core::name::LocalName;
use evoke_core::test::{Claim, Expected, Mismatch};
use evoke_core::vocabulary::Vocabulary;
use evoke_core::{
    At, Call, Case, Chosen, Clean, Contender, ContractDiff, Decision, Diagnostic, Effective, File,
    Finding, Fix, Gate, Input, Json, KeyPath, Level, Prompt, Proposed, Raw, Regression, Verdict,
    Version, render,
};

use crate::adapter::Trace;
use crate::commands::Exit;
use crate::commands::session::Decided;
use crate::hosts::Failure;
use crate::hosts::files::{Edited, Landed};
use crate::hosts::processes::Returned;
use crate::hosts::terminal::{Role, Text};

/// The owned files as a person reads them: the root, `~/…` under `$HOME`, and each local reflex's directory as
/// `evoke.toml` writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    pub root: String,
    pub reflexes: BTreeMap<LocalName, String>,
}

impl Paths {
    /// `<root>/<file>:<line>:<column>`; a manifest under its reflex's directory — a remote one's in the store,
    /// shown where it is.
    fn at(&self, at: &At) -> String {
        let file = match &at.file {
            File::Manifest { name } => {
                self.reflexes
                    .get(name)
                    .map(|dir| match dir.trim_start_matches("./") {
                        "" | "." => "reflex.toml".to_owned(),
                        dir => format!("{dir}/reflex.toml"),
                    })
            }
            _ => None,
        };
        let file = file.unwrap_or_else(|| at.file.to_string());
        if file.starts_with('/') || file.starts_with('~') {
            format!("{file}:{}:{}", at.line, at.column)
        } else {
            format!("{}/{file}:{}:{}", self.root, at.line, at.column)
        }
    }
}

/// What an exit says, unless it ran or was declined: one line for the terminal.
#[must_use]
pub fn exit(exit: &Exit, invoked: &str, paths: Option<&Paths>) -> Option<Text> {
    problem(exit).map(|problem| diagnostic(&problem, invoked, paths))
}

/// What an exit says, unless it ran or was declined: one JSON object with the same fields, for a `--json` filter.
#[must_use]
pub fn exit_json(exit: &Exit) -> Option<String> {
    problem(exit).map(|problem| serde_json::to_string(&problem).expect("a diagnostic serializes"))
}

/// Every exit that needs a line, in the one shape a problem takes: the reflex, the location, the message, the fix.
fn problem(exit: &Exit) -> Option<Diagnostic> {
    let (message, fix) = match exit {
        Exit::Ran | Exit::Declined(_) => return None,
        Exit::Human(problem) => return Some(problem.clone()),
        Exit::Failed(failed) => (failure(failed), failed.fix.clone()),
        Exit::Adapter(fault) => (fault.to_string(), fault.fix()),
    };
    Some(Diagnostic {
        reflex: None,
        at: None,
        message,
        fix,
    })
}

/// A failure as one message: what was attempted, and the cause when there is one.
#[must_use]
pub fn failure(failure: &Failure) -> String {
    match &failure.cause {
        Some(cause) => format!("{}: {}", failure.what, plain(cause)),
        None => failure.what.clone(),
    }
}

/// `  <reflex>: <message>  →  <fix>`, the reflex when there is one; a line to edit shows its path under the root.
#[must_use]
pub fn diagnostic(problem: &Diagnostic, invoked: &str, paths: Option<&Paths>) -> Text {
    let mut text = Text::from("  ");
    text.push(&plain(&head(problem)));
    fixed(&mut text, &fixing(problem, invoked, paths));
    text
}

/// The inactive lines of `show`: every problem with its fix, the arrows aligned.
#[must_use]
pub fn inactive(problems: &[Diagnostic], invoked: &str, paths: &Paths) -> Text {
    let heads: Vec<String> = problems.iter().map(head).collect();
    let width = heads.iter().map(|head| chars(head)).max().unwrap_or(0);
    Text::lines(problems.iter().zip(&heads).map(|(problem, head)| {
        let mut line = Text::from("  ");
        line.roled(Role::Warning, "inactive")
            .push(&plain(&format!("  {head:<width$}")));
        fixed(&mut line, &fixing(problem, invoked, Some(paths)));
        line
    }))
}

/// `  →  <fix>` on the end of a line: the arrow with its role, the fix as plain as its text.
fn fixed(text: &mut Text, fix: &str) {
    text.push("  ")
        .roled(Role::Fix, "→")
        .push("  ")
        .push(&plain(fix));
}

/// `<reflex>: <message>`, or the message alone.
fn head(problem: &Diagnostic) -> String {
    match &problem.reflex {
        Some(reflex) => format!("{reflex}: {}", problem.message),
        None => problem.message.clone(),
    }
}

/// The fixing command; a line to edit shows its path under the root when the paths are known.
fn fixing(problem: &Diagnostic, invoked: &str, paths: Option<&Paths>) -> String {
    match (&problem.fix, paths) {
        (Fix::EditLine { at }, Some(paths)) => paths.at(at),
        (fix, _) => fix.command(invoked),
    }
}

/// Text that cannot repaint the terminal. The input is untrusted and is echoed in a fault and in the line to
/// rerun, so a character `Clean` refuses, or a line feed, shows as its escape.
fn plain(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    for c in text.chars() {
        if c != '\n' && Clean::new(c.encode_utf8(&mut [0; 4])).is_ok() {
            plain.push(c);
        } else {
            plain.extend(c.escape_default());
        }
    }
    plain
}

/// One decision's line. `json` is what `--json` prints and a filter reads: the input, the decision's fields, the
/// trace — one `{ adapter, questions, ms }` per call, none when the cache answered — the result when a body ran,
/// the error when it failed. `log` is the same line with the adapter's answers and the input's candidates beside
/// it, which `why` reads back.
pub struct Line {
    pub input: Input,
    pub decision: Decision,
    pub trace: Vec<Trace>,
    pub answers: Raw,
    pub proposed: Vec<Proposed>,
    pub result: Option<Returned>,
    pub error: Option<String>,
}

impl Line {
    #[must_use]
    pub fn of(decided: &Decided) -> Self {
        Self {
            input: decided.request.state.request.clone(),
            decision: decided.decision.clone(),
            trace: decided.trace.clone(),
            answers: decided.answers.clone(),
            proposed: decided.request.proposed.clone(),
            result: None,
            error: None,
        }
    }

    #[must_use]
    pub fn json(&self) -> String {
        self.fields(false).to_string()
    }

    #[must_use]
    pub fn log(&self) -> String {
        self.fields(true).to_string()
    }

    fn fields(&self, whole: bool) -> Json {
        let Ok(Json::Object(decision)) = serde_json::to_value(&self.decision) else {
            unreachable!("a decision serializes as an object")
        };
        let mut line = serde_json::Map::new();
        line.insert(
            "input".to_owned(),
            Json::String(self.input.as_str().to_owned()),
        );
        line.extend(decision);
        line.insert(
            "trace".to_owned(),
            serde_json::to_value(&self.trace).expect("a trace serializes"),
        );
        if let Some(result) = &self.result {
            line.insert(
                "result".to_owned(),
                serde_json::to_value(result).expect("a result serializes"),
            );
        }
        if let Some(error) = &self.error {
            line.insert("error".to_owned(), Json::String(error.clone()));
        }
        if whole {
            line.insert(
                "answers".to_owned(),
                serde_json::to_value(&self.answers).expect("answers serialize"),
            );
            line.insert(
                "proposed".to_owned(),
                serde_json::to_value(&self.proposed).expect("candidates serialize"),
            );
        }
        Json::Object(line)
    }

    /// A log line read back; the decision is what remains once the line's own fields are taken.
    pub fn parse(text: &str) -> Result<Self, String> {
        let Json::Object(mut fields) =
            serde_json::from_str(text).map_err(|error| error.to_string())?
        else {
            return Err("not an object".to_owned());
        };
        let mut take = |key: &str| fields.remove(key).unwrap_or(Json::Null);
        let input = field("input", take("input"))?;
        let trace = field("trace", take("trace"))?;
        let answers = field("answers", take("answers"))?;
        let proposed = field("proposed", take("proposed"))?;
        let result = field("result", take("result"))?;
        let error = field("error", take("error"))?;
        let decision = field("decision", Json::Object(fields))?;
        Ok(Self {
            input,
            decision,
            trace,
            answers,
            proposed,
            result,
            error,
        })
    }

    /// The reflex the decision is about, when it is about one.
    fn reflex(&self) -> Option<&LocalName> {
        match &self.decision {
            Decision::Abstain { .. } => None,
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
                Some(&chosen.call.reflex)
            }
            Decision::Ask { asking, .. } => Some(&asking.reflex),
        }
    }

    fn contenders(&self) -> &[Contender] {
        match &self.decision {
            Decision::Abstain { contenders, .. } => contenders,
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => chosen
                .judged
                .as_ref()
                .map_or(&[], |judged| judged.contenders()),
            Decision::Ask { asking, .. } => asking.judged.contenders(),
        }
    }
}

/// One field of a log line, typed.
fn field<T: serde::de::DeserializeOwned>(what: &str, value: Json) -> Result<T, String> {
    serde_json::from_value(value).map_err(|error| format!("{what}: {error}"))
}

/// `try`: the ranking, every judgment about the winner's arguments, `fits`, and the outcome with its weakest
/// judgment.
#[must_use]
pub fn tried(decided: &Decided, route_floor: Option<evoke_core::Prob>) -> Text {
    let winner = decided.reading.winner.as_ref().map(|winner| &winner.reflex);
    // A winner that the gate still abstained on was under the route floor.
    let under = match (&decided.decision, winner, route_floor) {
        (Decision::Abstain { .. }, Some(_), Some(floor)) => Some(floor),
        _ => None,
    };
    let mut lines: Vec<Text> = block(
        winner,
        &decided.answers,
        &decided.request.proposed,
        &decided.reading.ranking,
        under,
    )
    .into_iter()
    .map(Text::from)
    .collect();
    lines.push(outcome(&decided.decision));
    indented(lines)
}

/// `why`: the input, the block `try` shows, and what came of it.
#[must_use]
pub fn why(line: &Line) -> Text {
    let mut lines = vec![Text::from(plain(&quoted(line.input.as_str())))];
    lines.extend(
        block(
            line.reflex(),
            &line.answers,
            &line.proposed,
            line.contenders(),
            None,
        )
        .into_iter()
        .map(Text::from),
    );
    let mut what = match (&line.decision, &line.result) {
        (Decision::Run { chosen } | Decision::Confirm { chosen, .. }, Some(_)) => {
            let mut what = Text::from("ran ");
            what.append(judged_call(chosen));
            what
        }
        (Decision::Run { chosen } | Decision::Confirm { chosen, .. }, None) => {
            let word = if line.error.is_some() {
                "failed "
            } else {
                "confirm "
            };
            let mut what = Text::from(word);
            what.append(judged_call(chosen));
            what
        }
        (Decision::Ask { missing, .. }, _) => {
            let asked: Vec<&str> = missing.iter().map(|missing| missing.arg.as_str()).collect();
            Text::from(format!("ask {}", asked.join(" ")))
        }
        (Decision::Abstain { .. }, _) => Text::from("abstain"),
    };
    let calls: Vec<String> = line
        .trace
        .iter()
        .map(|trace| format!("{}, {} questions", trace.adapter, trace.questions))
        .collect();
    let calls = if calls.is_empty() {
        "cached".to_owned()
    } else {
        calls.join(" · ")
    };
    what.push(" · ").push(&calls);
    lines.push(what);
    indented(lines)
}

/// The ranking alone, as an abstain shows it.
#[must_use]
pub fn abstained(decided: &Decided, floors: Option<&Gate>) -> Text {
    let under = decided.reading.winner.as_ref().and(floors.map(Gate::route));
    indented(vec![Text::from(ranking(&decided.answers, under))])
}

/// After an abstain, the reflexes that were never offered: `open and visit are inactive  →  evoke show`; nothing
/// when every reflex was.
#[must_use]
pub fn left_out<'n>(inactive: impl IntoIterator<Item = &'n LocalName>) -> Option<Text> {
    let names: Vec<String> = inactive.into_iter().map(ToString::to_string).collect();
    let (last, rest) = names.split_last()?;
    let listed = if rest.is_empty() {
        format!("{last} is inactive")
    } else {
        format!("{} and {last} are inactive", rest.join(", "))
    };
    let problem = Diagnostic {
        reflex: None,
        at: None,
        message: listed,
        fix: Fix::Show { reflex: None },
    };
    Some(diagnostic(&problem, "", None))
}

/// The run line: the call, then its confidence.
#[must_use]
pub fn running(chosen: &Chosen) -> Text {
    let mut text = Text::from("  ");
    text.append(call(&chosen.call));
    if let Some(judged) = &chosen.judged {
        text.push("  ")
            .roled(Role::Weak, &format!("{:.2}", judged.confidence().get()));
    }
    text
}

/// `evoke`'s own line before a confirm.
#[must_use]
pub fn confirming(chosen: &Chosen, prompt: &Prompt) -> Text {
    let mut text = Text::from("  ");
    text.append(own(chosen, &prompt.own));
    text
}

/// The call on one line, the reflex's name carrying the weight.
fn call(call: &Call) -> Text {
    let rendered = render(call);
    let name = call.reflex.as_str();
    let mut text = Text::new();
    text.roled(Role::Call, name).push(&rendered[name.len()..]);
    text
}

/// The prompt's own line, word for word as the core wrote it, with its roles found by its shape: the call, the
/// effect, then the weakest judgment when it comes next; the caps after it as they are. A line shaped otherwise
/// stays whole and plain.
fn own(chosen: &Chosen, own: &str) -> Text {
    let head = format!("{} · {}", render(&chosen.call), chosen.effect);
    let Some(rest) = own.strip_prefix(head.as_str()) else {
        return Text::from(own);
    };
    let mut text = call(&chosen.call);
    text.push(" · ")
        .roled(Role::Effect(chosen.effect), &chosen.effect.to_string());
    match rest.strip_prefix(" · weakest: ") {
        Some(judged) => {
            let (weakest, caps) = judged
                .split_once(" · ")
                .map_or((judged, ""), |(weakest, caps)| (weakest, caps));
            text.push(" · ")
                .roled(Role::Weak, &format!("weakest: {weakest}"));
            if !caps.is_empty() {
                text.push(" · ").push(caps);
            }
        }
        None => {
            text.push(rest);
        }
    }
    text
}

/// The confirm prompt itself, read on the terminal; `[t]each` only where there is an utterance to teach.
#[must_use]
pub fn confirm_prompt(prompt: &Prompt, teachable: bool) -> String {
    let choices = if teachable {
        "[y]es [n]o [t]each"
    } else {
        "[y]es [n]o"
    };
    format!("  {}  {choices} > ", prompt.template)
}

/// The ask prompt: numbered choices, a vocabulary's `[+] add one`, or a pick typed freely; `retry` says why the
/// last answer did not do.
#[must_use]
pub fn ask_prompt(missing: &Missing, retry: Option<&str>) -> String {
    use evoke_core::decide::Choices;
    let mut line = format!("  {}  ", missing.ask);
    if let Some(retry) = retry {
        let _ = write!(line, "{retry}  ");
    }
    match &missing.choices {
        Choices::Options { options } => {
            for (i, key) in options.keys().enumerate() {
                let _ = write!(line, "[{}] {key}  ", i + 1);
            }
        }
        Choices::Vocab { words } => {
            for (i, word) in words.keys().enumerate() {
                let _ = write!(line, "[{}] {word}  ", i + 1);
            }
            line.push_str("[+] add one  ");
        }
        Choices::Pick { .. } => {}
    }
    line.push_str("> ");
    line
}

/// Why an argument is asked again: `150 percent is outside 0–100`.
#[must_use]
pub fn because(why: &Why) -> String {
    match why {
        Why::Unstated => String::new(),
        Why::OutOfRange { span, range } => {
            format!("{} is outside {}–{}", span.text(), range.min(), range.max())
        }
    }
}

/// `[+] add one`, first prompt: the word; `retry` says why the last one did not do.
#[must_use]
pub fn word_prompt(retry: Option<&str>) -> String {
    match retry {
        Some(retry) => format!("  Word?  word {retry}  > "),
        None => "  Word?  > ".to_owned(),
    }
}

/// `[+] add one`, second prompt: what the word means.
#[must_use]
pub fn meaning_prompt(retry: Option<&str>) -> String {
    match retry {
        Some(retry) => format!("  Meaning?  {retry}  > "),
        None => "  Meaning?  > ".to_owned(),
    }
}

/// `evoke --help`: the version with the thesis, then every command by group — its line, then what it does, the
/// descriptions aligned — and the exit codes. Plain: it goes to stdout.
#[must_use]
pub fn help(version: &str) -> String {
    let width = COMMANDS
        .iter()
        .flat_map(|(_, lines)| lines.iter())
        .filter(|(_, about)| !about.is_empty())
        .map(|(line, _)| chars(line))
        .max()
        .unwrap_or(0);
    let mut text = format!("evoke {version} · you invoke a function; you evoke a reflex\n");
    for (group, lines) in COMMANDS {
        let _ = write!(text, "\n{group}\n");
        for (line, about) in lines {
            if about.is_empty() {
                let _ = writeln!(text, "  {line}");
            } else {
                let _ = writeln!(text, "  {line:<width$}  {about}");
            }
        }
    }
    text.push_str("\nexit  0 ran · 1 failed · 2 declined · 3 needs a human · 4 adapter failed");
    text
}

/// The commands as `--help` lists them: by group, each line with what it does; a line without a description
/// stands alone, and one indented under a command belongs to it.
const COMMANDS: [(&str, &[(&str, &str)]); 4] = [
    (
        "use",
        &[
            ("evoke \"<input>\"", "decide, gate, run"),
            ("evoke", "the REPL; a piped line is one input"),
            (
                "evoke try \"<input>\"",
                "decide only, and show every judgment",
            ),
            ("  --json", "one JSON line per input, for a filter"),
            ("  --tag <tag>", "only the reflexes carrying the tag"),
            ("  --", "the rest is input, even a command word"),
            ("evoke why", "the last decision, explained"),
            ("evoke run <call>", "by name, without the classifier"),
        ],
    ),
    (
        "install",
        &[
            (
                "evoke add <ref>… [--as <name>]",
                "fetch, lint, lock, install",
            ),
            ("evoke remove <name>", ""),
            (
                "evoke update [<name>]",
                "each remote reflex to its newest tag",
            ),
            (
                "evoke update --accept <name>",
                "a moved contract or effect, accepted",
            ),
            ("evoke sync", "the lock realised on this machine"),
            ("evoke trust", "this project, trusted at its content"),
        ],
    ),
    (
        "tune",
        &[
            (
                "evoke show [<name>]",
                "the installed reflexes, or one as used",
            ),
            (
                "evoke teach [\"<utterance>\"] <call>",
                "the utterance means this call",
            ),
            (
                "evoke teach [\"<utterance>\"] not <name>",
                "the utterance is not this reflex",
            ),
            ("  [\"<utterance>\"] omitted means the last input", ""),
            ("evoke vocab <name>", "the words and their meanings"),
            (
                "evoke vocab <name> add <word> \"<meaning>\" [--value <v>]",
                "",
            ),
            ("evoke vocab <name> remove <word>", ""),
            (
                "evoke config <name> <key> <value>",
                "a setting; a secret as --env <VAR>",
            ),
            ("evoke test [<name>]", "every example and test, judged"),
        ],
    ),
    (
        "author",
        &[
            ("evoke new <name>", "a working reflex from the template"),
            ("evoke check", "lint, types, contract against its tag"),
        ],
    ),
];

/// The line of a write: `+` the file and the line as it landed, or `-` the file and the key that went.
#[must_use]
pub fn written(edited: &Edited) -> Text {
    let mut text = Text::new();
    match &edited.landed {
        Landed::Set(line) => text
            .roled(Role::Added, "+")
            .push(&format!(" {}  {line}", edited.shown)),
        Landed::Removed(key) => text
            .roled(Role::Removed, "-")
            .push(&format!(" {}  {key}", edited.shown)),
    };
    text
}

/// One installed reflex as `show`, `add`, `remove` and `sync` list it: its name, where it comes from with its
/// locked version, the effect it runs under — none when its manifest does not read — what it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub from: String,
    pub effect: Option<Effect>,
    pub runs: String,
}

/// What stands before a row: two spaces listed, `+` added, `-` removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gutter {
    Listed,
    Added,
    Removed,
}

/// Rows with their columns aligned, each behind the gutter.
#[must_use]
pub fn rows(rows: &[Row], gutter: Gutter) -> Text {
    let effect = |row: &Row| {
        row.effect
            .map(|effect| effect.to_string())
            .unwrap_or_default()
    };
    let width = |column: &dyn Fn(&Row) -> usize| rows.iter().map(column).max().unwrap_or(0);
    let (names, froms, effects) = (
        width(&|row| chars(&row.name)),
        width(&|row| chars(&row.from)),
        width(&|row| chars(&effect(row))),
    );
    Text::lines(rows.iter().map(|row| {
        let mut line = Text::new();
        match gutter {
            Gutter::Listed => line.push("  "),
            Gutter::Added => line.roled(Role::Added, "+").push(" "),
            Gutter::Removed => line.roled(Role::Removed, "-").push(" "),
        };
        line.push(&format!("{:<names$}  {:<froms$}  ", row.name, row.from));
        let shown = effect(row);
        if let Some(effect) = row.effect {
            line.roled(Role::Effect(effect), &shown);
        }
        line.push(&" ".repeat(effects - chars(&shown)))
            .push("  ")
            .push(&row.runs);
        line.trim_end()
    }))
}

/// `evoke trust`: the root blessed.
#[must_use]
pub fn trusted(root: &str) -> Text {
    let mut text = Text::new();
    text.roled(Role::Added, "+")
        .push(&format!(" trusted {root}"));
    text
}

/// A file written whole where none was, or changed: `+ <path>`.
#[must_use]
pub fn created(path: &str) -> Text {
    let mut text = Text::new();
    text.roled(Role::Added, "+").push(&format!(" {path}"));
    text
}

/// `check`'s row: the reflex as its directory names it, the effect it claims, what it runs.
#[must_use]
pub fn checked_row(name: &LocalName, m: &Manifest) -> Text {
    let mut text = Text::from(format!("  {name}  "));
    text.roled(Role::Effect(m.effect), &m.effect.to_string())
        .push(&format!("  {}", runs(&m.run)));
    text
}

/// `check`'s contract block against the newest tag: the level, the version the next tag must carry when the
/// contract moved, and one aligned line per change.
#[must_use]
pub fn checked(from: Version, contract: &ContractDiff) -> Text {
    let next = match contract.level {
        Level::Same => None,
        Level::Minor => Some(Version {
            major: from.major,
            minor: from.minor + 1,
            patch: 0,
        }),
        Level::Major => Some(Version {
            major: from.major + 1,
            minor: 0,
            patch: 0,
        }),
    };
    let mut text = match next {
        Some(next) => format!("  {from} → {next}  {}", level(contract.level)),
        None => format!("  {from}  {}", level(contract.level)),
    };
    let lines: Vec<(String, String)> = contract.changes.iter().map(change).collect();
    text.push_str(&aligned(&lines));
    Text::from(text)
}

/// One contract change as `update` and `check` print it: the key and what happened to it.
#[must_use]
pub fn change(change: &Change) -> (String, String) {
    match change {
        Change::ArgRenamed { from, to } => (format!("args.{from}"), format!("renamed {to}")),
        Change::ArgRemoved { arg } => (format!("args.{arg}"), "removed".to_owned()),
        Change::OptionRemoved { arg, key } => {
            (format!("args.{arg}.options.{key}"), "removed".to_owned())
        }
        Change::SourceChanged { arg } => (format!("args.{arg}"), "source changed".to_owned()),
        Change::RangeChanged { arg } => (format!("args.{arg}"), "range changed".to_owned()),
        Change::RunChanged => ("run".to_owned(), "changed".to_owned()),
        Change::ArgAdded { arg } => (format!("args.{arg}"), "added".to_owned()),
        Change::OptionAdded { arg, key } => {
            (format!("args.{arg}.options.{key}"), "added".to_owned())
        }
        Change::ConfigAdded { key } => (format!("config.{key}"), "added".to_owned()),
        Change::ConfigRemoved { key } => (
            format!("config.{key}"),
            "removed; your setting is skipped".to_owned(),
        ),
    }
}

/// `test`'s block: per reflex its name and counts, then one line per failed case — the utterance and where the
/// decision missed, `· regression` when it passed at the last run.
#[must_use]
pub fn tested(verdicts: &[(Case, Verdict)], regressions: &[Regression]) -> Text {
    let mut reflexes: Vec<(&LocalName, Vec<&(Case, Verdict)>)> = Vec::new();
    for judged in verdicts {
        match reflexes.last_mut() {
            Some((name, cases)) if *name == &judged.0.reflex => cases.push(judged),
            _ => reflexes.push((&judged.0.reflex, vec![judged])),
        }
    }
    let width = reflexes
        .iter()
        .map(|(name, _)| name.as_str().chars().count())
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for (name, cases) in reflexes {
        let failed: Vec<&(Case, Verdict)> = cases
            .iter()
            .copied()
            .filter(|(_, verdict)| matches!(verdict, Verdict::Fail { .. }))
            .collect();
        let passed = cases.len() - failed.len();
        let mut line = Text::from(format!("  {:<width$}  {passed} passed", name.as_str()));
        if !failed.is_empty() {
            line.push(" · ")
                .roled(Role::Failed, &format!("{} failed", failed.len()));
        }
        lines.push(line);
        let utterances: Vec<String> = failed
            .iter()
            .map(|(case, _)| quoted(case.utterance.text().as_str()))
            .collect();
        let inner = utterances
            .iter()
            .map(|utterance| utterance.chars().count())
            .max()
            .unwrap_or(0);
        for ((case, verdict), utterance) in failed.iter().zip(&utterances) {
            let Verdict::Fail { mismatch } = verdict else {
                continue;
            };
            let mut line = Text::from(format!(
                "    {utterance:<inner$}  {}",
                missed(case, mismatch)
            ));
            if regressions
                .iter()
                .any(|regression| regression.case == *case)
            {
                line.push(" · ").roled(Role::Failed, "regression");
            }
            lines.push(line);
        }
    }
    Text::lines(lines)
}

/// `route: expected timer, read none`, `state: expected "dim", read "off"`.
fn missed(case: &Case, mismatch: &Mismatch) -> String {
    match mismatch {
        Mismatch::Route { read } => {
            let expected = match case.expect {
                Expected::Never => format!("not {}", case.reflex),
                Expected::Asserts(_) => case.reflex.to_string(),
            };
            let read = read
                .as_ref()
                .map_or_else(|| "none".to_owned(), ToString::to_string);
            format!("route: expected {expected}, read {read}")
        }
        Mismatch::Arg { arg, read } => {
            let expected = match &case.expect {
                Expected::Asserts(claims) => claims.get(arg).map_or_else(String::new, claim),
                Expected::Never => String::new(),
            };
            format!("{arg}: expected {expected}, read {}", claim(read))
        }
    }
}

/// A claim as a person reads it: `unstated`, `set` for a flag, or the text quoted.
fn claim(claim: &Claim) -> String {
    match claim {
        Claim::Unstated => "unstated".to_owned(),
        Claim::Flag => "set".to_owned(),
        Claim::Text(text) => quoted(text.as_str()),
    }
}

fn level(level: Level) -> &'static str {
    match level {
        Level::Same => "same",
        Level::Minor => "minor",
        Level::Major => "major",
    }
}

/// Key–value lines under a block, keys aligned, each on its own line indented four.
fn aligned(lines: &[(String, String)]) -> String {
    let width = lines
        .iter()
        .map(|(key, _)| key.chars().count())
        .max()
        .unwrap_or(0);
    let mut text = String::new();
    for (key, what) in lines {
        let _ = write!(text, "\n    {key:<width$}  {what}");
    }
    text
}

/// A lint finding at `add` and `check`: reported, never a refusal.
#[must_use]
pub fn finding(reflex: &LocalName, finding: &Finding) -> Text {
    let mut text = Text::from("  ");
    text.roled(Role::Warning, "lint")
        .push(&format!("  {reflex}: {}", finding.message));
    text
}

/// One reflex `update` moved: the versions, the contract level, whether its code changed, and what the move means
/// for your files, one line each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Updated {
    pub name: String,
    pub from: Version,
    pub to: Version,
    pub level: Level,
    pub code_changed: bool,
    pub lines: Vec<(String, String)>,
}

#[must_use]
pub fn updated(moved: &Updated) -> Text {
    let mut text = format!(
        "  {} {} → {}  {}",
        moved.name,
        moved.from,
        moved.to,
        level(moved.level)
    );
    if moved.code_changed {
        text.push_str(" · code changed");
    }
    text.push_str(&aligned(&moved.lines));
    Text::from(text)
}

/// `runs <file>` or `runs <program>`.
pub(crate) fn runs(run: &Run) -> String {
    match run {
        Run::File(entrypoint) => format!("runs {}", entrypoint.path()),
        Run::Argv { program, .. } => format!("runs {program}"),
        Run::Inline => String::new(),
    }
}

/// `show <name>`: the effective manifest as TOML, one key per line, `+` in the gutter of every line your overlay
/// decided.
#[must_use]
pub fn manifest(effective: &Effective) -> Text {
    let Ok(Json::Object(manifest)) = serde_json::to_value(&effective.manifest) else {
        unreachable!("a manifest serializes as an object")
    };
    let yours = |path: &[&str]| {
        effective
            .yours
            .contains(&KeyPath::new(path.iter().copied()))
    };
    let mut lines: Vec<(bool, String)> = Vec::new();
    for key in ["description", "not_for", "tags", "effect", "confirm", "run"] {
        let Some(value) = manifest.get(key) else {
            continue;
        };
        let empty = value.as_array().is_some_and(Vec::is_empty);
        if !empty || yours(&[key]) {
            lines.push((yours(&[key]), pair(key, value)));
        }
    }
    if let Some(Json::Object(config)) = manifest.get("config")
        && !config.is_empty()
    {
        lines.push((false, String::new()));
        lines.push((false, "[config]".to_owned()));
        for (key, spec) in config {
            let value = match spec.get("secret") {
                Some(Json::Bool(true)) => spec.clone(),
                _ => spec["about"].clone(),
            };
            lines.push((false, pair(key, &value)));
        }
    }
    if let Some(Json::Object(args)) = manifest.get("args") {
        for (name, argument) in args {
            lines.push((false, String::new()));
            lines.push((false, format!("[args.{name}]")));
            let Json::Object(argument) = argument else {
                continue;
            };
            for (key, value) in argument {
                match (key.as_str(), value) {
                    ("options", Json::Object(options)) => {
                        for (option, text) in options {
                            lines.push((
                                yours(&["args", name, "options", option]),
                                format!("options.{} = {}", toml_key(option), toml(text)),
                            ));
                        }
                    }
                    ("optional", Json::Bool(false)) => {}
                    ("was", Json::Array(was)) if was.is_empty() => {}
                    ("ask", _) => lines.push((yours(&["args", name, "ask"]), pair(key, value))),
                    _ => lines.push((false, pair(key, value))),
                }
            }
        }
    }
    for table in ["examples", "tests"] {
        if let Some(Json::Object(records)) = manifest.get(table)
            && !records.is_empty()
        {
            lines.push((false, String::new()));
            lines.push((false, format!("[{table}]")));
            for (utterance, record) in records {
                lines.push((yours(&[table, utterance]), pair(utterance, record)));
            }
        }
    }
    Text::lines(lines.iter().map(|(yours, line)| {
        let mut text = Text::new();
        match (line.is_empty(), yours) {
            (true, _) => {}
            (false, true) => {
                text.roled(Role::Added, "+").push(" ").push(line);
            }
            (false, false) => {
                text.push("  ").push(line);
            }
        }
        text
    }))
}

/// `vocab <name>`: every word with its meaning, as the file writes them.
#[must_use]
pub fn vocabulary(words: &Vocabulary) -> Text {
    Text::lines(words.iter().map(|(word, meaning)| {
        let value = match &meaning.value {
            None => Json::String(meaning.what.to_string()),
            Some(value) => serde_json::json!({ "what": meaning.what, "value": value }),
        };
        Text::from(format!("  {}", pair(word.as_str(), &value)))
    }))
}

/// `<key> = <value>` as TOML writes it.
fn pair(key: &str, value: &Json) -> String {
    format!("{} = {}", toml_key(key), toml(value))
}

/// A key as TOML writes it: bare when it can be, else quoted.
fn toml_key(key: &str) -> String {
    toml_edit::Key::new(key).display_repr().into_owned()
}

/// A value as TOML writes it, on one line: a string as a basic string, an object as an inline table.
fn toml(value: &Json) -> String {
    match value {
        Json::Null => "\"\"".to_owned(),
        Json::Bool(b) => b.to_string(),
        // A float without a fraction prints as its integer: a manifest's range is `[1, 100]`, not `[1.0, 100.0]`.
        Json::Number(n) => n.as_i64().map_or_else(
            || n.as_f64().unwrap_or_default().to_string(),
            |i| i.to_string(),
        ),
        Json::String(s) => Json::String(s.clone()).to_string(),
        Json::Array(items) => format!(
            "[{}]",
            items.iter().map(toml).collect::<Vec<_>>().join(", ")
        ),
        Json::Object(entries) if entries.is_empty() => "{}".to_owned(),
        Json::Object(entries) => format!(
            "{{ {} }}",
            entries
                .iter()
                .map(|(key, value)| pair(key, value))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// The ranking, the winner's argument lines and the fits line, unindented.
fn block(
    winner: Option<&LocalName>,
    answers: &Raw,
    proposed: &[Proposed],
    contenders: &[Contender],
    under_floor: Option<evoke_core::Prob>,
) -> Vec<String> {
    let mut lines = vec![ranking(answers, under_floor)];
    let arguments: Vec<(&str, &String)> = winner
        .map(|winner| {
            answers
                .0
                .keys()
                .filter_map(|question| match QuestionId::parse(question) {
                    Ok(QuestionId::Arg(reflex, _)) if reflex == *winner => {
                        Some((question.rsplit('.').next().unwrap_or(question), question))
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let width = arguments
        .iter()
        .map(|(arg, _)| arg.len())
        .fold("fits".len(), usize::max)
        + 2;
    for (arg, question) in arguments {
        let keys = sorted(answers, question)
            .into_iter()
            .map(|(key, p)| format!("{} {p:.2}", candidate(&key, proposed)))
            .collect::<Vec<_>>()
            .join(" · ");
        lines.push(format!("{arg:<width$}{keys}"));
    }
    let fits: Vec<String> = contenders
        .iter()
        .filter_map(|contender| {
            contender
                .fits
                .map(|fits| format!("{} {:.2}", contender.reflex, fits.get()))
        })
        .collect();
    if !fits.is_empty() {
        lines.push(format!("{:<width$}{}", "fits", fits.join(" · ")));
    }
    lines
}

/// The route's answers, most probable first, and the floor when the winner was under it.
fn ranking(answers: &Raw, under_floor: Option<evoke_core::Prob>) -> String {
    let mut line = sorted(answers, "route")
        .iter()
        .map(|(key, p)| format!("{key} {p:.2}"))
        .collect::<Vec<_>>()
        .join(" · ");
    if let Some(floor) = under_floor {
        let _ = write!(line, " · route floor {:.2}", floor.get());
    }
    line
}

/// A question's answers, most probable first; ties keep the adapter's order.
fn sorted(answers: &Raw, question: &str) -> Vec<(String, f64)> {
    let mut sorted: Vec<(String, f64)> = answers
        .0
        .get(question)
        .map(|answer| answer.iter().map(|(key, p)| (key.clone(), *p)).collect())
        .unwrap_or_default();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
    sorted
}

/// A candidate's key shown as its quoted span; every other key as itself.
fn candidate(key: &str, proposed: &[Proposed]) -> String {
    proposed
        .iter()
        .find(|proposed| format!("{}-{}", proposed.span.start(), proposed.span.end()) == key)
        .map_or_else(
            || key.to_owned(),
            |proposed| quoted(proposed.span.text().as_str()),
        )
}

/// The outcome line: `run`, `ask <missing>`, `confirm` with the prompt's own line, or `abstain`.
fn outcome(decision: &Decision) -> Text {
    match decision {
        Decision::Abstain { .. } => Text::from("abstain"),
        Decision::Run { chosen } => match &chosen.judged {
            Some(judged) => {
                let mut text = Text::from("run · ");
                text.roled(Role::Weak, &weakest(judged.weakest()));
                text
            }
            None => Text::from("run"),
        },
        Decision::Ask { asking, missing } => {
            let missing: Vec<&str> = missing.iter().map(|missing| missing.arg.as_str()).collect();
            let mut text = Text::from(format!("ask {} · ", missing.join(" ")));
            text.roled(Role::Weak, &weakest(asking.judged.weakest()));
            text
        }
        Decision::Confirm { chosen, prompt, .. } => {
            let mut text = Text::from("confirm · ");
            text.append(own(chosen, &prompt.own));
            text
        }
    }
}

/// `<call> · weakest: <argument or question> <p>`, as `why` names a judged call.
fn judged_call(chosen: &Chosen) -> Text {
    let mut text = call(&chosen.call);
    if let Some(judged) = &chosen.judged {
        text.push(" · ")
            .roled(Role::Weak, &weakest(judged.weakest()));
    }
    text
}

/// `weakest: <argument or question> <p>`, as the confirm prompt names it.
fn weakest(judgment: &evoke_core::Judgment) -> String {
    let name = match &judgment.question {
        QuestionId::Arg(_, arg) => arg.to_string(),
        question => question.to_string(),
    };
    format!("weakest: {name} {:.2}", judgment.p.get())
}

fn indented(lines: Vec<Text>) -> Text {
    Text::lines(lines.into_iter().map(|line| {
        let mut indented = Text::from("  ");
        indented.append(line);
        indented
    }))
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

fn quoted(text: &str) -> String {
    Json::String(text.to_owned()).to_string()
}

#[cfg(test)]
mod tests {
    use evoke_core::name::VocabName;
    use evoke_core::{Fault, identity};

    use super::*;

    fn paths() -> Paths {
        Paths {
            root: "~/.config/evoke".to_owned(),
            reflexes: BTreeMap::from([(LocalName::new("lights").unwrap(), "./lamps".to_owned())]),
        }
    }

    /// A duplicate key at line 7 of a file, the diagnostic naming its reflex when the file has one.
    fn at(file: File) -> Diagnostic {
        let reflex = match &file {
            File::Manifest { name } | File::Overlay { name } => Some(name.clone()),
            _ => None,
        };
        Diagnostic {
            reflex,
            at: None,
            message: "duplicate key".to_owned(),
            fix: Fix::EditLine {
                at: At {
                    file,
                    line: 7,
                    column: 1,
                },
            },
        }
    }

    #[test]
    fn every_kind_prints_as_one_line_with_its_fix() {
        let problem = Diagnostic {
            reflex: Some(LocalName::new("lights").unwrap()),
            at: None,
            message: "vocabulary \"rooms\" is empty".to_owned(),
            fix: Fix::VocabAdd {
                vocab: VocabName::new("rooms").unwrap(),
            },
        };
        let line = diagnostic(&problem, "evoke try \"x\"", Some(&paths()));
        assert_eq!(
            line.to_string(),
            "  lights: vocabulary \"rooms\" is empty  →  evoke vocab rooms add <word> \"<meaning>\""
        );
        assert_eq!(line.roles(), vec![(Role::Fix, "→")]);
        let failed = Failure {
            what: "reading evoke.toml".to_owned(),
            cause: Some("permission denied".to_owned()),
            fix: Fix::Rerun,
        };
        assert_eq!(
            exit(&Exit::Failed(failed), "evoke try \"x\"", None)
                .unwrap()
                .to_string(),
            "  reading evoke.toml: permission denied  →  evoke try \"x\""
        );
        assert_eq!(
            exit(
                &Exit::Adapter(Fault::Status { status: 429 }),
                "evoke try \"x\"",
                None
            )
            .unwrap()
            .to_string(),
            "  the adapter answered 429  →  evoke try \"x\""
        );
        assert_eq!(exit(&Exit::Ran, "", None), None);
        assert_eq!(
            exit(&Exit::Declined(crate::commands::Decline::Refused), "", None),
            None
        );
    }

    #[test]
    fn a_line_to_edit_is_the_file_where_it_is() {
        assert_eq!(
            diagnostic(&at(File::Project), "", Some(&paths())).to_string(),
            "  duplicate key  →  ~/.config/evoke/evoke.toml:7:1"
        );
        assert_eq!(
            diagnostic(&at(File::Project), "", None).to_string(),
            "  duplicate key  →  evoke.toml:7:1"
        );
        let lights = LocalName::new("lights").unwrap();
        assert_eq!(
            diagnostic(
                &at(File::Manifest {
                    name: lights.clone()
                }),
                "",
                Some(&paths())
            )
            .to_string(),
            "  lights: duplicate key  →  ~/.config/evoke/lamps/reflex.toml:7:1"
        );
        assert_eq!(
            diagnostic(&at(File::Overlay { name: lights }), "", Some(&paths())).to_string(),
            "  lights: duplicate key  →  ~/.config/evoke/overlays/lights.toml:7:1"
        );
    }

    #[test]
    fn under_json_a_failure_is_one_object_with_the_same_fields() {
        let unrecorded = Exit::Adapter(Fault::Unrecorded {
            identity: identity("what time is it"),
        });
        assert_eq!(
            exit_json(&unrecorded).unwrap(),
            r#"{"message":"\"what time is it\" is not recorded","fix":{"type":"rerun"}}"#
        );
        let human = Exit::Human(at(File::Project));
        assert_eq!(
            exit_json(&human).unwrap(),
            r#"{"message":"duplicate key","fix":{"type":"edit_line","at":{"file":{"type":"project"},"line":7,"column":1}}}"#
        );
        assert_eq!(exit_json(&Exit::Ran), None);
    }

    #[test]
    fn the_input_cannot_repaint_the_terminal() {
        let hidden = Exit::Adapter(Fault::Unrecorded {
            identity: identity("kill the \u{202e}lights"),
        });
        assert_eq!(
            exit(&hidden, "evoke try \"kill the \u{202e}lights\"", None)
                .unwrap()
                .to_string(),
            "  \"kill the \\u{202e}lights\" is not recorded  →  evoke try \"kill the \\u{202e}lights\""
        );
        assert_eq!(plain("a\nb\tc\u{7}"), "a\\nb\\tc\\u{7}");
    }

    #[test]
    fn a_log_line_reads_back_to_what_was_written() {
        let text = r#"{"input":"kill the lights in the den","outcome":"run","reflex":"lights","args":{"room":{"type":"word","word":"den"},"state":{"type":"option","key":"off"}},"call":"lights room=\"den\" state=\"off\"","effect":"write","confidence":0.85,"weakest":{"question":"lights.room","top":"den","p":0.85},"judgments":[{"question":"route","top":"lights","p":0.91},{"question":"lights.room","top":"den","p":0.85},{"question":"lights.state","top":"off","p":0.88}],"runner_up":{"reflex":"timer","route":0.02,"fits":0.05},"contenders":[{"reflex":"lights","route":0.91,"fits":0.7},{"reflex":"timer","route":0.02,"fits":0.05}],"trace":[{"adapter":"replay","questions":6,"ms":0}],"result":{"text":"den lights off"},"answers":{"route":{"lights":0.91,"timer":0.02,"none":0.07}},"proposed":[]}"#;
        let line = Line::parse(text).unwrap();
        assert_eq!(line.input.as_str(), "kill the lights in the den");
        assert!(matches!(line.decision, Decision::Run { .. }));
        assert_eq!(
            line.result.as_ref().map(|r| r.text.as_str()),
            Some("den lights off")
        );
        let again = Line::parse(&line.log()).unwrap();
        assert_eq!(again.log(), line.log());
        assert!(!line.json().contains("\"answers\""));
        let explained = why(&line);
        assert_eq!(
            explained.to_string(),
            "  \"kill the lights in the den\"\n  lights 0.91 · none 0.07 · timer 0.02\n  fits  lights 0.70 · timer 0.05\n  ran lights room=\"den\" state=\"off\" · weakest: room 0.85 · replay, 6 questions"
        );
        assert_eq!(
            explained.roles(),
            vec![(Role::Call, "lights"), (Role::Weak, "weakest: room 0.85")]
        );
    }

    /// The run decision of the log line above, as a `Chosen`.
    fn chosen() -> Chosen {
        let text = r#"{"outcome":"run","reflex":"lights","args":{"room":{"type":"word","word":"den"},"state":{"type":"option","key":"off"}},"call":"lights room=\"den\" state=\"off\"","effect":"write","confidence":0.85,"weakest":{"question":"lights.room","top":"den","p":0.85},"judgments":[{"question":"route","top":"lights","p":0.91},{"question":"lights.room","top":"den","p":0.85}],"contenders":[{"reflex":"lights","route":0.91,"fits":0.7}]}"#;
        match serde_json::from_str::<Decision>(text).unwrap() {
            Decision::Run { chosen } => chosen,
            _ => unreachable!("the line is a run"),
        }
    }

    #[test]
    fn the_run_line_weights_the_name_and_greys_the_confidence() {
        let line = running(&chosen());
        assert_eq!(
            line.to_string(),
            "  lights room=\"den\" state=\"off\"  0.85"
        );
        assert_eq!(
            line.roles(),
            vec![(Role::Call, "lights"), (Role::Weak, "0.85")]
        );
    }

    #[test]
    fn the_confirm_line_is_the_prompts_own_words_with_their_roles() {
        let chosen = chosen();
        let own = |text: &str| Prompt {
            own: text.to_owned(),
            template: Clean::line("Set the den lights off?").unwrap(),
        };
        let judged = own(
            "lights room=\"den\" state=\"off\" · write · weakest: room 0.85 · unused \"now · later\"",
        );
        let line = confirming(&chosen, &judged);
        assert_eq!(line.to_string(), format!("  {}", judged.own));
        assert_eq!(
            line.roles(),
            vec![
                (Role::Call, "lights"),
                (Role::Effect(Effect::Write), "write"),
                (Role::Weak, "weakest: room 0.85"),
            ]
        );
        let capped = own("lights room=\"den\" state=\"off\" · write · also timer (fits 0.40)");
        let line = confirming(&chosen, &capped);
        assert_eq!(line.to_string(), format!("  {}", capped.own));
        assert_eq!(line.roles().len(), 2);
        let other = own("something else entirely");
        assert_eq!(
            confirming(&chosen, &other).to_string(),
            "  something else entirely"
        );
        assert!(confirming(&chosen, &other).roles().is_empty());
    }

    #[test]
    fn rows_light_each_effect_and_sign_the_gutter() {
        let rows = rows(
            &[
                Row {
                    name: "lights".to_owned(),
                    from: "radhi/home/lights 1.2.0".to_owned(),
                    effect: Some(Effect::Write),
                    runs: "runs lights.mts".to_owned(),
                },
                Row {
                    name: "power".to_owned(),
                    from: "./power".to_owned(),
                    effect: Some(Effect::Destructive),
                    runs: "runs power.mts".to_owned(),
                },
                Row {
                    name: "broken".to_owned(),
                    from: "./broken".to_owned(),
                    effect: None,
                    runs: String::new(),
                },
            ],
            Gutter::Added,
        );
        assert_eq!(
            rows.to_string(),
            "+ lights  radhi/home/lights 1.2.0  write        runs lights.mts\n+ power   ./power                  destructive  runs power.mts\n+ broken  ./broken"
        );
        assert_eq!(
            rows.roles(),
            vec![
                (Role::Added, "+"),
                (Role::Effect(Effect::Write), "write"),
                (Role::Added, "+"),
                (Role::Effect(Effect::Destructive), "destructive"),
                (Role::Added, "+"),
            ]
        );
        assert_eq!(self::rows(&[], Gutter::Listed), Text::new());
    }

    #[test]
    fn the_help_fits_eighty_columns_and_names_every_group() {
        let help = help("0.1.0");
        assert!(help.starts_with("evoke 0.1.0 · "));
        for group in [
            "\nuse\n",
            "\ninstall\n",
            "\ntune\n",
            "\nauthor\n",
            "\nexit  0 ran",
        ] {
            assert!(help.contains(group), "{group:?} is missing");
        }
        let widest = help.lines().map(chars).max().unwrap_or(0);
        assert!(widest <= 80, "a help line is {widest} columns wide");
        assert!(!help.ends_with('\n'));
    }

    #[test]
    fn a_write_is_signed_and_a_label_warns() {
        let removed = written(&Edited {
            path: std::path::PathBuf::from("/x/vocab/rooms.toml"),
            shown: "vocab/rooms.toml".to_owned(),
            text: String::new(),
            landed: Landed::Removed("attic".to_owned()),
        });
        assert_eq!(removed.to_string(), "- vocab/rooms.toml  attic");
        assert_eq!(removed.roles(), vec![(Role::Removed, "-")]);
        assert_eq!(created("reflex.d.ts").roles(), vec![(Role::Added, "+")]);
        let lint = finding(
            &LocalName::new("lights").unwrap(),
            &Finding {
                rule: evoke_core::contract::LintRule::SizeCap,
                path: KeyPath::new(["description"]),
                message: "no tests".to_owned(),
            },
        );
        assert_eq!(lint.to_string(), "  lint  lights: no tests");
        assert_eq!(lint.roles(), vec![(Role::Warning, "lint")]);
    }
}
