//! `evoke "<input>"`: read into steps, decide, gate, run. In: the parsed command and the environment. Out: `Exit`.
//! Each input is read for its steps on the session; one step is decided as it always was — a run starts the body
//! under its declaration; a confirm and an ask prompt on the terminal, `[t]each` writing the overlay
//! first, `[+]` adding a word to the vocabulary and reloading the plan; an abstain shows the ranking. More than
//! one is a weave: the plan's own questions first, the plan shown, then each step at its turn through the same
//! loop, a result threaded into a later step, and the worst step's exit. Every decision is logged, a step's with
//! its number and what became of it, a plan stopped before any step ran included. Ctrl-C while a body runs, or
//! while a weave's steps take their turns, ends the body's group, marks every step that did not finish `skipped ·
//! cancelled`, logs every line, and ends the process as an interrupted one. The stdin filter answers every line
//! and exits with the first non-zero code; the REPL reads lines from the terminal — edited, with its history under
//! XDG — until the end of input, then exits 0.

use evoke_core::call::Value;
use evoke_core::decide::{Choices, Missing, Why};
use evoke_core::manifest::{Kind, Source};
use evoke_core::name::{ArgName, LocalName, OptionKey, VocabName, Word};
use evoke_core::text::NonEmpty;
use evoke_core::vocabulary::Meaning;
use evoke_core::weave::{
    self, Asked, Because, Binding, Bound, Handled, Handling, Outcome, Progress,
    Returned as Yielded, Status, Step, Todo, Why as Stopped,
};
use evoke_core::{
    Chosen, Clean, Decision, Diagnostic, Executed, Fix, Gate, Lesson, Prompt, Running, Utterance,
    VocabChange, Weave, fill, identity, picked, teach, vocab_edit,
};
use indexmap::IndexMap;

use super::session::{self, Confirmed, Decided, Opening, Session, Woven};
use super::{Decline, Exit};
use super::{each_line, needs_terminal};
use crate::adapter::Adapter;
use crate::args::{Arguments, Command, Inputs};
use crate::hosts::processes::Returned;
use crate::hosts::terminal::Text;
use crate::hosts::{Environment, Failure, interrupt, terminal};
use crate::report::{self, Line, StepLine};

pub fn run(command: &Command, arguments: &Arguments, environment: &Environment) -> Exit {
    let session = match session::open(command, arguments.json, environment, Opening::Deciding) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let adapter = match session.adapter(&command.stand_in()) {
        Ok(adapter) => adapter,
        Err(exit) => return session.reporter.exit(&command.stand_in(), exit),
    };
    let mut using = Using {
        session,
        adapter,
        arguments,
    };
    match &arguments.input {
        Inputs::One(input) => using.act(input),
        Inputs::Stdin => each_line(arguments.json, |input| using.act(input)),
        Inputs::Terminal => using.repl(),
    }
}

/// The session with what deciding needs beside it: the adapter that answers, and the arguments.
struct Using<'a> {
    session: Session<'a>,
    adapter: Box<dyn Adapter>,
    arguments: &'a Arguments,
}

impl Using<'_> {
    /// The adapter's thresholds, when it ships them.
    fn floors(&self) -> Option<&Gate> {
        self.adapter.declared().gate.as_ref()
    }

    /// The REPL: one line at a time from the terminal, edited and remembered where it can be; a blank line is
    /// nothing; the end of input ends the session.
    fn repl(&mut self) -> Exit {
        let opened = self.session.state.history().and_then(|history| {
            terminal::repl(history).ok_or_else(|| Failure {
                what: "opening the terminal".to_owned(),
                cause: None,
                fix: Fix::Rerun,
            })
        });
        let mut repl = match opened {
            Ok(repl) => repl,
            Err(failure) => return self.session.reporter.exit("<input>", Exit::Failed(failure)),
        };
        loop {
            let typed = match repl.line("> ") {
                Ok(Some(typed)) => typed,
                Ok(None) => return Exit::Ran,
                Err(failure) => {
                    return self.session.reporter.exit("<input>", Exit::Failed(failure));
                }
            };
            if typed.trim().is_empty() {
                continue;
            }
            self.act(&typed);
        }
    }

    /// One input, end to end, its exit reported: read into its steps; one step is the foundation's own loop,
    /// more are a weave. Ctrl-C noted on the way — while a body ran, or a weave took its turns — has been acted
    /// on by then, every body ended and every line logged: the process ends as an interrupted one.
    fn act(&mut self, input: &str) -> Exit {
        let woven =
            match self
                .session
                .weave(&*self.adapter, input, &self.arguments.tags, Vec::new())
            {
                Ok(woven) => woven,
                Err(exit) => return self.session.reporter.exit(input, exit),
            };
        let exit = match woven.single(&self.arguments.tags) {
            Some(decided) => {
                let decision = decided.decision.clone();
                self.round(input, None, &decided, decision, &[]).exit
            }
            None => self.many(input, woven),
        };
        if interrupt::interrupted() {
            interrupt::end();
        }
        exit
    }

    /// One decision through the foundation's loop — abstain, ask, confirm, run — as one input takes it, or as one
    /// round of a weave's step does, numbered `at`: the line printed under `--json` and logged, the exit
    /// reported; what became of it, for the weave.
    fn round(
        &mut self,
        input: &str,
        at: Option<(usize, usize)>,
        decided: &Decided,
        decision: Decision,
        bound: &[Bound],
    ) -> Rounded {
        let json = self.arguments.json;
        let mut line = Line::of(decided);
        line.decision = decision.clone();
        line.step = at.map(|(n, of)| StepLine {
            n,
            of,
            status: Status::Ran,
            why: None,
            bound: bound.to_vec(),
        });
        let chosen = match self.readied(input, at, decided, decision, bound, &mut line) {
            Ok(chosen) => chosen,
            Err(rounded) => return rounded,
        };
        line.contained = Some(self.session.contained.clone());
        let ran = self
            .session
            .run(&chosen, &decided.request.state.request, decided.spent());
        match ran {
            Ok(returned) => {
                if !json && !returned.text.is_empty() {
                    terminal::result(&returned.text);
                }
                line.result = Some(returned.clone());
                let mut rounded = self.stopped(input, &mut line, Exit::Ran, None);
                rounded.result = Some(returned);
                rounded
            }
            Err(failure) => {
                line.error = Some(report::failure(&failure));
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
        let json = self.arguments.json;
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
                            let spoken = decided.request.state.request.as_str();
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
                            // in its place, or a machine that does not hold the declaration.
                            Some((n, of)) if !bound.is_empty() || !contained.is_full() => {
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
    /// `skipped · cancelled`, one decision keeps the body's failure as its `error`, and nothing more prints — the
    /// process ends once every line is logged.
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

    /// A weave: the plan's own questions first — a step's argument nothing binds, asked as at its turn, and
    /// again while the answer is out of range; a reference that takes nothing, confirmed — then the plan shown,
    /// and each step at its turn through the foundation's own loop; the worst step's exit. A request that is
    /// only what not to do is nothing to do. A plan stopped before any step ran logs every step's line with what
    /// stopped it.
    fn many(&mut self, input: &str, woven: Woven) -> Exit {
        let json = self.arguments.json;
        if woven.weave.steps.is_empty() {
            if json {
                // One line per input holds under --json: an abstain that judged nothing.
                terminal::result(&report::nothing_to_do_json(input));
            } else {
                terminal::note(&report::nothing_to_do());
            }
            return Exit::Declined(Decline::Refused);
        }
        let woven = match self.settled(input, woven) {
            Ok(woven) => woven,
            Err(exit) => return exit,
        };
        if !json {
            terminal::note(&report::planned(&woven.weave));
        }
        if woven.weave.verdict.outcome == Outcome::Refuse {
            return self.refused(input, &woven);
        }
        if woven.weave.verdict.outcome == Outcome::Confirm {
            if !self.session.has_tty() {
                return self.unanswered(input, &woven, needs_terminal("a confirm"));
            }
            let reasons: Vec<String> = woven
                .weave
                .verdict
                .because
                .iter()
                .map(report::verdict)
                .collect();
            let prompt = Prompt {
                own: reasons.join("; "),
                template: Clean::new("Run the plan as it stands?").expect("a clean line"),
            };
            let own = Text::from(format!("  {}", prompt.own));
            match self.session.confirmed(&own, &prompt, false) {
                Ok(Some(Confirmed::Yes)) => {}
                Ok(_) => {
                    let exit = Exit::Declined(Decline::Refused);
                    return self.stopped_whole(input, &woven, exit, |_| {
                        let message = prompt.own.clone();
                        (Status::Declined, Stopped::Said { message })
                    });
                }
                Err(exit) => return self.session.reporter.exit(input, exit),
            }
        }
        self.executed(input, &woven)
    }

    /// The plan settled: asked up front while it asks, each answer standing in for the planner's own decision
    /// of its step when the plan is read again. A stop is reported here: the question declined, or one no one
    /// can answer, every step's line saying so.
    fn settled(&mut self, input: &str, woven: Woven) -> Result<Woven, Exit> {
        let tags = self.arguments.tags.clone();
        let mut woven = woven;
        let mut seeds: Vec<(Asked, Decided)> = Vec::new();
        let mut shown: Vec<usize> = Vec::new();
        while woven.weave.verdict.outcome == Outcome::Ask {
            let fresh = match self.asked_up_front(&woven, &mut shown) {
                Ok(UpFront::Seeded(fresh)) => fresh,
                Ok(UpFront::Declined { step, ask }) => {
                    let exit = Exit::Declined(Decline::Refused);
                    return Err(self.stopped_whole(input, &woven, exit, |s| {
                        if s.n == step {
                            let message = ask.clone();
                            (Status::Declined, Stopped::Said { message })
                        } else {
                            let message = PLAN_DECLINED.to_owned();
                            (Status::Skipped, Stopped::Said { message })
                        }
                    }));
                }
                Err(Exit::Human(problem)) => return Err(self.unanswered(input, &woven, problem)),
                Err(exit) => return Err(self.session.reporter.exit(input, exit)),
            };
            for seed in fresh {
                match seeds.iter_mut().find(|(asked, _)| *asked == seed.0) {
                    Some(held) => *held = seed,
                    None => seeds.push(seed),
                }
            }
            let again = self
                .session
                .weave(&*self.adapter, input, &tags, seeds.clone())
                .map_err(|exit| self.session.reporter.exit(input, exit))?;
            // An answer the plan did not take — it stands exactly as before — is a stop, never a loop. One it
            // took and asks about again, out of range, is asked again with the reason, as one input is.
            let taken = !seeds.is_empty()
                && seeds.iter().all(|(_, seeded)| {
                    again
                        .weave
                        .steps
                        .iter()
                        .any(|step| step.decision == seeded.decision)
                });
            if !taken {
                let problem = Diagnostic {
                    reflex: None,
                    at: None,
                    message: "the answer did not settle the plan".to_owned(),
                    fix: Fix::Rerun,
                };
                return Err(self.unanswered(input, &again, problem));
            }
            woven = again;
        }
        Ok(woven)
    }

    /// The plan's own questions before anything runs. A step's required argument no binding covers is asked as
    /// it would be at its turn, the ask narrowed to what nothing binds, and the step's decision filled for the
    /// plan to stand again; the step's line is shown ahead of its first question, and `shown` remembers it.
    /// Several fields, or one record of several, no answer here can settle: a stop that names them.
    fn asked_up_front(&mut self, woven: &Woven, shown: &mut Vec<usize>) -> Result<UpFront, Exit> {
        let tags = self.arguments.tags.clone();
        let json = self.arguments.json;
        let of = woven.weave.steps.len();
        let unsettled = woven
            .weave
            .verdict
            .because
            .iter()
            .find(|b| matches!(b, Because::Several { .. } | Because::OneOfMany { .. }));
        if let Some(because) = unsettled {
            return Err(Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message: report::verdict(because),
                fix: Fix::Rerun,
            }));
        }
        let mut needs: Vec<usize> = Vec::new();
        for because in &woven.weave.verdict.because {
            if let Because::Needs { step, .. } = because
                && !needs.contains(step)
            {
                needs.push(*step);
            }
        }
        let mut seeded = Vec::new();
        for n in needs {
            let step = woven
                .weave
                .step(n)
                .expect("the verdict names a step of the plan");
            let Decision::Ask { asking, missing } = &step.decision else {
                continue;
            };
            let bound: Vec<&ArgName> = woven
                .weave
                .binds
                .iter()
                .filter(|b| b.to == n)
                .map(|b| &b.arg)
                .collect();
            let mut unbound: Vec<Missing> = missing
                .iter()
                .filter(|m| !bound.contains(&&m.arg))
                .cloned()
                .collect();
            if unbound.is_empty() {
                continue;
            }
            if !self.session.has_tty() {
                return Err(Exit::Human(needs_terminal("an ask")));
            }
            let first = unbound.remove(0);
            let asks = NonEmpty::new(first, unbound);
            if !json && !shown.contains(&n) {
                terminal::note(&report::step(n, of, report::step_body(step, &woven.weave)));
                shown.push(n);
            }
            let given = match self.answers(&step.text, &asking.reflex, &asks)? {
                Answered::Given(given) => given,
                Answered::Declined { ask } => return Ok(UpFront::Declined { step: n, ask }),
            };
            let filled = fill(&self.session.plan, asking.clone(), given, self.floors());
            let mut decided = woven
                .decided_for(step, &tags)
                .expect("every step was decided")
                .clone();
            decided.decision = filled;
            seeded.push((weave::asked_for(step, &tags), decided));
        }
        Ok(UpFront::Seeded(seeded))
    }

    /// A plan refused whole: nothing runs; each step's line, the abstaining one refused, the rest skipped.
    fn refused(&self, input: &str, woven: &Woven) -> Exit {
        if !self.arguments.json
            && let Some(hint) = report::left_out(self.session.plan.inactive().keys())
        {
            terminal::note(&hint);
        }
        let exit = Exit::Declined(Decline::Abstained);
        self.stopped_whole(input, woven, exit, |step| {
            if matches!(step.decision, Decision::Abstain { .. }) {
                (Status::Refused, Stopped::NoReflex)
            } else {
                let message = "the request was refused".to_owned();
                (Status::Skipped, Stopped::Said { message })
            }
        })
    }

    /// A question only a person can answer, and none did: every step's line `unanswered` with it.
    fn unanswered(&self, input: &str, woven: &Woven, problem: Diagnostic) -> Exit {
        let message = problem.message.clone();
        self.stopped_whole(input, woven, Exit::Human(problem), |_| {
            let message = message.clone();
            (Status::Unanswered, Stopped::Said { message })
        })
    }

    /// The plan stopped before any step ran: each step's line with what became of it — printed under `--json`,
    /// logged — then the exit, reported once. Under `--json` a diagnostic is already every line's `why`, so
    /// nothing more prints.
    fn stopped_whole(
        &self,
        input: &str,
        woven: &Woven,
        exit: Exit,
        became: impl Fn(&Step) -> (Status, Stopped),
    ) -> Exit {
        let tags = self.arguments.tags.clone();
        let of = woven.weave.steps.len();
        for step in &woven.weave.steps {
            let Some(decided) = woven.decided_for(step, &tags) else {
                continue;
            };
            let mut line = Line::of(decided);
            line.decision = step.decision.clone();
            let (status, why) = became(step);
            line.step = Some(StepLine {
                n: step.n,
                of,
                status,
                why: Some(why),
                bound: Vec::new(),
            });
            if self.arguments.json {
                terminal::result(&line.json());
            }
            if let Err(failure) = self.session.state.log(&line.log()) {
                return self.session.reporter.exit(input, Exit::Failed(failure));
            }
        }
        if self.arguments.json && matches!(exit, Exit::Human(_)) {
            return exit;
        }
        self.session.reporter.exit(input, exit)
    }

    /// The plan run: stage by stage, each round through the foundation's loop; a step decided again with its
    /// bound values in its words when the run asks for it; what did not run, said so at the end. Armed: Ctrl-C
    /// from here on is a cancel the run acts on — the round under way ended and reported `skipped · cancelled`,
    /// no round started after it — not the end of the process.
    fn executed(&mut self, input: &str, woven: &Woven) -> Exit {
        let _armed = interrupt::arm();
        let tags = self.arguments.tags.clone();
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
                        self.session.decide(
                            &*self.adapter,
                            &asked.text,
                            &asked.tags,
                            asked.only.as_ref(),
                        )
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
                        };
                        let decided = rewritten
                            .iter()
                            .find(|(asked, _)| *asked == again)
                            .map(|(_, decided)| decided)
                            .or_else(|| woven.decided_for(step, &tags))
                            .expect("every round has its decision")
                            .clone();
                        // Ctrl-C noted: a round handed after it never starts, and reads skipped · cancelled.
                        if interrupt::interrupted() {
                            progress
                                .handled
                                .push(self.cancelled(input, of, &handling, &decided));
                            continue;
                        }
                        let rounded = self.round(
                            input,
                            Some((handling.step, of)),
                            &decided,
                            handling.decision.clone(),
                            &handling.bound,
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
    fn cancelled(&self, input: &str, of: usize, handling: &Handling, decided: &Decided) -> Handled {
        let mut line = Line::of(decided);
        line.decision = handling.decision.clone();
        line.step = Some(StepLine {
            n: handling.step,
            of,
            status: Status::Skipped,
            why: Some(Stopped::Cancelled),
            bound: handling.bound.clone(),
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
        let tags = self.arguments.tags.clone();
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
                    .map(|(_, decided)| decided),
                None => None,
            }
            .or_else(|| woven.decided_for(step, &tags));
            let Some(decided) = decided else {
                continue;
            };
            if !self.arguments.json {
                let body = if let (Some(text), Some(why)) = (&refused, &outcome.why) {
                    report::step_refused(text, why)
                } else {
                    let mut body = report::step_body(step, &woven.weave);
                    body.push(" · skipped");
                    if outcome.why == Some(Stopped::Cancelled) {
                        body.push(" · cancelled");
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
            let mut line = Line::of(decided);
            if refused.is_none() {
                line.decision = step.decision.clone();
            }
            line.step = Some(StepLine {
                n: outcome.step,
                of,
                status: outcome.status,
                why: outcome.why.clone(),
                bound: outcome.bound.clone(),
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
                .unwrap_or_else(|| {
                    let source = executed
                        .steps
                        .iter()
                        .find(|s| matches!(s.why, Some(Stopped::NothingToTake)))
                        .and_then(|s| woven.weave.binds.iter().find(|b| b.to == s.step))
                        .and_then(|b| woven.weave.step(b.from))
                        .and_then(|s| s.reflex.clone());
                    let failed = Exit::Failed(Failure {
                        what: "running the plan".to_owned(),
                        cause: Some("a step yielded nothing the next could take".to_owned()),
                        fix: Fix::Show { reflex: source },
                    });
                    self.session.reporter.exit(input, failed)
                }),
            Status::Declined => Exit::Declined(Decline::Refused),
            Status::Refused => Exit::Declined(Decline::Abstained),
            Status::Unanswered => exits
                .into_iter()
                .find(|exit| matches!(exit, Exit::Human(_)))
                .unwrap_or(Exit::Declined(Decline::Refused)),
        }
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
        let taught = Utterance::new(input)
            .map_err(|why| refused(format!("{} {why}", report::quoted(input))))
            .and_then(|utterance| {
                teach(&utterance, Lesson::stated(chosen), &self.session.plan).map_err(Exit::Human)
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
        if self.arguments.json {
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
        if self.arguments.json {
            terminal::result(&line.json());
        }
        let exit = match self.session.state.log(&line.log()) {
            Err(failure) if exit == Exit::Ran => Exit::Failed(failure),
            _ => exit,
        };
        if self.arguments.json && line.error.is_some() && matches!(exit, Exit::Failed(_)) {
            return exit;
        }
        self.session.reporter.exit(input, exit)
    }

    /// Every missing argument asked in turn; the question declined at the end of input, when one was.
    fn answers(
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
                Choices::Pick { pick } => {
                    if let Some(value) = picked(typed, *pick) {
                        return Ok(Some(value));
                    }
                    retry = Some(format!("{} is not {}", report::quoted(typed), pick.wants()));
                    continue;
                }
            }
            retry = Some(format!("{} is not one of them", report::quoted(typed)));
        }
    }

    /// `[+] add one`: the word and its meaning asked, written to the vocabulary, the plan reloaded; a word already
    /// there under identity is taken as it is. None at the end of input.
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
        let change = VocabChange::Add {
            word: word.clone(),
            meaning: Meaning { what, value: None },
        };
        self.session
            .apply(input, &vocab_edit(vocabulary.clone(), change))?;
        self.session.reload(input)?;
        Ok(Some(word))
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

/// What became of one round of a step: its exit, already reported; its status and why it stopped; what its body
/// returned.
struct Rounded {
    exit: Exit,
    status: Status,
    why: Option<Stopped>,
    result: Option<Returned>,
}

/// What a step's questions came to: every value, or the question declined at the end of input.
enum Answered {
    Given(IndexMap<ArgName, Value>),
    Declined { ask: String },
}

/// The plan's own questions asked up front: the decisions answered into, or the step whose question was declined.
enum UpFront {
    Seeded(Vec<(Asked, Decided)>),
    Declined { step: usize, ask: String },
}

/// Why the other steps never ran when one's question was declined before anything did.
const PLAN_DECLINED: &str = "the plan was declined";

/// The step's words with its bound values written in, as the run decided them again.
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
            Some((binding.clone(), b.value.clone()))
        })
        .collect();
    weave::running::rewrite(step, &values)
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

/// A number from 1, or the choice's own text.
fn chosen_from(typed: &str, keys: &[&str]) -> Option<usize> {
    if let Ok(number) = typed.parse::<usize>()
        && (1..=keys.len()).contains(&number)
    {
        return Some(number - 1);
    }
    keys.iter().position(|key| *key == typed)
}
