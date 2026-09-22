//! One invocation's ground, prepared once: the project located, snapshotted and — outside home — checked against
//! its trust, parsed by the core with its lock, its remote reflexes read from the store, its adapter's declaration
//! read, its plan compiled; the state and the store; the terminal, opened on first need. On it, each input is
//! decided — asked, answered from the cache or the adapter, or fresh for `test`, read and gated — a chosen call is
//! run, an edit lands in an owned file, the lock and `evoke.d.ts` are written whole, each write of an owned file
//! re-blessing the trust. In: the environment and the command. Out: a `Session`, then a `Decided` per input, a
//! body's `Returned`, an `Edited` file. `open` prints every problem with its fix; every other method leaves its
//! `Exit` to the command to report, once.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use evoke_core::document::Text;
use evoke_core::manifest::{Effect, Run};
use evoke_core::name::{AdapterId, ConfigKey, LocalName, Tag};
use evoke_core::plan::{Held, Millis};
use evoke_core::project::{Location, Locked, LockedAdapter, Setting};
use evoke_core::text::NonEmpty;
use evoke_core::{
    Chosen, Decision, Declared, Diagnostic, Document, Edit, Effective, File, Fix, Input, Installed,
    Item, Lock, Manifest, Owned, Plan, Project, Prompt, Raw, Reading, Request, Scope, Version,
    argv, compile, effective, envelope, gate, identity, lock, manifest, overlay, project,
    project_dts, read, render_lock, request, vocabulary,
};
use indexmap::IndexMap;

use super::{Exit, Reporter};
use crate::adapter::{self, Adapter, Trace};
use crate::args::Command;
use crate::hosts::files::{self, Edited, Root, Snapshot};
use crate::hosts::processes::{self, Returned, Warm};
use crate::hosts::state::State;
use crate::hosts::store::Store;
use crate::hosts::terminal::{self, Tty};
use crate::hosts::{Deadline, Environment, Failure};
use crate::report::{self, Paths, Row};

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
}

/// What the confirm prompt read.
pub enum Confirmed {
    Yes,
    No,
    Teach,
}

/// One input decided: what was asked, what was answered, how it read, and the outcome.
pub struct Decided {
    pub request: Request,
    pub answers: Raw,
    pub reading: Reading,
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
    let input = command.placeholder();
    let mut reporter = Reporter {
        command,
        json,
        paths: Paths {
            root: String::new(),
            reflexes: BTreeMap::new(),
        },
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
                Location::Local { path } => path.clone(),
                Location::Remote { .. } => {
                    shipped.get(name).map_or_else(String::new, |(_, dir)| {
                        files::shown(dir, ground.environment)
                    })
                }
            };
            (name.clone(), shown)
        })
        .collect();
    let plan = compile(&installed, declared.limits.as_ref()).map_err(Exit::Human)?;
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
#[allow(clippy::type_complexity)]
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
    // A local reflex consents to its own effect; one without a manifest is inactive, and the tightest is as good.
    let consented = wording
        .as_ref()
        .map_or(Effect::Destructive, |effective| effective.manifest.effect);
    Ok((
        Item {
            wording,
            consented,
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
            message: format!("{name} is not locked"),
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
            message: format!("{name} is not in the store"),
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
                message: format!("{name} has no reflex.toml in the store"),
                fix: Fix::Sync,
            }]),
            None,
        ),
    };
    Ok((
        Item {
            wording,
            consented: locked.effect,
            configured,
        },
        shipped.map(|manifest| (manifest, entry.dir)),
    ))
}

/// A remote reflex the store cannot place: inactive with the problem while installing, refused otherwise.
fn unplaced(
    opening: Opening,
    configured: IndexMap<ConfigKey, Held>,
    problem: Diagnostic,
) -> Result<(Item, Option<(Manifest, PathBuf)>), Exit> {
    if opening == Opening::Installing {
        Ok((
            Item {
                wording: Err(vec![problem]),
                consented: Effect::Destructive,
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

/// How each configured key is held: the value, or a variable and whether it is set — never a secret's value.
fn held(
    settings: &IndexMap<ConfigKey, Setting>,
    environment: &Environment,
) -> IndexMap<ConfigKey, Held> {
    settings
        .iter()
        .map(|(key, setting)| {
            let held = match setting {
                Setting::Plain { value } => Held::Plain {
                    value: value.clone(),
                },
                Setting::Env { var } => Held::Env {
                    var: var.clone(),
                    set: environment.get(var.as_str()).is_some(),
                },
            };
            (key.clone(), held)
        })
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
                    Location::Local { path } => path.clone(),
                    Location::Remote { pin, .. } => {
                        match self.lock.as_ref().and_then(|lock| lock.reflexes.get(name)) {
                            Some(locked) if *pin != Some(locked.tag) => {
                                format!("{location} {}", locked.tag)
                            }
                            _ => location.to_string(),
                        }
                    }
                };
                let (effect, runs) = self
                    .installed
                    .reflexes
                    .get(name)
                    .and_then(|item| {
                        item.wording.as_ref().ok().map(|effective| {
                            (
                                Some(effective.manifest.effect.max(item.consented)),
                                report::runs(&effective.manifest.run),
                            )
                        })
                    })
                    .unwrap_or_default();
                Some(Row {
                    name: name.to_string(),
                    from,
                    effect,
                    runs,
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

    /// The runtime found on `PATH` and recorded, when an installed reflex runs a file; none found and none
    /// recorded is returned as the problem to end on.
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
            self.state.set_runtime(&found).map_err(Exit::Failed)?;
            return Ok(None);
        }
        if self.state.runtime().map_err(Exit::Failed)?.is_some() {
            return Ok(None);
        }
        Ok(Some(Diagnostic {
            reflex: None,
            at: None,
            message: "no JavaScript runtime (node) is on PATH".to_owned(),
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
    /// answers. A cached entry that no longer reads against the request is a miss.
    pub fn decide(
        &self,
        adapter: &dyn Adapter,
        input: &str,
        tags: &[Tag],
    ) -> Result<Decided, Exit> {
        let _busy = terminal::busy("deciding");
        self.decided(adapter, input, tags, true)
    }

    /// The same, never through the cache — neither read nor kept — and without a spinner: what `test` asks, in
    /// a batch that spins once for all of it, so every repeat is a fresh answer.
    pub fn decide_uncached(
        &self,
        adapter: &dyn Adapter,
        input: &str,
        tags: &[Tag],
    ) -> Result<Decided, Exit> {
        self.decided(adapter, input, tags, false)
    }

    fn decided(
        &self,
        adapter: &dyn Adapter,
        input: &str,
        tags: &[Tag],
        cached: bool,
    ) -> Result<Decided, Exit> {
        let request = request(&self.plan, input, tags, Scope::Full).map_err(Exit::Human)?;
        let deadline = Deadline::after(self.plan.deadline());
        let id = identity(request.state.request.as_str());
        let hit = if cached {
            self.state
                .answers(&self.plan.digest(), &id)
                .map_err(Exit::Failed)?
                .and_then(|answers| {
                    read(&self.plan, &request, answers.clone())
                        .ok()
                        .map(|reading| (answers, reading))
                })
        } else {
            None
        };
        let (answers, reading, trace) = if let Some((answers, reading)) = hit {
            (answers, reading, Vec::new())
        } else {
            let answers = adapter.answer(&request, deadline).map_err(Exit::Adapter)?;
            let trace = Trace {
                adapter: adapter.declared().id.clone(),
                questions: request.questions.len(),
                ms: deadline.elapsed(),
            };
            let reading = read(&self.plan, &request, answers.clone()).map_err(Exit::Adapter)?;
            if cached {
                self.state
                    .keep(&self.plan.digest(), &id, &answers)
                    .map_err(Exit::Failed)?;
            }
            (answers, reading, vec![trace])
        };
        let decision = gate(
            &self.plan,
            reading.clone(),
            adapter.declared().gate.as_ref(),
        );
        Ok(Decided {
            request,
            answers,
            reading,
            decision,
            trace,
        })
    }

    /// An edit landed in its owned file, and its line printed.
    pub fn apply(&self, input: &str, edit: &Edit) -> Result<Edited, Exit> {
        let edited = self.land(input, edit)?;
        terminal::note(&report::written(&edited));
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
                        message: format!("{name} has no manifest to read the overlay against"),
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

    /// The loader, when an active reflex runs a file: started before the request, so it is ready with the
    /// decision. No runtime recorded on this machine is `evoke sync`.
    pub fn warm(&self) -> Result<Option<Warm>, Exit> {
        let files = self
            .plan
            .active()
            .values()
            .any(|active| matches!(active.run, Run::File(_)));
        if !files {
            return Ok(None);
        }
        let runtime = self.state.runtime().map_err(Exit::Failed)?;
        let Some(runtime) = runtime else {
            return Err(Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message: "no JavaScript runtime is recorded on this machine".to_owned(),
                fix: Fix::Sync,
            }));
        };
        processes::warm(&runtime, self.environment)
            .map(Some)
            .map_err(Exit::Failed)
    }

    /// The body: the envelope into the warm loader, or the argv spawned; the loader is dismissed when unused.
    /// Timed from now — the plan's deadline less what the adapter spent of it — so a prompt in between never
    /// counts.
    pub fn run(
        &self,
        chosen: &Chosen,
        input: &Input,
        spent: Millis,
        warm: Option<Warm>,
    ) -> Result<Returned, Failure> {
        let reflex = &chosen.call.reflex;
        let what = format!("running {reflex}");
        let active = &self.plan.active()[reflex];
        let deadline = Deadline::after(self.plan.deadline()).less(spent);
        let envelope = envelope(chosen, active, input, deadline.left());
        let (_, dir) = &self.shipped[reflex];
        match &active.run {
            Run::Argv { .. } => {
                dismiss(warm);
                let argv = argv(chosen, active).map_err(|refused| Failure {
                    what: what.clone(),
                    cause: Some(refused.message),
                    fix: refused.fix,
                })?;
                processes::program(&what, &argv, &envelope, self.environment, deadline)
            }
            Run::File(_) | Run::Inline => {
                let warm = warm.expect("a file body warmed the loader");
                warm.feed(&what, &envelope, dir, self.environment, deadline)
            }
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
        loop {
            let Some(typed) = self.prompt(&report::confirm_prompt(prompt, teachable))? else {
                return Ok(None);
            };
            match typed.trim().to_lowercase().as_str() {
                "y" | "yes" => return Ok(Some(Confirmed::Yes)),
                "n" | "no" => return Ok(Some(Confirmed::No)),
                "t" | "teach" if teachable => return Ok(Some(Confirmed::Teach)),
                _ => {}
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

/// A loader that has nothing to run.
pub fn dismiss(warm: Option<Warm>) {
    if let Some(warm) = warm {
        warm.dismiss();
    }
}
