//! `evoke "<input>"`: decide, gate, run. In: the parsed command and the environment. Out: `Exit`. Each input is
//! decided on the session; a run feeds the loader warmed at t=0 or spawns the argv; a confirm and an ask prompt on
//! the terminal, `[t]each` writing the overlay first, `[+]` adding a word to the vocabulary and reloading the plan;
//! an abstain shows the ranking. Every decision is logged. The stdin filter answers every line and exits with the
//! first non-zero code; the REPL reads lines from the terminal — edited, with its history under XDG — until the
//! end of input, then exits 0.

use evoke_core::call::Value;
use evoke_core::decide::{Choices, Missing, Why};
use evoke_core::manifest::{Kind, Source};
use evoke_core::name::{ArgName, LocalName, OptionKey, VocabName, Word};
use evoke_core::text::NonEmpty;
use evoke_core::vocabulary::Meaning;
use evoke_core::{
    Chosen, Clean, Decision, Diagnostic, Fix, Gate, Lesson, Utterance, VocabChange, fill, identity,
    picked, teach, vocab_edit,
};
use indexmap::IndexMap;

use super::session::{self, Confirmed, Opening, Session, dismiss};
use super::{Decline, Exit};
use crate::adapter::Adapter;
use crate::args::{Arguments, Command, Inputs};
use crate::hosts::{Environment, Failure, terminal};
use crate::report::{self, Line};

pub fn run(command: &Command, arguments: &Arguments, environment: &Environment) -> Exit {
    let session = match session::open(command, arguments.json, environment, Opening::Deciding) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let adapter = match session.adapter(&command.placeholder()) {
        Ok(adapter) => adapter,
        Err(exit) => return session.reporter.exit(&command.placeholder(), exit),
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

    /// One input, end to end, its exit reported.
    fn act(&mut self, input: &str) -> Exit {
        let json = self.arguments.json;
        let warm = match self.session.warm() {
            Ok(warm) => warm,
            Err(exit) => return self.session.reporter.exit(input, exit),
        };
        let decided = match self
            .session
            .decide(&*self.adapter, input, &self.arguments.tags)
        {
            Ok(decided) => decided,
            Err(exit) => {
                dismiss(warm);
                return self.session.reporter.exit(input, exit);
            }
        };
        let mut line = Line::of(&decided);
        let mut decision = decided.decision.clone();
        let chosen = loop {
            match decision {
                Decision::Abstain { .. } => {
                    dismiss(warm);
                    if !json {
                        terminal::note(&report::abstained(&decided, self.floors()));
                    }
                    return self.logged(input, &line, Exit::Declined(Decline::Abstained));
                }
                Decision::Ask { asking, missing } => {
                    if !self.session.has_tty() {
                        dismiss(warm);
                        return self.no_terminal(input, &line, "an ask");
                    }
                    let given = match self.answers(input, &asking.reflex, &missing) {
                        Ok(Some(given)) => given,
                        Ok(None) => {
                            dismiss(warm);
                            return self.logged(input, &line, Exit::Declined(Decline::Refused));
                        }
                        Err(exit) => {
                            dismiss(warm);
                            return self.logged(input, &line, exit);
                        }
                    };
                    decision = fill(&self.session.plan, asking, given, self.floors());
                    line.decision = decision.clone();
                }
                Decision::Confirm { chosen, prompt, .. } => {
                    if !self.session.has_tty() {
                        dismiss(warm);
                        return self.no_terminal(input, &line, "a confirm");
                    }
                    let own = report::confirming(&chosen, &prompt);
                    match self.session.confirmed(&own, &prompt, true) {
                        Ok(Some(Confirmed::Yes)) => break chosen,
                        Ok(Some(Confirmed::Teach)) => {
                            if let Err(exit) = self.teach(input, &chosen) {
                                dismiss(warm);
                                return self.logged(input, &line, exit);
                            }
                            break chosen;
                        }
                        Ok(Some(Confirmed::No) | None) => {
                            dismiss(warm);
                            return self.logged(input, &line, Exit::Declined(Decline::Refused));
                        }
                        Err(exit) => {
                            dismiss(warm);
                            return self.logged(input, &line, exit);
                        }
                    }
                }
                Decision::Run { chosen } => {
                    if !json {
                        terminal::note(&report::running(&chosen));
                    }
                    break chosen;
                }
            }
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
                line.result = Some(returned);
                self.logged(input, &line, Exit::Ran)
            }
            Err(failure) => {
                line.error = Some(report::failure(&failure));
                self.logged(input, &line, Exit::Failed(failure))
            }
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
    fn no_terminal(&self, input: &str, line: &Line, what: &str) -> Exit {
        let human = Exit::Human(Diagnostic {
            reflex: None,
            at: None,
            message: format!("{what} needs a terminal"),
            fix: Fix::Rerun,
        });
        if self.arguments.json {
            terminal::result(&line.json());
            let _ = self.session.state.log(&line.log());
            return human;
        }
        self.logged(input, line, human)
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
            retry = Some(format!("{typed} is not one of them"));
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

/// A number from 1, or the choice's own text.
fn chosen_from(typed: &str, keys: &[&str]) -> Option<usize> {
    if let Ok(number) = typed.parse::<usize>()
        && (1..=keys.len()).contains(&number)
    {
        return Some(number - 1);
    }
    keys.iter().position(|key| *key == typed)
}
