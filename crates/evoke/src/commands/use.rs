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
//! XDG — until the end of input, then exits 0. The loop each step takes is `rounds`', shared with `run <file>`.

use evoke_core::decide::Missing;
use evoke_core::manifest::written;
use evoke_core::name::ArgName;
use evoke_core::text::NonEmpty;
use evoke_core::weave::{self, Asked, Because, Outcome, Status, Why as Stopped};
use evoke_core::{Clean, Decision, Diagnostic, Fix, Prompt, fill};
use indexmap::IndexMap;

use super::rounds::{Answered, Engine, Handed, Rounds, Teach, whole_plan};
use super::session::{self, Decided, Opening, Woven};
use super::r#try::refused_for;
use super::{Decline, Exit};
use super::{each_line, needs_terminal};
use crate::args::{Arguments, Command, Inputs};
use crate::hosts::{Environment, Failure, interrupt, terminal};
use crate::report;

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
        rounds: Rounds {
            session,
            engine: Engine::Ready(adapter),
            json: arguments.json,
            tags: arguments.tags.clone(),
            pinned: None,
        },
        arguments,
    };
    match &arguments.input {
        Inputs::One(input) => using.act(input),
        Inputs::Stdin => each_line(arguments.json, |input| using.act(input)),
        Inputs::Terminal => using.repl(),
    }
}

/// The rounds a sentence's steps take, and the arguments.
struct Using<'a> {
    rounds: Rounds<'a>,
    arguments: &'a Arguments,
}

impl Using<'_> {
    /// The REPL: one line at a time from the terminal, edited and remembered where it can be; a blank line is
    /// nothing; the end of input ends the session.
    fn repl(&mut self) -> Exit {
        let opened = self.rounds.session.state.history().and_then(|history| {
            terminal::repl(history).ok_or_else(|| Failure {
                what: "opening the terminal".to_owned(),
                cause: None,
                fix: Fix::Rerun,
            })
        });
        let mut repl = match opened {
            Ok(repl) => repl,
            Err(failure) => {
                return self
                    .rounds
                    .session
                    .reporter
                    .exit("<input>", Exit::Failed(failure));
            }
        };
        loop {
            let typed = match repl.line("> ") {
                Ok(Some(typed)) => typed,
                Ok(None) => return Exit::Ran,
                Err(failure) => {
                    return self
                        .rounds
                        .session
                        .reporter
                        .exit("<input>", Exit::Failed(failure));
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
        let woven = match self.woven(input, Vec::new()) {
            Ok(woven) => woven,
            Err(exit) => return self.rounds.session.reporter.exit(input, exit),
        };
        let exit = match woven.single(&self.arguments.tags, &self.rounds.session.plan) {
            Some(decided) => {
                let decision = decided.decision.clone();
                let handed = Handed {
                    bound: &[],
                    taken: &IndexMap::new(),
                    shared: &IndexMap::new(),
                    from: &[],
                };
                self.rounds
                    .round(input, None, &decided, decision, handed)
                    .exit
            }
            None => self.many(input, woven),
        };
        if interrupt::interrupted() {
            interrupt::end();
        }
        exit
    }

    /// The input read into its steps through the adapter, `seeded` decisions standing in for the planner's own.
    fn woven(&mut self, input: &str, seeded: Vec<(Asked, Decided)>) -> Result<Woven, Exit> {
        let adapter = self.rounds.engine.resolved(&self.rounds.session, input)?;
        self.rounds
            .session
            .weave(adapter, input, &self.arguments.tags, seeded)
    }

    /// A weave: the plan's own questions first — a step's argument nothing binds, asked as at its turn, and
    /// again while the answer is out of range; a reference that takes nothing, confirmed — then the plan shown,
    /// and each step at its turn through the foundation's own loop; the worst step's exit. A request that is
    /// only what not to do is nothing to do, and one that opens with a condition is refused in one line. A plan
    /// stopped before any step ran logs every step's line with what stopped it. A plan a playbook wrote asks one
    /// yes over the whole — the playbook's own confirm when the plan is its alone — and logs the expansion's line
    /// as step 0 first.
    fn many(&mut self, input: &str, woven: Woven) -> Exit {
        let json = self.arguments.json;
        if woven.weave.steps.is_empty() {
            if json {
                // One line per input holds under --json: an abstain that judged nothing.
                terminal::result(&report::nothing_to_do_json(input, refused_for(&woven)));
            } else {
                terminal::note(&report::nothing_to_do(refused_for(&woven)));
            }
            return Exit::Declined(Decline::Refused);
        }
        // Nothing a person can answer settles these: the plan shows first, then the line that names them.
        let unsettled =
            woven.weave.verdict.because.iter().find(|because| {
                matches!(because, Because::Several { .. } | Because::OneOfMany { .. })
            });
        if let Some(because) = unsettled {
            if !json {
                terminal::note(&report::planned(&woven.weave));
            }
            let problem = Diagnostic {
                reflex: None,
                at: None,
                message: report::verdict(because),
                fix: Fix::Rerun,
            };
            return self.rounds.unanswered(input, &woven, problem);
        }
        let woven = match self.settled(input, woven) {
            Ok(woven) => woven,
            Err(exit) => return exit,
        };
        if !json {
            terminal::note(&report::planned(&woven.weave));
        }
        if let Err(exit) = self.rounds.expansions(input, &woven) {
            return exit;
        }
        if woven.weave.verdict.outcome == Outcome::Refuse {
            return self.rounds.refused(input, &woven);
        }
        if woven.weave.verdict.outcome == Outcome::Confirm {
            let reasons: Vec<String> = woven
                .weave
                .verdict
                .because
                .iter()
                .map(report::verdict)
                .collect();
            let (template, teach) = reviewed(&woven);
            let proceed = self
                .rounds
                .proceed(input, &woven, reasons.join("; "), template, teach);
            if let Err(exit) = proceed {
                return exit;
            }
        }
        self.rounds.executed(input, &woven)
    }
}

/// The yes over a plan a playbook wrote: its own confirm as the question when the typed plan is one playbook's
/// alone, `Run the plan as it stands?` otherwise; teachable whenever exactly one playbook stands in the plan —
/// the part of the sentence that picked it taught to the playbook, never the whole sentence.
fn reviewed(woven: &Woven) -> (Clean, Option<Teach>) {
    {
        let reviewed: Vec<(&str, &Prompt)> = woven
            .weave
            .verdict
            .because
            .iter()
            .filter_map(|because| match because {
                Because::Reviewed { text, prompt, .. } => Some((text.as_str(), prompt)),
                _ => None,
            })
            .collect();
        let [(text, prompt)] = reviewed.as_slice() else {
            return (whole_plan(), None);
        };
        let alone = woven.weave.steps.iter().all(|step| !step.from.is_empty());
        let template = if alone {
            prompt.template.clone()
        } else {
            whole_plan()
        };
        let chosen = woven
            .decided
            .iter()
            .find(|(asked, _)| asked.text == *text)
            .and_then(|(_, decided)| match &decided.decision {
                Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(chosen.clone()),
                Decision::Ask { .. } | Decision::Abstain { .. } => None,
            });
        let teach = chosen.map(|chosen| Teach {
            spoken: (*text).to_owned(),
            chosen,
        });
        (template, teach)
    }
}

impl Using<'_> {
    /// The plan settled: asked up front while it asks, each answer standing in for the planner's own decision
    /// of its step when the plan is read again. A stop is reported here: the question declined, or one no one
    /// can answer, every step's line saying so.
    fn settled(&mut self, input: &str, woven: Woven) -> Result<Woven, Exit> {
        let mut woven = woven;
        let mut seeds: Vec<(Asked, Decided)> = Vec::new();
        let mut shown: Vec<String> = Vec::new();
        while woven.weave.verdict.outcome == Outcome::Ask {
            let fresh = match self.asked_up_front(&woven, &mut shown) {
                Ok(UpFront::Seeded(fresh)) => fresh,
                Ok(UpFront::Declined { step, ask }) => {
                    let exit = Exit::Declined(Decline::Refused);
                    return Err(self.rounds.stopped_whole(input, &woven, exit, |s| {
                        if s.n == step {
                            let message = ask.clone();
                            (Status::Declined, Stopped::Said { message })
                        } else {
                            let message = PLAN_DECLINED.to_owned();
                            (Status::Skipped, Stopped::Said { message })
                        }
                    }));
                }
                Err(Exit::Human(problem)) => {
                    return Err(self.rounds.unanswered(input, &woven, problem));
                }
                Err(exit) => return Err(self.rounds.session.reporter.exit(input, exit)),
            };
            for seed in fresh {
                match seeds.iter_mut().find(|(asked, _)| *asked == seed.0) {
                    Some(held) => *held = seed,
                    None => seeds.push(seed),
                }
            }
            let again = self
                .woven(input, seeds.clone())
                .map_err(|exit| self.rounds.session.reporter.exit(input, exit))?;
            // An answer the plan did not take — it stands exactly as before — is a stop, never a loop. One it
            // took and asks about again, out of range, is asked again with the reason, as one input is. A
            // playbook's answered slot is taken once its steps stand with the slot's value, or once its part
            // folded into one.
            let taken = !seeds.is_empty()
                && seeds.iter().all(|(asked, seeded)| {
                    again.weave.steps.iter().any(|step| {
                        step.decision == seeded.decision || expanded(step, &seeded.decision)
                    }) || again
                        .weave
                        .folded
                        .iter()
                        .any(|folded| folded.text == asked.text)
                });
            if !taken {
                let problem = Diagnostic {
                    reflex: None,
                    at: None,
                    message: "the answer did not settle the plan".to_owned(),
                    fix: Fix::Rerun,
                };
                return Err(self.rounds.unanswered(input, &again, problem));
            }
            woven = again;
        }
        Ok(woven)
    }

    /// The plan's own questions before anything runs. A step's required argument no binding covers is asked as
    /// it would be at its turn, the ask narrowed to what nothing binds, and the step's decision filled for the
    /// plan to stand again; the step's line is shown ahead of its first question, and `shown` remembers it.
    /// Several fields, or one record of several, no answer here can settle: a stop that names them.
    fn asked_up_front(&mut self, woven: &Woven, shown: &mut Vec<String>) -> Result<UpFront, Exit> {
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
            if !self.rounds.session.has_tty() {
                return Err(Exit::Human(needs_terminal("an ask")));
            }
            let first = unbound.remove(0);
            let asks = NonEmpty::new(first, unbound);
            // Shown once, by its words: an expansion renumbers the steps.
            if !json && !shown.contains(&step.text) {
                terminal::note(&report::step(n, of, report::step_body(step, &woven.weave)));
                shown.push(step.text.clone());
            }
            let given = match self.rounds.answers(&step.text, &asking.reflex, &asks)? {
                Answered::Given(given) => given,
                Answered::Declined { ask } => return Ok(UpFront::Declined { step: n, ask }),
            };
            let filled = fill(
                &self.rounds.session.plan,
                asking.clone(),
                given,
                self.rounds.floors(),
            );
            let mut decided = woven.planned(step, &tags).expect("every step was decided");
            decided.decision = filled;
            seeded.push((weave::asked_for(step, &tags), decided));
        }
        Ok(UpFront::Seeded(seeded))
    }
}

/// Whether a step came from the playbook a seeded decision is about, with the seed's values in its slots: the
/// seed was taken, and expanded.
fn expanded(step: &weave::Step, seeded: &Decision) -> bool {
    let (reflex, args) = match seeded {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
            (&chosen.call.reflex, &chosen.call.args)
        }
        Decision::Ask { .. } | Decision::Abstain { .. } => return false,
    };
    step.from.iter().any(|from| {
        from.playbook == *reflex
            && from.slots.iter().all(|(slot, value)| {
                args.get(slot)
                    .and_then(written)
                    .is_some_and(|theirs| theirs == *value)
            })
    })
}

/// The plan's own questions asked up front: the decisions answered into, or the step whose question was declined.
enum UpFront {
    Seeded(Vec<(Asked, Decided)>),
    Declined { step: usize, ask: String },
}

/// Why the other steps never ran when one's question was declined before anything did.
const PLAN_DECLINED: &str = "the plan was declined";
