//! The foundation's loop — abstain, ask, confirm, run — over one decision, or over each step of a plan at its
//! turn, shared by `evoke "<input>"` and `evoke run <file>`: the yes over a whole plan, each round's line printed
//! under `--json` and logged, a plan stopped before any step ran, the run stage by stage, and what did not run
//! said so at the end. In: the session, the adapter — resolved up front, or at the first step a plan read from a
//! file decides again — the plan with its decisions, and the plan file every line names when there is one. Out:
//! `Exit`, every line printed and logged.

use evoke_core::call::Value;
use evoke_core::decide::{Choices, Missing, Recent, Why};
use evoke_core::document::Json;
use evoke_core::manifest::{Kind, Recognizer, Source, written};
use evoke_core::name::{ArgName, LocalName, OptionKey, Tag, VocabName, Word};
use evoke_core::needs::Entry;
use evoke_core::text::NonEmpty;
use evoke_core::vocabulary::Meaning;
use evoke_core::weave::{
    self, Asked, Because, Binding, Bound, From, Handled, Handling, Outcome, Progress,
    Returned as Yielded, Shared, Status, Step, Todo, When, Why as Stopped,
};
use evoke_core::{
    Chosen, Clean, Decision, Diagnostic, Executed, Fix, Gate, Lesson, Prompt, Running, Utterance,
    VocabChange, Weave, fill, identity, picked, teach, vocab_edit,
};
use indexmap::IndexMap;

use super::session::{Confirmed, Decided, Failed, Session, Woven};
use super::{Decline, Exit, needs_terminal};
use crate::adapter::Adapter;
use crate::hosts::processes::Returned;
use crate::hosts::terminal::Text;
use crate::hosts::{interrupt, terminal};
use crate::report::{self, Expanded, Line, PinnedAt, StepLine};

/// The adapter that answers: resolved up front, as deciding a sentence needs it, or at the first step a plan
/// read from a file decides again — so a file that answers whole runs on a machine that holds no key.
pub enum Engine {
    Ready(Box<dyn Adapter>),
    Later,
}

impl Engine {
    /// The adapter, resolved now when it was not: its credential or recording in hand, asked whether it accepts
    /// the plan. The exit is the caller's to report.
    pub fn resolved(&mut self, session: &Session<'_>, input: &str) -> Result<&dyn Adapter, Exit> {
        if matches!(self, Self::Later) {
            *self = Self::Ready(session.adapter(input)?);
        }
        match self {
            Self::Ready(adapter) => Ok(&**adapter),
            Self::Later => unreachable!("the adapter was just resolved"),
        }
    }
}

/// The session with what a round needs beside it: who answers, whether lines are JSON, the tags a decision is
/// narrowed by, and the plan file every line names when a plan runs from one.
pub struct Rounds<'a> {
    pub session: Session<'a>,
    pub engine: Engine,
    pub json: bool,
    pub tags: Vec<Tag>,
    pub pinned: Option<PinnedAt>,
    /// The process's results, newest first: what a whole sentence's ask offers back, a value the words lack
    /// recalled from a result that yielded it.
    pub results: Vec<Recent>,
}

impl Rounds<'_> {
    /// The adapter's thresholds, when it ships them: the declaration's, which a resolved adapter repeats.
    pub fn floors(&self) -> Option<&Gate> {
        self.session.declared.gate.as_ref()
    }

    /// A decision's line, naming the plan file when the plan runs from one.
    fn line(&self, decided: &Decided) -> Line {
        let mut line = Line::of(decided);
        line.pinned.clone_from(&self.pinned);
        line
    }

    /// One decision through the foundation's loop — abstain, ask, confirm, run — as one input takes it, or as one
    /// round of a weave's step does, numbered `at`, with what the plan handed it beside it: the line printed
    /// under `--json` and logged, the exit reported; what became of it, for the weave.
    pub fn round(
        &mut self,
        input: &str,
        at: Option<(usize, usize)>,
        decided: &Decided,
        decision: Decision,
        handed: Handed<'_>,
    ) -> Rounded {
        let json = self.json;
        let mut line = self.line(decided);
        line.decision = decision.clone();
        line.step = at.map(|(n, of)| StepLine {
            n,
            of,
            status: Status::Ran,
            why: None,
            bound: handed.bound.to_vec(),
            shared: handed.shared.clone(),
            from: handed.from.to_vec(),
            when: handed.when.cloned(),
        });
        let chosen = match self.readied(input, at, decided, decision, handed.bound, &mut line) {
            Ok(chosen) => chosen,
            Err(rounded) => return rounded,
        };
        line.contained = Some(self.session.contained.clone());
        let ran = self
            .session
            .run(&chosen, handed.taken, &decided.input, decided.spent());
        match ran {
            Ok(returned) => {
                if !json && !returned.text.is_empty() {
                    terminal::result(&returned.text);
                }
                line.result = Some(returned.clone());
                if let Some(data) = &returned.data {
                    self.results.insert(
                        0,
                        Recent {
                            reflex: chosen.call.reflex.clone(),
                            data: data.clone(),
                        },
                    );
                }
                let mut rounded = self.stopped(input, &mut line, Exit::Ran, None);
                rounded.result = Some(returned);
                rounded
            }
            Err(Failed { failure, frames }) => {
                line.error = Some(report::failure(&failure));
                line.frames = frames;
                self.stopped(input, &mut line, Exit::Failed(failure), None)
            }
        }
    }

    /// The loop up to the run: an abstain stops; an ask is answered and filled until nothing is missing; a
    /// confirm is put. The call ready to run, or what became of the round instead.
    fn readied(
        &mut self,
        input: &str,
        at: Option<(usize, usize)>,
        decided: &Decided,
        decision: Decision,
        bound: &[Bound],
        line: &mut Line,
    ) -> Result<Chosen, Rounded> {
        let json = self.json;
        let contained = self.session.contained.clone();
        let mut decision = decision;
        loop {
            match decision {
                Decision::Abstain { .. } => {
                    if !json && at.is_none() {
                        terminal::note(&report::abstained(decided, self.floors()));
                        if let Some(hint) = report::left_out(self.session.plan.inactive().keys()) {
                            terminal::note(&hint);
                        }
                    }
                    let exit = Exit::Declined(Decline::Abstained);
                    return Err(self.stopped(input, line, exit, Some(Stopped::NoReflex)));
                }
                Decision::Ask { asking, missing } => {
                    if !self.session.has_tty() {
                        return Err(self.no_terminal(input, line, "an ask"));
                    }
                    line.recalled.extend(recalled_in(&missing));
                    let given = match self.answers(input, &asking.reflex, &missing) {
                        Ok(Answered::Given(given)) => given,
                        Ok(Answered::Declined { ask }) => {
                            let exit = Exit::Declined(Decline::Refused);
                            let why = Stopped::Said { message: ask };
                            return Err(self.stopped(input, line, exit, Some(why)));
                        }
                        Err(exit) => return Err(self.stopped(input, line, exit, None)),
                    };
                    decision = fill(&self.session.plan, asking, given, self.floors());
                    line.decision = decision.clone();
                }
                Decision::Confirm { chosen, prompt, .. } => {
                    if !self.session.has_tty() {
                        return Err(self.no_terminal(input, line, "a confirm"));
                    }
                    let own = match at {
                        Some((n, of)) => report::step(
                            n,
                            of,
                            report::step_confirming(&chosen, &prompt, &contained),
                        ),
                        None => report::confirming(&chosen, &prompt, &contained),
                    };
                    match self.session.confirmed(&own, &prompt, true) {
                        Ok(Some(Confirmed::Yes)) => return Ok(chosen),
                        Ok(Some(Confirmed::Teach)) => {
                            let spoken = decided.input.as_str();
                            self.teach(spoken, &chosen);
                            return Ok(chosen);
                        }
                        Ok(Some(Confirmed::No) | None) => {
                            let exit = Exit::Declined(Decline::Refused);
                            let why = Stopped::Said {
                                message: prompt.own.clone(),
                            };
                            return Err(self.stopped(input, line, exit, Some(why)));
                        }
                        Err(exit) => return Err(self.stopped(input, line, exit, None)),
                    }
                }
                Decision::Run { chosen } => {
                    if !json {
                        match at {
                            None => terminal::note(&report::running(&chosen, &contained)),
                            // The plan showed the step; at its turn, only what the plan could not: a bound value
                            // in its place, or a machine that does not hold the declaration. A whole result
                            // shows nothing new: the plan named it, and it stays on its source's line.
                            Some((n, of))
                                if bound.iter().any(|b| b.value.is_some())
                                    || !contained.is_full() =>
                            {
                                terminal::note(&report::step(
                                    n,
                                    of,
                                    report::step_running(&chosen, &contained),
                                ));
                            }
                            Some(_) => {}
                        }
                    }
                    return Ok(chosen);
                }
            }
        }
    }

    /// A round at its end: the line's status and reason from the exit, the line printed under `--json` and
    /// logged, the exit reported. Ctrl-C noted while the round was under way is what stopped it: a step reads
    /// `skipped · cancelled`, one decision keeps the body's failure as its `error` and reads `cancelled`, and
    /// nothing more prints — the process ends once every line is logged.
    fn stopped(&self, input: &str, line: &mut Line, exit: Exit, why: Option<Stopped>) -> Rounded {
        let cancelled = exit != Exit::Ran && interrupt::interrupted();
        let (status, why) = if cancelled {
            (Status::Skipped, Some(Stopped::Cancelled))
        } else {
            (status_of(&exit), why.or_else(|| why_of(&exit)))
        };
        if let Some(step) = &mut line.step {
            step.status = status;
            step.why.clone_from(&why);
            if cancelled {
                line.error = None;
            }
        } else if cancelled {
            line.cancelled = true;
        }
        // A failure after the line was built — a file that would not take a word — is the line's own `error`,
        // so under `--json` one object stands for the input.
        if let Exit::Failed(failure) = &exit
            && line.error.is_none()
            && line.step.is_none()
        {
            line.error = Some(report::failure(failure));
        }
        let exit = if cancelled { Exit::Ran } else { exit };
        let exit = self.logged(input, line, exit);
        Rounded {
            exit,
            status,
            why,
            result: None,
        }
    }

    /// A plan read from a file, run: printed as any plan is; refused whole when it refuses or asks, since it holds
    /// a plan `try --save` would not have written; then the yes over the whole plan at this terminal whatever
    /// the verdict — a file's numbers are whoever wrote them — and each step at its turn.
    pub fn run_pinned(&mut self, input: &str, woven: &Woven, shown: &str) -> Exit {
        if !self.json {
            terminal::note(&report::planned(&woven.weave));
        }
        match woven.weave.verdict.outcome {
            Outcome::Refuse => return self.refused(input, woven),
            Outcome::Ask => {
                let message = woven
                    .weave
                    .verdict
                    .because
                    .first()
                    .map_or_else(|| "the plan asks".to_owned(), report::verdict);
                let problem = Diagnostic {
                    reflex: None,
                    at: None,
                    message,
                    fix: Fix::Save {
                        file: shown.to_owned(),
                        input: input.to_owned(),
                    },
                };
                return self.unanswered(input, woven, problem);
            }
            Outcome::Run | Outcome::Confirm => {}
        }
        let reasons: Vec<String> = woven
            .weave
            .verdict
            .because
            .iter()
            .map(report::verdict)
            .collect();
        let mut own = format!("the plan of {shown}");
        if !reasons.is_empty() {
            own.push_str(" · ");
            own.push_str(&reasons.join("; "));
        }
        if let Err(exit) = self.expansions(input, woven) {
            return exit;
        }
        if let Err(exit) = self.proceed(input, woven, own, whole_plan(), None) {
            return exit;
        }
        self.executed(input, woven)
    }

    /// The yes over a whole plan: evoke's own line, then the template — `Run the plan as it stands?`, or the
    /// playbook's own confirm when the plan is one playbook's — with `[y]es [n]o`, and `[t]each` where one
    /// playbook stands in a typed plan. `n` runs nothing, every step's line declined; no terminal, every line
    /// unanswered. The exit is reported.
    pub fn proceed(
        &mut self,
        input: &str,
        woven: &Woven,
        own: String,
        template: Clean,
        teach: Option<Teach>,
    ) -> Result<(), Exit> {
        if !self.session.has_tty() {
            return Err(self.unanswered(input, woven, needs_terminal("a confirm")));
        }
        let prompt = Prompt { own, template };
        let line = Text::from(format!("  {}", prompt.own));
        match self.session.confirmed(&line, &prompt, teach.is_some()) {
            Ok(Some(Confirmed::Yes)) => Ok(()),
            Ok(Some(Confirmed::Teach)) => {
                if let Some(teach) = teach {
                    self.teach(&teach.spoken, &teach.chosen);
                }
                Ok(())
            }
            Ok(_) => {
                let exit = Exit::Declined(Decline::Refused);
                Err(self.stopped_whole(input, woven, exit, |_| {
                    let message = prompt.own.clone();
                    (Status::Declined, Stopped::Said { message })
                }))
            }
            Err(exit) => Err(self.session.reporter.exit(input, exit)),
        }
    }

    /// Each playbook's expansion logged as step 0 of its plan, before the steps' lines: the part that picked it,
    /// its decision, the playbook and what each slot took, with no status — the steps' lines say what became of
    /// the plan. Printed under `--json` as any line.
    pub fn expansions(&self, input: &str, woven: &Woven) -> Result<(), Exit> {
        let of = woven.weave.steps.len();
        for because in &woven.weave.verdict.because {
            let Because::Reviewed { playbook, text, .. } = because else {
                continue;
            };
            let Some((_, decided)) = woven
                .decided
                .iter()
                .find(|(asked, _)| asked.text == *text && asked.only.is_none())
                .or_else(|| woven.decided.iter().find(|(asked, _)| asked.text == *text))
            else {
                continue;
            };
            let slots: IndexMap<ArgName, String> = match &decided.decision {
                Decision::Run { chosen } | Decision::Confirm { chosen, .. } => chosen
                    .call
                    .args
                    .iter()
                    .filter_map(|(arg, value)| Some((arg.clone(), written(value)?)))
                    .collect(),
                Decision::Ask { .. } | Decision::Abstain { .. } => IndexMap::new(),
            };
            let mut line = self.line(decided);
            line.expansion = Some(Expanded {
                of,
                playbook: playbook.clone(),
                slots,
            });
            if self.json {
                terminal::result(&line.json());
            }
            if let Err(failure) = self.session.state.log(&line.log()) {
                return Err(self.session.reporter.exit(input, Exit::Failed(failure)));
            }
        }
        Ok(())
    }

    /// A plan refused whole: nothing runs. A part that matches nothing: each step's line, the abstaining one
    /// refused, the rest skipped. A whole result no step before its taker hands, or several do: the line that
    /// names the step and what it takes, with the reflex to look at; the taker refused, the rest skipped. A
    /// playbook's refusal — a step routing to its own plan, a plan too deep or too long, a part left out beside
    /// it: its line, the step it names refused, the rest skipped.
    pub fn refused(&self, input: &str, woven: &Woven) -> Exit {
        let json = self.json;
        let named: Vec<&Because> = woven
            .weave
            .verdict
            .because
            .iter()
            .filter(|because| {
                matches!(
                    because,
                    Because::NoSource { .. }
                        | Because::SeveralSources { .. }
                        | Because::Nested { .. }
                        | Because::TooDeep { .. }
                        | Because::TooLong { .. }
                        | Because::Excluded { .. }
                        | Because::BranchIntoPlan { .. }
                        | Because::NoField { .. }
                        | Because::BadValue { .. }
                        | Because::MaybeSource { .. }
                )
            })
            .collect();
        if named.is_empty() {
            if !json && let Some(hint) = report::left_out(self.session.plan.inactive().keys()) {
                terminal::note(&hint);
            }
            let exit = Exit::Declined(Decline::Abstained);
            return self.stopped_whole(input, woven, exit, |step| {
                if matches!(step.decision, Decision::Abstain { .. }) {
                    (Status::Refused, Stopped::NoReflex)
                } else {
                    let message = "the request was refused".to_owned();
                    (Status::Skipped, Stopped::Said { message })
                }
            });
        }
        if !json {
            let invoked = self.session.reporter.command.placeholder();
            for because in &named {
                let problem = Diagnostic {
                    reflex: None,
                    at: None,
                    message: report::verdict(because),
                    fix: refusal_fix(&self.session, because, woven),
                };
                terminal::note(&report::diagnostic(
                    &problem,
                    &invoked,
                    Some(&self.session.reporter.paths),
                ));
            }
        }
        let exit = Exit::Declined(Decline::Refused);
        self.stopped_whole(input, woven, exit, |step| {
            let own: Vec<String> = named
                .iter()
                .filter(|because| step_of(because) == Some(step.n))
                .map(|because| report::verdict(because))
                .collect();
            if own.is_empty() {
                let message = "the request was refused".to_owned();
                (Status::Skipped, Stopped::Said { message })
            } else {
                let message = own.join("; ");
                (Status::Refused, Stopped::Said { message })
            }
        })
    }

    /// A question only a person can answer, and none did: every step's line `unanswered` with it.
    pub fn unanswered(&self, input: &str, woven: &Woven, problem: Diagnostic) -> Exit {
        let message = problem.message.clone();
        self.stopped_whole(input, woven, Exit::Human(problem), |_| {
            let message = message.clone();
            (Status::Unanswered, Stopped::Said { message })
        })
    }

    /// The plan stopped before any step ran: each step's line with what became of it — printed under `--json`,
    /// logged — then the exit, reported once. Under `--json` a diagnostic is already every line's `why`, so
    /// nothing more prints.
    pub fn stopped_whole(
        &self,
        input: &str,
        woven: &Woven,
        exit: Exit,
        became: impl Fn(&Step) -> (Status, Stopped),
    ) -> Exit {
        let of = woven.weave.steps.len();
        for step in &woven.weave.steps {
            let Some(decided) = woven.planned(step, &self.tags) else {
                continue;
            };
            let mut line = self.line(&decided);
            let (status, why) = became(step);
            line.step = Some(StepLine {
                n: step.n,
                of,
                status,
                why: Some(why),
                bound: Vec::new(),
                shared: step.shared.clone(),
                from: step.from.clone(),
                when: step.when.clone(),
            });
            if self.json {
                terminal::result(&line.json());
            }
            if let Err(failure) = self.session.state.log(&line.log()) {
                return self.session.reporter.exit(input, Exit::Failed(failure));
            }
        }
        if self.json && matches!(exit, Exit::Human(_)) {
            return exit;
        }
        self.session.reporter.exit(input, exit)
    }

    /// The plan run: stage by stage, each round through the foundation's loop; a step decided again with its
    /// bound values in its words when the run asks for it — by the adapter, resolved now if a plan file left
    /// it unresolved; what did not run, said so at the end. Armed: Ctrl-C from here on is a cancel the run acts
    /// on — the round under way ended and reported `skipped · cancelled`, no round started after it — not the
    /// end of the process.
    pub fn executed(&mut self, input: &str, woven: &Woven) -> Exit {
        let _armed = interrupt::arm();
        let of = woven.weave.steps.len();
        let mut progress = Progress::default();
        let mut rewritten: Vec<(Asked, Decided)> = Vec::new();
        let mut exits: Vec<Exit> = Vec::new();
        loop {
            let running =
                weave::running::execute(&self.session.plan, self.floors(), &woven.weave, &progress);
            let todo = match running {
                Running::Done { executed } => {
                    return self.ended(input, woven, &executed, &rewritten, exits);
                }
                Running::Todo { todo } => todo,
            };
            match todo {
                Todo::Decide { asked, .. } => {
                    // The adapter's call cannot be cut short: unarmed for it, Ctrl-C ends the process at once, as
                    // it does while the plan is made — nothing is running then.
                    let decided = {
                        let _paused = interrupt::pause();
                        self.engine
                            .resolved(&self.session, input)
                            .and_then(|adapter| {
                                self.session.decide(
                                    adapter,
                                    &asked.text,
                                    &asked.tags,
                                    asked.only.as_ref(),
                                )
                            })
                    };
                    let decided = match decided {
                        Ok(decided) => decided,
                        Err(exit) => return self.session.reporter.exit(input, exit),
                    };
                    progress
                        .decided
                        .push((asked.clone(), decided.decision.clone()));
                    rewritten.push((asked, decided));
                }
                Todo::Handle { handling } => {
                    for handling in handling {
                        let step = woven
                            .weave
                            .step(handling.step)
                            .expect("the run names a step of the plan");
                        let again = Asked {
                            text: handling.input.clone(),
                            tags: Vec::new(),
                            only: step.reflex.clone(),
                            whole: false,
                        };
                        let decided = rewritten
                            .iter()
                            .find(|(asked, _)| *asked == again)
                            .map(|(_, decided)| decided.clone())
                            .or_else(|| woven.planned(step, &self.tags))
                            .expect("every round has its decision");
                        // Ctrl-C noted: a round handed after it never starts, and reads skipped · cancelled.
                        if interrupt::interrupted() {
                            progress
                                .handled
                                .push(self.cancelled(input, step, of, &handling, &decided));
                            continue;
                        }
                        let handed = Handed {
                            bound: &handling.bound,
                            taken: &handling.taken,
                            shared: &step.shared,
                            from: &step.from,
                            when: step.when.as_ref(),
                        };
                        let rounded = self.round(
                            input,
                            Some((handling.step, of)),
                            &decided,
                            handling.decision.clone(),
                            handed,
                        );
                        progress.handled.push(Handled {
                            step: handling.step,
                            round: handling.round,
                            status: rounded.status,
                            why: rounded.why,
                            result: rounded.result.map(|returned| Yielded {
                                text: returned.text,
                                data: returned.data,
                            }),
                        });
                        exits.push(rounded.exit);
                    }
                }
            }
        }
    }

    /// A round handed after Ctrl-C was noted: it never starts, and its line — logged, printed under `--json` —
    /// reads `skipped · cancelled`.
    fn cancelled(
        &self,
        input: &str,
        step: &Step,
        of: usize,
        handling: &Handling,
        decided: &Decided,
    ) -> Handled {
        let mut line = self.line(decided);
        line.decision = handling.decision.clone();
        line.step = Some(StepLine {
            n: handling.step,
            of,
            status: Status::Skipped,
            why: Some(Stopped::Cancelled),
            bound: handling.bound.clone(),
            shared: step.shared.clone(),
            from: step.from.clone(),
            when: step.when.clone(),
        });
        self.logged(input, &line, Exit::Ran);
        Handled {
            step: handling.step,
            round: handling.round,
            status: Status::Skipped,
            why: Some(Stopped::Cancelled),
            result: None,
        }
    }

    /// The run over: every step that never ran said so — refused once its bound values were in its words, or
    /// skipped, cancelled — its line logged unless its round was; then the worst step's exit — the one already
    /// reported at its turn, or a source that yielded nothing its taker could use.
    fn ended(
        &self,
        input: &str,
        woven: &Woven,
        executed: &Executed,
        rewritten: &[(Asked, Decided)],
        exits: Vec<Exit>,
    ) -> Exit {
        let of = woven.weave.steps.len();
        for outcome in &executed.steps {
            if !matches!(outcome.status, Status::Skipped | Status::Refused) {
                continue;
            }
            let Some(step) = woven.weave.step(outcome.step) else {
                continue;
            };
            // A step refused with its values in its words was decided again on them: that decision is its line.
            let refused = (outcome.status == Status::Refused)
                .then(|| rewritten_text(step, &woven.weave, &outcome.bound));
            let decided = match &refused {
                Some(text) => rewritten
                    .iter()
                    .find(|(asked, _)| asked.text == *text && asked.only == step.reflex)
                    .or_else(|| {
                        rewritten.iter().rev().find(|(asked, decided)| {
                            asked.only == step.reflex
                                && matches!(decided.decision, Decision::Abstain { .. })
                        })
                    })
                    .map(|(_, decided)| decided.clone()),
                None => None,
            }
            .or_else(|| woven.planned(step, &self.tags));
            let Some(decided) = decided else {
                continue;
            };
            if !self.json {
                let body = if let (Some(text), Some(why)) = (&refused, &outcome.why) {
                    report::step_refused(text, why)
                } else {
                    let mut body = report::step_body(step, &woven.weave);
                    body.push(" · skipped");
                    match &outcome.why {
                        Some(Stopped::Cancelled) => {
                            body.push(" · cancelled");
                        }
                        Some(why @ Stopped::NotChosen { from, .. }) => {
                            body.push(" · ")
                                .push(&report::stopped(why, step.when.as_ref()));
                            // No step the source picks among was chosen: a value no step lists.
                            if none_chosen(&woven.weave, executed, *from) {
                                body.push(", which no step lists");
                            }
                        }
                        _ => {}
                    }
                    body
                };
                terminal::note(&report::step(outcome.step, of, body));
            }
            // A round the host reported so — the one Ctrl-C ended, or one handed after it — was logged at its
            // turn: the step's line is not logged twice.
            if outcome
                .rounds
                .last()
                .is_some_and(|round| round.status == outcome.status)
            {
                continue;
            }
            let mut line = self.line(&decided);
            line.step = Some(StepLine {
                n: outcome.step,
                of,
                status: outcome.status,
                why: outcome.why.clone(),
                bound: outcome.bound.clone(),
                shared: step.shared.clone(),
                from: step.from.clone(),
                when: step.when.clone(),
            });
            let exit = if outcome.status == Status::Refused {
                Exit::Declined(Decline::Abstained)
            } else {
                Exit::Ran
            };
            self.logged(input, &line, exit);
        }
        match executed.worst {
            Status::Ran | Status::Skipped => Exit::Ran,
            Status::Failed => exits
                .into_iter()
                .find(|exit| matches!(exit, Exit::Failed(_) | Exit::Adapter(_)))
                .unwrap_or_else(|| self.unhanded(input, woven, executed)),
            Status::Declined => Exit::Declined(Decline::Refused),
            Status::Refused => Exit::Declined(Decline::Abstained),
            Status::Unanswered => exits
                .into_iter()
                .find(|exit| matches!(exit, Exit::Human(_)))
                .unwrap_or(Exit::Declined(Decline::Refused)),
        }
    }

    /// The whole's exit when a step was skipped for what its source returned — nothing it takes, or more than
    /// can be handed: the source is the failure, and the line names it, with the source's reflex to look at.
    fn unhanded(&self, input: &str, woven: &Woven, executed: &Executed) -> Exit {
        let (cause, source) = executed
            .steps
            .iter()
            .find_map(|s| match &s.why {
                Some(Stopped::NothingToTake { from }) => Some((
                    format!("step {from} returned nothing step {} takes", s.step),
                    *from,
                )),
                Some(Stopped::TooLarge { from }) => Some((
                    format!(
                        "step {from} returned more than 1 MiB, which step {} cannot take",
                        s.step
                    ),
                    *from,
                )),
                _ => None,
            })
            .unwrap_or_default();
        let failed = Exit::Failed(crate::hosts::Failure {
            what: "running the plan".to_owned(),
            cause: Some(cause),
            fix: Fix::Show {
                reflex: woven.weave.step(source).and_then(|s| s.reflex.clone()),
            },
        });
        self.session.reporter.exit(input, failed)
    }

    /// `[t]each`: the overlay line for what the utterance stated, written when the file still reads; a refusal —
    /// an utterance the core will not file, a lesson it cannot type, a file that will not take it — is printed,
    /// and the confirmation stands: the person said yes to the call.
    fn teach(&mut self, input: &str, chosen: &Chosen) {
        let refused = |message: String| {
            Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message,
                fix: Fix::Rerun,
            })
        };
        let lesson = Lesson::stated(chosen);
        if self.session.taught_already(&lesson, input) {
            terminal::note(&report::already_example(input, &lesson.reflex));
            return;
        }
        let taught = Utterance::new(input)
            .map_err(|why| refused(format!("{} {why}", report::quoted(input))))
            .and_then(|utterance| {
                teach(&utterance, lesson, &self.session.plan).map_err(Exit::Human)
            })
            .and_then(|edit| self.session.apply(input, &edit));
        if let Err(exit) = taught {
            self.session.reporter.exit(input, exit);
        }
    }

    /// The prompt needs a terminal and there is none: under `--json` the decision line stands for the problem;
    /// else the fix prints.
    fn no_terminal(&self, input: &str, line: &mut Line, what: &str) -> Rounded {
        let human = Exit::Human(needs_terminal(what));
        let why = why_of(&human);
        if let Some(step) = &mut line.step {
            step.status = Status::Unanswered;
            step.why.clone_from(&why);
        }
        if self.json {
            terminal::result(&line.json());
            let _ = self.session.state.log(&line.log());
            return Rounded {
                exit: human,
                status: Status::Unanswered,
                why,
                result: None,
            };
        }
        let exit = self.logged(input, line, human);
        Rounded {
            exit,
            status: Status::Unanswered,
            why,
            result: None,
        }
    }

    /// The line into the log — and, under `--json`, to stdout — then the exit, reported. A body's failure is the
    /// line's own `error`: under `--json` the exit prints nothing more, so a line stays one object.
    fn logged(&self, input: &str, line: &Line, exit: Exit) -> Exit {
        if self.json {
            terminal::result(&line.json());
        }
        let exit = match self.session.state.log(&line.log()) {
            Err(failure) if exit == Exit::Ran => Exit::Failed(failure),
            _ => exit,
        };
        if self.json && line.error.is_some() && matches!(exit, Exit::Failed(_)) {
            return exit;
        }
        self.session.reporter.exit(input, exit)
    }

    /// Every missing argument asked in turn; the question declined at the end of input, when one was.
    pub fn answers(
        &mut self,
        input: &str,
        reflex: &LocalName,
        missing: &NonEmpty<Missing>,
    ) -> Result<Answered, Exit> {
        let mut given = IndexMap::new();
        for missing in missing.iter() {
            let vocabulary = self.vocabulary(reflex, &missing.arg);
            let Some(value) = self.asked(input, missing, vocabulary.as_ref())? else {
                let ask = missing.ask.to_string();
                return Ok(Answered::Declined { ask });
            };
            given.insert(missing.arg.clone(), value);
        }
        Ok(Answered::Given(given))
    }

    /// One missing argument asked until it has a value; none at the end of input. `+` at a vocabulary's prompt
    /// adds a word first.
    fn asked(
        &mut self,
        input: &str,
        missing: &Missing,
        vocabulary: Option<&VocabName>,
    ) -> Result<Option<Value>, Exit> {
        let mut retry: Option<String> = match &missing.because {
            Why::Unstated => None,
            Why::OutOfRange { .. } => Some(report::because(&missing.because)),
        };
        loop {
            let prompt = report::ask_prompt(missing, retry.as_deref());
            let Some(typed) = self.session.prompt(&prompt)? else {
                return Ok(None);
            };
            let typed = typed.trim();
            if typed.is_empty() {
                retry = None;
                continue;
            }
            match &missing.choices {
                Choices::Options { options } => {
                    let keys: Vec<&str> = options.keys().map(OptionKey::as_str).collect();
                    if let Some(at) = chosen_from(typed, &keys) {
                        let (key, _) = options.get_index(at).expect("the index is in range");
                        return Ok(Some(Value::Option { key: key.clone() }));
                    }
                }
                Choices::Vocab { words } => {
                    if typed == "+"
                        && let Some(vocabulary) = vocabulary
                    {
                        return Ok(self
                            .added(input, vocabulary, words)?
                            .map(|word| Value::Word { word, value: None }));
                    }
                    let keys: Vec<&str> = words.keys().map(Word::as_str).collect();
                    if let Some(at) = chosen_from(typed, &keys) {
                        let (word, _) = words.get_index(at).expect("the index is in range");
                        return Ok(Some(Value::Word {
                            word: word.clone(),
                            value: None,
                        }));
                    }
                }
                Choices::Pick { pick, recent } => {
                    // A recalled value by its number, as an options prompt reads one; a number pick names them
                    // as hints only, since a number typed is its own answer.
                    let listed = recent
                        .as_deref()
                        .filter(|_| *pick != Recognizer::Number)
                        .unwrap_or_default();
                    let keys: Vec<&str> = listed.iter().map(String::as_str).collect();
                    let chosen = chosen_from(typed, &keys).map_or(typed, |at| keys[at]);
                    if let Some(value) = picked(chosen, *pick) {
                        return Ok(Some(value));
                    }
                    retry = Some(format!("{} is not {}", report::quoted(typed), pick.wants()));
                    continue;
                }
            }
            retry = Some(format!("{} is not one of them", report::quoted(typed)));
        }
    }

    /// `[+] add one`: the word and its meaning asked — and its path, where a body's declaration takes the word's
    /// value as one — written to the vocabulary, the plan reloaded; a word already there under identity is taken
    /// as it is. None at the end of input.
    fn added(
        &mut self,
        input: &str,
        vocabulary: &VocabName,
        words: &IndexMap<Word, Clean>,
    ) -> Result<Option<Word>, Exit> {
        let mut retry = None;
        let word = loop {
            let Some(typed) = self
                .session
                .prompt(&report::word_prompt(retry.as_deref()))?
            else {
                return Ok(None);
            };
            let typed = typed.trim();
            if typed.is_empty() {
                continue;
            }
            let id = identity(typed);
            if let Some(word) = words.keys().find(|word| identity(word.as_str()) == id) {
                return Ok(Some(word.clone()));
            }
            match Word::new(typed) {
                Ok(word) => break word,
                Err(why) => retry = Some(report::plain(&why)),
            }
        };
        let mut retry = None;
        let what = loop {
            let Some(typed) = self
                .session
                .prompt(&report::meaning_prompt(retry.as_deref()))?
            else {
                return Ok(None);
            };
            let typed = typed.trim();
            if typed.is_empty() {
                continue;
            }
            match Clean::line(typed) {
                Ok(what) => break what,
                Err(why) => retry = Some(format!("{} {why}", report::quoted(typed))),
            }
        };
        let value = if self.pathed(vocabulary) {
            let mut retry = None;
            loop {
                let Some(typed) = self
                    .session
                    .prompt(&report::path_prompt(retry.as_deref()))?
                else {
                    return Ok(None);
                };
                let typed = typed.trim();
                if typed.is_empty() {
                    continue;
                }
                if !typed.starts_with('/') && !typed.starts_with("~/") {
                    retry = Some(format!("{} is not a path", report::quoted(typed)));
                    continue;
                }
                match Clean::line(typed) {
                    Ok(path) => break Some(path.to_string()),
                    Err(why) => retry = Some(format!("{} {why}", report::quoted(typed))),
                }
            }
        } else {
            None
        };
        let change = VocabChange::Add {
            word: word.clone(),
            meaning: Meaning { what, value },
        };
        self.session
            .apply(input, &vocab_edit(vocabulary.clone(), change))?;
        self.session.reload(input)?;
        Ok(Some(word))
    }

    /// Whether an active reflex's declaration takes a word of the vocabulary as a path: `reads = ["{place}"]`
    /// over an argument that draws from it.
    fn pathed(&self, vocabulary: &VocabName) -> bool {
        self.session.plan.active().values().any(|active| {
            active
                .needs
                .reads
                .iter()
                .chain(&active.needs.writes)
                .any(|entry| match entry {
                    Entry::Value(name) => matches!(
                        active.args.get(name.as_str()).map(|argument| &argument.kind),
                        Some(Kind::Value { source: Source::Vocab(named), .. }) if named == vocabulary
                    ),
                    Entry::Home(_) | Entry::Absolute(_) => false,
                })
        })
    }

    /// The vocabulary an argument draws from, when it draws from one.
    fn vocabulary(&self, reflex: &LocalName, arg: &ArgName) -> Option<VocabName> {
        let argument = self.session.plan.active().get(reflex)?.args.get(arg)?;
        match &argument.kind {
            Kind::Value {
                source: Source::Vocab(name),
                ..
            } => Some(name.clone()),
            _ => None,
        }
    }
}

/// The fix when the plan refuses: for a whole result no step hands, the reflex to look at — one installed that
/// returns the name, else the taker itself, whose `show` says what it takes; for a playbook's refusal, the
/// playbook; for a part left out beside a plan, the sentence again.
pub fn refusal_fix(session: &Session<'_>, because: &Because, woven: &Woven) -> Fix {
    let taker = step_of(because)
        .and_then(|n| woven.weave.step(n))
        .and_then(|step| step.reflex.clone());
    let reflex = match because {
        Because::NoSource { name, .. } => session
            .plan
            .active()
            .iter()
            .find(|(_, active)| active.returns.as_ref() == Some(name))
            .map(|(reflex, _)| reflex.clone())
            .or(taker),
        Because::Nested { playbook, .. }
        | Because::TooDeep { playbook, .. }
        | Because::TooLong { playbook, .. } => Some(playbook.clone()),
        Because::Excluded { .. } => return Fix::Rerun,
        // A step that may not run is the author's: the playbook that lists it.
        Because::BranchIntoPlan { .. }
        | Because::NoField { .. }
        | Because::BadValue { .. }
        | Because::MaybeSource { .. } => step_of(because)
            .and_then(|n| woven.weave.step(n))
            .and_then(|step| step.from.last())
            .map(|from| from.playbook.clone())
            .or(taker),
        _ => taker,
    };
    Fix::Show { reflex }
}

/// Whether no step a source picks among was chosen: every step waiting on it was skipped as not chosen, so the
/// source yielded a value no step lists.
fn none_chosen(weave: &Weave, executed: &Executed, source: usize) -> bool {
    executed.steps.iter().all(|outcome| {
        weave
            .step(outcome.step)
            .and_then(|step| step.when.as_ref())
            .is_none_or(|when| when.step != source)
            || matches!(outcome.why, Some(Stopped::NotChosen { .. }))
    })
}

/// The yes over the whole plan, teachable: the part of the sentence that picked the playbook, and the playbook's
/// decision, taught as an example at `[t]each`.
pub struct Teach {
    pub spoken: String,
    pub chosen: Chosen,
}

/// The template of the yes over a plan that is no one playbook's: a file's, or a typed plan of several parts.
#[must_use]
pub fn whole_plan() -> Clean {
    Clean::new("Run the plan as it stands?").expect("a clean line")
}

/// What the plan handed a round beside its decision: the values bound into it, the whole results it takes, the
/// words shared into its step, the playbooks its step came from, and what picks the step when it may not run.
#[derive(Clone, Copy)]
pub struct Handed<'a> {
    pub bound: &'a [Bound],
    pub taken: &'a IndexMap<ArgName, Json>,
    pub shared: &'a IndexMap<ArgName, Shared>,
    pub from: &'a [From],
    pub when: Option<&'a When>,
}

/// What became of one round of a step: its exit, already reported; its status and why it stopped; what its body
/// returned.
pub struct Rounded {
    pub exit: Exit,
    pub status: Status,
    pub why: Option<Stopped>,
    pub result: Option<Returned>,
}

/// What a step's questions came to: every value, or the question declined at the end of input.
pub enum Answered {
    Given(IndexMap<ArgName, Value>),
    Declined { ask: String },
}

/// The step's words with its bound values written in, as the run decided them again; a whole result carries no
/// value and never reaches the words.
fn rewritten_text(step: &Step, weave: &Weave, bound: &[Bound]) -> String {
    let values: Vec<(Binding, String)> = bound
        .iter()
        .filter_map(|b| {
            let binding = weave.binds.iter().find(|binding| {
                binding.to == step.n
                    && binding.arg == b.arg
                    && binding.from == b.from
                    && binding.field == b.field
            })?;
            Some((binding.clone(), b.value.clone()?))
        })
        .collect();
    weave::running::rewrite(step, &values)
}

/// The step a refusal is about: the one that takes what no step, or several, hand it; the one that routes to a
/// plan it stands in, or too deep; the one that may not run, or takes from one. None for a refusal of the whole.
fn step_of(because: &Because) -> Option<usize> {
    match because {
        Because::NoSource { step, .. }
        | Because::SeveralSources { step, .. }
        | Because::Nested { step, .. }
        | Because::TooDeep { step, .. }
        | Because::BranchIntoPlan { step, .. }
        | Because::NoField { step, .. }
        | Because::BadValue { step, .. }
        | Because::MaybeSource { step, .. } => Some(*step),
        _ => None,
    }
}

/// A step's status as its exit ranks it.
fn status_of(exit: &Exit) -> Status {
    match exit {
        Exit::Ran => Status::Ran,
        Exit::Failed(_) | Exit::Adapter(_) => Status::Failed,
        Exit::Declined(Decline::Abstained) => Status::Refused,
        Exit::Declined(Decline::Refused) => Status::Declined,
        Exit::Human(_) => Status::Unanswered,
    }
}

/// Why a step stopped, in the exit's own words, where the exit says.
fn why_of(exit: &Exit) -> Option<Stopped> {
    match exit {
        Exit::Ran | Exit::Declined(Decline::Refused) => None,
        Exit::Declined(Decline::Abstained) => Some(Stopped::NoReflex),
        Exit::Failed(failure) => Some(Stopped::Said {
            message: report::failure(failure),
        }),
        Exit::Human(problem) => Some(Stopped::Said {
            message: problem.message.clone(),
        }),
        Exit::Adapter(fault) => Some(Stopped::Said {
            message: fault.to_string(),
        }),
    }
}

/// What an ask offered back from the process's results, per argument: the line's record of it, for `why`.
fn recalled_in(missing: &NonEmpty<Missing>) -> impl Iterator<Item = (ArgName, Vec<String>)> {
    missing.iter().filter_map(|missing| match &missing.choices {
        Choices::Pick {
            recent: Some(values),
            ..
        } => Some((missing.arg.clone(), values.clone())),
        _ => None,
    })
}

/// A number from 1, or the choice's own text.
fn chosen_from(typed: &str, keys: &[&str]) -> Option<usize> {
    if let Ok(number) = typed.parse::<usize>()
        && (1..=keys.len()).contains(&number)
    {
        return Some(number - 1);
    }
    keys.iter().position(|key| *key == typed)
}
