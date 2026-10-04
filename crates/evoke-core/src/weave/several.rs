//! One action asked for several values, a step for each, in three phases of the plan: a part that says the step
//! before it once more, «and the same for US East»; other values of an argument in a step's words, «list sessions
//! for Sam, Ana and Jo», each the engine says the request asks for as well; words that point at several values said
//! before, «order both a pro». Each step is the person's own words with its value in place, decided narrowed to the
//! step's reflex, and stands where it reads that value and every other as the step did. In: the draft as the phases
//! before left it, the `Answers` so far. Out: the draft with a step per value, or what is needed next.

use super::planning::{
    Draft, Planner, alike, args_of, asked_of, basis_of, index_of, reflex_of, repair_of,
    same_source, says, split_before, stated_of,
};
use super::reading::{self, Segment};
use super::{Asked, Need, Repair, named};
use crate::adapter::{Key, Prob, Question, QuestionId, Request, Scope};
use crate::call::Value;
use crate::decide::{Basis, Cap, Decision, Judgment, held};
use crate::manifest::{Kind, Source};
use crate::name::{ArgName, LocalName};
use crate::pack;
use crate::plan::{Active, Plan};
use crate::text::{self, Input, Span};

/// At this share the answer that the request asks the same for another value as well makes a step of it.
const AS_WELL: f64 = 0.5;

/// A part said once more, put in the step before it: the part, the step, the text decided, and each argument with
/// the value put in it.
struct SaidAgain {
    k: usize,
    h: usize,
    asked: Asked,
    values: Vec<(ArgName, String)>,
}

/// Another value of an argument a step's call holds or asks for, found in the step's words: where its words stand
/// there, the value as the argument reads it — a listed word's key, a typed value as typed — and the question of
/// what the request asks of it.
struct OtherValue {
    words: Span,
    value: String,
    id: QuestionId,
    question: Question,
}

/// A step whose words hold other values of one argument of its call: the step, the argument, the words of the value
/// its call holds, the other values, and the words of every value of another argument code finds there, which a
/// list of this argument is never read across. Where its call asks for the argument, the reading of its first value
/// stands for the step.
struct OtherValues {
    k: usize,
    arg: ArgName,
    held: Span,
    others: Vec<OtherValue>,
    fences: Vec<Span>,
    first: Option<FirstValue>,
}

/// A call that asked for an argument, read again from its words with the list of the argument's values said once as
/// the first: those words, their reading, and where the list stands in the step's words.
struct FirstValue {
    text: String,
    reading: Decision,
    list: (usize, usize),
}

/// A step whose call asks for an argument whose words hold two of its values or more: the step, the argument, each
/// value's words and value in the words' order, the step's words with the list said once as the first value, and
/// where the list stands.
struct AskedList {
    k: usize,
    arg: ArgName,
    values: Vec<(Span, String)>,
    text: String,
    list: (usize, usize),
}

/// What code finds of one argument's values in a step's words: other values beside the one its call holds, or,
/// where its call asks for the argument, a list of them.
enum Values {
    Beside(Box<OtherValues>),
    Asked(AskedList),
}

/// A step written again for the values the request asks the same for: the step, the argument, the step's words with
/// the list said once as its own value, each value asked for as well, and, where its call asks for the argument, the
/// reading of its first value, which stands for it.
struct Written {
    k: usize,
    arg: ArgName,
    own: String,
    values: Vec<WrittenValue>,
    first: Option<Decision>,
}

/// One value a step is written again for: the value, the step's words with the list said once as it, and the
/// judgment its route stands on, the share of the answer that the same is asked for it as well.
struct WrittenValue {
    value: String,
    text: String,
    judgment: Judgment,
}

/// A step that asks for an argument, written once per value its pointing words count: the step, the argument, the
/// pointing words as typed, and each value with the text that holds its words in their place.
struct PointedValues {
    k: usize,
    arg: ArgName,
    words: String,
    values: Vec<(String, Asked)>,
}

/// A step's words as code reads them for the values of its arguments: the step, its reflex and its decision, its
/// text, how far the person's own words reach in it (a word a rewrite shared comes after them), the request those
/// words make narrowed to the reflex, the words each value its call holds stands on, the typed spans no argument
/// took, and the words the reader's own rules set apart.
struct StepWords<'d> {
    plan: &'d Plan,
    k: usize,
    reflex: &'d LocalName,
    active: &'d Active,
    decision: &'d Decision,
    text: &'d str,
    reach: usize,
    request: Request,
    held: Vec<(ArgName, Span)>,
    unconsumed: &'d [Span],
    ruled: Vec<(usize, usize)>,
}

impl StepWords<'_> {
    /// Every value of an argument code finds in the step's words, in the words' order: a listed word found or
    /// proposed, or a typed value of its kind no argument took.
    fn values_found(&self, arg: &ArgName, source: &Source) -> Vec<(Span, String)> {
        let question = QuestionId::Arg(self.reflex.clone(), arg.clone());
        let mut found: Vec<(Span, String)> = Vec::new();
        match source {
            Source::Pick(pick) => {
                for candidate in &self.request.proposed {
                    if self.unconsumed.contains(&candidate.span)
                        && crate::decide::proposes(pick.recognizer(), &candidate.value)
                    {
                        found.push((
                            candidate.span.clone(),
                            candidate.span.text().as_str().to_owned(),
                        ));
                    }
                }
            }
            Source::Vocab(_) | Source::Options(_) => {
                for listed in self.request.listed.get(&question).into_iter().flatten() {
                    found.push((listed.span.clone(), listed.key.as_str().to_owned()));
                }
                if let Some(Question::Choice(choice)) = self.request.questions.get(&question) {
                    for proposal in
                        crate::decide::proposals(self.plan, &self.request, self.reflex, arg, choice)
                    {
                        found.push((proposal.span, proposal.key.as_str().to_owned()));
                    }
                }
            }
        }
        found.sort_by_key(|(span, _)| span.start());
        found
    }

    /// The values found less the held value said again and the values another step of the reflex holds, and less
    /// those whose words stand outside the person's own, inside a value's, or in words the reader's own rules set
    /// apart.
    fn values_kept(
        &self,
        draft: &Draft,
        arg: &ArgName,
        mine: Option<&str>,
        found: Vec<(Span, String)>,
    ) -> Vec<(Span, String)> {
        let claimed: Vec<String> = (0..draft.segs.len())
            .filter(|&j| j != self.k && reflex_of(&draft.decisions[j]) == Some(self.reflex))
            .filter_map(|j| args_of(&draft.decisions[j])?.get(arg).and_then(stated_of))
            .collect();
        let mut others: Vec<(Span, String)> = Vec::new();
        for (span, value) in found {
            let value = value.to_lowercase();
            if mine == Some(value.as_str())
                || claimed.contains(&value)
                || span.end() > self.reach
                || self.held.iter().any(|(_, held)| overlaps(held, &span))
                || self
                    .ruled
                    .iter()
                    .any(|&(start, end)| start <= span.start() && span.end() <= end)
                || others
                    .iter()
                    .any(|(other, theirs)| overlaps(other, &span) || *theirs == value)
            {
                continue;
            }
            others.push((span, value));
        }
        others
    }

    /// The words of every value of another argument code finds in the step's words, which a list of this one never
    /// runs across: a value the call holds, a listed word of another argument, a typed value none of the list's.
    fn fences(&self, arg: &ArgName, list: &[Span]) -> Vec<Span> {
        let question = QuestionId::Arg(self.reflex.clone(), arg.clone());
        let mut fences: Vec<Span> = self
            .held
            .iter()
            .filter(|(other, _)| other != arg)
            .map(|(_, span)| span.clone())
            .collect();
        for (other, listed) in &self.request.listed {
            if *other != question {
                fences.extend(listed.iter().map(|listed| listed.span.clone()));
            }
        }
        fences.extend(
            self.request
                .proposed
                .iter()
                .map(|candidate| candidate.span.clone())
                .filter(|span| !list.iter().any(|value| overlaps(value, span))),
        );
        fences
    }

    /// The values of an argument the step's call asks for, where its words hold two of them or more and no value of
    /// another argument among them: the list, with the step's words that say it once as the first value.
    fn asked_list(&self, draft: &Draft, arg: &ArgName) -> Option<AskedList> {
        if args_of(self.decision)?.contains_key(arg) {
            return None;
        }
        let Kind::Value { source, .. } = &self.active.args.get(arg)?.kind else {
            return None;
        };
        let values = self.values_kept(draft, arg, None, self.values_found(arg, source));
        let (Some((first, _)), Some((last, _))) = (values.first(), values.last()) else {
            return None;
        };
        if values.len() < 2 {
            return None;
        }
        let bounds = (first.start(), last.end());
        let spans: Vec<Span> = values.iter().map(|(span, _)| span.clone()).collect();
        if self
            .fences(arg, &spans)
            .iter()
            .any(|fence| bounds.0 <= fence.start() && fence.end() <= bounds.1)
        {
            return None;
        }
        let text = said_once(self.text, bounds, first.text().as_str());
        Some(AskedList {
            k: self.k,
            arg: arg.clone(),
            values,
            text,
            list: bounds,
        })
    }
}

impl Planner<'_> {
    /// A part said once more. A part that says the call before it is wanted once more — a phrase of the pack's
    /// `refer.again` or `refer.repeats`, values the step before it takes, and words that carry nothing: «and the
    /// same for US East» after «error rate for payments in EU West», «ana the same» after «order sam an air» — names
    /// no action of its own. It is put in that step's words, each value in place of the words its argument holds
    /// there, every word still the person's own, «error rate for payments in US East», decided narrowed to that
    /// step's reflex, and kept where it reads each value into its argument and every other value as the step read
    /// it, the part's words as typed kept beside; only where every step before the part reads as that step's
    /// reflex, since «the same» after two actions may mean both. Elsewhere the part stays as it read alone.
    pub(super) fn repeated(&self, draft: &mut Draft) -> Result<(), Need> {
        let mut wanted: Vec<SaidAgain> = Vec::new();
        for k in 1..draft.segs.len() {
            if !draft.origins[k].is_empty()
                || draft.verb(k)
                || repair_of(draft, k).is_some()
                || split_before(&draft.taken, &draft.segs, k).is_none()
            {
                continue;
            }
            let Some(h) = draft.neighbour(k) else {
                continue;
            };
            if !draft.origins[h].is_empty() {
                continue;
            }
            if (0..k).any(|j| {
                draft.origins[j].is_empty()
                    && reflex_of(&draft.decisions[j]) != reflex_of(&draft.decisions[h])
            }) {
                continue;
            }
            let Some((text, values)) = self.said_again(draft, h, &draft.segs[k].text) else {
                continue;
            };
            let Some(reflex) = reflex_of(&draft.decisions[h]) else {
                continue;
            };
            let asked = Self::narrowed(&text, reflex, named(&draft.decisions[h]));
            wanted.push(SaidAgain {
                k,
                h,
                asked,
                values,
            });
        }
        self.undecided(wanted.iter().map(|said| &said.asked))?;
        for said in wanted {
            let decision = self
                .decided(&said.asked)
                .cloned()
                .expect("every part said again is decided");
            if replaces(&draft.decisions[said.h], &decision, &said.values) {
                draft.rewrite(said.k, &said.asked.text, decision);
            }
        }
        Ok(())
    }

    /// The step before a part said once more, written with the part's values: the text, and each argument with the
    /// value put in it. None where the part's words are not a phrase that says so, values of the step's reflex and
    /// words that carry nothing; where a value is of an argument the step's call holds by no words of its own, or
    /// one it holds equal; and where the step picks a playbook.
    fn said_again(
        &self,
        draft: &Draft,
        h: usize,
        part: &str,
    ) -> Option<(String, Vec<(ArgName, String)>)> {
        let decision = &draft.decisions[h];
        let (reflex, active) = self.active_of(decision)?;
        if !active.steps.is_empty() {
            return None;
        }
        let lexicon = pack::lexicon(part);
        let mut phrases = lexicon.phrases(|pack| &pack.refer.again);
        phrases.extend(lexicon.phrases(|pack| &pack.refer.repeats));
        if !says(part, &phrases) {
            return None;
        }
        let input = Input::new(part).ok()?;
        let values = self.part_values(reflex, active, part)?;
        if !values_said_again(&lexicon, input.as_str(), &phrases, &values) {
            return None;
        }
        // The step's words, each value of the part in place of the words its argument holds there.
        let step = &draft.segs[h].text;
        let args = args_of(decision)?;
        let (_, held) = self.held_words(step, decision)?;
        let mut places: Vec<(Span, String)> = Vec::new();
        let mut put: Vec<(ArgName, String)> = Vec::new();
        for (arg, span, value) in &values {
            let holds = args.get(arg)?;
            if holds.text().map(str::to_lowercase) == Some(value.to_lowercase()) {
                return None;
            }
            let (_, words) = held.iter().find(|(name, _)| name == arg)?;
            places.push((words.clone(), span.text().as_str().to_owned()));
            put.push((arg.clone(), value.to_lowercase()));
        }
        places.sort_by_key(|(words, _)| std::cmp::Reverse(words.start()));
        let mut chars: Vec<char> = step.chars().collect();
        for (words, value) in places {
            chars.splice(words.start()..words.end().min(chars.len()), value.chars());
        }
        Some((chars.into_iter().collect(), put))
    }

    /// Each value a part holds of the arguments of a reflex, with its words and the value as its argument reads it:
    /// a listed word code finds, or a typed value of the one argument whose recognizer reads it. None where it holds
    /// none, or two whose words overlap.
    fn part_values(
        &self,
        reflex: &LocalName,
        active: &Active,
        part: &str,
    ) -> Option<Vec<(ArgName, Span, String)>> {
        let request = crate::decide::request(
            self.plan,
            part,
            self.tags,
            Some(reflex),
            None,
            Scope::Full,
            &[],
        )
        .ok()?;
        let mut values: Vec<(ArgName, Span, String)> = Vec::new();
        for (arg, argument) in &active.args {
            let Kind::Value { source, .. } = &argument.kind else {
                continue;
            };
            let id = QuestionId::Arg(reflex.clone(), arg.clone());
            match source {
                Source::Pick(pick) => {
                    let of_kind: Vec<&crate::propose::Proposed> = request
                        .proposed
                        .iter()
                        .filter(|c| {
                            c.value.is_typed()
                                && crate::decide::proposes(pick.recognizer(), &c.value)
                        })
                        .collect();
                    let takers = active
                        .args
                        .values()
                        .filter(|other| {
                            matches!(&other.kind, Kind::Value { source: Source::Pick(theirs), .. }
                                if theirs.recognizer() == pick.recognizer())
                        })
                        .count();
                    if let ([candidate], 1) = (of_kind.as_slice(), takers) {
                        values.push((
                            arg.clone(),
                            candidate.span.clone(),
                            candidate.span.text().as_str().to_owned(),
                        ));
                    }
                }
                Source::Vocab(_) | Source::Options(_) => {
                    if let [listed] = request.listed.get(&id).map_or(&[][..], Vec::as_slice) {
                        values.push((
                            arg.clone(),
                            listed.span.clone(),
                            listed.key.as_str().to_owned(),
                        ));
                    }
                }
            }
        }
        let overlapping = values
            .iter()
            .enumerate()
            .any(|(i, (_, a, _))| values[..i].iter().any(|(_, b, _)| overlaps(a, b)));
        (!values.is_empty() && !overlapping).then_some(values)
    }

    /// A second value makes a second step. Where the words of a step of the person's own hold another value of an
    /// argument its call holds by words — a listed word the reader's own finder holds, a typed value of the
    /// argument's kind that no argument took — and the value stands in nothing the request rules out, is not the held
    /// value said again and is held by no other step of that reflex, the engine is asked whether the request asks the
    /// same for it as well, and where it does, the step is written again for that value: «list sessions for Sam, Ana
    /// and Jo» is three steps, «list sessions for Ana» among them. A call that asks for the argument because its
    /// words hold two of its values or more is read from the first, «check the error rate on search in virginia and
    /// in ireland», and its other values are asked as any (`several_found`, `claimed`).
    pub(super) fn several(&self, draft: &mut Draft) -> Result<(), Need> {
        let found = self.several_found(draft)?;
        self.claimed(draft, found)
    }

    /// Every step's other values (`several_of`). A call that asks for the argument is first read from its words with
    /// the list said once as the first value, decided narrowed to its reflex, and stands on that reading where it
    /// holds that value and asks for nothing more; its other values are then asked against that reading.
    fn several_found(&self, draft: &Draft) -> Result<Vec<OtherValues>, Need> {
        let mut several = Vec::new();
        let mut lists: Vec<(AskedList, Asked)> = Vec::new();
        for k in 0..draft.segs.len() {
            match self.several_of(draft, k) {
                Some(Values::Beside(found)) => several.push(*found),
                Some(Values::Asked(list)) => {
                    let decision = &draft.decisions[list.k];
                    if let Some(reflex) = reflex_of(decision) {
                        let asked = Self::narrowed(&list.text, reflex, named(decision));
                        lists.push((list, asked));
                    }
                }
                None => {}
            }
        }
        self.undecided(lists.iter().map(|(_, asked)| asked))?;
        for (list, asked) in lists {
            let reading = self
                .decided(&asked)
                .cloned()
                .expect("every first value is decided");
            let Some((words, value)) = list.values.first() else {
                continue;
            };
            if !fills(&draft.decisions[list.k], &reading, &list.arg, value) {
                continue;
            }
            let others = self.questions_of(
                draft,
                list.k,
                &reading,
                &list.arg,
                list.values[1..].to_vec(),
            );
            if others.is_empty() {
                continue;
            }
            several.push(OtherValues {
                k: list.k,
                arg: list.arg.clone(),
                held: words.clone(),
                others,
                fences: Vec::new(),
                first: Some(FirstValue {
                    text: list.text.clone(),
                    reading,
                    list: list.list,
                }),
            });
        }
        Ok(several)
    }

    /// What a step's words hold of one argument's values (`Values`): other values beside the one its call holds, of
    /// one argument a step and none where two have them; else, where its call asks for an argument whose words hold
    /// a list of its values, that list.
    fn several_of(&self, draft: &Draft, k: usize) -> Option<Values> {
        let words = self.step_words(draft, k)?;
        let mut beside: Vec<OtherValues> = words
            .held
            .iter()
            .filter_map(|(arg, held)| self.beside_held(draft, &words, arg, held))
            .collect();
        match beside.len() {
            0 => {}
            1 => return beside.pop().map(|found| Values::Beside(Box::new(found))),
            _ => return None,
        }
        let Decision::Ask { missing, .. } = words.decision else {
            return None;
        };
        let mut lists: Vec<AskedList> = missing
            .iter()
            .filter_map(|asked| words.asked_list(draft, &asked.arg))
            .collect();
        match lists.len() {
            1 => lists.pop().map(Values::Asked),
            _ => None,
        }
    }

    /// A step's words as code reads them for its arguments' values (`StepWords`). None for a step a playbook
    /// wrote, a second verb's, one refused, one filled from a session's results, one a second verb takes from, one
    /// the items rule cut from a whole, one whose reflex is a playbook, and one whose words a rewrite made other
    /// than the person's.
    fn step_words<'d>(&'d self, draft: &'d Draft, k: usize) -> Option<StepWords<'d>> {
        let id = draft.ids[k];
        if !draft.origins[k].is_empty()
            || draft.verb(k)
            || draft.refused_at(k)
            || draft.recalled.contains(&id)
            || draft.verbs.iter().any(|(_, of)| *of == id)
            || draft.wholes.iter().any(|whole| whole.part == id)
        {
            return None;
        }
        let decision = &draft.decisions[k];
        let (reflex, active) = self.active_of(decision)?;
        if !active.steps.is_empty() {
            return None;
        }
        let text = draft.segs[k].text.as_str();
        // The person's own words, where they stand in the request: a word a rewrite shared comes after them.
        let own = draft.as_typed(k);
        if !text.starts_with(own.as_str()) {
            return None;
        }
        index_of(&draft.chars, &own, draft.segs[k].start)?;
        let (request, held) = self.held_words(text, decision)?;
        let unconsumed: &[Span] = match decision {
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => &chosen.unconsumed,
            Decision::Ask { asking, .. } => &asking.unconsumed,
            Decision::Abstain { .. } => &[],
        };
        Some(StepWords {
            plan: self.plan,
            k,
            reflex,
            active,
            decision,
            text,
            reach: own.chars().count(),
            request,
            held,
            unconsumed,
            ruled: ruled_out(text),
        })
    }

    /// The other values of an argument a step's call holds by words, each with its question; none where the call
    /// holds it by what another part gave or a result recalled, or where the words hold no other.
    fn beside_held(
        &self,
        draft: &Draft,
        words: &StepWords<'_>,
        arg: &ArgName,
        held: &Span,
    ) -> Option<OtherValues> {
        if matches!(
            basis_of(words.decision).and_then(|basis| basis.get(arg)),
            Some(Basis::Shared { .. } | Basis::Recalled { .. })
        ) {
            return None;
        }
        let Kind::Value { source, .. } = &words.active.args.get(arg)?.kind else {
            return None;
        };
        let mine = args_of(words.decision)?
            .get(arg)?
            .text()
            .map(str::to_lowercase);
        let others =
            words.values_kept(draft, arg, mine.as_deref(), words.values_found(arg, source));
        if others.is_empty() {
            return None;
        }
        let mut list: Vec<Span> = vec![held.clone()];
        list.extend(others.iter().map(|(span, _)| span.clone()));
        let fences = words.fences(arg, &list);
        let others = self.questions_of(draft, words.k, words.decision, arg, others);
        (!others.is_empty()).then(|| OtherValues {
            k: words.k,
            arg: arg.clone(),
            held: held.clone(),
            others,
            fences,
            first: None,
        })
    }

    /// The questions of an argument's other values: whether the request asks the same for each as well, against the
    /// call read back, by where each value's words stand in the request.
    fn questions_of(
        &self,
        draft: &Draft,
        k: usize,
        call: &Decision,
        arg: &ArgName,
        others: Vec<(Span, String)>,
    ) -> Vec<OtherValue> {
        let Some((reflex, active)) = self.active_of(call) else {
            return Vec::new();
        };
        let (Some(args), Some(argument)) = (args_of(call), active.args.get(arg)) else {
            return Vec::new();
        };
        let Some(base) = index_of(&draft.chars, &draft.as_typed(k), draft.segs[k].start) else {
            return Vec::new();
        };
        let read = crate::hold::read_back(self.plan, reflex, active, args);
        let held_text = args
            .get(arg)
            .and_then(Value::text)
            .unwrap_or_default()
            .to_owned();
        others
            .into_iter()
            .filter_map(|(span, value)| {
                let (id, question) = reading::again(
                    (&read, argument.ask.as_str(), &held_text),
                    span.text().as_str(),
                    (base + span.start(), base + span.end()),
                    (reflex, arg),
                )
                .ok()?;
                Some(OtherValue {
                    words: span,
                    value,
                    id,
                    question,
                })
            })
            .collect()
    }

    /// The other values asked for as well, each a step. The engine is asked of every value found, in a request of
    /// its own, `weave.again_<start>_<end>_<reflex>__<argument>`: the same is asked for it as well; it is the answer
    /// in place of the one read; it is ruled out; it says something else. Each it says the same for, at `AS_WELL`,
    /// writes the step again (`written`), standing where that reads (`write_values`).
    fn claimed(&self, draft: &mut Draft, several: Vec<OtherValues>) -> Result<(), Need> {
        if several.is_empty() {
            return Ok(());
        }
        self.ask(several.iter().flat_map(|found| {
            found
                .others
                .iter()
                .map(|other| (other.id.clone(), other.question.clone()))
        }))?;
        let written: Vec<Written> = several
            .into_iter()
            .filter_map(|found| self.written(draft, found))
            .collect();
        let texts: Vec<Asked> = written
            .iter()
            .flat_map(|values| Self::texts_of(draft, values))
            .collect();
        self.undecided(&texts)?;
        // The later steps first, so that a step put in moves no earlier one.
        for values in written.into_iter().rev() {
            self.write_values(draft, values);
        }
        Ok(())
    }

    /// A step written again for the values asked for as well: its words with the list said once as each — the held
    /// value and those asked for, never read across another argument's value; for a call that asks for the argument,
    /// every value its words hold — every word still the person's own, each value's route the share of the answer
    /// that asked it. None where none is asked for.
    fn written(&self, draft: &Draft, found: OtherValues) -> Option<Written> {
        let asked: Vec<&OtherValue> = found
            .others
            .iter()
            .filter(|other| self.said(&other.id, reading::ALSO) >= AS_WELL)
            .collect();
        if asked.is_empty() {
            return None;
        }
        let list = match &found.first {
            Some(first) => first.list,
            None => (
                asked
                    .iter()
                    .map(|other| other.words.start())
                    .fold(found.held.start(), usize::min),
                asked
                    .iter()
                    .map(|other| other.words.end())
                    .fold(found.held.end(), usize::max),
            ),
        };
        if found
            .fences
            .iter()
            .any(|fence| list.0 <= fence.start() && fence.end() <= list.1)
        {
            return None;
        }
        let text = &draft.segs[found.k].text;
        let reflex = reflex_of(&draft.decisions[found.k])?;
        let top = Key::new(reflex.as_str()).ok()?;
        let own = match &found.first {
            Some(first) => first.text.clone(),
            None => said_once(text, list, found.held.text().as_str()),
        };
        let values = asked
            .iter()
            .filter_map(|other| {
                let p = Prob::new(self.said(&other.id, reading::ALSO))?;
                Some(WrittenValue {
                    value: other.value.clone(),
                    text: said_once(text, list, other.words.text().as_str()),
                    judgment: Judgment {
                        question: other.id.clone(),
                        top: top.clone(),
                        p,
                    },
                })
            })
            .collect();
        Some(Written {
            k: found.k,
            arg: found.arg,
            own,
            values,
            first: found.first.map(|first| first.reading),
        })
    }

    /// The texts a step written again is decided on: its own value's, on the step's route; each other value's, on
    /// the answer that asked it.
    fn texts_of(draft: &Draft, written: &Written) -> Vec<Asked> {
        let decision = &draft.decisions[written.k];
        let Some(reflex) = reflex_of(decision) else {
            return Vec::new();
        };
        std::iter::once(Self::narrowed(&written.own, reflex, named(decision)))
            .chain(
                written
                    .values
                    .iter()
                    .map(|value| Self::narrowed(&value.text, reflex, Some(&value.judgment))),
            )
            .collect()
    }

    /// A step written again, standing where it reads: each value's text that reads that value into the argument and
    /// every other value as the step read it, asking nothing the step did not (`claims`), is a step of its own right
    /// after the step (`Again`), held for what the plan held the step for and holding the step's references. Where
    /// every value asked for is a step, the step itself stands on its own value alone where that reads the same
    /// call, read apart and shown as typed (`Apart`); a call that asks stands on its first value only then, and asks
    /// as it read otherwise: no value is dropped.
    fn write_values(&self, draft: &mut Draft, written: Written) {
        let k = written.k;
        let original = draft.decisions[k].clone();
        let step = written.first.clone().unwrap_or_else(|| original.clone());
        let Some(reflex) = reflex_of(&original).cloned() else {
            return;
        };
        let named = named(&original).cloned();
        let holds = plan_holds(&original);
        let decide = |asked: &Asked| {
            self.decided(asked)
                .cloned()
                .expect("every value asked for again is decided")
        };
        let held_back = |decision: Decision| {
            holds
                .iter()
                .cloned()
                .fold(decision, |decision, cap| held(self.plan, decision, cap))
        };
        let kept: Vec<(String, Decision)> = written
            .values
            .iter()
            .filter_map(|value| {
                let decision = decide(&Self::narrowed(&value.text, &reflex, Some(&value.judgment)));
                claims(&step, &decision, &written.arg, &value.value)
                    .then(|| (value.text.clone(), held_back(decision)))
            })
            .collect();
        if kept.is_empty() {
            return;
        }
        let every = kept.len() == written.values.len();
        match written.first {
            None if every => {
                let alone = decide(&Self::narrowed(&written.own, &reflex, named.as_ref()));
                if alike(&step, &alone) {
                    read_apart(draft, k, &written.own, held_back(alone));
                }
            }
            None => {}
            Some(first) if every => read_apart(draft, k, &written.own, held_back(first)),
            Some(_) => return,
        }
        let seg = draft.segs[k].clone();
        let shared = draft.shared.get(k).cloned().unwrap_or_default();
        let of = draft.ids[k];
        for (i, (text, decision)) in kept.into_iter().enumerate() {
            let place = Segment {
                text: text.clone(),
                start: seg.start,
                end: seg.end,
                left: None,
            };
            let id = draft.insert_at(k + 1 + i, place, decision, shared.clone());
            draft.copies.push((id, of));
            draft.repaired.push((text, Repair::Again));
        }
    }

    /// A pointer that counts. A step that asks for an argument, whose words hold a word of the pack that points at
    /// several things said before — `refer.many`, `refer.each`, `recalled.both`: two where a word of
    /// `recalled.both` stands, two at least otherwise — where the person's own steps before it hold by their own
    /// words as many distinct values of the argument's kind, is written once per value in its place, the pointing
    /// words replaced by the value's as typed: «look up ana & maria, order both a pro» orders one for ana and one for
    /// maria. Each is decided narrowed to the step's reflex and kept where it reads its value and every other value
    /// as the step did; each waits for a yes that names the pointing words, since code alone read them. Never where
    /// the reading took other words of the step as answering the argument.
    pub(super) fn pointers(&self, draft: &mut Draft) -> Result<(), Need> {
        let found: Vec<PointedValues> = (1..draft.segs.len())
            .filter_map(|k| self.pointed_values(draft, k))
            .collect();
        self.undecided(
            found
                .iter()
                .flat_map(|pointed| pointed.values.iter().map(|(_, asked)| asked)),
        )?;
        // The later steps first, so that a step put in moves no earlier one.
        for pointed in found.into_iter().rev() {
            self.write_pointed(draft, pointed);
        }
        Ok(())
    }

    /// Step `k` written once per value its pointing words count (`PointedValues`), where it asks for one argument
    /// whose values the steps before it hold as many as the words count; none otherwise, and none where words other
    /// than the pointing ones answer the argument: «put that one on the clipboard? I need both» asks for «that one».
    fn pointed_values(&self, draft: &Draft, k: usize) -> Option<PointedValues> {
        let id = draft.ids[k];
        if !draft.origins[k].is_empty()
            || draft.verb(k)
            || draft.refused_at(k)
            || draft.recalled.contains(&id)
        {
            return None;
        }
        let Decision::Ask { asking, missing } = &draft.decisions[k] else {
            return None;
        };
        let active = self.plan.active().get(&asking.reflex)?;
        if !active.steps.is_empty() {
            return None;
        }
        let text = &draft.segs[k].text;
        let (from, to, count) = pointing_words(text)?;
        let counted: Vec<(&ArgName, Vec<(String, String)>)> = missing
            .iter()
            .filter_map(|asked| {
                if asked
                    .words
                    .as_ref()
                    .is_some_and(|words| words.end() <= from || to <= words.start())
                {
                    return None;
                }
                let Kind::Value { source, .. } = &active.args.get(&asked.arg)?.kind else {
                    return None;
                };
                let values = self.values_before(draft, k, source);
                let counts = match count {
                    Some(n) => values.len() == n,
                    None => values.len() >= 2,
                };
                counts.then_some((&asked.arg, values))
            })
            .collect();
        // One argument the words count: two would leave open which values are whose.
        let [(arg, values)] = counted.as_slice() else {
            return None;
        };
        let chars: Vec<char> = text.chars().collect();
        let words: String = chars[from.min(chars.len())..to.min(chars.len())]
            .iter()
            .collect();
        let named = named(&draft.decisions[k]);
        let values = values
            .iter()
            .map(|(value, typed)| {
                let written = said_once(text, (from, to), typed);
                (
                    value.clone(),
                    Self::narrowed(&written, &asking.reflex, named),
                )
            })
            .collect();
        Some(PointedValues {
            k,
            arg: (*arg).clone(),
            words,
            values,
        })
    }

    /// A step that asks, written once per value its pointing words count, standing where every text reads its value
    /// and every other value as the step held them (`fills`): the first in the step's place, the rest right after
    /// it, each held for what the plan held the step for and for the pointing words its value was read from.
    fn write_pointed(&self, draft: &mut Draft, pointed: PointedValues) {
        let k = pointed.k;
        let step = draft.decisions[k].clone();
        let readings: Vec<(String, Decision)> = pointed
            .values
            .iter()
            .map(|(value, asked)| {
                let decision = self
                    .decided(asked)
                    .cloned()
                    .expect("every value pointed at is decided");
                (value.clone(), decision)
            })
            .collect();
        if !readings
            .iter()
            .all(|(value, decision)| fills(&step, decision, &pointed.arg, value))
        {
            return;
        }
        let mut holds = plan_holds(&step);
        holds.push(Cap::Pointed {
            arg: pointed.arg.clone(),
            words: pointed.words.clone(),
        });
        let held_back = |decision: Decision| {
            holds
                .iter()
                .cloned()
                .fold(decision, |decision, cap| held(self.plan, decision, cap))
        };
        let seg = draft.segs[k].clone();
        let shared = draft.shared.get(k).cloned().unwrap_or_default();
        for (i, ((_, decision), (_, asked))) in readings.into_iter().zip(pointed.values).enumerate()
        {
            if i == 0 {
                draft.rewrite(k, &asked.text, held_back(decision));
            } else {
                draft.repaired.push((asked.text.clone(), Repair::Spliced));
                let place = Segment {
                    text: asked.text,
                    start: seg.start,
                    end: seg.end,
                    left: None,
                };
                draft.insert_at(k + i, place, held_back(decision), shared.clone());
            }
        }
    }

    /// The distinct values of a source the steps of the person's own before step `k` hold by their own words, in
    /// order, each with those words as typed: never a value another part gave or one recalled.
    fn values_before(&self, draft: &Draft, k: usize, source: &Source) -> Vec<(String, String)> {
        let mut values: Vec<(String, String)> = Vec::new();
        for j in 0..k {
            let decision = &draft.decisions[j];
            if !draft.origins[j].is_empty() {
                continue;
            }
            let (Some((_, theirs)), Some(args)) = (self.active_of(decision), args_of(decision))
            else {
                continue;
            };
            let basis = basis_of(decision);
            let Some((_, held)) = self.held_words(&draft.segs[j].text, decision) else {
                continue;
            };
            for (arg, words) in &held {
                let same = matches!(
                    theirs.args.get(arg).map(|argument| &argument.kind),
                    Some(Kind::Value { source: other, .. }) if same_source(source, other)
                );
                let given = matches!(
                    basis.and_then(|b| b.get(arg)),
                    Some(Basis::Shared { .. } | Basis::Recalled { .. })
                );
                let Some(value) = args.get(arg).and_then(stated_of) else {
                    continue;
                };
                if same && !given && !values.iter().any(|(theirs, _)| *theirs == value) {
                    values.push((value, words.text().as_str().to_owned()));
                }
            }
        }
        values
    }

    /// The words of a step's text that hold each value its decision read, as the reader holds them — a typed
    /// value's span, a listed word's by what it stands on, else where code finds that word in the words, else the
    /// word itself — with the request the text makes narrowed to the decision's reflex, whose findings they come
    /// from. None where the decision is no call, or the words cannot be read.
    fn held_words(
        &self,
        text: &str,
        decision: &Decision,
    ) -> Option<(Request, Vec<(ArgName, Span)>)> {
        let reflex = reflex_of(decision)?;
        let args = args_of(decision)?;
        let basis = basis_of(decision);
        let input = Input::new(text).ok()?;
        let request = crate::decide::request(
            self.plan,
            text,
            self.tags,
            Some(reflex),
            None,
            Scope::Full,
            &[],
        )
        .ok()?;
        let tokens = crate::words::tokens(input.as_str());
        let held = args
            .iter()
            .filter_map(|(arg, value)| {
                let words = words_of(value, basis.and_then(|b| b.get(arg)))
                    .or_else(|| {
                        let key = value.text()?;
                        let id = QuestionId::Arg(reflex.clone(), arg.clone());
                        request
                            .listed
                            .get(&id)?
                            .iter()
                            .find(|listed| listed.key.as_str() == key)
                            .map(|listed| listed.span.clone())
                    })
                    .or_else(|| {
                        let word = value.text()?.to_lowercase();
                        let token = tokens.iter().find(|token| token.plain == word)?;
                        Span::of(&input, token.start, token.end)
                    })?;
                Some((arg.clone(), words))
            })
            .collect();
        Some((request, held))
    }

    /// The texts a rule needs decided that are not yet, each once and in the order asked: the need to decide them
    /// side by side.
    fn undecided<'t>(&self, asked: impl IntoIterator<Item = &'t Asked>) -> Result<(), Need> {
        let mut missing: Vec<Asked> = Vec::new();
        for asked in asked {
            if self.decided(asked).is_none() && !missing.contains(asked) {
                missing.push(asked.clone());
            }
        }
        if missing.is_empty() {
            Ok(())
        } else {
            Err(Need::Decide { asked: missing })
        }
    }
}

/// Step `k` read again from its words with its own value alone, its words as typed kept beside, as a rewrite keeps
/// them: a step read apart, which the steps written again for its other values follow.
fn read_apart(draft: &mut Draft, k: usize, text: &str, decision: Decision) {
    draft.rewrite(k, text, decision);
    // The rewrite names the words it wrote a splice, its last repair: these were read apart.
    if let Some((_, repair)) = draft.repaired.last_mut() {
        *repair = Repair::Apart;
    }
}

/// Whether two spans of one text share a character.
fn overlaps(a: &Span, b: &Span) -> bool {
    a.start() < b.end() && b.start() < a.end()
}

/// The words that hold one value a decision read, by what it stands on: a typed value's span; the words a view,
/// a yes or the anchored choice read a listed word from. None for a flag, and for a word read from no words.
fn words_of(value: &Value, basis: Option<&Basis>) -> Option<Span> {
    match (value, basis) {
        (Value::Flag, _) => None,
        (Value::Pick { span, .. }, _) => Some(span.clone()),
        (_, Some(Basis::View { words, .. } | Basis::Words { words, .. })) => Some(words.clone()),
        (
            _,
            Some(Basis::Views {
                anchored: Some(anchored),
                ..
            }),
        ) => Some(anchored.words.clone()),
        _ => None,
    }
}

/// Where the reader's own rules set a text's words apart, in its characters: each stretch code sets apart by its
/// words — a contrast's «not X», the person's own action, a courtesy — and each clause that opens with a negation.
fn ruled_out(text: &str) -> Vec<(usize, usize)> {
    let places = reading::places(text, true);
    let mut out: Vec<(usize, usize)> = reading::stretches(text, &places)
        .iter()
        .map(|apart| (apart.start, apart.end))
        .collect();
    out.extend(
        reading::segments(text, &places)
            .iter()
            .filter(|seg| seg.excluded())
            .map(|seg| (seg.start, seg.end)),
    );
    out
}

/// What a plan held a decision for, beside what its own reading caps: a part beside it that may add a detail, the
/// whole request that named its reflex, words that point at several things its value was read from.
fn plan_holds(decision: &Decision) -> Vec<Cap> {
    match decision {
        Decision::Confirm { because, .. } => because
            .iter()
            .filter(|cap| {
                matches!(
                    cap,
                    Cap::Detail { .. } | Cap::Named { .. } | Cap::Pointed { .. }
                )
            })
            .cloned()
            .collect(),
        Decision::Ask { asking, .. } => asking.held.clone(),
        Decision::Run { .. } | Decision::Abstain { .. } => Vec::new(),
    }
}

/// Whether a step written again for another value reads as the step did with that value in place: the same
/// reflex, the value in the argument, every other value the step held as it held it and none more, and nothing
/// asked that the step did not ask.
fn claims(step: &Decision, again: &Decision, arg: &ArgName, value: &str) -> bool {
    let (Some(before), Some(after)) = (args_of(step), args_of(again)) else {
        return false;
    };
    let asked = asked_of(step);
    reflex_of(step) == reflex_of(again)
        && after.get(arg).and_then(stated_of).as_deref() == Some(value)
        && before.len() == after.len()
        && before.iter().all(|(name, held)| {
            name == arg
                || after
                    .get(name)
                    .is_some_and(|other| stated_of(other) == stated_of(held))
        })
        && asked_of(again).iter().all(|asking| asked.contains(asking))
}

/// Whether the step before a part, written again with the part's values, reads as it did with them in place:
/// the same reflex, each value in its argument, every other value as the step held it and none more, and nothing
/// asked that the step did not ask.
fn replaces(step: &Decision, again: &Decision, values: &[(ArgName, String)]) -> bool {
    let (Some(before), Some(after)) = (args_of(step), args_of(again)) else {
        return false;
    };
    let asked = asked_of(step);
    let put = |name: &ArgName| values.iter().find(|(arg, _)| arg == name);
    reflex_of(step) == reflex_of(again)
        && before.len() == after.len()
        && before.iter().all(|(name, held)| match put(name) {
            Some((_, value)) => after.get(name).and_then(stated_of).as_deref() == Some(value),
            None => after
                .get(name)
                .is_some_and(|other| stated_of(other) == stated_of(held)),
        })
        && asked_of(again).iter().all(|asking| asked.contains(asking))
}

/// Whether a step whose call asks for an argument, read again with one of its values in its words, holds that
/// value in the argument and every other value as the step held it, none more, and asks for nothing the step did not
/// ask besides the argument.
fn fills(step: &Decision, again: &Decision, arg: &ArgName, value: &str) -> bool {
    let (Some(before), Some(after)) = (args_of(step), args_of(again)) else {
        return false;
    };
    let asked = asked_of(step);
    reflex_of(step) == reflex_of(again)
        && !before.contains_key(arg)
        && after.get(arg).and_then(stated_of).as_deref() == Some(value)
        && after.len() == before.len() + 1
        && before.iter().all(|(name, held)| {
            after
                .get(name)
                .is_some_and(|other| stated_of(other) == stated_of(held))
        })
        && asked_of(again)
            .iter()
            .all(|asking| *asking != arg && asked.contains(asking))
}

/// A step's words with the list standing from `start` to `end` said once, as the given words.
fn said_once(text: &str, (start, end): (usize, usize), words: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let head: String = chars[..start.min(chars.len())].iter().collect();
    let tail: String = chars[end.min(chars.len())..].iter().collect();
    format!("{head}{words}{tail}")
}

/// Whether a part's words are values said once more: every word a value, a word of the phrase that says so, or a
/// word that carries nothing.
fn values_said_again(
    lexicon: &pack::Lexicon,
    words: &str,
    phrases: &[&str],
    values: &[(ArgName, Span, String)],
) -> bool {
    let phrase_words: Vec<String> = phrases
        .iter()
        .flat_map(|phrase| phrase.split(' '))
        .map(str::to_owned)
        .collect();
    crate::words::tokens(words).iter().all(|token| {
        token.plain.is_empty()
            || crate::words::function(lexicon, &token.plain)
            || crate::words::courtesy(&token.plain)
            || phrase_words.contains(&text::fold(&token.plain))
            || values
                .iter()
                .any(|(_, span, _)| token.start < span.end() && span.start() < token.end)
    })
}

/// The words of a text that point at several things said before, as the pack lists them — `refer.many`,
/// `refer.each`, `recalled.both` — where they first stand, the longest run of such words, in characters of the
/// words as compared; and how many things they name: two where a word of `recalled.both` is among them, none said
/// otherwise. None where the text holds none.
fn pointing_words(text: &str) -> Option<(usize, usize, Option<usize>)> {
    let lexicon = pack::lexicon(text);
    let tokens = crate::words::tokens(text);
    let folded: Vec<String> = tokens
        .iter()
        .map(|token| text::fold(&token.plain))
        .collect();
    let both = lexicon.phrases(|pack| &pack.recalled.both);
    let mut phrases = lexicon.phrases(|pack| &pack.refer.many);
    phrases.extend(lexicon.phrases(|pack| &pack.refer.each));
    phrases.extend(both.iter().copied());
    let mut covered = vec![false; tokens.len()];
    for i in 0..tokens.len() {
        for phrase in &phrases {
            let words: Vec<&str> = phrase.split(' ').collect();
            let holds = folded
                .get(i..i + words.len())
                .is_some_and(|run| run.iter().map(String::as_str).eq(words.iter().copied()));
            if holds {
                for word in covered.iter_mut().skip(i).take(words.len()) {
                    *word = true;
                }
            }
        }
    }
    let first = covered.iter().position(|word| *word)?;
    let last = (first..tokens.len()).take_while(|&i| covered[i]).last()?;
    let count = (first..=last)
        .any(|i| both.contains(&folded[i].as_str()))
        .then_some(2);
    Some((tokens[first].from, tokens[last].to, count))
}
