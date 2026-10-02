//! `evoke add <ref>… [--as name]`: each remote ref fetched at its pin or its newest tag and kept in the store — one
//! fetch per repository and tag, however many reflexes come from it — and each local ref read where it is; every
//! manifest read and linted; the installed examples routed over the new set, a few at a time, to name each phrase
//! a newcomer steals; a newcomer playbook's steps decided over the new set, filled from its own records, to show
//! what each reaches here, and every installed playbook's steps decided again, to name a step the newcomer now
//! takes; then the project, the lock and `evoke.d.ts` written, the runtime recorded, and for each newcomer its
//! row with what it may touch, the machine's status once when it does not hold a declaration whole, then the
//! reach, the lint, theft and inactive lines. Nothing is written until every newcomer is in hand; a theft test that
//! could not finish — the adapter faulted, or has no key yet — is reported with `evoke test`, never a refusal. A
//! step that reaches nothing is reported, never a refusal: the plan refuses when it is made; so are a claim its
//! steps pass and a step that reaches a reflex without a tag the playbook carries, which a request narrowed by
//! that tag would starve.
//! In: the refs, one name or none, the environment. Out: `Exit`.

use std::path::PathBuf;

use evoke_core::document::Text;
use evoke_core::manifest::Effect;
use evoke_core::name::{LocalName, Tag};
use evoke_core::project::{Location, Locked, Reference};
use evoke_core::{
    Case, Decision, Diagnostic, Document, File, Finding, Fix, Gate, Installed, Item, Manifest,
    Plan, Raw, Request, Routed, Scope, Table, Theft, Version, add_entry, cases, compile, effective,
    filled, gate, lint, manifest, reading, request, thieves,
};

use super::session::{self, Opening, Session};
use super::{Exit, default_name, human};
use crate::adapter::{self, Adapter};
use crate::args::{Command, Ref};
use crate::hosts::{Deadline, Environment, contain, files, git, terminal, threads};
use crate::report::{self, Gutter, StepBecame, TestedStep};

pub fn run(
    command: &Command,
    refs: &[Ref],
    name: Option<&LocalName>,
    environment: &Environment,
) -> Exit {
    let mut session = match session::open(command, false, environment, Opening::Installing) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.stand_in();
    let exit = added(&mut session, &input, refs, name);
    session.reporter.exit(&input, exit)
}

/// A reflex read and, when remote, kept in the store: not yet installed.
struct Newcomer {
    name: LocalName,
    written: String,
    location: Location,
    manifest: Manifest,
    findings: Vec<Finding>,
    /// The lock entry a remote reflex takes; a local one is never locked.
    locked: Option<Locked>,
}

fn added(session: &mut Session<'_>, input: &str, refs: &[Ref], name: Option<&LocalName>) -> Exit {
    let mut remotes = git::Remotes::default();
    let mut newcomers = Vec::new();
    for r in refs {
        let found = match &r.location {
            Location::Local { .. } => local(session, input, r, name).map(|one| vec![one]),
            Location::Remote { reference, pin } => {
                fetched(session, input, &mut remotes, r, reference, *pin, name)
            }
        };
        match found {
            Ok(found) => newcomers.extend(found),
            Err(exit) => return exit,
        }
    }
    let typed: Vec<String> = refs.iter().map(|r| r.written.clone()).collect();
    if let Err(exit) = placed(session, &newcomers, &typed) {
        return exit;
    }
    let (thefts, reaches, unfinished) = match tested(session, input, &newcomers) {
        Ok((thefts, reaches)) => (thefts, reaches, None),
        Err(exit) => (Vec::new(), Vec::new(), Some(exit)),
    };
    for newcomer in &newcomers {
        if let Err(exit) = session.land(input, &add_entry(&newcomer.name, &newcomer.location)) {
            return exit;
        }
    }
    let locked: Vec<(&LocalName, &Locked)> = newcomers
        .iter()
        .filter_map(|newcomer| Some((&newcomer.name, newcomer.locked.as_ref()?)))
        .collect();
    if !locked.is_empty() {
        let mut lock = session.lock.clone().unwrap_or_else(|| session.new_lock());
        lock.adapter = session.locked_adapter();
        for (name, entry) in locked {
            lock.reflexes.insert(name.clone(), entry.clone());
        }
        if let Err(exit) = session.write_lock(&lock) {
            return exit;
        }
    }
    if let Err(exit) = session.reload(input).and_then(|()| session.write_types()) {
        return exit;
    }
    let runtime = match session.record_runtime() {
        Ok(runtime) => runtime,
        Err(exit) => return exit,
    };
    let names: Vec<&LocalName> = newcomers.iter().map(|newcomer| &newcomer.name).collect();
    terminal::note(&report::rows(
        &session.rows(names.iter().copied()),
        Gutter::Added,
    ));
    if let Some(status) = report::status(&session.contained) {
        terminal::note(&status);
    }
    for reach in &reaches {
        terminal::note(&report::reached(&reach.steps));
    }
    for newcomer in &newcomers {
        for finding in &newcomer.findings {
            terminal::note(&report::finding(&newcomer.name, finding));
        }
    }
    for reach in &reaches {
        for line in reach.lines() {
            session.reporter.note(input, &line);
        }
    }
    let playbooks: Vec<LocalName> = session
        .plan
        .active()
        .iter()
        .filter(|(_, active)| !active.steps.is_empty())
        .map(|(name, _)| name.clone())
        .collect();
    for theft in &thefts {
        session
            .reporter
            .note(input, &stolen_line(theft, &playbooks));
    }
    if let Some(exit) = unfinished {
        session.reporter.note(input, &unfinished_line(&exit));
    }
    let problems = session.inactive_of(names.iter().copied());
    if !problems.is_empty() {
        terminal::note(&report::inactive(
            &problems,
            &session.reporter.command.placeholder(),
            &session.reporter.paths,
        ));
    }
    runtime.map_or(Exit::Ran, Exit::Human)
}

/// A local ref: the directory as `evoke.toml` will name it, relative to the project; its manifest read and
/// linted where it is. Nothing is fetched or kept.
fn local(
    session: &mut Session<'_>,
    input: &str,
    r: &Ref,
    name: Option<&LocalName>,
) -> Result<Newcomer, Exit> {
    let Some(path) = files::relative(&session.root.path, &r.written).map_err(Exit::Failed)? else {
        return Err(human(format!("{} is not a directory", r.written), Fix::New));
    };
    let dir: PathBuf = session.root.path.join(&path).components().collect();
    let text = files::read(&dir.join("reflex.toml")).map_err(Exit::Failed)?;
    let Some(text) = text else {
        return Err(human(format!("no reflex.toml in {}", r.written), Fix::New));
    };
    let local = if let Some(name) = name {
        name.clone()
    } else {
        let last = path.rsplit('/').next().unwrap_or(&path);
        LocalName::new(last).map_err(|why| {
            human(
                format!("{}: {why}", r.written),
                Fix::AddRef {
                    reference: r.written.clone(),
                    name: None,
                },
            )
        })?
    };
    let shown = session::local_shown(&session.root.path, &path, session.environment());
    session.reporter.paths.reflexes.insert(local.clone(), shown);
    let manifest = parsed(session, input, &local, &text)?;
    let findings = lint(&manifest);
    Ok(Newcomer {
        name: local,
        written: r.written.clone(),
        location: Location::Local { path },
        manifest,
        findings,
        locked: None,
    })
}

/// One remote ref's reflexes, fetched at the tag it pins or the newest, kept in the store, read and linted.
fn fetched(
    session: &mut Session<'_>,
    input: &str,
    remotes: &mut git::Remotes,
    r: &Ref,
    reference: &Reference,
    pin: Option<Version>,
    name: Option<&LocalName>,
) -> Result<Vec<Newcomer>, Exit> {
    let tags = remotes.tags(reference).map_err(Exit::Failed)?;
    let tag = match pin {
        Some(pin) => tags.iter().find(|tag| tag.version == pin).ok_or_else(|| {
            human(
                format!("{reference} has no tag {pin}"),
                Fix::AddRefs {
                    references: vec![reference.to_string()],
                },
            )
        })?,
        None => tags.last().ok_or_else(|| {
            human(
                format!(
                    "{reference} has no version tag; its author publishes one with git push --tags"
                ),
                Fix::Rerun,
            )
        })?,
    };
    let fetched = remotes.fetch(reference, tag).map_err(Exit::Failed)?;
    if fetched.reflexes.is_empty() {
        return Err(human(
            format!("no reflex.toml at {reference} {}", tag.version),
            Fix::Rerun,
        ));
    }
    let one = fetched.reflexes.len() == 1 && fetched.reflexes[0].dir == reference.dir;
    if !one && name.is_some() {
        return Err(human(
            format!("{reference} is a collection; --as names one reflex"),
            Fix::AddRefs {
                references: vec![reference.to_string()],
            },
        ));
    }
    let mut found = Vec::new();
    for tree in fetched.reflexes {
        let reference = Reference {
            repo: reference.repo.clone(),
            dir: tree.dir.clone(),
        };
        let location = Location::Remote {
            reference: reference.clone(),
            pin,
        };
        let written = if one {
            r.written.clone()
        } else {
            location.to_string()
        };
        let local = match name {
            Some(name) => name.clone(),
            None => default_name(&reference).map_err(|why| {
                human(
                    format!("{reference}: {why}"),
                    Fix::AddRef {
                        reference: written.clone(),
                        name: None,
                    },
                )
            })?,
        };
        let text = tree
            .files
            .iter()
            .find(|(path, _)| path.as_str() == "reflex.toml")
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default();
        let kept = session.store.keep(tree.files).map_err(Exit::Failed)?;
        let shown = files::shown(&kept.dir, session.environment());
        session.reporter.paths.reflexes.insert(local.clone(), shown);
        let manifest = parsed(session, input, &local, &text)?;
        let findings = lint(&manifest);
        found.push(Newcomer {
            locked: Some(Locked {
                reference,
                tag: tag.version,
                commit: fetched.commit.clone(),
                h1: kept.h1,
                effect: manifest.effect,
                needs: manifest.needs.clone(),
            }),
            name: local,
            written,
            location,
            manifest,
            findings,
        });
    }
    Ok(found)
}

/// A newcomer's manifest, or every line to fix: all but the last printed, the last the exit.
fn parsed(
    session: &Session<'_>,
    input: &str,
    name: &LocalName,
    text: &str,
) -> Result<Manifest, Exit> {
    manifest(Document {
        file: File::Manifest { name: name.clone() },
        text: Text::Toml(text),
    })
    .map_err(|problems| session.reporter.human(input, problems))
}

/// Every newcomer's name is free: not another newcomer's, not an installed reflex's from elsewhere, not one
/// already locked. An entry the project names but the lock lacks is taken over. One taken name refuses the add;
/// when the add brought several, the fix installs the ones that are free.
fn placed(session: &Session<'_>, newcomers: &[Newcomer], typed: &[String]) -> Result<(), Exit> {
    let conflicts: Vec<Option<Diagnostic>> = newcomers
        .iter()
        .enumerate()
        .map(|(i, newcomer)| conflict(session, &newcomers[..i], newcomer))
        .collect();
    let Some(first) = conflicts.iter().flatten().next() else {
        return Ok(());
    };
    let free: Vec<String> = newcomers
        .iter()
        .zip(&conflicts)
        .filter(|(_, conflict)| conflict.is_none())
        .map(|(newcomer, _)| newcomer.written.clone())
        .collect();
    // A collection installed whole already: one line, and the update that moves it.
    if newcomers.len() > 1
        && conflicts
            .iter()
            .all(|conflict| matches!(conflict, Some(problem) if matches!(problem.fix, Fix::Update { .. })))
    {
        return Err(Exit::Human(Diagnostic {
            reflex: None,
            at: None,
            message: format!("every reflex of {} is already installed", typed.join(", ")),
            fix: Fix::Update { reflex: None },
        }));
    }
    let mut problem = first.clone();
    if newcomers.len() > 1 && !free.is_empty() {
        problem.fix = Fix::AddRefs { references: free };
    }
    Err(Exit::Human(problem))
}

/// Why a newcomer cannot take its name, when it cannot: `earlier` are the newcomers before it.
fn conflict(
    session: &Session<'_>,
    earlier: &[Newcomer],
    newcomer: &Newcomer,
) -> Option<Diagnostic> {
    let choose = Fix::AddRef {
        reference: newcomer.written.clone(),
        name: None,
    };
    if earlier.iter().any(|other| other.name == newcomer.name) {
        return Some(Diagnostic {
            reflex: None,
            at: None,
            message: format!("two of these would be named {}", newcomer.name),
            fix: choose,
        });
    }
    // The same tree the lock already holds, by content: the reflex is installed, under its other name.
    if let (Some(lock), Some(entry)) = (session.lock.as_ref(), newcomer.locked.as_ref())
        && let Some((other, _)) = lock
            .reflexes
            .iter()
            .find(|(name, held)| *name != &newcomer.name && held.h1 == entry.h1)
    {
        return Some(Diagnostic {
            reflex: Some(newcomer.name.clone()),
            at: None,
            message: format!("is {other} under another name"),
            fix: Fix::Show {
                reflex: Some(other.clone()),
            },
        });
    }
    let locked = session
        .lock
        .as_ref()
        .is_some_and(|lock| lock.reflexes.contains_key(&newcomer.name));
    let (message, fix) = match session.project.reflexes.get(&newcomer.name) {
        Some(existing) if *existing != newcomer.location => {
            (format!("is already installed from {existing}"), choose)
        }
        Some(_) if locked => (
            "is already installed".to_owned(),
            Fix::Update {
                reflex: Some(newcomer.name.clone()),
            },
        ),
        _ => return None,
    };
    Some(Diagnostic {
        reflex: Some(newcomer.name.clone()),
        at: None,
        message,
        fix,
    })
}

/// What a newcomer playbook's steps reach on this set: each step's sentence and the reflex it routes to, and
/// each reflex reached that lacks a tag the playbook carries, by its step.
struct Reach {
    name: LocalName,
    claim: Effect,
    steps: Vec<TestedStep>,
    untagged: Vec<(usize, LocalName, Vec<Tag>)>,
}

impl Reach {
    /// The lines after the rows: a step that reaches nothing here, with the lesson that settles it; a claim under
    /// the steps' worst; a reflex reached that lacks the playbook's tag.
    fn lines(&self) -> Vec<Diagnostic> {
        let mut lines = Vec::new();
        for step in &self.steps {
            if matches!(step.became, StepBecame::NoReflex) {
                lines.push(Diagnostic {
                    reflex: Some(self.name.clone()),
                    at: None,
                    message: format!(
                        "step {} {} reaches nothing here",
                        step.n,
                        report::quoted(&step.sentence)
                    ),
                    fix: Fix::Teach {
                        utterance: step.sentence.clone(),
                        reflex: None,
                    },
                });
            }
        }
        let worst = self
            .steps
            .iter()
            .filter_map(|step| match &step.became {
                StepBecame::Routes { effect, .. } => Some(*effect),
                _ => None,
            })
            .max();
        if let Some(worst) = worst
            && worst > self.claim
        {
            lines.push(Diagnostic {
                reflex: Some(self.name.clone()),
                at: None,
                message: format!("claims {}; its steps reach {worst}", self.claim),
                fix: Fix::Show {
                    reflex: Some(self.name.clone()),
                },
            });
        }
        for (n, reflex, lacking) in &self.untagged {
            let tags: Vec<&str> = lacking.iter().map(Tag::as_str).collect();
            lines.push(Diagnostic {
                reflex: Some(self.name.clone()),
                at: None,
                message: format!(
                    "step {n} reaches {reflex}, which lacks the tag {}",
                    tags.join(", ")
                ),
                fix: Fix::Show {
                    reflex: Some(reflex.clone()),
                },
            });
        }
        lines
    }
}

/// The set with the newcomers in it, compiled as the session's plan is: what every test at `add` decides over.
fn newset(session: &Session<'_>, newcomers: &[Newcomer]) -> Result<(Installed, Plan), Exit> {
    let mut set = session.installed.clone();
    for newcomer in newcomers {
        set.reflexes.insert(
            newcomer.name.clone(),
            Item {
                wording: Ok(effective(&newcomer.manifest, None)),
                consented: newcomer.manifest.effect,
                needs: newcomer.manifest.needs.clone(),
                configured: session.configured(&newcomer.name),
            },
        );
    }
    let platform = contain::judged(session.environment()).map_err(Exit::Human)?;
    let plan = compile(
        &set,
        &session::plain_values(&session.project),
        session.declared.limits.as_ref(),
        Some(platform),
    )
    .map_err(Exit::Human)?;
    Ok((set, plan))
}

/// The tests at `add`, over the set with the newcomers in it: the conflict test — every installed example routed,
/// each reflex's fit asked with the route, to name each phrase a newcomer steals, and every installed playbook's
/// steps decided again, to name a step the newcomer now takes — and the reach of each newcomer playbook's
/// steps. Nothing is asked when no newcomer is active.
fn tested(
    session: &Session<'_>,
    input: &str,
    newcomers: &[Newcomer],
) -> Result<(Vec<Theft>, Vec<Reach>), Exit> {
    let names: Vec<LocalName> = newcomers
        .iter()
        .map(|newcomer| newcomer.name.clone())
        .collect();
    let (set, plan) = newset(session, newcomers)?;
    if !plan.active().keys().any(|name| names.contains(name)) {
        return Ok((Vec::new(), Vec::new()));
    }
    let examples: Vec<Case> = cases(&session.installed)
        .into_iter()
        .filter(|case| case.from == Table::Examples && !names.contains(&case.reflex))
        .collect();
    let playbooks: Vec<&LocalName> = plan
        .active()
        .iter()
        .filter(|(_, active)| !active.steps.is_empty())
        .map(|(name, _)| name)
        .collect();
    if examples.is_empty() && playbooks.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let adapter = adapter::resolve(
        &session.project.adapter,
        session.project.adapters.get(&session.project.adapter),
        session.environment(),
    )
    .map_err(|problems| session.reporter.human(input, problems))?;
    adapter.accepts(&plan.digest()).map_err(Exit::Human)?;
    let mut thefts = stolen(&plan, &*adapter, &names, examples)?;
    let mut reaches = Vec::new();
    for name in playbooks {
        let reach = reached(&set, &plan, &*adapter, name)?;
        if names.contains(name) {
            reaches.push(reach);
            continue;
        }
        // An installed playbook's step that now reaches a newcomer: the newcomer took it.
        for step in &reach.steps {
            if let StepBecame::Routes { reflex, .. } = &step.became
                && names.contains(reflex)
            {
                thefts.push(Theft {
                    phrase: evoke_core::Utterance::new(&step.sentence)
                        .expect("a sentence is one line"),
                    owner: name.clone(),
                    thief: reflex.clone(),
                    fits: None,
                });
            }
        }
    }
    Ok((thefts, reaches))
}

/// The conflict test: every installed example routed over the new plan, each reflex's fit asked with the route,
/// through the adapter, a few at a time.
fn stolen(
    plan: &Plan,
    adapter: &dyn Adapter,
    names: &[LocalName],
    examples: Vec<Case>,
) -> Result<Vec<Theft>, Exit> {
    if examples.is_empty() {
        return Ok(Vec::new());
    }
    let readings = {
        let busy = terminal::busy_over("checking for thefts", examples.len());
        threads::try_each(&examples, |case| {
            let request = request(
                plan,
                case.utterance.text().as_str(),
                &[],
                None,
                None,
                Scope::Fits,
                &[],
            )
            .map_err(Exit::Human)?;
            let floors = adapter.declared().gate.as_ref();
            let (_, _, reading) =
                reading(plan, floors, request, asked(plan, adapter), Exit::Adapter)?;
            busy.tick();
            Ok(reading)
        })?
    };
    let routed: Vec<Routed> = examples
        .into_iter()
        .zip(readings)
        .map(|(case, reading)| Routed {
            case,
            winner: reading.winner.map(|winner| winner.reflex),
            ranking: reading.ranking,
        })
        .collect();
    let floor = adapter.declared().gate.as_ref().and_then(Gate::fits);
    Ok(thieves(names, &routed, floor))
}

/// A round of questions asked of the adapter, under the plan's deadline.
fn asked<'a>(
    plan: &'a Plan,
    adapter: &'a dyn Adapter,
) -> impl FnMut(&Request) -> Result<Raw, Exit> + 'a {
    move |round| {
        adapter
            .answer(round, Deadline::after(plan.deadline()))
            .map_err(Exit::Adapter)
    }
}

/// One playbook's steps decided over the plan: its records decided first, each step filled from the first whose
/// reading fills every slot — a step no record fills is untested — and decided once, uncached, no body run.
fn reached(
    set: &Installed,
    plan: &Plan,
    adapter: &dyn Adapter,
    name: &LocalName,
) -> Result<Reach, Exit> {
    let active = &plan.active()[name];
    let records: Vec<Case> = cases(set)
        .into_iter()
        .filter(|case| case.reflex == *name)
        .collect();
    let decide = |text: &str| -> Result<Decision, Exit> {
        let request =
            request(plan, text, &[], None, None, Scope::Full, &[]).map_err(Exit::Human)?;
        let floors = adapter.declared().gate.as_ref();
        let (_, _, reading) = reading(plan, floors, request, asked(plan, adapter), Exit::Adapter)?;
        Ok(gate(plan, reading, floors))
    };
    let own = {
        let busy = terminal::busy_over(format!("reading {name}"), records.len());
        threads::try_each(&records, |case| {
            let decision = decide(case.utterance.text().as_str());
            busy.tick();
            decision
        })?
    };
    let own: Vec<&Decision> = own.iter().collect();
    let texts: Vec<(usize, String, Option<String>)> = active
        .steps
        .iter()
        .enumerate()
        .map(|(i, sentence)| (i + 1, sentence.to_string(), filled(sentence, &own)))
        .collect();
    let decided = {
        let filled: Vec<&(usize, String, Option<String>)> =
            texts.iter().filter(|(_, _, text)| text.is_some()).collect();
        let busy = terminal::busy_over(format!("deciding {name}'s steps"), filled.len());
        threads::try_each(&filled, |(_, _, text)| {
            let decision = decide(text.as_deref().unwrap_or_default());
            busy.tick();
            decision
        })?
    };
    let mut decided = decided.into_iter();
    let mut steps: Vec<TestedStep> = texts
        .into_iter()
        .map(|(n, sentence, text)| {
            let became = match text {
                None => StepBecame::Untested {
                    slot: active
                        .steps
                        .get(n - 1)
                        .and_then(|sentence| sentence.slots().next().map(|(slot, _)| slot.clone()))
                        .expect("an unfilled step holds a slot"),
                },
                Some(_) => match decided.next().expect("every filled step was decided") {
                    Decision::Abstain { .. } => StepBecame::NoReflex,
                    Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
                        routes_to(plan, name, &chosen.call.reflex)
                    }
                    Decision::Ask { asking, .. } => routes_to(plan, name, &asking.reflex),
                },
            };
            TestedStep {
                n,
                sentence,
                became,
                regression: false,
            }
        })
        .collect();
    super::test::branched(plan, &active.steps, &mut steps);
    let untagged = steps
        .iter()
        .filter_map(|step| {
            let (StepBecame::Routes { reflex, .. } | StepBecame::NoField { reflex, .. }) =
                &step.became
            else {
                return None;
            };
            let theirs = &plan.active().get(reflex)?.tags;
            let lacking: Vec<Tag> = active
                .tags
                .iter()
                .filter(|tag| !theirs.contains(tag))
                .cloned()
                .collect();
            (!lacking.is_empty()).then(|| (step.n, reflex.clone(), lacking))
        })
        .collect();
    Ok(Reach {
        name: name.clone(),
        claim: active.effect,
        steps,
        untagged,
    })
}

/// What a step routed to: a reflex with its effect, or the playbook it stands in.
fn routes_to(plan: &Plan, playbook: &LocalName, reflex: &LocalName) -> StepBecame {
    if reflex == playbook {
        return StepBecame::Nested;
    }
    let effect = plan
        .active()
        .get(reflex)
        .map_or(Effect::Destructive, |active| active.effect);
    StepBecame::Routes {
        reflex: reflex.clone(),
        effect,
    }
}

/// A theft as a line: the thief, the phrase and its owner — won, or fitted over the floor — and the lesson that
/// settles it; a playbook's step the newcomer now reaches names the step and the test that shows it.
fn stolen_line(theft: &Theft, playbooks: &[LocalName]) -> Diagnostic {
    if playbooks.contains(&theft.owner) {
        return Diagnostic {
            reflex: Some(theft.owner.clone()),
            at: None,
            message: format!(
                "step \"{}\" now reaches {}",
                theft.phrase.text(),
                theft.thief
            ),
            fix: Fix::Test,
        };
    }
    let message = match theft.fits {
        None => format!("steals \"{}\" from {}", theft.phrase.text(), theft.owner),
        Some(fits) => format!(
            "also fits \"{}\" of {} ({:.2})",
            theft.phrase.text(),
            theft.owner,
            fits.get()
        ),
    };
    Diagnostic {
        reflex: Some(theft.thief.clone()),
        at: None,
        message,
        fix: Fix::TeachNot {
            utterance: theft.phrase.text().to_string(),
            reflex: theft.thief.clone(),
        },
    }
}

/// The theft test did not finish — a fault, a missing key, a plan it could not compile: the install stands, and
/// `evoke test` routes every example again.
fn unfinished_line(exit: &Exit) -> Diagnostic {
    Diagnostic {
        reflex: None,
        at: None,
        message: format!("the theft test did not finish: {}", report::said(exit)),
        fix: Fix::Test,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> LocalName {
        LocalName::new(text).unwrap()
    }

    fn routes(n: usize, sentence: &str, reflex: &str, effect: Effect) -> TestedStep {
        TestedStep {
            n,
            sentence: sentence.to_owned(),
            became: StepBecame::Routes {
                reflex: name(reflex),
                effect,
            },
            regression: false,
        }
    }

    fn said(reach: &Reach) -> Vec<String> {
        reach
            .lines()
            .iter()
            .map(|line| format!("{} → {}", line.message, line.fix.command("")))
            .collect()
    }

    #[test]
    fn a_reached_reflex_without_the_playbook_s_tag_is_reported() {
        let tag = Tag::new("outage").unwrap();
        let mut reach = Reach {
            name: name("outage"),
            claim: Effect::Destructive,
            steps: vec![
                routes(1, "check checkout's errors", "errors", Effect::Read),
                routes(2, "roll checkout back", "rollback", Effect::Destructive),
            ],
            untagged: vec![(2, name("rollback"), vec![tag])],
        };
        assert_eq!(
            said(&reach),
            ["step 2 reaches rollback, which lacks the tag outage → evoke show rollback"]
        );
        // Every reflex reached carries the tag, and the claim is the worst of the steps: nothing to say.
        reach.untagged.clear();
        assert!(said(&reach).is_empty());
        // A claim its steps pass is said as before.
        reach.claim = Effect::Write;
        assert_eq!(
            said(&reach),
            ["claims write; its steps reach destructive → evoke show outage"]
        );
    }
}
