//! `evoke "<input>"`: read into steps, decide, gate, run. In: the parsed command and the environment. Out: `Exit`.
//! Each input is read for its steps on the session; one step is decided as it always was — a run feeds the loader
//! warmed at t=0 or spawns the argv; a confirm and an ask prompt on the terminal, `[t]each` writing the overlay
//! first, `[+]` adding a word to the vocabulary and reloading the plan; an abstain shows the ranking. More than
//! one is a weave: the plan's own questions first, the plan shown, then each step at its turn through the same
//! loop, a result threaded into a later step, and the worst step's exit. Every decision is logged, a step's with
//! its number. The stdin filter answers every line and exits with the first non-zero code; the REPL reads lines
//! from the terminal — edited, with its history under XDG — until the end of input, then exits 0.

use evoke_core::call::Value;
use evoke_core::decide::{Choices, Missing, Why};
use evoke_core::manifest::{Kind, Source};
use evoke_core::name::{ArgName, LocalName, OptionKey, VocabName, Word};
use evoke_core::text::NonEmpty;
use evoke_core::vocabulary::Meaning;
use evoke_core::weave::{
    self, Asked, Because, Bound, Handled, Outcome, Progress, Returned as Yielded, Status, Todo,
    Why as Stopped,
};
use evoke_core::{
    Chosen, Clean, Decision, Diagnostic, Executed, Fix, Gate, Lesson, Prompt, Running, Utterance,
    VocabChange, fill, identity, picked, teach, vocab_edit,
};
use indexmap::IndexMap;

use super::session::{self, Confirmed, Decided, Opening, Session, Woven, dismiss};
use super::{Decline, Exit};
use crate::adapter::Adapter;
use crate::args::{Arguments, Command, Inputs};
use crate::hosts::processes::{Returned, Warm};
use crate::hosts::terminal::Text;
use crate::hosts::{Environment, Failure, terminal};
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
        Inputs::Stdin => {
            let mut first = Exit::Ran;
            for line in terminal::stdin_lines() {
                let exit = match line {
                    Ok(input) if input.trim().is_empty() => continue,
                    Ok(input) => using.act(&input),
                    Err(error) => {
                        let failed = Exit::Failed(Failure {
                            what: "reading stdin".to_owned(),
                            cause: Some(error.to_string()),
                            fix: Fix::Rerun,
                        });
                        let exit = using.session.reporter.exit("<input>", failed);
                        return if first == Exit::Ran { exit } else { first };
                    }
                };
                if first == Exit::Ran {
                    first = exit;
                }
            }
            first
        }
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
    /// more are a weave.
    fn act(&mut self, input: &str) -> Exit {
        let warm = match self.session.warm() {
            Ok(warm) => warm,
            Err(exit) => return self.session.reporter.exit(input, exit),
        };
        let woven =
            match self
                .session
                .weave(&*self.adapter, input, &self.arguments.tags, Vec::new())
            {
                Ok(woven) => woven,
                Err(exit) => {
                    dismiss(warm);
                    return self.session.reporter.exit(input, exit);
                }
            };
        match woven.single(&self.arguments.tags).cloned() {
            Some(decided) => {
                let decision = decided.decision.clone();
                self.round(input, None, &decided, decision, &[], warm).exit
            }
            None => self.many(input, woven, warm),
        }
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
        warm: Option<Warm>,
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
        let mut warm = warm;
        let chosen = match self.readied(input, at, decided, decision, bound, &mut line, &mut warm) {
            Ok(chosen) => chosen,
            Err(rounded) => return rounded,
        };
        let ran = self.session.run(
            &chosen,
            &decided.request.state.request,
            decided.spent(),
            warm,
        );
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
    /// confirm is put. The call ready to run, or what became of the round instead, the loader dismissed.
    #[allow(clippy::too_many_arguments)]
    fn readied(
        &mut self,
        input: &str,
        at: Option<(usize, usize)>,
        decided: &Decided,
        decision: Decision,
        bound: &[Bound],
        line: &mut Line,
        warm: &mut Option<Warm>,
    ) -> Result<Chosen, Rounded> {
        let json = self.arguments.json;
        let mut decision = decision;
        loop {
            match decision {
                Decision::Abstain { .. } => {
                    dismiss(warm.take());
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
                        dismiss(warm.take());
                        return Err(self.no_terminal(input, line, "an ask"));
                    }
                    let named: Vec<&str> = missing.iter().map(|m| m.arg.as_str()).collect();
                    let named = named.join(", ");
                    let given = match self.answers(input, &asking.reflex, &missing) {
                        Ok(Some(given)) => given,
                        Ok(None) => {
                            dismiss(warm.take());
                            let exit = Exit::Declined(Decline::Refused);
                            let why = Stopped::Said { message: named };
                            return Err(self.stopped(input, line, exit, Some(why)));
                        }
                        Err(exit) => {
                            dismiss(warm.take());
                            return Err(self.stopped(input, line, exit, None));
                        }
                    };
                    decision = fill(&self.session.plan, asking, given, self.floors());
                    line.decision = decision.clone();
                }
                Decision::Confirm { chosen, prompt, .. } => {
                    if !self.session.has_tty() {
                        dismiss(warm.take());
                        return Err(self.no_terminal(input, line, "a confirm"));
                    }
                    let own = match at {
                        Some((n, of)) => {
                            report::step(n, of, report::step_confirming(&chosen, &prompt))
                        }
                        None => report::confirming(&chosen, &prompt),
                    };
                    match self.session.confirmed(&own, &prompt, true) {
                        Ok(Some(Confirmed::Yes)) => return Ok(chosen),
                        Ok(Some(Confirmed::Teach)) => {
                            let spoken = decided.request.state.request.as_str();
                            if let Err(exit) = self.teach(spoken, &chosen) {
                                dismiss(warm.take());
                                return Err(self.stopped(input, line, exit, None));
                            }
                            return Ok(chosen);
                        }
                        Ok(Some(Confirmed::No) | None) => {
                            dismiss(warm.take());
                            let exit = Exit::Declined(Decline::Refused);
                            let why = Stopped::Said {
                                message: prompt.own.clone(),
                            };
                            return Err(self.stopped(input, line, exit, Some(why)));
                        }
                        Err(exit) => {
                            dismiss(warm.take());
                            return Err(self.stopped(input, line, exit, None));
                        }
                    }
                }
                Decision::Run { chosen } => {
                    if !json {
                        match at {
                            None => terminal::note(&report::running(&chosen)),
                            // The plan showed the step; at its turn, only what the plan could not: a bound value
                            // in its place.
                            Some((n, of)) if !bound.is_empty() => {
                                terminal::note(&report::step(n, of, report::step_running(&chosen)));
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
    /// logged, the exit reported.
    fn stopped(&self, input: &str, line: &mut Line, exit: Exit, why: Option<Stopped>) -> Rounded {
        let why = why.or_else(|| why_of(&exit));
        if let Some(step) = &mut line.step {
            step.status = status_of(&exit);
            step.why.clone_from(&why);
        }
        let exit = self.logged(input, line, exit);
        Rounded {
            exit,
            why,
            result: None,
        }
    }

    /// A weave: the plan's own questions first — a step's argument nothing binds, asked as at its turn; a
    /// reference that takes nothing, confirmed — then the plan shown, and each step at its turn through the
    /// foundation's own loop; the worst step's exit.
    fn many(&mut self, input: &str, woven: Woven, warm: Option<Warm>) -> Exit {
        let tags = self.arguments.tags.clone();
        let json = self.arguments.json;
        let mut woven = woven;
        while woven.weave.verdict.outcome == Outcome::Ask {
            let seeded = match self.asked_up_front(&woven) {
                Ok(Some(seeded)) => seeded,
                Ok(None) => {
                    dismiss(warm);
                    return Exit::Declined(Decline::Refused);
                }
                Err(exit) => {
                    dismiss(warm);
                    return self.session.reporter.exit(input, exit);
                }
            };
            let again = match self.session.weave(&*self.adapter, input, &tags, seeded) {
                Ok(again) => again,
                Err(exit) => {
                    dismiss(warm);
                    return self.session.reporter.exit(input, exit);
                }
            };
            // A plan that asks the same again could not take the answer: a stop, never a loop.
            if again.weave.verdict == woven.weave.verdict {
                dismiss(warm);
                let human = Exit::Human(Diagnostic {
                    reflex: None,
                    at: None,
                    message: "the answer did not settle the plan".to_owned(),
                    fix: Fix::Rerun,
                });
                return self.session.reporter.exit(input, human);
            }
            woven = again;
        }
        if !json {
            terminal::note(&report::planned(&woven.weave));
        }
        if woven.weave.verdict.outcome == Outcome::Refuse {
            dismiss(warm);
            return self.refused(input, &woven);
        }
        if woven.weave.verdict.outcome == Outcome::Confirm {
            if !self.session.has_tty() {
                dismiss(warm);
                return self
                    .session
                    .reporter
                    .exit(input, Exit::Human(needs_terminal("a confirm")));
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
                    dismiss(warm);
                    return Exit::Declined(Decline::Refused);
                }
                Err(exit) => {
                    dismiss(warm);
                    return self.session.reporter.exit(input, exit);
                }
            }
        }
        self.executed(input, &woven, warm)
    }

    /// The plan's own questions before anything runs. A step's required argument no binding covers is asked as
    /// it would be at its turn, the ask narrowed to what nothing binds, and the step's decision filled for the
    /// plan to stand again; none when a question was declined. Several fields, or one record of several, no
    /// answer here can settle: a stop that names them.
    fn asked_up_front(&mut self, woven: &Woven) -> Result<Option<Vec<(Asked, Decided)>>, Exit> {
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
            if !json {
                terminal::note(&report::step(n, of, report::step_body(step, &woven.weave)));
            }
            let Some(given) = self.answers(&step.text, &asking.reflex, &asks)? else {
                return Ok(None);
            };
            let filled = fill(&self.session.plan, asking.clone(), given, self.floors());
            let mut decided = woven
                .decided_for(step, &tags)
                .expect("every step was decided")
                .clone();
            decided.decision = filled;
            seeded.push((weave::asked_for(step, &tags), decided));
        }
        Ok(Some(seeded))
    }

    /// A plan refused whole: nothing runs; each step's line, the abstaining one refused, the rest skipped.
    fn refused(&self, input: &str, woven: &Woven) -> Exit {
        let tags = self.arguments.tags.clone();
        let of = woven.weave.steps.len();
        if !self.arguments.json
            && let Some(hint) = report::left_out(self.session.plan.inactive().keys())
        {
            terminal::note(&hint);
        }
        for step in &woven.weave.steps {
            let Some(decided) = woven.decided_for(step, &tags) else {
                continue;
            };
            let mut line = Line::of(decided);
            line.decision = step.decision.clone();
            let (status, why) = if matches!(step.decision, Decision::Abstain { .. }) {
                (Status::Refused, Stopped::NoReflex)
            } else {
                (
                    Status::Skipped,
                    Stopped::Said {
                        message: "the request was refused".to_owned(),
                    },
                )
            };
            line.step = Some(StepLine {
                n: step.n,
                of,
                status,
                why: Some(why),
                bound: Vec::new(),
            });
            self.logged(input, &line, Exit::Ran);
        }
        Exit::Declined(Decline::Abstained)
    }

    /// The plan run: stage by stage, each round through the foundation's loop; a step decided again with its
    /// bound values in its words when the run asks for it; what did not run, said so at the end.
    fn executed(&mut self, input: &str, woven: &Woven, warm: Option<Warm>) -> Exit {
        let tags = self.arguments.tags.clone();
        let of = woven.weave.steps.len();
        let mut progress = Progress::default();
        let mut rewritten: Vec<(Asked, Decided)> = Vec::new();
        let mut warm = warm;
        let mut exits: Vec<Exit> = Vec::new();
        loop {
            let running =
                weave::running::execute(&self.session.plan, self.floors(), &woven.weave, &progress);
            let todo = match running {
                Running::Done { executed } => {
                    dismiss(warm);
                    return self.ended(input, woven, &executed, exits);
                }
                Running::Todo { todo } => todo,
            };
            match todo {
                Todo::Decide { asked } => {
                    let decided = self.session.decide(
                        &*self.adapter,
                        &asked.text,
                        &asked.tags,
                        asked.only.as_ref(),
                    );
                    let decided = match decided {
                        Ok(decided) => decided,
                        Err(exit) => {
                            dismiss(warm);
                            return self.session.reporter.exit(input, exit);
                        }
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
                        let loader = match warm.take() {
                            Some(loader) => Some(loader),
                            None => match self.session.warm() {
                                Ok(loader) => loader,
                                Err(exit) => return self.session.reporter.exit(input, exit),
                            },
                        };
                        let rounded = self.round(
                            input,
                            Some((handling.step, of)),
                            &decided,
                            handling.decision.clone(),
                            &handling.bound,
                            loader,
                        );
                        progress.handled.push(Handled {
                            step: handling.step,
                            round: handling.round,
                            status: status_of(&rounded.exit),
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

    /// The run over: every step that never ran said so, its line logged; then the worst step's exit — the one
    /// already reported at its turn, or a source that yielded nothing its taker could use.
    fn ended(&self, input: &str, woven: &Woven, executed: &Executed, exits: Vec<Exit>) -> Exit {
        let tags = self.arguments.tags.clone();
        let of = woven.weave.steps.len();
        for outcome in &executed.steps {
            if outcome.status != Status::Skipped {
                continue;
            }
            let Some(step) = woven.weave.step(outcome.step) else {
                continue;
            };
            let Some(decided) = woven.decided_for(step, &tags) else {
                continue;
            };
            if !self.arguments.json {
                let mut body = report::step_body(step, &woven.weave);
                body.push(" · skipped");
                terminal::note(&report::step(outcome.step, of, body));
            }
            let mut line = Line::of(decided);
            line.decision = step.decision.clone();
            line.step = Some(StepLine {
                n: outcome.step,
                of,
                status: Status::Skipped,
                why: outcome.why.clone(),
                bound: outcome.bound.clone(),
            });
            self.logged(input, &line, Exit::Ran);
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

    /// `[t]each`: the overlay line for what the utterance stated, written when the file still reads; a refusal is
    /// printed and the confirmation stands.
    fn teach(&self, input: &str, chosen: &Chosen) -> Result<(), Exit> {
        let utterance = Utterance::new(input).map_err(|why| {
            Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message: why,
                fix: Fix::Rerun,
            })
        })?;
        let edit =
            teach(&utterance, Lesson::stated(chosen), &self.session.plan).map_err(Exit::Human)?;
        if let Err(refused) = self.session.apply(input, &edit) {
            self.session.reporter.exit(input, refused);
        }
        Ok(())
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
                why,
                result: None,
            };
        }
        let exit = self.logged(input, line, human);
        Rounded {
            exit,
            why,
            result: None,
        }
    }

    /// The line into the log — and, under `--json`, to stdout — then the exit, reported.
    fn logged(&self, input: &str, line: &Line, exit: Exit) -> Exit {
        if self.arguments.json {
            terminal::result(&line.json());
        }
        let exit = match self.session.state.log(&line.log()) {
            Err(failure) if exit == Exit::Ran => Exit::Failed(failure),
            _ => exit,
        };
        self.session.reporter.exit(input, exit)
    }

    /// Every missing argument asked in turn; none at the end of input.
    fn answers(
        &mut self,
        input: &str,
        reflex: &LocalName,
        missing: &NonEmpty<Missing>,
    ) -> Result<Option<IndexMap<ArgName, Value>>, Exit> {
        let mut given = IndexMap::new();
        for missing in missing.iter() {
            let vocabulary = self.vocabulary(reflex, &missing.arg);
            match self.asked(input, missing, vocabulary.as_ref())? {
                Some(value) => {
                    given.insert(missing.arg.clone(), value);
                }
                None => return Ok(None),
            }
        }
        Ok(Some(given))
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
                    retry = Some(format!("\"{typed}\" is not {}", pick.wants()));
                    continue;
                }
            }
            retry = Some(format!("\"{typed}\" is not one of them"));
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
                Err(why) => retry = Some(why),
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
                Err(why) => retry = Some(format!("\"{typed}\" {why}")),
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

/// What became of one round of a step: its exit, already reported; why it stopped; what its body returned.
struct Rounded {
    exit: Exit,
    why: Option<Stopped>,
    result: Option<Returned>,
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

/// `<what> needs a terminal`: the stop when a prompt has none to read.
fn needs_terminal(what: &str) -> Diagnostic {
    Diagnostic {
        reflex: None,
        at: None,
        message: format!("{what} needs a terminal"),
        fix: Fix::Rerun,
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
