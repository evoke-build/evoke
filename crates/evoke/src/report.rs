//! Render decisions, `try`, `why`, `show`, `vocab`, `update`, `check`, `test`, the installed rows, the prompts,
//! every failure with its fix, and the one line a decision leaves — printed by `--json`, kept by the log; pure.
//! In: core values, an `Exit`, the invoked line and the owned files' paths as shown. Out: `Text` for the terminal
//! — the plain words, with the roles it may weight: the call, the effect, the top of a distribution, the weakest
//! judgment, the arrow of a fix, the sign of a write — and the prompts and the JSON line as plain strings. The
//! blocks of `try`, `why`, `use`, `show`, `add`, `update`, `check`, `test` and the JSON line are pinned by the
//! transcripts; the failure line, its JSON object and the roles by the tests here.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use evoke_core::adapter::QuestionId;
use evoke_core::calibrate::{self, BarRow, BinRow, Calibration, LogBlock, Miss, QuestionRow};
use evoke_core::contract::Change;
use evoke_core::decide::{Missing, Why};
use evoke_core::manifest::{Effect, Manifest, Recognizer, Run, Sentence, written as slotted};
use evoke_core::name::{ArgName, FieldName, LocalName};
use evoke_core::test::{Claim, Expected, Mismatch};
use evoke_core::vocabulary::Vocabulary;
use evoke_core::weave::{Because, Bound, From, Repair, Shared, Status, Step, When, Why as Stopped};
use evoke_core::{
    At, Call, Case, Chosen, Clean, Contained, ContractDiff, Decision, Diagnostic, Digest,
    Effective, File, Finding, Fix, Gate, Input, Json, KeyPath, Level, Needs, Prompt, Proposed, Raw,
    Regression, Verdict, Version, Weave, render,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::Trace;
use crate::commands::Exit;
use crate::commands::session::{Decided, Woven};
use crate::hosts::Failure;
use crate::hosts::files::Landed;
use crate::hosts::processes::Returned;
use crate::hosts::terminal::{Role, Text};

pub mod sentence;

/// The owned files as a person reads them: the root, `~/…` under `$HOME`, and each local reflex's directory as
/// `evoke.toml` writes it; the home itself, so a path a host names in full shows the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    pub root: String,
    pub reflexes: BTreeMap<LocalName, String>,
    pub home: Option<String>,
}

impl Paths {
    /// Before a project is located: no root, no reflexes, the home from the environment.
    #[must_use]
    pub fn of(environment: &crate::hosts::Environment) -> Self {
        Self {
            root: String::new(),
            reflexes: BTreeMap::new(),
            home: environment.get("HOME").map(str::to_owned),
        }
    }

    /// Text with every path under the home shortened to `~/…`, as the owned paths are shown.
    fn tilde(&self, text: &str) -> String {
        match self.home.as_deref() {
            Some(home) if !home.is_empty() && home != "/" => {
                text.replace(&format!("{home}/"), "~/")
            }
            _ => text.to_owned(),
        }
    }

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

/// What an exit says, as one message without its fix: what a line about it quotes.
#[must_use]
pub fn said(exit: &Exit) -> String {
    problem(exit)
        .map(|problem| head(&problem))
        .unwrap_or_default()
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

/// `  <reflex>: <message>  →  <fix>`, the reflex when there is one; a line to edit shows its path under the root,
/// and a path under the home shows as `~/…`.
#[must_use]
pub fn diagnostic(problem: &Diagnostic, invoked: &str, paths: Option<&Paths>) -> Text {
    let head = head(problem);
    let head = paths.map_or(head.clone(), |paths| paths.tilde(&head));
    let mut text = Text::from("  ");
    text.push(&plain(&head));
    fixed(&mut text, &fixing(problem, invoked, paths));
    text
}

/// The inactive lines of `show`: every problem with its fix, the arrows aligned.
#[must_use]
pub fn inactive(problems: &[Diagnostic], invoked: &str, paths: &Paths) -> Text {
    let heads: Vec<String> = problems
        .iter()
        .map(|problem| paths.tilde(&head(problem)))
        .collect();
    let width = heads.iter().map(|head| chars(head)).max().unwrap_or(0);
    Text::lines(problems.iter().zip(&heads).map(|(problem, head)| {
        let mut line = Text::from("  ");
        line.roled(Role::Warning, "inactive")
            .push(&plain(&format!("  {head:<width$}")));
        fixed(&mut line, &fixing(problem, invoked, Some(paths)));
        line
    }))
}

/// The lines of an overlay that address nothing the reflex has, as `update` reported them: each skipped.
pub fn orphaned(name: &LocalName, paths: &[KeyPath]) -> Text {
    Text::lines(paths.iter().map(|path| {
        let mut line = Text::from("  ");
        line.roled(Role::Warning, "skipped").push(&plain(&format!(
            "  overlays/{name}.toml: {path} addresses nothing"
        )));
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
        (Fix::MakeDir { .. }, Some(paths)) => paths.tilde(&problem.fix.command(invoked)),
        (fix, _) => fix.command(invoked),
    }
}

/// Text that cannot repaint the terminal. The input is untrusted and is echoed in a fault and in the line to
/// rerun, so a character `Clean` refuses, or a line feed, shows as its escape.
#[must_use]
pub fn plain(text: &str) -> String {
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
/// the error when it failed with the frames of an error a file body threw. `log` is the same line with the
/// adapter's answers, the input's candidates and the values an ask recalled beside it, which `why` reads back.
pub struct Line {
    pub input: Input,
    pub decision: Decision,
    pub trace: Vec<Trace>,
    pub answers: Raw,
    pub proposed: Vec<Proposed>,
    /// What an ask offered back from the process's results, per argument: logged for `why`, never printed under
    /// `--json`.
    pub recalled: IndexMap<ArgName, Vec<String>>,
    pub result: Option<Returned>,
    pub error: Option<String>,
    /// The frames of the error a file body threw, from its first, when it threw one.
    pub frames: Vec<String>,
    /// Whether Ctrl-C ended the body: one decision's word for what a weave's step says as `skipped · cancelled`.
    pub cancelled: bool,
    /// Whether the machine held the body's declaration, when a body was run.
    pub contained: Option<Contained>,
    /// A weave's step: which of how many, what became of it, the values bound into it. None for one decision.
    pub step: Option<StepLine>,
    /// A playbook's expansion, logged as step 0 of its plan with no status: the steps' lines say what became of
    /// the plan. None for a decision or a step.
    pub expansion: Option<Expanded>,
    /// The plan file the step came from, when it was run from one.
    pub pinned: Option<PinnedAt>,
    /// What the reflex does, by the first line of its description: logged for `why`.
    pub summary: Option<String>,
    /// The step's words as they were typed, where the plan wrote them anew: logged for `why`.
    pub typed: Option<String>,
    /// How the step's words were settled, where a rule settled them: logged for `why`.
    pub repair: Option<Repair>,
    /// What became of one decision, logged for `why`; a step's is its line's own.
    pub status: Option<Status>,
    /// The prompt a playbook's plan is reviewed under, on its expansion's line: logged for `why`.
    pub review: Option<Prompt>,
}

/// The expansion's own line: how many steps the plan holds, the playbook, and what each slot took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expanded {
    pub of: usize,
    pub playbook: LocalName,
    pub slots: IndexMap<ArgName, String>,
}

/// The plan file a line's step came from: the path as shown, and the SHA-256 of the bytes read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedAt {
    pub file: String,
    pub id: Digest,
}

/// A step of a weave as its line says it.
#[derive(Clone, Debug, PartialEq)]
pub struct StepLine {
    pub n: usize,
    pub of: usize,
    pub status: Status,
    pub why: Option<evoke_core::weave::Why>,
    pub bound: Vec<Bound>,
    /// The words the request stated once for several steps that reached this one's arguments.
    pub shared: IndexMap<ArgName, Shared>,
    /// Each day of the step's call read beside the day of a step it takes from: by argument, that step.
    pub beside: IndexMap<ArgName, usize>,
    /// The playbooks the step came from, outermost first.
    pub from: Vec<From>,
    /// What picks the step, when it may not run.
    pub when: Option<When>,
}

impl Line {
    #[must_use]
    pub fn of(decided: &Decided) -> Self {
        Self {
            input: decided.input.clone(),
            decision: decided.decision.clone(),
            trace: decided.trace.clone(),
            answers: decided.answers.clone(),
            proposed: decided.proposed.clone(),
            recalled: IndexMap::new(),
            result: None,
            error: None,
            frames: Vec::new(),
            cancelled: false,
            contained: None,
            step: None,
            expansion: None,
            pinned: None,
            summary: None,
            typed: None,
            repair: None,
            status: None,
            review: None,
        }
    }

    /// The line of a call by name: no input, nothing judged, no adapter called.
    #[must_use]
    pub fn unjudged(decision: &Decision) -> Self {
        Self {
            input: Input::new("").expect("an empty input is under the cap"),
            decision: decision.clone(),
            trace: Vec::new(),
            answers: Raw::default(),
            proposed: Vec::new(),
            recalled: IndexMap::new(),
            result: None,
            error: None,
            frames: Vec::new(),
            cancelled: false,
            contained: None,
            step: None,
            expansion: None,
            pinned: None,
            summary: None,
            typed: None,
            repair: None,
            status: None,
            review: None,
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
        if let Some(step) = &self.step {
            line.insert("step".to_owned(), Json::from(step.n));
            line.insert("steps".to_owned(), Json::from(step.of));
        }
        if let Some(expansion) = &self.expansion {
            line.insert("step".to_owned(), Json::from(0));
            line.insert("steps".to_owned(), Json::from(expansion.of));
        }
        line.insert(
            "input".to_owned(),
            Json::String(self.input.as_str().to_owned()),
        );
        line.extend(decision);
        line.insert(
            "trace".to_owned(),
            serde_json::to_value(&self.trace).expect("a trace serializes"),
        );
        self.weaving(&mut line);
        if let Some(pinned) = &self.pinned {
            line.insert(
                "pinned".to_owned(),
                serde_json::to_value(pinned).expect("a plan file's pin serializes"),
            );
        }
        if let Some(contained) = &self.contained {
            line.insert(
                "contained".to_owned(),
                serde_json::to_value(contained).expect("a status serializes"),
            );
        }
        if let Some(result) = &self.result {
            line.insert(
                "result".to_owned(),
                serde_json::to_value(result).expect("a result serializes"),
            );
        }
        if let Some(error) = &self.error {
            line.insert("error".to_owned(), Json::String(error.clone()));
        }
        if !self.frames.is_empty() {
            line.insert(
                "frames".to_owned(),
                serde_json::to_value(&self.frames).expect("frames serialize"),
            );
        }
        if self.cancelled {
            line.insert("cancelled".to_owned(), Json::Bool(true));
        }
        if let Some(step) = &self.step {
            line.insert(
                "status".to_owned(),
                serde_json::to_value(step.status).expect("a status serializes"),
            );
            if let Some(why) = &step.why {
                line.insert(
                    "why".to_owned(),
                    serde_json::to_value(why).expect("a reason serializes"),
                );
            }
        }
        if whole {
            self.kept(&mut line);
        }
        Json::Object(line)
    }

    /// What the log alone keeps: the adapter's answers, the input's candidates, what an ask recalled, and what
    /// `why` reads back of the reflex, the words, the repair, what became of one decision and a playbook's
    /// review.
    fn kept(&self, line: &mut serde_json::Map<String, Json>) {
        line.insert(
            "answers".to_owned(),
            serde_json::to_value(&self.answers).expect("answers serialize"),
        );
        line.insert(
            "proposed".to_owned(),
            serde_json::to_value(&self.proposed).expect("candidates serialize"),
        );
        if !self.recalled.is_empty() {
            line.insert(
                "recalled".to_owned(),
                serde_json::to_value(&self.recalled).expect("recalled values serialize"),
            );
        }
        if let Some(summary) = &self.summary {
            line.insert("summary".to_owned(), Json::String(summary.clone()));
        }
        if let Some(typed) = &self.typed {
            line.insert("typed".to_owned(), Json::String(typed.clone()));
        }
        if let Some(repair) = self.repair {
            line.insert(
                "repair".to_owned(),
                serde_json::to_value(repair).expect("a repair serializes"),
            );
        }
        if let (Some(status), None) = (self.status, &self.step) {
            line.insert(
                "became".to_owned(),
                serde_json::to_value(status).expect("a status serializes"),
            );
        }
        if let Some(review) = &self.review {
            line.insert(
                "review".to_owned(),
                serde_json::to_value(review).expect("a prompt serializes"),
            );
        }
    }

    /// A weave's fields after the trace: a step's bound values, the words shared into it and the playbooks it came
    /// from; an expansion's playbook and what each slot took.
    fn weaving(&self, line: &mut serde_json::Map<String, Json>) {
        if let Some(step) = &self.step {
            if !step.bound.is_empty() {
                line.insert(
                    "bound".to_owned(),
                    serde_json::to_value(&step.bound).expect("bound values serialize"),
                );
            }
            if !step.shared.is_empty() {
                line.insert(
                    "shared".to_owned(),
                    serde_json::to_value(&step.shared).expect("shared words serialize"),
                );
            }
            if !step.beside.is_empty() {
                line.insert(
                    "beside".to_owned(),
                    serde_json::to_value(&step.beside).expect("a step's days serialize"),
                );
            }
            if !step.from.is_empty() {
                line.insert(
                    "from".to_owned(),
                    serde_json::to_value(&step.from).expect("a step's origins serialize"),
                );
            }
            if let Some(when) = &step.when {
                line.insert(
                    "when".to_owned(),
                    serde_json::to_value(when).expect("what picks a step serializes"),
                );
            }
        }
        if let Some(expansion) = &self.expansion {
            line.insert(
                "playbook".to_owned(),
                Json::String(expansion.playbook.to_string()),
            );
            line.insert(
                "slots".to_owned(),
                serde_json::to_value(&expansion.slots).expect("slots serialize"),
            );
        }
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
        let recalled = field("recalled", take("recalled")).unwrap_or_default();
        let result = field("result", take("result"))?;
        let error = field("error", take("error"))?;
        let frames = field("frames", take("frames")).unwrap_or_default();
        let cancelled = field("cancelled", take("cancelled")).unwrap_or_default();
        let contained = field("contained", take("contained"))?;
        let pinned = field("pinned", take("pinned"))?;
        let summary = field("summary", take("summary")).unwrap_or_default();
        let typed = field("typed", take("typed")).unwrap_or_default();
        let repair = field("repair", take("repair")).unwrap_or_default();
        let status = field("became", take("became")).unwrap_or_default();
        let review = field("review", take("review")).unwrap_or_default();
        let mut expansion = None;
        let step = match (take("step"), take("steps")) {
            (Json::Null, _) => None,
            // Step 0 is a playbook's expansion: the plan's own line, with no status.
            (n, of) if n.as_u64() == Some(0) => {
                expansion = Some(Expanded {
                    of: field("steps", of)?,
                    playbook: field("playbook", take("playbook"))?,
                    slots: field("slots", take("slots")).unwrap_or_default(),
                });
                None
            }
            (n, of) => Some(StepLine {
                n: field("step", n)?,
                of: field("steps", of)?,
                status: field("status", take("status"))?,
                why: field("why", take("why"))?,
                bound: field("bound", take("bound")).unwrap_or_default(),
                shared: field("shared", take("shared")).unwrap_or_default(),
                beside: field("beside", take("beside")).unwrap_or_default(),
                from: field("from", take("from")).unwrap_or_default(),
                when: field("when", take("when"))?,
            }),
        };
        let decision = field("decision", Json::Object(fields))?;
        Ok(Self {
            input,
            decision,
            trace,
            answers,
            proposed,
            recalled,
            result,
            error,
            frames,
            cancelled,
            contained,
            step,
            expansion,
            pinned,
            summary,
            typed,
            repair,
            status,
            review,
        })
    }

    /// The reflex the decision is about, when it is about one.
    #[must_use]
    pub fn reflex(&self) -> Option<&LocalName> {
        match &self.decision {
            Decision::Abstain { .. } => None,
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
                Some(&chosen.call.reflex)
            }
            Decision::Ask { asking, .. } => Some(&asking.reflex),
        }
    }
}

/// The reflex that won the route: the decision's own, or — for an abstain — the one the route named when the
/// gate still refused it, since `none` is no name.
fn winner_of(decision: &Decision) -> Option<LocalName> {
    match decision {
        Decision::Abstain { judgments, .. } => judgments
            .iter()
            .find(|judgment| judgment.question == QuestionId::Route)
            .and_then(|judgment| LocalName::new(judgment.top.as_str()).ok()),
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
            Some(chosen.call.reflex.clone())
        }
        Decision::Ask { asking, .. } => Some(asking.reflex.clone()),
    }
}

/// One field of a log line, typed.
fn field<T: serde::de::DeserializeOwned>(what: &str, value: Json) -> Result<T, String> {
    serde_json::from_value(value).map_err(|error| format!("{what}: {error}"))
}

/// Why a step stopped, in a person's words; a step not chosen names the field it waits on, when the step is
/// known.
#[must_use]
pub fn stopped(why: &Stopped, when: Option<&When>) -> String {
    match why {
        Stopped::EarlierStep => "an earlier step stopped".to_owned(),
        Stopped::NothingToTake { from } => format!("step {from} yielded nothing it takes"),
        Stopped::TooLarge { from } => format!("step {from} returned more than 1 MiB"),
        Stopped::FoundNothing => "its source found nothing".to_owned(),
        Stopped::NotChosen { from, value } => match when {
            Some(when) => format!("not chosen: step {from} yielded {} \"{value}\"", when.field),
            None => format!("not chosen: step {from} yielded \"{value}\""),
        },
        Stopped::NoReflex => "no reflex".to_owned(),
        Stopped::ReadAs { reflex } => format!("read as {reflex}"),
        Stopped::Cancelled => "cancelled".to_owned(),
        Stopped::Said { message } => message.clone(),
    }
}

/// The ranking alone, as an abstain shows it.
#[must_use]
pub fn abstained(decided: &Decided, floors: Option<&Gate>) -> Text {
    let under = winner_of(&decided.decision).and(floors.map(Gate::route));
    indented(vec![ranking(&decided.answers, under)])
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

/// The run line: the call, then its confidence, then the machine's status when it does not hold the whole
/// declaration.
#[must_use]
pub fn running(chosen: &Chosen, contained: &Contained) -> Text {
    let mut text = Text::from("  ");
    text.append(run_line(chosen));
    held(&mut text, contained);
    text
}

/// ` · partly contained` or ` · not contained` on the end of a run's own line; nothing when the machine holds it.
fn held(text: &mut Text, contained: &Contained) {
    let word = match contained {
        Contained::Full => return,
        Contained::Partial { .. } => "partly contained",
        Contained::None { .. } => "not contained",
    };
    text.push(" · ").roled(Role::Warning, word);
}

/// The machine's status, once, where it does not hold a whole declaration: `  not contained  <why>`.
#[must_use]
pub fn status(contained: &Contained) -> Option<Text> {
    let (word, why) = match contained {
        Contained::Full => return None,
        Contained::Partial { why } => ("partly contained", why),
        Contained::None { why } => ("not contained", why),
    };
    let mut text = Text::from("  ");
    text.roled(Role::Warning, word).push("  ").push(&plain(why));
    Some(text)
}

/// The call, then its confidence.
fn run_line(chosen: &Chosen) -> Text {
    let mut text = call(&chosen.call);
    if let Some(judged) = &chosen.judged {
        text.push("  ")
            .roled(Role::Weak, &format!("{:.2}", judged.confidence().get()));
    }
    text
}

/// The plan: what stands out of it, then one line per step, numbered as the run refers to them: a call with its
/// confidence; the own line of a step that will confirm, and why it waits under it; `asks <arg>` for what a
/// step still needs; `takes <field> from <n>` where a result threads in; `after <n>` where the words order it;
/// `with <n>` where the step runs beside earlier ones; `no reflex` where nothing matched.
#[must_use]
pub fn planned(weave: &Weave) -> Text {
    let mut lines = notes(weave);
    lines.extend(
        weave
            .steps
            .iter()
            .map(|step| numbered(step.n, weave.steps.len(), step_body(step, weave))),
    );
    indented(lines)
}

/// What stands out of a plan of one step, before its line.
#[must_use]
pub fn noted(weave: &Weave) -> Text {
    indented(notes(weave))
}

/// What stands out of the plan, a line each: what was folded into a step, what was set aside as a remark or
/// kept out of the plan as words that may add a detail, and what was left out, when the request said what not
/// to do.
fn notes(weave: &Weave) -> Vec<Text> {
    let mut lines = Vec::new();
    for folded in &weave.folded {
        lines.push(Text::from(if weave.steps.len() > 1 {
            format!("folded {} into {}", quoted(&folded.text), folded.into)
        } else {
            format!("folded {}", quoted(&folded.text))
        }));
    }
    for aside in &weave.asides {
        lines.push(Text::from(if aside.remark {
            format!("set aside {}", quoted(&aside.text))
        } else {
            format!("not in the plan {}", quoted(&aside.text))
        }));
    }
    if !weave.excluded.is_empty() {
        let parts: Vec<String> = weave.excluded.iter().map(|part| quoted(part)).collect();
        lines.push(Text::from(format!("left out {}", parts.join(", "))));
    }
    lines
}

/// One line of a weave at its turn, numbered as the plan numbers it, with what the plan could not show: a
/// bound value in its place, a prompt's own line, a step skipped.
#[must_use]
pub fn step(n: usize, of: usize, body: Text) -> Text {
    let mut text = Text::from("  ");
    text.append(numbered(n, of, body).hang(2));
    text
}

/// A step's call at its turn, with its confidence and the machine's status.
#[must_use]
pub fn step_running(chosen: &Chosen, contained: &Contained) -> Text {
    let mut text = run_line(chosen);
    held(&mut text, contained);
    text
}

/// A step's own line ahead of its confirm prompt, and why it waits under it.
#[must_use]
pub fn step_confirming(chosen: &Chosen, prompt: &Prompt, contained: &Contained) -> Text {
    waiting(chosen, prompt, Some(contained))
}

/// A step refused at its turn, its bound values in its words: `"<words>" · no reflex`.
#[must_use]
pub fn step_refused(text: &str, why: &Stopped) -> Text {
    let mut body = Text::from(quoted(text));
    body.push(" · ").push(&stopped(why, None));
    body
}

/// The `--json` line of a request refused with no step — one that is only what not to do, or opens with a
/// condition: an abstain over the whole input, with no judgment and no contender, since nothing was asked, and
/// the verdict's reason under `because`.
#[must_use]
pub fn nothing_to_do_json(input: &str, because: &Because) -> String {
    serde_json::json!({
        "input": input,
        "outcome": "abstain",
        "judgments": [],
        "contenders": [],
        "trace": [],
        "because": [because],
    })
    .to_string()
}

/// A request refused with no step: nothing to run, said in the verdict's one line.
#[must_use]
pub fn nothing_to_do(because: &Because) -> Text {
    indented(vec![Text::from(verdict(because))])
}

/// A step as the plan shows it, before it runs.
#[must_use]
pub fn step_body(step: &Step, weave: &Weave) -> Text {
    // The steps whose results reach this one, or pick it: the lines say so, and `after` need not.
    let taken: Vec<usize> = weave
        .binds
        .iter()
        .filter(|b| b.to == step.n)
        .map(|b| b.from)
        .chain(step.when.iter().map(|when| when.step))
        .collect();
    let reason = match &step.decision {
        Decision::Confirm { prompt, .. } => prompt.reason.as_str(),
        _ => "",
    };
    let mut text = match &step.decision {
        Decision::Run { chosen } => run_line(chosen),
        Decision::Confirm { chosen, prompt, .. } => own(chosen, &prompt.own),
        Decision::Ask { asking, missing } => {
            let bound: Vec<&str> = weave
                .binds
                .iter()
                .filter(|b| b.to == step.n)
                .map(|b| b.arg.as_str())
                .collect();
            let asks: Vec<&str> = missing
                .iter()
                .map(|m| m.arg.as_str())
                .filter(|arg| !bound.contains(arg))
                .collect();
            let mut text = call(&Call {
                reflex: asking.reflex.clone(),
                args: asking.args.clone(),
            });
            if !asks.is_empty() {
                text.push(&format!(" · asks {}", asks.join(", ")));
            }
            text
        }
        Decision::Abstain { .. } => {
            let mut text = Text::from(quoted(&step.text));
            text.push(" · no reflex");
            text
        }
    };
    // One segment for every binding, each name beside its step: a field's, or the name a whole result goes by.
    let segments: Vec<String> = weave
        .binds
        .iter()
        .filter(|b| b.to == step.n)
        .map(|b| format!("{} from {}", b.field, b.from))
        .collect();
    if !segments.is_empty() {
        text.push(&format!(" · takes {}", segments.join(", ")));
    }
    let after: Vec<String> = step
        .after
        .iter()
        .filter(|n| !taken.contains(n))
        .map(ToString::to_string)
        .collect();
    if !after.is_empty() {
        text.push(&format!(" · after {}", after.join(", ")));
    }
    // The earlier steps of its stage: what runs beside it, which the lines alone would not show.
    let with: Vec<String> = weave
        .stages
        .iter()
        .find(|stage| stage.contains(&step.n))
        .map(|stage| {
            stage
                .iter()
                .filter(|n| **n < step.n)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    if !with.is_empty() {
        text.push(&format!(" · with {}", with.join(", ")));
    }
    // Which playbook wrote the step, and which of its steps; then a slot whose value reached an argument named
    // otherwise, so each value on the line says which slot filled it.
    let args = match &step.decision {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(&chosen.call.args),
        Decision::Ask { asking, .. } => Some(&asking.args),
        Decision::Abstain { .. } => None,
    };
    for from in &step.from {
        text.push(&format!(" · {} {}", from.playbook, from.step));
    }
    for from in &step.from {
        for (slot, value) in &from.slots {
            let Some(args) = args else { continue };
            if args.contains_key(slot) {
                continue;
            }
            if let Some((arg, _)) = args
                .iter()
                .find(|(_, theirs)| slotted(theirs).as_deref() == Some(value.as_str()))
            {
                text.push(&format!(" · {slot} as {arg}"));
            }
        }
    }
    branched(&mut text, step, weave);
    because_under(&mut text, reason);
    text
}

/// What a step's result picks among — the steps standing after it, each under its value — and what picks the
/// step, when it may not run: ` · then 4 on "yes", 5 on "no"` on the one, ` · if 3 yields landing "no"` on the
/// other.
fn branched(text: &mut Text, step: &Step, weave: &Weave) {
    let picks: Vec<String> = weave
        .steps
        .iter()
        .filter_map(|other| {
            let when = other.when.as_ref().filter(|when| when.step == step.n)?;
            Some(format!("{} on \"{}\"", other.n, when.is.as_str()))
        })
        .collect();
    if !picks.is_empty() {
        text.push(&format!(" · then {}", picks.join(", ")));
    }
    if let Some(when) = &step.when {
        text.push(&format!(
            " · if {} yields {} \"{}\"",
            when.step,
            when.field,
            when.is.as_str()
        ));
    }
}

/// The plan as one JSON line: the weave's fields, then `trace`, every adapter call the plan took.
#[must_use]
pub fn plan_json(woven: &Woven) -> String {
    let Ok(Json::Object(mut plan)) = serde_json::to_value(&woven.weave) else {
        unreachable!("a plan serializes as an object")
    };
    plan.insert(
        "trace".to_owned(),
        serde_json::to_value(&woven.trace).expect("a trace serializes"),
    );
    Json::Object(plan).to_string()
}

/// `<n>  <body>`, the number right-aligned to the count; what stands under the body's first line stays under
/// it.
fn numbered(n: usize, of: usize, body: Text) -> Text {
    let width = of.to_string().len();
    let mut text = Text::from(format!("{n:>width$}  "));
    text.append(body.hang(width + 2));
    text
}

/// Why a plan does not simply run, in one line a person reads.
#[must_use]
pub fn verdict(because: &Because) -> String {
    match because {
        Because::NothingToDo => "nothing to do: what you said not to do is no step".to_owned(),
        Because::NoReflex { step } => format!("step {step} matches no reflex"),
        Because::Needs { step, arg } => format!("step {step} needs {arg}"),
        Because::Several {
            step,
            source,
            fields,
        } => {
            let fields: Vec<&str> = fields
                .iter()
                .map(evoke_core::name::FieldName::as_str)
                .collect();
            format!(
                "step {step} takes one of several things step {source} yields — {} — which one?",
                fields.join(", ")
            )
        }
        Because::OneOfMany {
            step,
            source,
            field,
        } => format!(
            "step {step} takes one {field} from step {source}, which finds several — which one?"
        ),
        Because::TakesNothing { step, sources } => {
            let sources: Vec<String> = sources.iter().map(ToString::to_string).collect();
            format!(
                "step {step} refers to step {}, but takes nothing from it",
                sources.join(" and ")
            )
        }
        Because::NoSource { step, name } => {
            format!("step {step} takes {name}, which no step before it returns")
        }
        Because::SeveralSources {
            step,
            name,
            sources,
        } => {
            if let [source] = sources.as_slice() {
                format!(
                    "step {step} takes one {name}, and step {source} returns one for each record"
                )
            } else {
                let sources: Vec<String> = sources.iter().map(ToString::to_string).collect();
                format!(
                    "step {step} takes one {name}, and steps {} each return one",
                    sources.join(" and ")
                )
            }
        }
        Because::Reviewed { prompt, .. } => prompt.own.clone(),
        Because::Nested { step, playbook } => {
            format!("step {step} routes to {playbook}, which is expanding already")
        }
        Because::TooDeep { step, .. } => {
            format!("step {step} is a plan inside a plan inside a plan")
        }
        Because::TooLong { steps, .. } => {
            format!(
                "the plan holds {steps} steps; {} is the most",
                evoke_core::manifest::MOST_STEPS
            )
        }
        Because::Conditional { text } => format!(
            "{} is a condition, which no step can judge; ask for the check first, then say what to do",
            quoted(text)
        ),
        Because::Excluded { text, playbook } => format!(
            "{} leaves out a step the {playbook} plan may hold; say the steps you want",
            quoted(text)
        ),
        Because::BranchIntoPlan { step, .. } => {
            format!("step {step} branches into a plan; a branch is one step")
        }
        Because::NoField { step, field } => {
            format!("step {step} branches on {field}, which the step before it does not yield")
        }
        Because::BadValue { step, field, is } => format!(
            "step {step} waits on {field} = \"{}\", which no {field} reads",
            is.as_str()
        ),
        Because::MaybeSource { step, name, source } => {
            format!("step {step} takes {name} from step {source}, which may not run")
        }
    }
}

/// Why a plan waits for one yes, a reason a line: what the verdict says, and for a playbook its own line with
/// why its call waits under it; `head` before the first, where the plan came from a file.
#[must_use]
pub fn reasons(head: Option<&str>, because: &[Because]) -> Text {
    let mut lines: Vec<Text> = Vec::new();
    for because in because {
        let mut line = Text::from(verdict(because));
        if let Because::Reviewed { prompt, .. } = because {
            because_under(&mut line, &prompt.reason);
        }
        lines.push(line);
    }
    if let Some(head) = head {
        let mut first = Text::from(head);
        if let Some(line) = lines.first().cloned() {
            first.push(" · ").append(line);
            lines.remove(0);
        }
        lines.insert(0, first);
    }
    indented(lines)
}

/// `show <name>` on a reflex that takes whole results: per name, which installed reflexes return it.
#[must_use]
pub fn taken(takes: &[(FieldName, Vec<LocalName>)]) -> Text {
    indented(
        takes
            .iter()
            .map(|(name, returners)| {
                let returned = match returners.as_slice() {
                    [] => "nothing installed returns".to_owned(),
                    [one] => format!("{one} returns"),
                    [rest @ .., last] => format!(
                        "{} and {last} return",
                        rest.iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                };
                Text::from(format!("takes {name}, which {returned}"))
            })
            .collect(),
    )
}

/// `evoke`'s own line before a confirm, the machine's status on its end when it does not hold the declaration,
/// and why the call waits under it.
#[must_use]
pub fn confirming(chosen: &Chosen, prompt: &Prompt, contained: &Contained) -> Text {
    let mut text = Text::from("  ");
    text.append(waiting(chosen, prompt, Some(contained)).hang(2));
    text
}

/// The prompt's own line, the machine's status on its end, and under it the line that says why the call waits.
fn waiting(chosen: &Chosen, prompt: &Prompt, contained: Option<&Contained>) -> Text {
    let mut text = own(chosen, &prompt.own);
    if let Some(contained) = contained {
        held(&mut text, contained);
    }
    because_under(&mut text, &prompt.reason);
    text
}

/// Why a call waits, on a line of its own under the call.
fn because_under(text: &mut Text, reason: &str) {
    if !reason.is_empty() {
        text.push("\n  ").roled(Role::Weak, &plain(reason));
    }
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
/// effect, then the weakest judgment when it comes next. A line shaped otherwise stays whole and plain.
fn own(chosen: &Chosen, own: &str) -> Text {
    let head = format!("{} · {}", render(&chosen.call), chosen.effect);
    let Some(rest) = own.strip_prefix(head.as_str()) else {
        return Text::from(own);
    };
    let mut text = call(&chosen.call);
    text.push(" · ")
        .roled(Role::Effect(chosen.effect), &chosen.effect.to_string());
    match rest.strip_prefix(" · weakest: ") {
        Some(weakest) => {
            text.push(" · ")
                .roled(Role::Weak, &format!("weakest: {weakest}"));
        }
        None => {
            text.push(rest);
        }
    }
    text
}

/// The confirm prompt itself, read on the terminal; `[t]each` only where there is an utterance to teach.
#[must_use]
pub fn confirm_prompt(prompt: &Prompt, teachable: bool, retry: Option<&str>) -> String {
    let choices = if teachable {
        "[y]es [n]o [t]each"
    } else {
        "[y]es [n]o"
    };
    let retry = retry.map_or_else(String::new, |retry| format!("{retry}  "));
    format!("  {}  {retry}{choices} > ", prompt.template)
}

/// What a listed choice offers last: none of them, which declines the question.
pub const NONE_OF_THESE: &str = "none of these";

/// What takes the word a listed ask makes ready.
pub const YES: [&str; 2] = ["y", "yes"];

/// The ask prompt: numbered choices and `[0] none of these`, a vocabulary's `[+] add one`, or a pick typed
/// freely — what the words read as numbered before it, with `[0] none of these`, or else the values recalled
/// from the process's results; a number pick's are named as hints; `retry` says why the last answer did not do.
/// A listed word made ready stands before the choices, `sam?  [y]es, or`, joined to the words it was read from.
#[must_use]
pub fn ask_prompt(missing: &Missing, retry: Option<&str>) -> String {
    use evoke_core::decide::Choices;
    let mut line = format!("  {}  ", missing.ask);
    let ready = missing
        .likely
        .as_ref()
        .filter(|_| matches!(missing.choices, Choices::Vocab { .. }));
    match (retry, ready) {
        (Some(retry), Some(word)) if retry.starts_with(WROTE) => {
            let _ = write!(line, "{retry}: {word}?  [y]es, or  ");
        }
        (Some(retry), Some(word)) => {
            let _ = write!(line, "{retry}  {word}?  [y]es, or  ");
        }
        (Some(retry), None) => {
            let _ = write!(line, "{retry}  ");
        }
        (None, Some(word)) => {
            let _ = write!(line, "{word}?  [y]es, or  ");
        }
        (None, None) => {}
    }
    match &missing.choices {
        Choices::Options { options } => {
            for (i, key) in options.keys().enumerate() {
                let _ = write!(line, "[{}] {key}  ", i + 1);
            }
            let _ = write!(line, "[0] {NONE_OF_THESE}  ");
        }
        Choices::Vocab { words } => {
            for (i, word) in words.keys().enumerate() {
                let _ = write!(line, "[{}] {word}  ", i + 1);
            }
            let _ = write!(line, "[+] add one  [0] {NONE_OF_THESE}  ");
        }
        Choices::Pick {
            pick,
            recent,
            readings,
        } => {
            let offered = offered(recent.as_deref(), readings);
            if offered.is_empty() {
                // Nothing to name: the value is typed.
            } else if *pick == Recognizer::Number {
                // A number pick names its values as hints: a number typed is its own answer.
                let _ = write!(line, "{}  ", offered.join(" · "));
            } else {
                for (i, value) in offered.iter().enumerate() {
                    let _ = write!(line, "[{}] {value}  ", i + 1);
                }
                if !readings.is_empty() {
                    let _ = write!(line, "[0] {NONE_OF_THESE}  ");
                }
            }
        }
    }
    line.push_str("> ");
    line
}

/// How an ask's reason opens when it quotes the words that answer it.
const WROTE: &str = "you wrote ";

/// The values a pick's prompt names: what the words read as, where they read as any; else the values recalled.
#[must_use]
pub fn offered<'a>(recent: Option<&'a [String]>, readings: &'a [Clean]) -> Vec<&'a str> {
    if readings.is_empty() {
        recent
            .unwrap_or_default()
            .iter()
            .map(String::as_str)
            .collect()
    } else {
        readings.iter().map(Clean::as_str).collect()
    }
}

/// Why an argument is asked, where the question alone does not say: `150 percent is outside 0–100`; that what
/// the request names is not on the list, with the words where the reading found them; the words that answer the
/// ask, with what they read as or as read as no value.
#[must_use]
pub fn because(missing: &Missing) -> Option<String> {
    use evoke_core::decide::Choices;
    let words = missing
        .words
        .as_ref()
        .map(|words| plain(&quoted(words.text().as_str())));
    let listed = matches!(
        missing.choices,
        Choices::Options { .. } | Choices::Vocab { .. }
    );
    match (&missing.because, words) {
        (Why::OutOfRange { span, range }, _) => Some(format!(
            "{} is outside {}–{}",
            span.text(),
            range.min(),
            range.max()
        )),
        (Why::NotOffered, Some(words)) if listed => Some(format!("{words} is not on the list")),
        (Why::NotOffered, None) if listed => Some("what you named is not on the list".to_owned()),
        (_, Some(words)) if listed => Some(format!("{WROTE}{words}")),
        (_, Some(words)) => match &missing.choices {
            Choices::Pick {
                pick: Recognizer::Quoted,
                ..
            } => Some(format!("{WROTE}{words}")),
            Choices::Pick { readings, .. } if !readings.is_empty() => {
                Some(format!("{WROTE}{words}"))
            }
            Choices::Pick { pick, .. } => Some(format!("{words} is not {}", pick.wants())),
            _ => None,
        },
        (_, None) => None,
    }
}

/// A question declined, at the end of input or by none of these: nothing runs, and the plan says which step's
/// value it lacks.
#[must_use]
pub fn declined(arg: &ArgName, step: Option<usize>) -> Text {
    Text::from(match step {
        Some(step) => format!("  nothing runs: step {step} has no {arg}"),
        None => format!("  nothing runs without {arg}"),
    })
}

/// `teach` of a phrase the reflex's examples already hold with the same values: nothing to write.
#[must_use]
pub fn already_example(text: &str, reflex: &LocalName) -> Text {
    Text::from(format!(
        "  {} is already an example of {reflex}",
        plain(&quoted(text))
    ))
}

/// After `config`: a path the declaration reads or writes, which is not there yet.
#[must_use]
pub fn not_there_yet(reflex: &LocalName, path: &str) -> Text {
    Text::from(format!(
        "  {reflex}: {path} is not there yet; the body needs it to exist"
    ))
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

/// `[+] add one`, third prompt, where a body's declaration takes the word's value as a path: the path.
#[must_use]
pub fn path_prompt(retry: Option<&str>) -> String {
    match retry {
        Some(retry) => format!("  Path?  {retry}  > "),
        None => "  Path?  > ".to_owned(),
    }
}

/// `evoke --help`: the version with its line, then every command by group — its line, then what it does, in
/// a second column where the terminal is wide enough for one and under the line where it is not — then the exit
/// codes and the manual. Plain: it goes to stdout. `columns` is the terminal's width, when stdout is one.
#[must_use]
pub fn help(version: &str, columns: Option<usize>) -> String {
    let described = || {
        COMMANDS
            .iter()
            .flat_map(|(_, lines)| lines.iter())
            .filter(|(_, about)| !about.is_empty())
    };
    let width = described().map(|(line, _)| chars(line)).max().unwrap_or(0);
    let widest = described()
        .map(|(_, about)| 2 + width + 2 + chars(about))
        .max()
        .unwrap_or(0);
    let stacked = columns.is_some_and(|columns| columns < widest);
    let mut text = format!("evoke {version} · you invoke a function; you evoke a reflex\n");
    for (group, lines) in COMMANDS {
        let _ = write!(text, "\n{group}\n");
        for (line, about) in lines {
            if about.is_empty() {
                let _ = writeln!(text, "  {line}");
            } else if stacked {
                let _ = writeln!(text, "  {line}\n      {about}");
            } else {
                let _ = writeln!(text, "  {line:<width$}  {about}");
            }
        }
    }
    text.push_str(if stacked {
        "\nexit    0 ran · 1 failed · 2 declined\n        3 needs a human · 4 adapter failed"
    } else {
        "\nexit    0 ran · 1 failed · 2 declined · 3 needs a human · 4 adapter failed"
    });
    text.push_str("\nmanual  https://evoke.build/manual/");
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
            ("  --save <file>", "the plan as a file, for run"),
            ("  --", "the rest is input, even a command word"),
            ("evoke why", "the last sentence, explained"),
            ("evoke run <call>", "by name, without the classifier"),
            ("evoke run <file>", "a saved plan, run as it stands"),
            ("  --json", "one JSON line: the call and its result"),
            ("  <call> is <name> [<arg>=<value> | <flag>]…", ""),
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
            ("  --forget <name>", "the overlay's line for it, removed"),
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
            (
                "evoke calibrate [<name>]",
                "the confidence measured on the records",
            ),
            ("  --repeat <k>", "each input decided k times: the spread"),
            ("  --json", "one JSON object, for a script"),
        ],
    ),
    (
        "author",
        &[
            ("evoke new <name>", "a working reflex from the template"),
            ("  --playbook", "a playbook instead: a plan of steps"),
            ("evoke check", "lint, types, contract against its tag"),
        ],
    ),
];

/// The line of a write: `+` the file and the line as it landed, `-` the file and the key that went, or `+` the
/// file alone when it was written whole.
#[must_use]
pub fn written(shown: &str, landed: &Landed) -> Text {
    let mut text = Text::new();
    match landed {
        Landed::Set(line) => text
            .roled(Role::Added, "+")
            .push(&format!(" {shown}  {line}")),
        Landed::Removed(key) => text
            .roled(Role::Removed, "-")
            .push(&format!(" {shown}  {key}")),
        Landed::Whole => text.roled(Role::Added, "+").push(&format!(" {shown}")),
    };
    text
}

/// One installed reflex as `show`, `add`, `remove` and `sync` list it: its name, where it comes from with its
/// locked version, the effect it runs under — none when its manifest does not read — what it runs, and what it
/// may touch, on a second line when it declares anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub from: String,
    pub effect: Option<Effect>,
    pub runs: String,
    pub needs: Needs,
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
    Text::lines(rows.iter().flat_map(|row| {
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
        let mut lines = vec![line.trim_end()];
        if !row.needs.is_none() {
            let mut needs = Text::from(format!("  {:<names$}  ", ""));
            needs
                .roled(Role::Weak, "needs")
                .push(" ")
                .push(&row.needs.to_string());
            lines.push(needs);
        }
        lines
    }))
}

/// `evoke test` with no case to decide: the reflex named has no examples and no tests, or no active reflex has.
#[must_use]
pub fn nothing_to_test(name: Option<&LocalName>) -> Text {
    let who = name.map_or_else(
        || "no active reflex has".to_owned(),
        |name| format!("{name} has no"),
    );
    indented(vec![Text::from(format!(
        "nothing to test: {who} examples or tests"
    ))])
}

/// `evoke calibrate` with no record to decide: the reflex named has no examples and no tests, or no active reflex
/// has.
#[must_use]
pub fn nothing_to_calibrate(name: Option<&LocalName>) -> Text {
    let who = name.map_or_else(
        || "no active reflex has".to_owned(),
        |name| format!("{name} has no"),
    );
    indented(vec![Text::from(format!(
        "nothing to calibrate: {who} examples or tests"
    ))])
}

/// `calibrate`'s block: the head, the outcomes, the whole call right by bins with `unknown` and `abstained`
/// apart, each bar with its bound and its neighbourhood, each judgment on its own, Brier, the misses, the repeats,
/// the log's block, and the closing line. A claimed confidence is weak; a wrong count and `over-confident` are
/// failures; `thin` and `unknown` warn. Nothing is said by colour alone.
#[must_use]
pub fn calibrated(calibration: &Calibration, log: &LogBlock) -> Text {
    let c = calibration;
    let mut lines = vec![
        heading(c),
        outcomes(c),
        Text::from("whole call right, by confidence"),
    ];
    lines.extend(binned(c));
    lines.extend(bars(c));
    if !c.questions.is_empty() {
        lines.push(Text::from("each judgment on its own"));
        lines.extend(questions(&c.questions));
    }
    if let Some(brier) = &c.brier {
        lines.push(Text::from(format!(
            "Brier {:.3} · reliability {:.3} · resolution {:.3}",
            brier.brier, brier.reliability, brier.resolution
        )));
    }
    if !c.misses.is_empty() {
        lines.push(Text::from("misses"));
        lines.extend(misses(&c.misses, c.repeats));
    }
    if let Some(variance) = &c.variance {
        lines.push(Text::from(format!(
            "repeats {} · winner flips {} of {} · verdict flips {} · spread median {:.2}, 90th {:.2}, max {:.2} · straddling a bar {} · wrong at or over a bar {}",
            c.repeats,
            variance.flips,
            c.inputs,
            variance.verdict_flips,
            variance.spread.median,
            variance.spread.p90,
            variance.spread.max,
            variance.straddling,
            variance.wrong_at_bar
        )));
        for moved in &variance.moved {
            let confidence = moved.confidence.map_or_else(
                || "no call".to_owned(),
                |(lo, hi)| format!("confidence {}", ranged(lo.get(), hi.get())),
            );
            let counted = |counts: &indexmap::IndexMap<String, usize>| -> String {
                counts
                    .iter()
                    .map(|(key, n)| format!("{key} ×{n}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let mut line = Text::from(format!(
                "  {}  {confidence} · route {} · {} · {}",
                plain(&quoted(&moved.utterance)),
                ranged(moved.route.0.get(), moved.route.1.get()),
                counted(&moved.winners),
                counted(&moved.outcomes)
            ));
            if moved.wrong > 0 {
                line.push(" · ")
                    .roled(Role::Failed, &format!("wrong ×{}", moved.wrong));
            }
            lines.push(line);
        }
    }
    lines.extend(logged(log));
    lines.push(closing(c));
    indented(lines)
}

/// `<adapter> · 93 records over 13 reflexes · 88 inputs, decided once`.
fn heading(c: &Calibration) -> Text {
    let reflexes = if c.reflexes == 1 {
        "reflex"
    } else {
        "reflexes"
    };
    let decided = match c.repeats {
        1 => "once".to_owned(),
        k => format!("{k} times"),
    };
    Text::from(format!(
        "{} · {} records over {} {reflexes} · {} inputs, decided {decided}",
        c.adapter, c.records, c.reflexes, c.inputs
    ))
}

fn outcomes(c: &Calibration) -> Text {
    let counts = [
        (c.outcomes.run, "run"),
        (c.outcomes.confirm, "confirm"),
        (c.outcomes.ask, "ask"),
        (c.outcomes.abstain, "abstain"),
    ];
    let shown: Vec<String> = counts
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, word)| format!("{n} {word}"))
        .collect();
    Text::from(shown.join(" · "))
}

/// `lo–hi`, or one number when both are the same.
fn ranged(lo: f64, hi: f64) -> String {
    let (lo, hi) = (format!("{lo:.2}"), format!("{hi:.2}"));
    if lo == hi { lo } else { format!("{lo}–{hi}") }
}

/// `(34–100)`: the interval in whole percents.
fn interval(interval: (evoke_core::Prob, evoke_core::Prob)) -> String {
    format!(
        "({:.0}–{:.0})",
        100.0 * interval.0.get(),
        100.0 * interval.1.get()
    )
}

/// `100%`.
fn percent(right: usize, of: usize) -> String {
    #[expect(clippy::cast_precision_loss)]
    let share = if of == 0 {
        0.0
    } else {
        100.0 * right as f64 / of as f64
    };
    format!("{share:.0}%")
}

/// A bin's range, `0.60–0.80`; the first from zero is `under 0.50`.
fn range_of(bin: &BinRow) -> String {
    if bin.lo.get() == 0.0 {
        format!("under {:.2}", bin.hi.get())
    } else {
        format!("{:.2}–{:.2}", bin.lo.get(), bin.hi.get())
    }
}

/// The bins' rows, then `unknown` and `abstained` apart, their columns aligned: the range, the count and its
/// unit, then the share with its interval and the claim.
fn binned(c: &Calibration) -> Vec<Text> {
    let share = |right: usize, count: usize, interval: (evoke_core::Prob, evoke_core::Prob)| {
        Text::from(format!(
            "right {right} · {:>4} {}",
            percent(right, count),
            self::interval(interval)
        ))
    };
    let mut rows: Vec<(Text, usize, &str, Text)> = c
        .bins
        .iter()
        .map(|bin| {
            let mut rest = share(bin.right, bin.calls, bin.interval);
            rest.push(" · ")
                .roled(Role::Weak, &format!("claimed {:.2}", bin.claimed.get()));
            if bin.over_confident {
                rest.push(" · ").roled(Role::Failed, "over-confident");
            }
            if bin.thin {
                rest.push(" · ").roled(Role::Warning, "thin");
            }
            (Text::from(range_of(bin)), bin.calls, "calls", rest)
        })
        .collect();
    if c.unknown > 0 {
        let mut label = Text::new();
        label.roled(Role::Warning, "unknown");
        rows.push((
            label,
            c.unknown,
            "calls",
            Text::from("a false record routed to a reflex no record names"),
        ));
    }
    if c.abstained.count > 0 {
        let a = &c.abstained;
        rows.push((
            Text::from("abstained"),
            a.count,
            "inputs",
            share(a.right, a.count, a.interval),
        ));
    }
    let labels = rows
        .iter()
        .map(|(label, ..)| chars(&label.to_string()))
        .max()
        .unwrap_or(0);
    let counts = rows
        .iter()
        .map(|(_, count, ..)| count.to_string().len())
        .max()
        .unwrap_or(0);
    rows.into_iter()
        .map(|(label, count, unit, rest)| {
            let unit = if count == 1 {
                unit.trim_end_matches('s')
            } else {
                unit
            };
            let padding = labels - chars(&label.to_string());
            let mut line = Text::from("  ");
            line.append(label)
                .push(&format!("{:padding$}  {count:>counts$} {unit:<6}  ", ""))
                .append(rest);
            line
        })
        .collect()
}

/// Each bar's line under a gate — `no read calls` without one of its effect — and its neighbourhood.
fn bars(c: &Calibration) -> Vec<Text> {
    let bars: [(&str, Option<&BarRow>); 2] = [
        ("read", c.bars.read.as_ref()),
        ("write", c.bars.write.as_ref()),
    ];
    let width = bars
        .iter()
        .filter_map(|(_, bar)| bar.map(|bar| bar.calls.max(bar.wrong).to_string().len()))
        .max()
        .unwrap_or(1);
    let labels = bars
        .iter()
        .filter(|(_, bar)| bar.is_some())
        .map(|(effect, _)| chars(effect))
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for (effect, bar) in bars {
        let Some(bar) = bar else {
            continue;
        };
        let label = format!("wrong at or over {effect} {:.2}", bar.bar.get());
        let padding = labels - chars(effect);
        let mut line = Text::from(format!("{label}{:padding$}   ", ""));
        if bar.wrong > 0 {
            line.roled(Role::Failed, &format!("{:>width$}", bar.wrong));
        } else {
            line.push(&format!("{:>width$}", bar.wrong));
        }
        line.push(&format!(" of {:>width$}", bar.calls));
        if bar.calls > 0 {
            line.push(&format!(
                " · {:.0} per thousand, at most {:.0}",
                bar.per_thousand, bar.at_most
            ));
        }
        if bar.calls < calibrate::THIN {
            line.push(" · ").roled(Role::Warning, "thin");
        }
        lines.push(line);
        let cells: Vec<String> = bar
            .near
            .iter()
            .enumerate()
            .map(|(i, near)| {
                if i == 0 {
                    format!(
                        "at {:.2} {} run, {} wrong",
                        near.at.get(),
                        near.run,
                        near.wrong
                    )
                } else {
                    format!("at {:.2} {}, {}", near.at.get(), near.run, near.wrong)
                }
            })
            .collect();
        lines.push(Text::from(format!(
            "near {effect} {:.2}   {}",
            bar.bar.get(),
            cells.join(" · ")
        )));
    }
    // The calls over their effect's bar, held against the input: what the hold lets run, and what it holds.
    if let Some(hold) = &c.bars.whole {
        let mut line = Text::from(format!("wrong at or over whole {:.2}   ", hold.bar.get()));
        if hold.wrong > 0 {
            line.roled(Role::Failed, &hold.wrong.to_string());
        } else {
            line.push("0");
        }
        line.push(&format!(" of {} run", hold.run));
        if hold.run > 0 {
            line.push(&format!(
                " · {:.0} per thousand, at most {:.0}",
                hold.per_thousand, hold.at_most
            ));
        }
        line.push(&format!(
            " · {} of {} held for a yes, {} right",
            hold.calls - hold.run,
            hold.calls,
            hold.held_right
        ));
        if hold.calls < calibrate::THIN {
            line.push(" · ").roled(Role::Warning, "thin");
        }
        lines.push(line);
    }
    lines
}

fn questions(rows: &[QuestionRow]) -> Vec<Text> {
    let width = rows
        .iter()
        .map(|row| row.judgments.to_string().len())
        .max()
        .unwrap_or(0);
    rows.iter()
        .map(|row| {
            let kind = match row.kind {
                calibrate::QuestionKind::Route => "route",
                calibrate::QuestionKind::Options => "options",
                calibrate::QuestionKind::Vocab => "vocab",
                calibrate::QuestionKind::Pick => "pick",
                calibrate::QuestionKind::Flag => "flag",
            };
            let mut line = Text::from(format!(
                "  {kind:<7} {:>width$} judgments  right {:>4} {}",
                row.judgments,
                percent(row.right, row.judgments),
                interval(row.interval)
            ));
            line.push(" · ")
                .roled(Role::Weak, &format!("claimed {:.2}", row.claimed.get()));
            line
        })
        .collect()
}

/// One line per miss: the record, what was decided, where it missed, and over repeats how many missed so.
fn misses(misses: &[Miss], repeats: usize) -> Vec<Text> {
    let utterances: Vec<String> = misses
        .iter()
        .map(|miss| plain(&quoted(miss.case.utterance.text().as_str())))
        .collect();
    let width = utterances.iter().map(|u| chars(u)).max().unwrap_or(0);
    misses
        .iter()
        .zip(&utterances)
        .map(|(miss, utterance)| {
            let mut line = Text::from(format!(
                "  {utterance:<width$}  {}",
                calibrate::word(miss.outcome)
            ));
            if let Some(reflex) = &miss.reflex {
                line.push(" ").roled(Role::Call, reflex.as_str());
            }
            if let Some(confidence) = miss.confidence {
                line.push(" ")
                    .roled(Role::Weak, &format!("{:.2}", confidence.get()));
            }
            line.push(" · ").push(&missed(&miss.case, &miss.mismatch));
            if repeats > 1 {
                line.push(&format!(" · {} of {repeats} repeats", miss.wrong));
            }
            line
        })
        .collect()
}

/// The log's block: the lines under the adapter by what became of each, the confidence of what stopped at a
/// confirm, the lines that were records judged; `no decisions` when none.
fn logged(log: &LogBlock) -> Vec<Text> {
    let mut head = if log.decisions == 0 {
        Text::from(format!("the log · no decisions under {}", log.adapter))
    } else {
        let counts = [
            (log.ran, "ran"),
            (log.confirmed_ran, "confirmed and ran"),
            (log.confirmed_stopped, "confirmed and stopped"),
            (log.asked, "asked and stopped"),
            (log.abstained, "abstained"),
            (log.failed, "failed"),
            (log.skipped, "never ran"),
        ];
        let decisions = if log.decisions == 1 {
            "decision"
        } else {
            "decisions"
        };
        let mut text = Text::from(format!(
            "the log · {} {decisions} under {}",
            log.decisions, log.adapter
        ));
        for (n, word) in counts {
            if n > 0 {
                text.push(&format!(" · {n} {word}"));
            }
        }
        text
    };
    if log.unread > 0 {
        let lines = if log.unread == 1 {
            "line does"
        } else {
            "lines do"
        };
        head.push(" · ")
            .roled(Role::Warning, &format!("{} {lines} not read", log.unread));
    }
    let mut lines = vec![head];
    if !log.stopped_by_confidence.is_empty() {
        let cells: Vec<String> = log
            .stopped_by_confidence
            .iter()
            .map(|counted| {
                format!(
                    "{:.2}–{:.2} {}",
                    counted.lo.get(),
                    counted.hi.get(),
                    counted.count
                )
            })
            .collect();
        lines.push(Text::from(format!(
            "  stopped at confirm, by confidence   {}",
            cells.join(" · ")
        )));
    }
    if log.records.count > 0 {
        let r = &log.records;
        let were = if r.count == 1 {
            "was a record"
        } else {
            "were records"
        };
        lines.push(Text::from(format!(
            "  {} {were} · right {} · {} {}",
            r.count,
            r.right,
            percent(r.right, r.count),
            interval(r.interval)
        )));
    }
    lines
}

/// The last line: `thin` while every bin is under a hundred calls; else whether a bin of a hundred is
/// over-confident at 95 %.
fn closing(c: &Calibration) -> Text {
    let over: Vec<String> = c
        .bins
        .iter()
        .filter(|bin| !bin.thin && bin.over_confident)
        .map(range_of)
        .collect();
    let mut line = Text::new();
    if c.bins.iter().all(|bin| bin.thin) {
        line.roled(Role::Warning, "thin")
            .push(": under 100 calls in every bin, nothing proven at any bar");
    } else if over.is_empty() {
        line.push("at 95 %: no bin of 100 calls is over-confident");
    } else {
        line.push("at 95 %: ")
            .roled(Role::Failed, "over-confident")
            .push(&format!(" at {}", over.join(", ")));
    }
    line
}

/// `evoke update` with nothing to move, `evoke sync` with nothing to place: every remote reflex is where the lock
/// says.
#[must_use]
pub fn up_to_date() -> Text {
    Text::from("  up to date")
}

/// `evoke trust`: the root blessed.
#[must_use]
pub fn trusted(root: &str) -> Text {
    let mut text = Text::new();
    text.roled(Role::Added, "+")
        .push(&format!(" trusted {root}"));
    text
}

/// `evoke trust` in the home project, which needs no blessing.
#[must_use]
pub fn home_trusted(root: &str) -> Text {
    Text::from(format!(
        "  {root} is your home project, trusted by construction"
    ))
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
        .push(&format!("  {}", runs(&m.run, &m.steps)));
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
        Change::ArgRemoved { arg } | Change::TakesRemoved { arg, .. } => {
            (format!("args.{arg}"), "removed".to_owned())
        }
        Change::OptionRemoved { arg, key } => {
            (format!("args.{arg}.options.{key}"), "removed".to_owned())
        }
        Change::SourceChanged { arg } => (format!("args.{arg}"), "source changed".to_owned()),
        Change::RangeChanged { arg } => (format!("args.{arg}"), "range changed".to_owned()),
        Change::RecentChanged { arg } => (format!("args.{arg}"), "recent changed".to_owned()),
        Change::RunChanged => ("run".to_owned(), "changed".to_owned()),
        Change::StepsChanged => ("steps".to_owned(), "the steps moved".to_owned()),
        Change::NeedsWidened { added } => ("needs".to_owned(), format!("widened: {added}")),
        Change::NeedsNarrowed { removed } => {
            ("needs".to_owned(), format!("narrowed: {removed} dropped"))
        }
        Change::Required { arg } => (format!("args.{arg}"), "now required".to_owned()),
        Change::ConfigSecret { key, secret: true } => (
            format!("config.{key}"),
            "now a secret; a plain setting turns the reflex inactive".to_owned(),
        ),
        Change::ConfigSecret { key, secret: false } => {
            (format!("config.{key}"), "no longer a secret".to_owned())
        }
        Change::ArgAdded { arg } => (format!("args.{arg}"), "added".to_owned()),
        Change::Optional { arg } => (format!("args.{arg}"), "now optional".to_owned()),
        Change::OptionAdded { arg, key } => {
            (format!("args.{arg}.options.{key}"), "added".to_owned())
        }
        Change::ConfigAdded { key } => (format!("config.{key}"), "added".to_owned()),
        Change::ConfigRemoved { key } => (
            format!("config.{key}"),
            "removed; your setting is skipped".to_owned(),
        ),
        Change::YieldAdded { field } => (format!("yields.{field}"), "added".to_owned()),
        Change::YieldRemoved { field } => (format!("yields.{field}"), "removed".to_owned()),
        Change::YieldChanged { field } => (format!("yields.{field}"), "changed".to_owned()),
        Change::TakesAdded { arg, name } => (format!("args.{arg}"), format!("added, takes {name}")),
        Change::TakesChanged { arg, name } => (format!("args.{arg}"), format!("takes {name} now")),
        Change::ReturnsAdded { name } => ("returns".to_owned(), format!("added: {name}")),
        Change::ReturnsRemoved { .. } => ("returns".to_owned(), "removed".to_owned()),
        Change::ReturnsChanged { name } => ("returns".to_owned(), format!("changed: {name}")),
        Change::PlatformAdded { platform } => {
            ("platforms".to_owned(), format!("added: {}", platform.key()))
        }
        Change::PlatformRemoved { platform } => (
            "platforms".to_owned(),
            format!("dropped: {}", platform.key()),
        ),
    }
}

/// A playbook as `test` judged its steps: each sentence and where it routed, and the effect it claims.
pub struct TestedPlaybook {
    pub name: LocalName,
    pub claim: Effect,
    pub steps: Vec<TestedStep>,
}

/// One step of a playbook as `test` judged it: its number, its sentence with the slots as written, what became
/// of it, and whether it passed at the last run.
pub struct TestedStep {
    pub n: usize,
    pub sentence: String,
    pub became: StepBecame,
    pub regression: bool,
}

/// What became of a playbook's step when its words were decided, filled from a record: the reflex it routes to
/// and that reflex's effect; a reflex that yields no such field as a step after it waits on; nothing; the
/// playbook it stands in; or untested, no record reading the slot named.
pub enum StepBecame {
    Routes { reflex: LocalName, effect: Effect },
    NoField { reflex: LocalName, field: FieldName },
    NoReflex,
    Nested,
    Untested { slot: ArgName },
}

/// `test`'s block: per reflex its name and counts, then one line per failed case — the utterance and where the
/// decision missed, `· regression` when it passed at the last run; for a playbook, how many of its steps route,
/// then each step that does not, and its claim when the steps reach a tighter effect.
#[must_use]
pub fn tested(
    verdicts: &[(Case, Verdict)],
    regressions: &[Regression],
    playbooks: &[TestedPlaybook],
) -> Text {
    let mut reflexes: Vec<(&LocalName, Vec<&(Case, Verdict)>)> = Vec::new();
    for judged in verdicts {
        match reflexes.last_mut() {
            Some((name, cases)) if *name == &judged.0.reflex => cases.push(judged),
            _ => reflexes.push((&judged.0.reflex, vec![judged])),
        }
    }
    // A playbook with no record still has its steps to show.
    for playbook in playbooks {
        if !reflexes.iter().any(|(name, _)| *name == &playbook.name) {
            reflexes.push((&playbook.name, Vec::new()));
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
        let playbook = playbooks.iter().find(|playbook| playbook.name == *name);
        if let Some(playbook) = playbook {
            let routes = playbook
                .steps
                .iter()
                .filter(|step| matches!(step.became, StepBecame::Routes { .. }))
                .count();
            let total = playbook.steps.len();
            if routes == total {
                line.push(&format!(" · {total} steps route"));
            } else {
                line.push(" · ")
                    .roled(Role::Failed, &format!("{routes} of {total} steps route"));
            }
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
        if let Some(playbook) = playbook {
            lines.extend(playbook_lines(playbook));
        }
    }
    Text::lines(lines)
}

/// Under a playbook's line: each step that does not route, and its claim when the steps reach a tighter effect
/// — reported, never failed.
fn playbook_lines(playbook: &TestedPlaybook) -> Vec<Text> {
    let mut lines = Vec::new();
    let inner = playbook.steps.len().to_string().len();
    for step in &playbook.steps {
        let became = match &step.became {
            StepBecame::Routes { .. } => continue,
            StepBecame::NoField { reflex, field } => {
                format!("reaches {reflex}, which yields no {field}")
            }
            StepBecame::NoReflex => "no reflex".to_owned(),
            StepBecame::Nested => "nested".to_owned(),
            StepBecame::Untested { slot } => format!("untested: no test reads {{{slot}}}"),
        };
        let mut line = Text::from(format!(
            "    {:>inner$}  {} · {became}",
            step.n,
            quoted(&step.sentence)
        ));
        if step.regression {
            line.push(" · ").roled(Role::Failed, "regression");
        }
        lines.push(line);
    }
    let worst = playbook
        .steps
        .iter()
        .filter_map(|step| match &step.became {
            StepBecame::Routes { reflex, effect } => Some((step.n, reflex, *effect)),
            _ => None,
        })
        .max_by_key(|(_, _, effect)| *effect);
    if let Some((n, reflex, effect)) = worst
        && effect > playbook.claim
    {
        lines.push(Text::from(format!(
            "    claims {}; step {n} reaches {reflex}, {effect}",
            playbook.claim
        )));
    }
    lines
}

/// `add`'s reach: what each step of a newcomer playbook routes to on this set, one line per step under its row —
/// the reflex and its effect, `nothing here`, or the slot no record reads.
#[must_use]
pub fn reached(steps: &[TestedStep]) -> Text {
    let width = steps.len().to_string().len();
    Text::lines(steps.iter().map(|step| {
        let mut line = Text::from(format!("  {:>width$}  {} → ", step.n, step.sentence));
        match &step.became {
            StepBecame::Routes { reflex, effect } => {
                line.roled(Role::Call, reflex.as_str())
                    .push(" · ")
                    .roled(Role::Effect(*effect), &effect.to_string());
            }
            StepBecame::NoField { reflex, field } => {
                line.roled(Role::Call, reflex.as_str())
                    .push(" · ")
                    .roled(Role::Warning, &format!("yields no {field}"));
            }
            StepBecame::NoReflex => {
                line.roled(Role::Warning, "nothing here");
            }
            StepBecame::Nested => {
                line.roled(Role::Warning, "its own plan");
            }
            StepBecame::Untested { slot } => {
                line.push(&format!("untested: no record reads {{{slot}}}"));
            }
        }
        line
    }))
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

/// `check`: a manifest key this evoke does not read — a newer evoke's, or a slip — parked and passed along whole.
#[must_use]
pub fn unknown_key(reflex: &LocalName, path: &KeyPath) -> Text {
    let mut text = Text::from("  ");
    text.roled(Role::Warning, "unknown")
        .push(&format!("  {reflex}: {path} is not a key this evoke reads"));
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

/// `runs <file>`, `runs <program>`, or `a plan of <n> steps` for a playbook.
pub(crate) fn runs(run: &Run, steps: &[Sentence]) -> String {
    match run {
        Run::File(entrypoint) => format!("runs {}", entrypoint.path()),
        Run::Argv { program, .. } => format!("runs {program}"),
        Run::Inline if !steps.is_empty() => format!("a plan of {} steps", steps.len()),
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
    for key in [
        "description",
        "not_for",
        "tags",
        "effect",
        "confirm",
        "run",
        "platforms",
        "returns",
    ] {
        let Some(value) = manifest.get(key) else {
            continue;
        };
        let empty = value.as_array().is_some_and(Vec::is_empty);
        if !empty || yours(&[key]) {
            lines.push((yours(&[key]), pair(key, value)));
        }
        // A playbook's plan follows its confirm: each sentence numbered as the plan prints its steps.
        if key == "confirm"
            && let Some(Json::Array(steps)) = manifest.get("steps")
            && !steps.is_empty()
        {
            lines.push((false, "steps".to_owned()));
            lines.extend(step_lines(steps).into_iter().map(|line| (false, line)));
        }
    }
    if let Some(Json::Object(needs)) = manifest.get("needs")
        && !needs.is_empty()
    {
        lines.push((false, String::new()));
        lines.push((false, "[needs]".to_owned()));
        for (key, value) in needs {
            lines.push((false, pair(key, value)));
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
    argument_lines(&mut lines, &manifest, &yours);
    if let Some(Json::Object(yields)) = manifest.get("yields")
        && !yields.is_empty()
    {
        lines.push((false, String::new()));
        lines.push((false, "[yields]".to_owned()));
        for (field, value) in yields {
            lines.push((false, pair(field, value)));
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

/// A playbook's steps as `show` prints them, numbered: a step that may not run ends with the field and value that
/// pick it, `when landing = "yes"`.
fn step_lines(steps: &[Json]) -> Vec<String> {
    let width = steps.len().to_string().len();
    steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let (sentence, when) = match step {
                Json::Object(table) => (
                    table.get("say").and_then(Json::as_str),
                    table.get("when").and_then(Json::as_object),
                ),
                _ => (step.as_str(), None),
            };
            let mut line = format!("  {:>width$}  {}", i + 1, sentence.unwrap_or_default());
            for (field, is) in when.into_iter().flatten() {
                let _ = write!(line, "  when {field} = {is}");
            }
            line
        })
        .collect()
}

/// The `[args.<name>]` tables of `show`: each asked argument's keys, an ask or an option marked when it is yours,
/// then each taken argument's one line.
fn argument_lines(
    lines: &mut Vec<(bool, String)>,
    manifest: &serde_json::Map<String, Json>,
    yours: &dyn Fn(&[&str]) -> bool,
) {
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
    if let Some(Json::Object(takes)) = manifest.get("takes") {
        for (name, result) in takes {
            lines.push((false, String::new()));
            lines.push((false, format!("[args.{name}]")));
            lines.push((false, pair("takes", result)));
        }
    }
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

/// The route's answers, most probable first, and the floor when the winner was under it.
fn ranking(answers: &Raw, under_floor: Option<evoke_core::Prob>) -> Text {
    let mut line = distribution(&sorted(answers, "route"), |key| match key {
        "none" => "none of them".to_owned(),
        _ => key.to_owned(),
    });
    if let Some(floor) = under_floor {
        line.push(&format!(
            " · a reflex is picked at {:.2} or more",
            floor.get()
        ));
    }
    line
}

/// Under this, a probability prints as `0.00`.
const SHOWN: f64 = 0.005;

/// A question's answers as sorted, `key p` each: the top one weighted; the ones that would print as `0.00`
/// folded into a count — but the sentinels `none` and `unstated`, which always show, since they are what the
/// answer was weighed against. `shown` writes a key as the person reads it.
fn distribution(sorted: &[(String, f64)], shown: impl Fn(&str) -> String) -> Text {
    let mut text = Text::new();
    let mut folded = 0;
    for (i, (key, p)) in sorted.iter().enumerate() {
        let sentinel = matches!(key.as_str(), "none" | "unstated");
        if i > 0 && *p < SHOWN && !sentinel {
            folded += 1;
            continue;
        }
        if i > 0 {
            text.push(" · ");
        }
        let key = shown(key);
        if i == 0 {
            text.roled(Role::Top, &key);
        } else {
            text.push(&key);
        }
        text.push(&format!(" {p:.2}"));
    }
    if folded > 0 {
        text.push(&format!(" · {folded} more under 0.01"));
    }
    text
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

fn indented(lines: Vec<Text>) -> Text {
    Text::lines(lines.into_iter().map(|line| {
        let mut indented = Text::from("  ");
        indented.append(line.hang(2));
        indented
    }))
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

/// A text in double quotes, escaped as JSON writes it.
#[must_use]
pub fn quoted(text: &str) -> String {
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
            home: Some("/Users/me".to_owned()),
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
    fn a_path_under_the_home_shows_as_tilde_and_what_an_exit_said_has_no_fix() {
        let failed = Exit::Failed(Failure {
            what: "reading /Users/me/.config/evoke/overlays/lights.toml".to_owned(),
            cause: Some("permission denied".to_owned()),
            fix: Fix::Rerun,
        });
        assert_eq!(
            exit(&failed, "evoke show", Some(&paths()))
                .unwrap()
                .to_string(),
            "  reading ~/.config/evoke/overlays/lights.toml: permission denied  →  evoke show"
        );
        assert_eq!(
            exit(&failed, "evoke show", None).unwrap().to_string(),
            "  reading /Users/me/.config/evoke/overlays/lights.toml: permission denied  →  evoke show"
        );
        assert_eq!(
            said(&failed),
            "reading /Users/me/.config/evoke/overlays/lights.toml: permission denied"
        );
        assert_eq!(
            said(&Exit::Adapter(Fault::Status { status: 429 })),
            "the adapter answered 429"
        );
        assert_eq!(said(&Exit::Ran), "");
        let home = Paths {
            home: Some("/".to_owned()),
            ..paths()
        };
        assert_eq!(home.tilde("/etc/x"), "/etc/x");
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
        let text = r#"{"input":"kill the lights in the den","outcome":"run","reflex":"lights","args":{"room":{"type":"word","word":"den"},"state":{"type":"option","key":"off"}},"call":"lights room=\"den\" state=\"off\"","effect":"write","confidence":0.85,"weakest":{"question":"lights.room","top":"den","p":0.85},"judgments":[{"question":"route","top":"lights","p":0.91},{"question":"lights.room","top":"den","p":0.85},{"question":"lights.state","top":"off","p":0.88}],"runner_up":{"reflex":"timer","route":0.02},"contenders":[{"reflex":"lights","route":0.91},{"reflex":"timer","route":0.02}],"trace":[{"adapter":"replay","questions":6,"ms":0}],"result":{"text":"den lights off"},"answers":{"route":{"lights":0.91,"timer":0.02,"none":0.07}},"proposed":[]}"#;
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
        // What the log alone keeps reads back, and never reaches the JSON line.
        let mut kept = line;
        kept.summary = Some("Turn the lights on, off or dim.".to_owned());
        kept.typed = Some("kill the lights, in the den".to_owned());
        kept.status = Some(Status::Ran);
        let again = Line::parse(&kept.log()).unwrap();
        assert_eq!(again.summary, kept.summary);
        assert_eq!(again.typed, kept.typed);
        assert_eq!(again.status, kept.status);
        assert!(!kept.json().contains("summary"));
        assert!(!kept.json().contains("became"));
    }

    #[test]
    fn a_distribution_weights_its_top_and_folds_what_prints_as_zero() {
        let answers: Raw = serde_json::from_value(serde_json::json!({
            "route": { "volume": 0.99, "wifi": 0.004, "mail": 0.0, "none": 0.0, "lock": 0.006 },
            "volume.level": { "0-10": 0.98, "unstated": 0.02 }
        }))
        .unwrap();
        let route = ranking(&answers, None);
        assert_eq!(
            route.to_string(),
            "volume 0.99 · lock 0.01 · none of them 0.00 · 2 more under 0.01"
        );
        assert_eq!(route.roles(), vec![(Role::Top, "volume")]);
        // One answer, however small, is never folded.
        let one: Raw =
            serde_json::from_value(serde_json::json!({ "route": { "lights": 0.001 } })).unwrap();
        assert_eq!(ranking(&one, None).to_string(), "lights 0.00");
        let under = ranking(&one, evoke_core::Prob::new(0.5));
        assert_eq!(
            under.to_string(),
            "lights 0.00 · a reflex is picked at 0.50 or more"
        );
    }

    /// The run decision of the log line above, as a `Chosen`.
    fn chosen() -> Chosen {
        let text = r#"{"outcome":"run","reflex":"lights","args":{"room":{"type":"word","word":"den"},"state":{"type":"option","key":"off"}},"call":"lights room=\"den\" state=\"off\"","effect":"write","confidence":0.85,"weakest":{"question":"lights.room","top":"den","p":0.85},"judgments":[{"question":"route","top":"lights","p":0.91},{"question":"lights.room","top":"den","p":0.85}],"contenders":[{"reflex":"lights","route":0.91}]}"#;
        match serde_json::from_str::<Decision>(text).unwrap() {
            Decision::Run { chosen } => chosen,
            _ => unreachable!("the line is a run"),
        }
    }

    #[test]
    fn the_run_line_weights_the_name_and_greys_the_confidence() {
        let line = running(&chosen(), &Contained::Full);
        assert_eq!(
            line.to_string(),
            "  lights room=\"den\" state=\"off\"  0.85"
        );
        assert_eq!(
            line.roles(),
            vec![(Role::Call, "lights"), (Role::Weak, "0.85")]
        );
        let partial = Contained::Partial {
            why: "Landlock ABI 2: truncation is not held (ABI 3)".to_owned(),
        };
        assert_eq!(
            running(&chosen(), &partial).to_string(),
            "  lights room=\"den\" state=\"off\"  0.85 · partly contained"
        );
        assert_eq!(
            status(&partial).unwrap().to_string(),
            "  partly contained  Landlock ABI 2: truncation is not held (ABI 3)"
        );
        assert_eq!(status(&Contained::Full), None);
        let none = Contained::None {
            why: "this kernel has no Landlock".to_owned(),
        };
        assert!(
            running(&chosen(), &none)
                .roles()
                .contains(&(Role::Warning, "not contained"))
        );
    }

    #[test]
    fn the_confirm_line_is_the_prompts_own_words_with_their_roles() {
        let chosen = chosen();
        let own = |text: &str, reason: &str| Prompt {
            own: text.to_owned(),
            reason: reason.to_owned(),
            template: Clean::line("Set the den lights off?").unwrap(),
        };
        let judged = own(
            "lights room=\"den\" state=\"off\" · write · weakest: room 0.85",
            "a write runs at 0.90 or more; \"now · later\" asks for another thing",
        );
        let line = confirming(&chosen, &judged, &Contained::Full);
        assert_eq!(
            line.to_string(),
            format!("  {}\n    {}", judged.own, judged.reason)
        );
        assert_eq!(
            line.roles(),
            vec![
                (Role::Call, "lights"),
                (Role::Effect(Effect::Write), "write"),
                (Role::Weak, "weakest: room 0.85"),
                (Role::Weak, judged.reason.as_str()),
            ]
        );
        // Under a step's number, why it waits stands under the call.
        assert_eq!(
            step(2, 12, step_confirming(&chosen, &judged, &Contained::Full)).to_string(),
            format!("   2  {}\n        {}", judged.own, judged.reason)
        );
        let unjudged = own("lights room=\"den\" state=\"off\" · write", "");
        let line = confirming(&chosen, &unjudged, &Contained::Full);
        assert_eq!(line.to_string(), format!("  {}", unjudged.own));
        assert_eq!(line.roles().len(), 2);
        let other = own("something else entirely", "");
        assert_eq!(
            confirming(&chosen, &other, &Contained::Full).to_string(),
            "  something else entirely"
        );
        assert!(
            confirming(&chosen, &other, &Contained::Full)
                .roles()
                .is_empty()
        );
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
                    needs: serde_json::from_value(
                        serde_json::json!({ "hosts": ["*"], "runs": ["hue"] }),
                    )
                    .unwrap(),
                },
                Row {
                    name: "power".to_owned(),
                    from: "./power".to_owned(),
                    effect: Some(Effect::Destructive),
                    runs: "runs power.mts".to_owned(),
                    needs: Needs::default(),
                },
                Row {
                    name: "broken".to_owned(),
                    from: "./broken".to_owned(),
                    effect: None,
                    runs: String::new(),
                    needs: Needs::default(),
                },
            ],
            Gutter::Added,
        );
        assert_eq!(
            rows.to_string(),
            "+ lights  radhi/home/lights 1.2.0  write        runs lights.mts\n          needs hosts * · runs hue\n+ power   ./power                  destructive  runs power.mts\n+ broken  ./broken"
        );
        assert_eq!(
            rows.roles(),
            vec![
                (Role::Added, "+"),
                (Role::Effect(Effect::Write), "write"),
                (Role::Weak, "needs"),
                (Role::Added, "+"),
                (Role::Effect(Effect::Destructive), "destructive"),
                (Role::Added, "+"),
            ]
        );
        assert_eq!(self::rows(&[], Gutter::Listed), Text::new());
    }

    #[test]
    fn the_help_fits_eighty_columns_and_names_every_group() {
        let help = help("0.1.0", None);
        assert!(help.starts_with("evoke 0.1.0 · "));
        for group in [
            "\nuse\n",
            "\ninstall\n",
            "\ntune\n",
            "\nauthor\n",
            "\nexit    0 ran",
            "\nmanual  https://evoke.build/manual/",
        ] {
            assert!(help.contains(group), "{group:?} is missing");
        }
        let widest = help.lines().map(chars).max().unwrap_or(0);
        assert!(widest <= 80, "a help line is {widest} columns wide");
        assert!(!help.ends_with('\n'));
    }

    #[test]
    fn a_narrow_terminal_gets_the_descriptions_under_their_lines() {
        let wide = help("0.1.0", None);
        assert_eq!(help("0.1.0", Some(80)), wide);
        assert_eq!(help("0.1.0", Some(200)), wide);
        let narrow = help("0.1.0", Some(60));
        assert_ne!(narrow, wide);
        assert!(narrow.contains("\n  evoke \"<input>\"\n      decide, gate, run\n"));
        assert!(narrow.contains("\n    --json\n      one JSON line per input, for a filter\n"));
        // One more line per described command or flag, and the exit codes on two.
        assert_eq!(narrow.lines().count(), wide.lines().count() + 30);
        let widest = narrow.lines().map(chars).max().unwrap_or(0);
        assert!(widest <= 60, "a line is {widest} columns wide");
    }

    #[test]
    fn a_write_is_signed_and_a_label_warns() {
        let removed = written("vocab/rooms.toml", &Landed::Removed("attic".to_owned()));
        assert_eq!(removed.to_string(), "- vocab/rooms.toml  attic");
        assert_eq!(removed.roles(), vec![(Role::Removed, "-")]);
        let whole = written("~/month.plan.json", &Landed::Whole);
        assert_eq!(whole.to_string(), "+ ~/month.plan.json");
        assert_eq!(whole.roles(), vec![(Role::Added, "+")]);
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
