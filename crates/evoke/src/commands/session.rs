//! One invocation's ground, prepared once: the project located, snapshotted and — outside home — checked against
//! its trust, parsed by the core with its lock, its remote reflexes read from the store, its adapter's declaration
//! read, its plan compiled; the state and the store; the terminal, opened on first need. On it, each input is
//! decided — asked, answered from the cache or the adapter, or fresh for `test`, read and gated — a chosen call is
//! run under its declaration, the layers around it, an edit lands in an owned file, the lock and `evoke.d.ts` are
//! written whole, each write of an owned file re-blessing the trust. In: the environment and the command. Out: a
//! `Session`, then a `Decided` per input, a body's `Returned`, an `Edited` file. `open` prints every problem with
//! its fix; every other method leaves its `Exit` to the command to report, once.

use std::path::{Path, PathBuf};
use std::thread;

use evoke_core::decide::Recent;
use evoke_core::document::{Json, Text};
use evoke_core::manifest::{Effect, Run};
use evoke_core::name::{AdapterId, ArgName, ConfigKey, LocalName, Tag};
use evoke_core::needs::{self, Origin};
use evoke_core::plan::{Held, Millis};
use evoke_core::project::{Location, Locked, LockedAdapter, Setting};
use evoke_core::text::NonEmpty;
use evoke_core::weave::{self, Asked, Need, Repair, Step};
use evoke_core::{
    Answer, Chosen, Contained, Decision, Declared, Diagnostic, Document, Edit, Effective, File,
    Fix, Input, Installed, Item, Lesson, Lock, Manifest, Needs, Owned, Plan, Planning, Project,
    Prompt, Proposed, Raw, Replanned, Request, Scope, Values, Version, Weave, argv, compile,
    effective, envelope, gate, lock, manifest, overlay, project, project_dts, reading, render_lock,
    request, resolve, validated, vocabulary,
};
use indexmap::IndexMap;

use super::{Exit, Reporter};
use crate::adapter::{self, Adapter, Trace};
use crate::args::Command;
use crate::hosts::files::{self, Edited, Root, Snapshot};
use crate::hosts::processes::{self, Body, Ended, Returned};
use crate::hosts::state::State;
use crate::hosts::store::Store;
use crate::hosts::terminal::{self, Tty};
use crate::hosts::{Deadline, Environment, Failure};
use crate::hosts::{clock, contain, interrupt, threads};
use crate::report::{self, Paths, Row};

/// A body that gave no result: what failed and its fix, and the frames of an error a file body threw, none for
/// a program or a body that gave no line.
#[derive(Clone, Debug, PartialEq)]
pub struct Failed {
    pub failure: Failure,
    pub frames: Vec<String>,
}

impl From<Failure> for Failed {
    /// A failure before the body ran, or a program's: no frames.
    fn from(failure: Failure) -> Self {
        Self {
            failure,
            frames: Vec::new(),
        }
    }
}

pub struct Session<'a> {
    pub root: Root,
    pub project: Project,
    pub lock: Option<Lock>,
    pub installed: Installed,
    pub plan: Plan,
    pub declared: Declared,
    pub state: State,
    pub store: Store,
    pub reporter: Reporter<'a>,
    /// Each reflex's shipped manifest and directory — a local one's as written, a remote one's in the store: what
    /// an overlay re-parses against, where a body lives.
    pub shipped: IndexMap<LocalName, (Manifest, PathBuf)>,
    /// Whether this machine holds a whole declaration, probed once.
    pub contained: Contained,
    environment: &'a Environment,
    tty: Option<Tty>,
}

/// What a command needs of the session: to decide, where an inactive reflex is left out and nothing active at all
/// is the stop; to tune, where the inactive ones are the command's own business; or to install, where a remote
/// reflex the store cannot place is inactive rather than refused, since the command is about to place it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opening {
    Deciding,
    Tuning,
    Installing,
    /// `show`: what is installed, placed or not — a remote reflex the store lacks is listed, inactive.
    Listing,
}

/// What the confirm prompt read.
pub enum Confirmed {
    Yes,
    No,
    Teach,
}

/// One input decided: the input as decided and its candidates, what was answered, the outcome, and the adapter
/// calls it took — what a line is rendered from.
#[derive(Clone)]
pub struct Decided {
    pub input: Input,
    pub proposed: Vec<Proposed>,
    pub answers: Raw,
    pub decision: Decision,
    pub trace: Vec<Trace>,
}

impl Decided {
    /// The adapter's share of the plan's deadline, spent before any prompt; nothing when the cache answered.
    #[must_use]
    pub fn spent(&self) -> Millis {
        Millis(
            self.trace
                .iter()
                .fold(0, |spent, trace| spent.saturating_add(trace.ms)),
        )
    }
}

/// One request read into its steps: the plan, what each text decided — the input, the answers, the decision —
/// for the lines a host prints and logs, and the weave's own answers, for a plan file to keep.
pub struct Woven {
    pub weave: Weave,
    pub decided: Vec<(Asked, Decided)>,
    /// Every adapter call the plan took: the weave's own questions, then each text decided, in order.
    pub trace: Vec<Trace>,
    /// How many rounds the adapter was asked in: texts decided side by side are asked in the same rounds.
    pub rounds: usize,
    /// The engine's answers to the split points, when it was asked.
    pub judged: Option<Raw>,
    /// The engine's answers to which earlier step each reference names, when it was asked.
    pub referred: Option<Raw>,
    /// The engine's answers to what the plan asked of the request once its parts were decided.
    pub verified: Option<Raw>,
}

impl Woven {
    /// A plan made again from a file: what the core decided from the file's answers, no adapter called; of what the
    /// plan asked of the request, the answers its steps written again stand on, as the file keeps them.
    #[must_use]
    pub fn replanned(replanned: Replanned, answers: &[Answer]) -> Self {
        let own: Vec<&Raw> = answers
            .iter()
            .filter(|answer| answer.text == replanned.weave.input)
            .map(|answer| &answer.raw)
            .collect();
        let verified = again_among(&replanned.weave, &own);
        let decided = replanned
            .decided
            .into_iter()
            .map(|decided| {
                let line = Decided {
                    input: Input::new(&decided.asked.text).expect("the core decided the text"),
                    proposed: decided.proposed,
                    answers: decided.raw,
                    decision: decided.decision,
                    trace: Vec::new(),
                };
                (decided.asked, line)
            })
            .collect();
        Self {
            weave: replanned.weave,
            decided,
            trace: Vec::new(),
            rounds: 0,
            judged: None,
            referred: None,
            verified,
        }
    }

    /// Every engine answer the plan took, as a plan file keeps them: the weave's own questions under the
    /// request's text, then each text decided, in order.
    #[must_use]
    pub fn answers(&self) -> Vec<Answer> {
        self.judged
            .iter()
            .chain(&self.referred)
            .chain(&self.verified)
            .map(|raw| Answer {
                text: self.weave.input.clone(),
                raw: raw.clone(),
            })
            .chain(self.decided.iter().map(|(asked, decided)| Answer {
                text: asked.text.clone(),
                raw: decided.answers.clone(),
            }))
            .collect()
    }

    /// A step as a host shows or runs it: its words' request, answers and reading, under the decision the plan
    /// holds for the step, caps included — a part merged back confirms wherever the step is shown. The run puts
    /// the plan's cap on again at the turn, to a decision filled or rewritten there.
    #[must_use]
    pub fn planned(&self, step: &Step, tags: &[Tag]) -> Option<Decided> {
        self.decided_for(step, tags).map(|decided| Decided {
            decision: step.decision.clone(),
            ..decided.clone()
        })
    }

    /// What the plan asked of the request that its steps written again stand on (`again_among`).
    #[must_use]
    pub fn again(&self) -> Option<Raw> {
        let verified: Vec<&Raw> = self.verified.iter().collect();
        again_among(&self.weave, &verified)
    }

    /// The one decision a request read as: one step, and so nothing bound — what the foundation alone would have
    /// made of it, under the plan's cap. A part left out beside it changes nothing: what was said not to do is
    /// no step. A step that takes a whole result no step hands is no decision but the plan's refusal; nor is a
    /// playbook's step, which asks its slot up front and then expands, never runs as a body.
    #[must_use]
    pub fn single(&self, tags: &[Tag], plan: &Plan) -> Option<Decided> {
        if self.weave.verdict.because.iter().any(|because| {
            matches!(
                because,
                weave::Because::NoSource { .. } | weave::Because::SeveralSources { .. }
            )
        }) {
            return None;
        }
        match self.weave.steps.as_slice() {
            [step] => {
                let playbook = step
                    .reflex
                    .as_ref()
                    .and_then(|reflex| plan.active().get(reflex))
                    .is_some_and(|active| !active.steps.is_empty());
                if playbook || !step.from.is_empty() {
                    return None;
                }
                self.planned(step, tags)
            }
            _ => None,
        }
    }

    /// What a step's words decided, as the planner asked for them: the text's own word, before the plan's.
    fn decided_for(&self, step: &Step, tags: &[Tag]) -> Option<&Decided> {
        let asked = self.weave.asked_for(step, tags);
        self.decided
            .iter()
            .find(|(a, _)| *a == asked)
            .map(|(_, decided)| decided)
    }
}

/// For each step of a plan written again for another value, whether the request asks the same for its value as
/// well: the answers among `raws`, the plan's own about the request, by the question's id; none where no step was.
fn again_among(weave: &Weave, raws: &[&Raw]) -> Option<Raw> {
    let again: IndexMap<String, IndexMap<String, f64>> = weave
        .steps
        .iter()
        .filter(|step| step.repair == Some(Repair::Again))
        .filter_map(|step| weave::named(&step.decision))
        .filter_map(|judgment| {
            let id = judgment.question.to_string();
            let answer = raws.iter().find_map(|raw| raw.0.get(&id))?;
            Some((id, answer.clone()))
        })
        .collect();
    (!again.is_empty()).then_some(Raw(again))
}

/// The texts sent ahead of the cut, each decided on its own thread from the cut's round on.
#[derive(Default)]
struct Ahead<'scope> {
    started: Vec<(
        Asked,
        thread::ScopedJoinHandle<'scope, Result<Decided, Exit>>,
    )>,
}

impl Ahead<'_> {
    /// A text's decision, when it was started ahead: waited for, as it stands.
    fn take(&mut self, asked: &Asked) -> Option<Result<Decided, Exit>> {
        let at = self
            .started
            .iter()
            .position(|(started, _)| started == asked)?;
        let (_, handle) = self.started.remove(at);
        Some(joined(handle))
    }

    /// Every text still running, waited for: the plan wanted none of them, and the plan file keeps them.
    fn rest(self) -> Vec<(Asked, Result<Decided, Exit>)> {
        self.started
            .into_iter()
            .map(|(asked, handle)| (asked, joined(handle)))
            .collect()
    }
}

/// A thread's result; a panic on it is one on this thread.
fn joined<T>(handle: thread::ScopedJoinHandle<'_, T>) -> T {
    handle
        .join()
        .unwrap_or_else(|panicked| std::panic::resume_unwind(panicked))
}

/// The owned files as parsed and compiled.
struct Prepared {
    project: Project,
    lock: Option<Lock>,
    installed: Installed,
    plan: Plan,
    declared: Declared,
    shipped: IndexMap<LocalName, (Manifest, PathBuf)>,
}

/// What `prepare` reads from: the root and its snapshot, the store, the environment, and why the session opens.
struct Ground<'a> {
    root: &'a Root,
    snapshot: &'a Snapshot,
    store: &'a Store,
    environment: &'a Environment,
    opening: Opening,
}

/// The straight sequence up to the plan: locate, first use, the state and the store, the snapshot, trust, then
/// `prepare`; when deciding, nothing active is a stop that names every problem, and an inactive reflex beside
/// active ones is left out — `show` lists why, and an abstain names it. The home project is written on first use
/// and trusted by construction; any other root must be blessed at its content.
pub fn open<'a>(
    command: &'a Command,
    json: bool,
    environment: &'a Environment,
    opening: Opening,
) -> Result<Session<'a>, Exit> {
    let input = command.stand_in();
    let mut reporter = Reporter {
        command,
        json,
        paths: Paths::of(environment),
    };
    let root = files::locate(environment)
        .map_err(|failure| reporter.exit(&input, Exit::Failed(failure)))?;
    reporter.paths.root = files::shown(&root.path, environment);
    files::write_home(&root).map_err(|failure| reporter.exit(&input, Exit::Failed(failure)))?;
    let state =
        State::of(environment).map_err(|failure| reporter.exit(&input, Exit::Failed(failure)))?;
    let store =
        Store::of(environment).map_err(|failure| reporter.exit(&input, Exit::Failed(failure)))?;
    let snapshot =
        files::snapshot(&root).map_err(|failure| reporter.exit(&input, Exit::Failed(failure)))?;
    still_trusted(&state, &root, &snapshot, &reporter.paths.root)
        .map_err(|exit| reporter.exit(&input, exit))?;
    let ground = Ground {
        root: &root,
        snapshot: &snapshot,
        store: &store,
        environment,
        opening,
    };
    let prepared =
        prepare(&ground, &mut reporter, &input).map_err(|exit| reporter.exit(&input, exit))?;
    if opening == Opening::Deciding && prepared.plan.active().is_empty() {
        let problems: Vec<Diagnostic> = prepared
            .plan
            .inactive()
            .values()
            .flat_map(NonEmpty::iter)
            .cloned()
            .collect();
        if !problems.is_empty() {
            return Err(reporter.problems(&input, problems));
        }
    }
    Ok(Session {
        root,
        project: prepared.project,
        lock: prepared.lock,
        installed: prepared.installed,
        plan: prepared.plan,
        declared: prepared.declared,
        state,
        store,
        reporter,
        shipped: prepared.shipped,
        contained: contain::status(),
        environment,
        tty: None,
    })
}

/// From the owned files to the plan: project, lock, the adapter's declaration, the installed set, compile. Every
/// problem but the last is printed; the last is the exit.
fn prepare(
    ground: &Ground<'_>,
    reporter: &mut Reporter<'_>,
    input: &str,
) -> Result<Prepared, Exit> {
    let snapshot = ground.snapshot;
    let project = project(Document {
        file: File::Project,
        text: Text::Toml(&snapshot.project),
    })
    .map_err(|problems| reporter.human(input, problems))?;
    let lock = snapshot
        .lock
        .as_deref()
        .map(|text| {
            lock(Document {
                file: File::Lock,
                text: Text::Toml(text),
            })
        })
        .transpose()
        .map_err(|problems| reporter.human(input, problems))?;
    let declared = adapter::declared(
        &project.adapter,
        project.adapters.get(&project.adapter),
        ground.environment,
    )
    .map_err(|problems| reporter.human(input, problems))?;
    let (installed, shipped) = installed(ground, &project, lock.as_ref(), declared.id.clone())?;
    reporter.paths.reflexes = project
        .reflexes
        .iter()
        .map(|(name, location)| {
            let shown = match location {
                Location::Local { path } => {
                    local_shown(&ground.root.path, path, ground.environment)
                }
                Location::Remote { .. } => {
                    shipped.get(name).map_or_else(String::new, |(_, dir)| {
                        files::shown(dir, ground.environment)
                    })
                }
            };
            (name.clone(), shown)
        })
        .collect();
    let platform = contain::judged(ground.environment).map_err(Exit::Human)?;
    let plan = compile(
        &installed,
        &plain_values(&project),
        declared.limits.as_ref(),
        Some(platform),
    )
    .map_err(Exit::Human)?;
    Ok(Prepared {
        project,
        lock,
        installed,
        plan,
        declared,
        shipped,
    })
}

/// The installed set as the host found it: every reflex `evoke.toml` names with its overlay and configuration —
/// a local one under its path, a remote one in the store by the lock's digest — and every vocabulary; beside it,
/// each shipped manifest with its directory.
#[expect(clippy::type_complexity)]
fn installed(
    ground: &Ground<'_>,
    project: &Project,
    lock: Option<&Lock>,
    adapter: AdapterId,
) -> Result<(Installed, IndexMap<LocalName, (Manifest, PathBuf)>), Exit> {
    let snapshot = ground.snapshot;
    let mut reflexes = IndexMap::new();
    let mut shipped = IndexMap::new();
    for (name, location) in &project.reflexes {
        let configured = project
            .config
            .get(name)
            .map(|settings| held(settings, ground.environment))
            .unwrap_or_default();
        let (item, manifest) = match location {
            Location::Remote { .. } => {
                let locked = lock.and_then(|lock| lock.reflexes.get(name));
                remote(ground, name, location, locked, configured)?
            }
            Location::Local { path } => {
                // Joined as written, then normalized: `~/dev/./hello` is `~/dev/hello` wherever it prints.
                let dir: PathBuf = ground.root.path.join(path).components().collect();
                let (item, manifest) =
                    local(&dir, name, path, snapshot, configured).map_err(Exit::Failed)?;
                (item, manifest.map(|manifest| (manifest, dir)))
            }
        };
        if let Some(manifest) = manifest {
            shipped.insert(name.clone(), manifest);
        }
        reflexes.insert(name.clone(), item);
    }
    let mut vocab = IndexMap::new();
    for (name, text) in &snapshot.vocab {
        let words = vocabulary(Document {
            file: File::Vocab { name: name.clone() },
            text: Text::Toml(text),
        })
        .map_err(|problems| {
            let last = problems
                .last()
                .cloned()
                .expect("the core names every problem");
            Exit::Human(last)
        })?;
        vocab.insert(name.clone(), words);
    }
    Ok((
        Installed {
            reflexes,
            vocab,
            adapter,
            evoke: Version::parse(crate::VERSION).expect("the crate version is X.Y.Z"),
        },
        shipped,
    ))
}

/// A local reflex's directory as a person reads it: as written while it stays under the project, `./lights`;
/// resolved and shown under the home, `~/hello`, when it climbs out.
pub(super) fn local_shown(root: &Path, path: &str, environment: &Environment) -> String {
    if Path::new(path)
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        files::shown(&files::normalised(&root.join(path)), environment)
    } else {
        path.to_owned()
    }
}

/// A local reflex as found: its manifest under the path as written, its overlay when there is one.
fn local(
    dir: &Path,
    name: &LocalName,
    path: &str,
    snapshot: &Snapshot,
    configured: IndexMap<ConfigKey, Held>,
) -> Result<(Item, Option<Manifest>), Failure> {
    let (wording, shipped) = match files::read(&dir.join("reflex.toml"))? {
        None => (
            Err(vec![Diagnostic {
                reflex: Some(name.clone()),
                at: None,
                message: format!("{path}/reflex.toml is missing"),
                fix: Fix::Remove {
                    reflex: name.clone(),
                },
            }]),
            None,
        ),
        Some(text) => worded(name, &text, snapshot),
    };
    // A local reflex consents to its own effect and needs; one without a manifest is inactive, and the tightest
    // is as good.
    let (consented, needs) = wording.as_ref().map_or_else(
        |_| (Effect::Destructive, Needs::default()),
        |effective| (effective.manifest.effect, effective.manifest.needs.clone()),
    );
    Ok((
        Item {
            wording,
            consented,
            needs,
            configured,
        },
        shipped,
    ))
}

/// A remote reflex as found: its manifest from the store entry the lock names, its overlay when there is one; it
/// consents to the lock's effect. Unlocked, or not in the store, it is refused — inactive while installing.
fn remote(
    ground: &Ground<'_>,
    name: &LocalName,
    location: &Location,
    locked: Option<&Locked>,
    configured: IndexMap<ConfigKey, Held>,
) -> Result<(Item, Option<(Manifest, PathBuf)>), Exit> {
    let Some(locked) = locked else {
        let problem = Diagnostic {
            reflex: Some(name.clone()),
            at: None,
            message: "is not locked".to_owned(),
            fix: Fix::AddRef {
                reference: location.to_string(),
                name: Some(name.clone()),
            },
        };
        return unplaced(ground.opening, configured, problem);
    };
    let Some(entry) = ground.store.entry(&locked.h1).map_err(Exit::Failed)? else {
        let problem = Diagnostic {
            reflex: Some(name.clone()),
            at: None,
            message: "is not in the store".to_owned(),
            fix: Fix::Sync,
        };
        return unplaced(ground.opening, configured, problem);
    };
    let text = entry
        .files
        .iter()
        .find(|(path, _)| path.as_str() == "reflex.toml")
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned());
    let (wording, shipped) = match text {
        Some(text) => worded(name, &text, ground.snapshot),
        None => (
            Err(vec![Diagnostic {
                reflex: Some(name.clone()),
                at: None,
                message: "has no reflex.toml in the store".to_owned(),
                fix: Fix::Sync,
            }]),
            None,
        ),
    };
    Ok((
        Item {
            wording,
            consented: locked.effect,
            needs: locked.needs.clone(),
            configured,
        },
        shipped.map(|manifest| (manifest, entry.dir)),
    ))
}

/// A remote reflex the store cannot place: inactive with the problem while installing or listing, refused
/// otherwise.
fn unplaced(
    opening: Opening,
    configured: IndexMap<ConfigKey, Held>,
    problem: Diagnostic,
) -> Result<(Item, Option<(Manifest, PathBuf)>), Exit> {
    if matches!(opening, Opening::Installing | Opening::Listing) {
        Ok((
            Item {
                wording: Err(vec![problem]),
                consented: Effect::Destructive,
                needs: Needs::default(),
                configured,
            },
            None,
        ))
    } else {
        Err(Exit::Human(problem))
    }
}

/// A shipped manifest's text read with the overlay named for it: the effective wording or the problems, and the
/// manifest itself when it parsed.
fn worded(
    name: &LocalName,
    text: &str,
    snapshot: &Snapshot,
) -> (Result<Effective, Vec<Diagnostic>>, Option<Manifest>) {
    let mut shipped = None;
    let wording = manifest(Document {
        file: File::Manifest { name: name.clone() },
        text: Text::Toml(text),
    })
    .and_then(|manifest| {
        let yours = snapshot
            .overlays
            .iter()
            .find(|(overlay, _)| overlay == name)
            .map(|(_, text)| {
                overlay(
                    Document {
                        file: File::Overlay { name: name.clone() },
                        text: Text::Toml(text),
                    },
                    &manifest,
                )
            })
            .transpose()?;
        let effective = effective(&manifest, yours.as_ref());
        shipped = Some(manifest);
        Ok(effective)
    });
    (wording, shipped)
}

/// How each configured key is held: plain, or a variable and whether it is set — never a value, never a secret.
fn held(
    settings: &IndexMap<ConfigKey, Setting>,
    environment: &Environment,
) -> IndexMap<ConfigKey, Held> {
    settings
        .iter()
        .map(|(key, setting)| {
            let held = match setting {
                Setting::Plain { .. } => Held::Plain,
                Setting::Env { var } => Held::Env {
                    var: var.clone(),
                    set: environment.get(var.as_str()).is_some(),
                },
            };
            (key.clone(), held)
        })
        .collect()
}

/// The plain settings' values, per reflex and key, as `evoke.toml` holds them: handed to `compile` beside the set,
/// so the digest holds no value.
#[must_use]
pub fn plain_values(project: &Project) -> Values {
    project
        .config
        .iter()
        .map(|(name, settings)| {
            let plain = settings
                .iter()
                .filter_map(|(key, setting)| match setting {
                    Setting::Plain { value } => Some((key.clone(), value.clone())),
                    Setting::Env { .. } => None,
                })
                .collect::<IndexMap<ConfigKey, String>>();
            (name.clone(), plain)
        })
        .filter(|(_, plain)| !plain.is_empty())
        .collect()
}

impl Session<'_> {
    /// The adapter ready to answer: resolved with its credential or recording, and asked whether it accepts the
    /// plan.
    pub fn adapter(&self, input: &str) -> Result<Box<dyn Adapter>, Exit> {
        let adapter = adapter::resolve(
            &self.project.adapter,
            self.project.adapters.get(&self.project.adapter),
            self.environment,
        )
        .map_err(|problems| self.reporter.human(input, problems))?;
        adapter.accepts(&self.plan.digest()).map_err(Exit::Human)?;
        Ok(adapter)
    }

    /// The owned files read again after a write of the session's own — a word added at a prompt, a reflex
    /// installed — so the plan holds what they now say.
    pub fn reload(&mut self, input: &str) -> Result<(), Exit> {
        let snapshot = files::snapshot(&self.root).map_err(Exit::Failed)?;
        let ground = Ground {
            root: &self.root,
            snapshot: &snapshot,
            store: &self.store,
            environment: self.environment,
            opening: Opening::Installing,
        };
        let prepared = prepare(&ground, &mut self.reporter, input)?;
        self.project = prepared.project;
        self.lock = prepared.lock;
        self.installed = prepared.installed;
        self.plan = prepared.plan;
        self.declared = prepared.declared;
        self.shipped = prepared.shipped;
        Ok(())
    }

    #[must_use]
    /// Whether the reflex's examples already hold the utterance with the lesson's record — shipped or yours — so
    /// teaching it again would write nothing new.
    pub fn taught_already(&self, lesson: &Lesson, text: &str) -> bool {
        self.installed
            .reflexes
            .get(&lesson.reflex)
            .and_then(|item| item.wording.as_ref().ok())
            .and_then(|effective| effective.manifest.examples.get(&evoke_core::identity(text)))
            .is_some_and(|(_, record)| *record == lesson.record)
    }

    pub fn environment(&self) -> &Environment {
        self.environment
    }

    /// How a reflex's settings are held, as the plan sees them.
    #[must_use]
    pub fn configured(&self, name: &LocalName) -> IndexMap<ConfigKey, Held> {
        self.project
            .config
            .get(name)
            .map(|settings| held(settings, self.environment))
            .unwrap_or_default()
    }

    /// The lock as this session would start one: the tool's version and the adapter that decides.
    #[must_use]
    pub fn new_lock(&self) -> Lock {
        Lock {
            evoke: self.installed.evoke,
            adapter: self.locked_adapter(),
            reflexes: IndexMap::new(),
        }
    }

    #[must_use]
    pub fn locked_adapter(&self) -> LockedAdapter {
        LockedAdapter {
            name: self.project.adapter.clone(),
            id: self.installed.adapter.clone(),
        }
    }

    /// The named reflexes as `show`, `add`, `remove` and `sync` list them: where each comes from with its locked
    /// version, the effect it runs under, what it runs.
    pub fn rows<'n>(&self, names: impl IntoIterator<Item = &'n LocalName>) -> Vec<Row> {
        names
            .into_iter()
            .filter_map(|name| {
                let location = self.project.reflexes.get(name)?;
                // A pinned ref already names its tag; an unpinned one shows the tag the lock holds.
                let from = match location {
                    Location::Local { path } => {
                        local_shown(&self.root.path, path, self.environment)
                    }
                    Location::Remote { pin, .. } => {
                        match self.lock.as_ref().and_then(|lock| lock.reflexes.get(name)) {
                            Some(locked) if *pin != Some(locked.tag) => {
                                format!("{location} {}", locked.tag)
                            }
                            _ => location.to_string(),
                        }
                    }
                };
                let (effect, runs, needs) = self
                    .installed
                    .reflexes
                    .get(name)
                    .and_then(|item| {
                        item.wording.as_ref().ok().map(|effective| {
                            (
                                Some(effective.manifest.effect.max(item.consented)),
                                report::runs(&effective.manifest.run, &effective.manifest.steps),
                                needs::narrowed(&effective.manifest.needs, &item.needs),
                            )
                        })
                    })
                    .unwrap_or_default();
                Some(Row {
                    name: name.to_string(),
                    from,
                    effect,
                    runs,
                    needs,
                })
            })
            .collect()
    }

    /// Every problem of the named reflexes, each with its fix.
    pub fn inactive_of<'n>(
        &self,
        names: impl IntoIterator<Item = &'n LocalName>,
    ) -> Vec<Diagnostic> {
        names
            .into_iter()
            .filter_map(|name| self.plan.inactive().get(name))
            .flat_map(|problems| problems.iter().cloned())
            .collect()
    }

    /// The overlay's text for a reflex, when there is one.
    pub fn overlay_text(&self, name: &LocalName) -> Result<Option<String>, Exit> {
        files::read(&self.root.path.join(format!("overlays/{name}.toml"))).map_err(Exit::Failed)
    }

    /// The runtime found on `PATH` and recorded as the binary it runs as, when an installed reflex runs a file;
    /// none found and none recorded is returned as the problem to end on.
    pub fn record_runtime(&self) -> Result<Option<Diagnostic>, Exit> {
        let files = self.installed.reflexes.values().any(|item| {
            item.wording
                .as_ref()
                .is_ok_and(|effective| matches!(effective.manifest.run, Run::File(_)))
        });
        if !files {
            return Ok(None);
        }
        if let Some(found) = processes::find_runtime(self.environment) {
            let real = processes::real_runtime(&found, self.environment).map_err(Exit::Failed)?;
            self.state.set_runtime(&real).map_err(Exit::Failed)?;
            return Ok(None);
        }
        if self.state.runtime().map_err(Exit::Failed)?.is_some() {
            return Ok(None);
        }
        Ok(Some(Diagnostic {
            reflex: None,
            at: None,
            message: "node is not on PATH; a .mts reflex needs Node 24 or newer".to_owned(),
            fix: Fix::Sync,
        }))
    }

    /// The lock written whole, then trust re-blessed.
    pub fn write_lock(&self, lock: &Lock) -> Result<(), Exit> {
        self.write_owned(&self.root.path.join("evoke.lock"), &render_lock(lock))
    }

    /// `evoke.d.ts` rendered whole from the installed set, silently, after every write that changes the set; it
    /// is generated, so trust does not cover it.
    pub fn write_types(&self) -> Result<(), Exit> {
        files::write(
            &self.root.path.join("evoke.d.ts"),
            &project_dts(&self.installed),
        )
        .map_err(Exit::Failed)
    }

    /// An owned file written — outside home, only while the root is still what was blessed, so a change made
    /// beside a running session is a stop and never adopted — then the trust entry refreshed to what the owned
    /// files now say, never made.
    fn write_owned(&self, path: &Path, text: &str) -> Result<(), Exit> {
        if self.root.home {
            return files::write(path, text).map_err(Exit::Failed);
        }
        let snapshot = files::snapshot(&self.root).map_err(Exit::Failed)?;
        still_trusted(
            &self.state,
            &self.root,
            &snapshot,
            &self.reporter.paths.root,
        )?;
        files::write(path, text).map_err(Exit::Failed)?;
        let snapshot = files::snapshot(&self.root).map_err(Exit::Failed)?;
        self.state
            .rebless(&self.root.path, &files::digest_of(&snapshot))
            .map_err(Exit::Failed)
    }

    /// One input: request → the cache, else the adapter → read → gate, a spinner turning while the adapter
    /// answers. The cache is keyed by the plan, the utterance and the questions asked, so a decision narrowed
    /// with `--tag` has an entry of its own; an entry that no longer reads against the request is a miss.
    pub fn decide(&self, adapter: &dyn Adapter, asked: &Asked) -> Result<Decided, Exit> {
        let _busy = terminal::busy("deciding");
        self.decided(adapter, asked, true, &[])
    }

    /// One request read into its steps: the engine asked whether each connective separates two things, each
    /// part decided as `decide` decides one, all through the cache. `seeded` decisions stand in for the
    /// planner's own — a step's ask answered before anything runs — and are read first. `recent` is the
    /// process's results, newest first, offered back at the ask of a whole sentence alone.
    pub fn weave(
        &self,
        adapter: &dyn Adapter,
        input: &str,
        tags: &[Tag],
        seeded: Vec<(Asked, Decided)>,
        recent: &[Recent],
    ) -> Result<Woven, Exit> {
        let _busy = terminal::busy("deciding");
        let mut answers = weave::Answers {
            decided: seeded
                .iter()
                .map(|(asked, decided)| (asked.clone(), decided.decision.clone()))
                .collect(),
            ..weave::Answers::default()
        };
        let mut decided = seeded;
        let mut trace = Vec::new();
        let mut rounds = 0;
        // A text sent ahead of the cut is decided on its own thread from the cut's round on, and taken when the
        // plan asks for it, or at the end, so that the plan file holds its answers whatever the plan made of it.
        thread::scope(|scope| {
            let mut ahead = Ahead::default();
            loop {
                let gate = adapter.declared().gate.as_ref();
                let planning = weave::planning::plan(&self.plan, gate, input, tags, &answers)
                    .map_err(Exit::Adapter)?;
                let before = trace.len();
                let need = match planning {
                    Planning::Done { weave } => {
                        for (asked, one) in ahead.rest() {
                            let one = one?;
                            trace.extend(one.trace.iter().cloned());
                            decided.push((asked, one));
                        }
                        return Ok(Woven {
                            weave,
                            decided,
                            trace,
                            rounds,
                            judged: answers.judged,
                            referred: answers.referred,
                            verified: answers.verified,
                        });
                    }
                    Planning::Need { need } => need,
                };
                match need {
                    Need::Judge {
                        request,
                        ahead: texts,
                    } => {
                        for asked in texts {
                            let started = asked.clone();
                            let handle = scope.spawn(move || self.part(adapter, &started, recent));
                            ahead.started.push((asked, handle));
                        }
                        answers.judged = Some(self.own(adapter, &request, &mut trace)?);
                    }
                    Need::Refer { request } => {
                        answers.referred = Some(self.own(adapter, &request, &mut trace)?);
                    }
                    Need::Verify { request } => {
                        let raw = self.own(adapter, &request, &mut trace)?;
                        answers.verified.get_or_insert_default().0.extend(raw.0);
                    }
                    Need::Decide { asked } => {
                        let round = self.round(adapter, &asked, &mut ahead, recent)?;
                        // Each text's calls come one after another; the texts' calls, side by side; a text
                        // started ahead had its first round beside the cut's.
                        rounds += round
                            .iter()
                            .map(|(one, early)| one.trace.len().saturating_sub(usize::from(*early)))
                            .max()
                            .unwrap_or(0);
                        for (asked, (one, _)) in asked.into_iter().zip(round) {
                            trace.extend(one.trace.iter().cloned());
                            answers.decided.push((asked.clone(), one.decision.clone()));
                            decided.push((asked, one));
                        }
                        continue;
                    }
                }
                // One of the plan's own requests: a round where the cache did not answer it.
                rounds += trace.len() - before;
            }
        })
    }

    /// One text of a plan decided as the plan asks. Memory reaches a whole sentence alone: a part of one, or a
    /// step's words rewritten, recalls nothing.
    fn part(
        &self,
        adapter: &dyn Adapter,
        asked: &Asked,
        recent: &[Recent],
    ) -> Result<Decided, Exit> {
        let recalled = if asked.whole { recent } else { &[] };
        self.decided(adapter, asked, true, recalled)
    }

    /// The texts of one round decided side by side, in the order asked; a text started ahead of the cut is
    /// taken as it stands, and marked so.
    fn round(
        &self,
        adapter: &dyn Adapter,
        asked: &[Asked],
        ahead: &mut Ahead<'_>,
        recent: &[Recent],
    ) -> Result<Vec<(Decided, bool)>, Exit> {
        let mut taken: Vec<Option<(Decided, bool)>> = asked.iter().map(|_| None).collect();
        let mut rest: Vec<(usize, Asked)> = Vec::new();
        for (i, one) in asked.iter().enumerate() {
            match ahead.take(one) {
                Some(early) => taken[i] = Some((early?, true)),
                None => rest.push((i, one.clone())),
            }
        }
        let decided = threads::try_each(&rest, |(_, asked)| self.part(adapter, asked, recent))?;
        for ((i, _), one) in rest.into_iter().zip(decided) {
            taken[i] = Some((one, false));
        }
        Ok(taken
            .into_iter()
            .map(|one| one.expect("every text asked is decided"))
            .collect())
    }

    /// One of the weave's own requests answered — the split points, the references: the cache when it holds
    /// every question asked, else the adapter, timed and kept.
    fn own(
        &self,
        adapter: &dyn Adapter,
        request: &Request,
        trace: &mut Vec<Trace>,
    ) -> Result<Raw, Exit> {
        // A hit that does not validate is a miss, and an answer is kept only once it validates, as `decided`
        // does through `read`: one malformed answer is never served again.
        let cached = self
            .state
            .answers(&self.plan.digest(), request)
            .map_err(Exit::Failed)?
            .filter(|answers| validated(request, answers.clone()).is_ok());
        if let Some(answers) = cached {
            return Ok(answers);
        }
        let answers = self.asked(adapter, request, trace)?;
        self.state
            .keep(&self.plan.digest(), request, &answers)
            .map_err(Exit::Failed)?;
        Ok(answers)
    }

    /// One request answered by the adapter alone, timed, and refused when it does not validate.
    fn asked(
        &self,
        adapter: &dyn Adapter,
        request: &Request,
        trace: &mut Vec<Trace>,
    ) -> Result<Raw, Exit> {
        let deadline = Deadline::after(self.plan.deadline());
        let answers = adapter.answer(request, deadline).map_err(Exit::Adapter)?;
        trace.push(Trace {
            adapter: adapter.declared().id.clone(),
            questions: request.questions.len(),
            ms: deadline.elapsed(),
        });
        validated(request, answers.clone()).map_err(Exit::Adapter)?;
        Ok(answers)
    }

    /// The same, never through the cache — neither read nor kept — and without a spinner: what `test` asks, in
    /// a batch that spins once for all of it, so every repeat is a fresh answer.
    pub fn decide_uncached(
        &self,
        adapter: &dyn Adapter,
        input: &str,
        tags: &[Tag],
    ) -> Result<Decided, Exit> {
        let asked = Asked {
            text: input.to_owned(),
            tags: tags.to_vec(),
            only: None,
            whole: true,
            named: None,
        };
        self.decided(adapter, &asked, false, &[])
    }

    /// A text decided as it was asked: over its tags, or its one reflex, on the route the whole request gave it
    /// where it gave one.
    fn decided(
        &self,
        adapter: &dyn Adapter,
        asked: &Asked,
        cached: bool,
        recent: &[Recent],
    ) -> Result<Decided, Exit> {
        let request = request(
            &self.plan,
            &asked.text,
            &asked.tags,
            asked.only.as_ref(),
            asked.named.as_ref(),
            Scope::Full,
            recent,
        )
        .map_err(Exit::Human)?;
        let mut trace = Vec::new();
        // The text's questions, then each round its answers open: every round through the cache, or never.
        let floors = adapter.declared().gate.as_ref();
        let (request, answers, reading) = reading(
            &self.plan,
            floors,
            request,
            |round| {
                if cached {
                    self.own(adapter, round, &mut trace)
                } else {
                    self.asked(adapter, round, &mut trace)
                }
            },
            Exit::Adapter,
        )?;
        let decision = gate(&self.plan, reading, floors);
        Ok(Decided {
            input: request.state.request,
            proposed: request.proposed,
            answers,
            decision,
            trace,
        })
    }

    /// An edit landed in its owned file, and its line printed.
    pub fn apply(&mut self, input: &str, edit: &Edit) -> Result<Edited, Exit> {
        let edited = self.land(input, edit)?;
        // Under `--json` stderr keeps quiet, so the line goes to the terminal itself, as a prompt's own line does.
        let written = report::written(&edited.shown, &edited.landed);
        if self.reporter.json {
            self.show(&written)?;
        } else {
            terminal::note(&written);
        }
        Ok(edited)
    }

    /// An edit landed in its owned file, silently: rendered by the host, read back through the core, written only
    /// when it reads — else every problem but the last is printed and the file stands — then trust re-blessed.
    pub fn land(&self, input: &str, edit: &Edit) -> Result<Edited, Exit> {
        let edited = files::edited(&self.root, edit).map_err(Exit::Failed)?;
        let text = Text::Toml(&edited.text);
        let read = match edit.file() {
            Owned::Project => project(Document {
                file: File::Project,
                text,
            })
            .map(drop),
            Owned::Vocab { name } => vocabulary(Document {
                file: File::Vocab { name: name.clone() },
                text,
            })
            .map(drop),
            Owned::Overlay { name } => {
                let Some((shipped, _)) = self.shipped.get(name) else {
                    return Err(Exit::Human(Diagnostic {
                        reflex: Some(name.clone()),
                        at: None,
                        message: "has no manifest to read the overlay against".to_owned(),
                        fix: Fix::Sync,
                    }));
                };
                overlay(
                    Document {
                        file: File::Overlay { name: name.clone() },
                        text,
                    },
                    shipped,
                )
                .map(drop)
            }
        };
        read.map_err(|problems| self.reporter.human(input, problems))?;
        self.write_owned(&edited.path, &edited.text)?;
        Ok(edited)
    }

    /// The body, under its declaration: the policy resolved with the call's values, a relative day against today's;
    /// the machine's facts about it
    /// gathered — a declared path or program it lacks is the failure, before anything runs — then the loader
    /// started with the envelope, or the argv spawned, in the body's directory with a private temporary folder,
    /// the layers around it. Timed from now — the plan's deadline less what the adapter spent of it — so a
    /// prompt in between never counts. A refusal past the declaration names the path and the key, and its fix.
    /// Ctrl-C while the body runs ends its group and is the failure's cause, `interrupted`, for the command to
    /// act on. A failure carries the frames of an error a file body threw, for the line to keep.
    pub fn run(
        &self,
        chosen: &Chosen,
        taken: &IndexMap<ArgName, Json>,
        input: &Input,
        spent: Millis,
    ) -> Result<Returned, Failed> {
        let _armed = interrupt::arm();
        let reflex = &chosen.call.reflex;
        let what = format!("running {reflex}");
        let active = &self.plan.active()[reflex];
        let home = self.environment.get("HOME").unwrap_or_default();
        let today = clock::today(self.environment)?;
        let deadline = Deadline::after(self.plan.deadline()).less(spent);
        let refused = |refused: Diagnostic| Failure {
            what: what.clone(),
            cause: Some(refused.message),
            fix: refused.fix,
        };
        let envelope = envelope(chosen, active, taken, input, deadline.left(), home, today)
            .map_err(refused)?;
        let (shipped, dir) = &self.shipped[reflex];
        let dir = std::fs::canonicalize(dir).map_err(|error| Failure {
            what: what.clone(),
            cause: Some(format!(
                "finding {}: {}",
                dir.display(),
                crate::hosts::cause(&error)
            )),
            fix: Fix::Sync,
        })?;
        let config = processes::config_values(&what, &envelope, self.environment)?;
        let values: IndexMap<ConfigKey, String> = config
            .iter()
            .map(|(key, value)| ((*key).clone(), value.clone()))
            .collect();
        let policy =
            resolve(&active.needs, &chosen.call, active, &values, home, today).map_err(refused)?;
        let argv = match &active.run {
            Run::Argv { .. } => Some(argv(chosen, active, home, today).map_err(refused)?),
            Run::File(_) | Run::Inline => None,
        };
        let runtime = match argv {
            Some(_) => None,
            None => Some(self.runtime(&what)?),
        };
        let scratch = files::scratch(self.environment)?;
        let facts = match contain::facts(
            &policy,
            runtime.as_deref(),
            &dir,
            &scratch.path,
            &self.state,
            self.environment,
        )? {
            Ok(facts) => facts,
            Err(lacking) => {
                let origin = self.origin(reflex, &dir);
                return Err(
                    refused(needs::lacking(&lacking, reflex, active, &origin, home)).into(),
                );
            }
        };
        let body = Body {
            what: &what,
            envelope: &envelope,
            config: &config,
            policy: &policy,
            facts: &facts,
            environment: self.environment,
            deadline,
        };
        let ran = match (&argv, &runtime) {
            (Some(argv), _) => processes::program(&body, argv, &dir),
            (None, Some(runtime)) => processes::file(&body, runtime, &dir),
            (None, None) => unreachable!("a file body has its runtime"),
        };
        ran.map_err(|ended| match ended {
            Ended::Failed { failure, frames } => Failed { failure, frames },
            Ended::Refused {
                error,
                refused,
                frames,
            } => {
                let origin = self.origin(reflex, &dir);
                // A fetched reflex's own declaration, resolved: `--accept` is the fix when it reaches what was refused.
                let upstream = matches!(origin, Origin::Fetched)
                    .then(|| {
                        resolve(&shipped.needs, &chosen.call, active, &values, home, today).ok()
                    })
                    .flatten();
                let named =
                    needs::refusal(&policy, upstream.as_ref(), reflex, &origin, &refused, home);
                let (cause, fix) = named.map_or((error, Fix::Rerun), |n| (n.message, n.fix));
                let failure = Failure {
                    what: what.clone(),
                    cause: Some(cause),
                    fix,
                };
                Failed { failure, frames }
            }
        })
    }

    /// The runtime recorded on this machine, for a file body; none recorded is `evoke sync`.
    fn runtime(&self, what: &str) -> Result<PathBuf, Failure> {
        self.state.runtime()?.ok_or_else(|| Failure {
            what: what.to_owned(),
            cause: Some("no JavaScript runtime is recorded on this machine".to_owned()),
            fix: Fix::Sync,
        })
    }

    /// Where a reflex's declaration is written: a local one's manifest, at its `[needs]` line; a fetched one's is
    /// upstream's.
    fn origin(&self, reflex: &LocalName, dir: &Path) -> Origin {
        if !matches!(
            self.project.reflexes.get(reflex),
            Some(Location::Local { .. })
        ) {
            return Origin::Fetched;
        }
        let file = File::Manifest {
            name: reflex.clone(),
        };
        let text = files::read(&dir.join("reflex.toml"))
            .ok()
            .flatten()
            .unwrap_or_default();
        Origin::Local {
            at: needs::declared_at(Document {
                file,
                text: Text::Toml(&text),
            }),
        }
    }

    /// Whether a terminal answers, opened on first need.
    pub fn has_tty(&mut self) -> bool {
        if self.tty.is_none() {
            self.tty = terminal::tty();
        }
        self.tty.is_some()
    }

    /// The text shown on the terminal and what was typed; none at the end of input.
    pub fn prompt(&mut self, text: &str) -> Result<Option<String>, Exit> {
        self.prompt_ready()?;
        let tty = self.tty.as_mut().expect("the terminal is open");
        tty.prompt(text).map_err(Exit::Failed)
    }

    /// A prompt needs the terminal; without one the run has failed.
    fn prompt_ready(&mut self) -> Result<(), Exit> {
        if self.has_tty() {
            return Ok(());
        }
        Err(Exit::Failed(Failure {
            what: "opening the terminal".to_owned(),
            cause: None,
            fix: Fix::Rerun,
        }))
    }

    /// One of `evoke`'s own lines, shown on the terminal itself ahead of a prompt.
    fn show(&mut self, line: &terminal::Text) -> Result<(), Exit> {
        self.prompt_ready()?;
        let tty = self.tty.as_mut().expect("the terminal is open");
        tty.show(&line.to_string()).map_err(Exit::Failed)
    }

    /// `evoke`'s own line — the call, the effect, the weakest judgment — then the confirm prompt until `y`, `n`
    /// or, where a lesson can be taught, `t`; none at the end of input. Under `--json` stderr keeps quiet, so the
    /// line goes to the terminal itself: a template is never shown without it ahead.
    pub fn confirmed(
        &mut self,
        own: &terminal::Text,
        prompt: &Prompt,
        teachable: bool,
    ) -> Result<Option<Confirmed>, Exit> {
        if self.reporter.json {
            self.show(own)?;
        } else {
            terminal::note(own);
        }
        let mut retry: Option<String> = None;
        loop {
            let asked = report::confirm_prompt(prompt, teachable, retry.as_deref());
            let Some(typed) = self.prompt(&asked)? else {
                return Ok(None);
            };
            match typed.trim().to_lowercase().as_str() {
                "t" | "teach" if teachable => return Ok(Some(Confirmed::Teach)),
                "" => retry = None,
                word if report::yes(word) => return Ok(Some(Confirmed::Yes)),
                word if report::no(word) => return Ok(Some(Confirmed::No)),
                _ => {
                    retry = Some(format!(
                        "{} is not one of them",
                        report::quoted(typed.trim())
                    ));
                }
            }
        }
    }
}

/// Outside home, whether the root is what was blessed: not trusted, or changed since, is a stop with `evoke trust`
/// as the fix. `shown` is the root as the report prints it.
fn still_trusted(state: &State, root: &Root, snapshot: &Snapshot, shown: &str) -> Result<(), Exit> {
    if root.home {
        return Ok(());
    }
    let because = match state.trust(&root.path).map_err(Exit::Failed)? {
        None => "is not trusted",
        Some(digest) if digest != files::digest_of(snapshot) => "changed since you trusted it",
        Some(_) => return Ok(()),
    };
    Err(Exit::Human(Diagnostic {
        reflex: None,
        at: None,
        message: format!("{shown} {because}"),
        fix: Fix::Trust,
    }))
}
