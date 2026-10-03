//! The plan: read, decide, repair, share, bind, order, judge — over the answers a host has gathered, stopping
//! at the first it lacks. In: a `Plan`, the adapter's gate, the request, the tags a decision is narrowed by, the
//! `Answers` so far. Out: the `Weave`, or what is needed next.

use indexmap::IndexMap;

use super::reading::{self, How, Left, Order, Ref, SURE, Segment, Split, Stretch, Unclean, Where};
use super::{
    Answers, Aside, Asked, Because, Binding, Count, Folded, From, Need, Outcome, Parted, Planning,
    Repair, Shared, Step, Verdict, Via, Weave, When, field_names, named,
};
use crate::account::{Does, Left as Run};
use crate::adapter::{Fault, Gate, Key, Prob, Question, QuestionId, Scope};
use crate::calendar::Day;
use crate::call::Value;
use crate::decide::{
    Basis, Cap, Choices, Decision, Judgment, Prompt, alone, carry, given, held, joined, names_of,
    words, yielded,
};
use crate::manifest::{self, Effect, Kind, MOST_STEPS, Recognizer, Source, Yield};
use crate::name::{ArgName, FieldName, LocalName, Tag, VocabName, Word};
use crate::pack;
use crate::plan::{Active, Plan};
use crate::propose::PickValue;
use crate::text::{self, Clean, Input, Span};
use crate::waits;

/// A split the engine judged below this is never tried.
const LOW: f64 = 0.35;
/// A split the engine called two things with at least this probability is never merged back.
const FIRM: f64 = 0.8;
/// Under this the engine doubted the split: a part that decided as another reflex tries the fan-out first.
const DOUBT: f64 = 0.5;
/// The most split points one request is asked about; past it — a pasted list, a hostile line — the request is
/// one input, since each question carries both sides of the sentence and the request would grow as its square.
const MOST_SPLITS: usize = 24;
/// At this share of *one thing*, a request the split points cut is one step, and no cut is made.
const ONE: f64 = 0.5;
/// At this share a run of a step's words says what to do, as the account reads it.
const SAYS: f64 = 0.5;
/// How deep a plan may stand inside a plan: a step of a playbook that routes to a playbook expands once more, and
/// a playbook inside that is refused.
const HOPS: usize = 2;
/// The plan of a request over the answers so far: the weave, or what is needed next; a fault when an answer does
/// not validate against its request. The gate is what a shared word's fill is gated by, as a person's answer is.
pub fn plan(
    plan: &Plan,
    gate: Option<&Gate>,
    input: &str,
    tags: &[Tag],
    answers: &Answers,
) -> Result<Planning, Fault> {
    let planner = Planner {
        plan,
        gate,
        tags,
        answers,
        request: reading::canonical(input),
    };
    Ok(match planner.planned()? {
        Ok(weave) => Planning::Done { weave },
        Err(need) => Planning::Need { need },
    })
}

struct Planner<'a> {
    plan: &'a Plan,
    gate: Option<&'a Gate>,
    tags: &'a [Tag],
    answers: &'a Answers,
    /// The request in the words' own order.
    request: String,
}

/// The plan as it takes shape: the segments with their decisions, the split points taken, what was left out,
/// how a segment that matched nothing was settled, the words shared into each segment's arguments, where each
/// segment came from when a playbook wrote it, and what a fold or a refusal names.
struct Draft {
    chars: Vec<char>,
    segs: Vec<Segment>,
    decisions: Vec<Decision>,
    taken: Vec<Split>,
    excluded: Vec<String>,
    /// The request's own segments among those left out, with where they stand.
    apart: Vec<Segment>,
    repaired: Vec<(String, Repair)>,
    shared: Vec<IndexMap<ArgName, Shared>>,
    /// Per segment, the expansions it came from, outermost first; none for the person's own.
    origins: Vec<Vec<Origin>>,
    /// Per segment, a number of its own: what a reason or a fold names, since expansions renumber.
    ids: Vec<usize>,
    next: usize,
    /// Every playbook expanded, in order.
    expansions: Vec<Expansion>,
    /// A step refused for what its words route to, by its id.
    refusals: Vec<(usize, Refused)>,
    /// The plan refused whole: past the cap, or a part left out beside a plan.
    refused: Vec<Because>,
    /// Every part of the request folded into a step.
    folded: Vec<Fold>,
    /// Where each part that left the plan stood in the request: folded, set aside, or out of it.
    out: Vec<(usize, usize)>,
    /// Per segment, the words as the person typed them, once a rewrite changed them; none until then.
    typed: Vec<Option<String>>,
    /// Whether the request asks one thing and stands as one step: no list, item or repair cuts it again.
    one: bool,
    /// The parts that match no reflex and ask for nothing: out of the plan.
    asides: Vec<Aside>,
    /// Where each contrast's «not X» stands, which code cut at both ends without asking.
    ruled: Vec<(usize, usize)>,
    /// What the whole each part the items rule cut came from read as.
    wholes: Vec<Whole>,
    /// Per segment a second verb made, by id: the id of the step whose object it shares.
    verbs: Vec<(usize, usize)>,
    /// The segments filled from this session's results by words that point at them, by id: a word of theirs
    /// names no step.
    recalled: Vec<usize>,
}

/// A whole the items rule cut that read as a call, for one of its parts: the part by its id, the whole's reflex,
/// and the words of each value its call held.
struct Whole {
    part: usize,
    reflex: LocalName,
    read: Vec<(ArgName, String)>,
}

/// A part of the request folded into a step: its words, the step by its id, the reflex the part read as, the
/// arguments it gave the step's call, and the word of it that picked the step among several.
struct Fold {
    text: String,
    into: usize,
    reflex: Option<LocalName>,
    gave: Vec<ArgName>,
    picked: Option<String>,
}

/// One playbook a segment came from: which expansion wrote it, its place in the playbook, and what picks it when
/// it may not run, bound once the steps stand.
#[derive(Clone)]
struct Origin {
    expansion: usize,
    from: From,
    when: Option<manifest::When>,
}

/// A playbook expanded: the part that picked it, the playbook, the values its sentences were filled from, its
/// decision's prompt before the runner-up's step is named, the runner-up whose `fits` capped it, and the effect
/// it claims.
struct Expansion {
    text: String,
    playbook: LocalName,
    args: IndexMap<ArgName, Value>,
    prompt: Prompt,
    effect: Effect,
}

/// A playbook's sentences filled: each text with where it came from, and what picks it when it may not run.
type Filled = Vec<(String, From, Option<manifest::When>)>;

/// Why a step that routes to a playbook is refused: the playbook is on its own chain, past the depth, or the
/// step may not run, and a branch is one step.
enum Refused {
    Nested(LocalName),
    TooDeep(LocalName),
    BranchIntoPlan(LocalName),
}

/// What the plan carries beside its steps once every phase ran: the review of each expansion, the refusals
/// named by step, the folds, and the steps that may not run.
struct Extra {
    reviewed: Vec<Because>,
    refusals: Vec<Because>,
    folded: Vec<Folded>,
    asides: Vec<Aside>,
    branches: Vec<Branching>,
    /// Where every step and every part out of the plan stands in the request.
    spans: Vec<(usize, usize)>,
    /// Whether the request stands as one step because it asks one thing.
    one: bool,
}

/// A step that may not run, once the segments stand: its number, what picks it, and the step whose result does —
/// the nearest before it in the same expansion without `when` — none when that step is a plan of its own, which
/// yields nothing.
struct Branching {
    step: usize,
    when: manifest::When,
    source: Option<usize>,
}

impl Draft {
    /// The draft over the request's segments: the ones left out apart, the rest the person's own.
    fn new(chars: Vec<char>, segments: &[Segment], taken: Vec<Split>) -> Self {
        // A part code set aside by its words stands apart as a part left out does: never decided, never run.
        let apart = |seg: &Segment| seg.excluded() || matches!(seg.left, Some(Left::Aside(_)));
        let segs: Vec<Segment> = segments.iter().filter(|seg| !apart(seg)).cloned().collect();
        let count = segs.len();
        Self {
            chars,
            segs,
            decisions: Vec::new(),
            taken,
            excluded: segments
                .iter()
                .filter(|seg| seg.excluded())
                .map(|seg| seg.text.clone())
                .collect(),
            apart: segments.iter().filter(|seg| apart(seg)).cloned().collect(),
            repaired: Vec::new(),
            shared: Vec::new(),
            origins: vec![Vec::new(); count],
            ids: (0..count).collect(),
            next: count,
            expansions: Vec::new(),
            refusals: Vec::new(),
            refused: Vec::new(),
            folded: Vec::new(),
            out: Vec::new(),
            typed: vec![None; count],
            one: false,
            asides: Vec::new(),
            ruled: Vec::new(),
            wholes: Vec::new(),
            verbs: Vec::new(),
            recalled: Vec::new(),
        }
    }

    /// Segment `k` replaced by several, each with its decision and origins, in every parallel list.
    fn replace(
        &mut self,
        k: usize,
        segs: Vec<Segment>,
        decisions: Vec<Decision>,
        origins: Vec<Vec<Origin>>,
    ) {
        let count = segs.len();
        let ids: Vec<usize> = (0..count).map(|i| self.next + i).collect();
        self.next += count;
        // A part folded into the segment is folded into the step of it that reads as the part did, or its first.
        let was = self.ids[k];
        for fold in &mut self.folded {
            if fold.into == was {
                let at = decisions
                    .iter()
                    .position(|decision| reflex_of(decision) == fold.reflex.as_ref())
                    .unwrap_or(0);
                fold.into = ids.get(at).copied().unwrap_or(was);
            }
        }
        if self.shared.len() == self.segs.len() {
            self.shared.splice(k..=k, vec![IndexMap::new(); count]);
        }
        self.segs.splice(k..=k, segs);
        self.decisions.splice(k..=k, decisions);
        self.origins.splice(k..=k, origins);
        self.ids.splice(k..=k, ids);
        self.typed.splice(k..=k, vec![None; count]);
    }

    /// Segments `a..=b` merged into one, the person's own; what was folded into one of them is folded into it.
    fn merge(&mut self, a: usize, b: usize, seg: Segment, decision: Decision) {
        for fold in &mut self.folded {
            if self.ids[a..=b].contains(&fold.into) {
                fold.into = self.next;
            }
        }
        if self.shared.len() == self.segs.len() {
            self.shared.splice(a..=b, [IndexMap::new()]);
        }
        self.segs.splice(a..=b, [seg]);
        self.decisions.splice(a..=b, [decision]);
        self.origins.splice(a..=b, [Vec::new()]);
        self.ids.splice(a..=b, [self.next]);
        self.typed.splice(a..=b, [None]);
        self.next += 1;
    }

    /// The one segment of a draft that held none, the person's own.
    fn only(&mut self, seg: Segment, decision: Decision) {
        self.segs.push(seg);
        self.decisions.push(decision);
        self.origins.push(Vec::new());
        self.ids.push(self.next);
        self.typed.push(None);
        self.next += 1;
    }

    /// Segment `k` gone, from every parallel list.
    fn remove(&mut self, k: usize) {
        if self.shared.len() == self.segs.len() {
            self.shared.remove(k);
        }
        self.segs.remove(k);
        self.decisions.remove(k);
        self.origins.remove(k);
        self.ids.remove(k);
        self.typed.remove(k);
    }

    /// Whether segment `k` is a second verb's step: never folded, merged or held as a neighbour.
    fn verb(&self, k: usize) -> bool {
        self.verbs.iter().any(|(verb, _)| *verb == self.ids[k])
    }

    /// The neighbour a part is read beside: the segment before it, else the one after; never a second verb's
    /// step, which stands in for none, and none for such a step.
    fn neighbour(&self, k: usize) -> Option<usize> {
        if self.verb(k) {
            return None;
        }
        if k > 0 {
            (0..k).rev().find(|&j| !self.verb(j))
        } else {
            (k + 1..self.segs.len()).find(|&j| !self.verb(j))
        }
    }

    /// A segment of the person's own put in right after `k`, in every parallel list: its id.
    fn insert(&mut self, k: usize, seg: Segment, decision: Decision) -> usize {
        let at = k + 1;
        if self.shared.len() == self.segs.len() {
            self.shared.insert(at, IndexMap::new());
        }
        self.segs.insert(at, seg);
        self.decisions.insert(at, decision);
        self.origins.insert(at, Vec::new());
        self.ids.insert(at, self.next);
        self.typed.insert(at, None);
        self.next += 1;
        self.next - 1
    }

    /// Whether the segment at `k` is refused for what its words route to: no word is carried into it.
    fn refused_at(&self, k: usize) -> bool {
        self.refusals.iter().any(|(id, _)| *id == self.ids[k])
    }

    /// The step number a segment id stands at now, from 1.
    fn position(&self, id: usize) -> usize {
        self.ids.iter().position(|i| *i == id).map_or(0, |i| i + 1)
    }

    /// Segment `k`'s words as the person typed them, where a rewrite wrote them anew; its text otherwise.
    fn as_typed(&self, k: usize) -> String {
        self.typed[k]
            .clone()
            .unwrap_or_else(|| self.segs[k].text.clone())
    }

    /// Segment `k`, a part of the person's own, folded into the step of this id: gone, its words kept as typed,
    /// with the arguments it gave the step and the word of it that picked the step.
    fn fold_into(&mut self, k: usize, into: usize, gave: Vec<ArgName>, picked: Option<String>) {
        let text = self.as_typed(k);
        let reflex = reflex_of(&self.decisions[k]).cloned();
        // What was folded into the part goes where the part goes.
        let was = self.ids[k];
        for fold in &mut self.folded {
            if fold.into == was {
                fold.into = into;
            }
        }
        self.folded.push(Fold {
            text,
            into,
            reflex,
            gave,
            picked,
        });
        self.leave(k);
    }

    /// Segment `k` out of the plan: gone, with where it stood kept.
    fn leave(&mut self, k: usize) {
        self.out.push((self.segs[k].start, self.segs[k].end));
        self.remove(k);
    }

    /// The first step, by its id, of a plan this playbook already wrote from values that hold every value `read`.
    fn expanded(&self, playbook: &LocalName, read: &IndexMap<ArgName, Value>) -> Option<usize> {
        let expansion = self.expansions.iter().position(|expansion| {
            expansion.playbook == *playbook
                && read.iter().all(|(arg, value)| {
                    expansion
                        .args
                        .get(arg)
                        .is_some_and(|theirs| stated(theirs) == stated(value))
                })
        })?;
        let first = self
            .origins
            .iter()
            .position(|chain| chain.iter().any(|origin| origin.expansion == expansion))?;
        Some(self.ids[first])
    }
}

/// A fragment settled as another item of its neighbour's task.
struct Fan {
    decision: Decision,
    text: String,
    how: Repair,
}

/// Two verbs before one object: kept whole, step `k`'s words without the second verb, and the second verb; or
/// cut, step `k` a bare verb, the next part's object and its verb.
enum Verbs {
    Whole {
        k: usize,
        base: String,
        second: String,
    },
    Cut {
        k: usize,
        object: String,
        verb: String,
    },
}

impl<'a> Planner<'a> {
    /// The decision a host made for a text, when it has.
    fn decided(&self, asked: &Asked) -> Option<&Decision> {
        self.answers
            .decided
            .iter()
            .find(|(a, _)| a == asked)
            .map(|(_, decision)| decision)
    }

    /// A segment's decision over the reflexes the tags allow; whole when the segment is the whole request, the
    /// one text a host may hand the session's results for.
    fn segment(&self, text: &str) -> Asked {
        Asked {
            text: text.to_owned(),
            tags: self.tags.to_vec(),
            only: None,
            whole: text == self.request.trim(),
            named: None,
        }
    }

    /// A text decided narrowed to one reflex, its route the one the whole request gave, where it gave one.
    fn narrowed(text: &str, reflex: &LocalName, named: Option<&Judgment>) -> Asked {
        Asked {
            text: text.to_owned(),
            tags: Vec::new(),
            only: Some(reflex.clone()),
            whole: false,
            named: named.cloned(),
        }
    }

    /// One decision, or the need for it.
    fn decide(&self, asked: Asked) -> Result<Decision, Need> {
        self.decided(&asked)
            .cloned()
            .ok_or(Need::Decide { asked: vec![asked] })
    }

    fn planned(&self) -> Result<Result<Weave, Need>, Fault> {
        let judged = match self.judge()? {
            Ok(judged) => judged,
            Err(need) => return Ok(Err(need)),
        };
        let taken: Vec<Split> = judged
            .iter()
            .filter(|split| split.p.is_some_and(|p| p.get() >= LOW))
            .cloned()
            .collect();
        // A segment that begins with a negation is left out: what the person said not to do is no step. One that
        // begins with a condition refuses the whole request before anything is decided: no step can judge it.
        let mut segments = reading::segments(&self.request, &taken);
        // A part that is the person's own action or a courtesy is set aside by its words.
        let places = reading::places(&self.request, true);
        let stretches = reading::stretches(&self.request, &places);
        reading::set_aside(&self.request, &mut segments, &stretches);
        if let Some(seg) = segments
            .iter()
            .find(|seg| seg.left == Some(Left::Conditional))
        {
            let because = Because::Conditional {
                text: seg.text.clone(),
            };
            return Ok(Ok(self.refused_whole(judged, because)));
        }
        let mut draft = Draft::new(self.request.chars().collect(), &segments, taken);
        draft.ruled = stretches
            .iter()
            .filter(|apart| apart.what == Stretch::Contrast)
            .map(|apart| (apart.start, apart.end))
            .collect();
        // A request that asks one thing is read whole, what it rules out with what it asks.
        let whole = self
            .segments(&mut draft)
            .and_then(|()| self.one(&mut draft));
        if let Err(need) = whole {
            return Ok(Err(need));
        }
        // A request that is only what not to do is nothing to do.
        if draft.segs.is_empty() {
            let extra = Extra {
                reviewed: Vec::new(),
                refusals: Vec::new(),
                folded: Vec::new(),
                asides: by_words(&draft.apart),
                branches: Vec::new(),
                spans: draft.apart.iter().map(|seg| (seg.start, seg.end)).collect(),
                one: false,
            };
            return Ok(Ok(self.finish(
                judged,
                &draft.taken,
                Vec::new(),
                draft.excluded,
                extra,
            )));
        }
        self.recalled(&mut draft);
        let phases = self
            .coordinated(&mut draft)
            .and_then(|()| self.recut(&mut draft, &judged))
            .and_then(|()| self.corrected(&mut draft))
            .and_then(|()| self.expand(&mut draft))
            .and_then(|()| self.share(&mut draft))
            .and_then(|()| self.verify(&mut draft))
            .and_then(|()| {
                self.same(&mut draft);
                self.fill(&mut draft)
            })
            .and_then(|()| self.expand(&mut draft));
        if let Err(need) = phases {
            return Ok(Err(need));
        }
        fold(&mut draft);
        self.again(&mut draft);
        self.lacking(&mut draft);
        self.doubted(&mut draft);
        let refs = match self.refer(&draft)? {
            Ok(refs) => refs,
            Err(need) => return Ok(Err(need)),
        };
        let extra = self.resolved(&draft);
        let steps = self.steps_of(&draft, refs);
        Ok(Ok(self.finish(
            judged,
            &draft.taken,
            steps,
            draft.excluded,
            extra,
        )))
    }

    /// The steps as the plan holds them, from the draft's segments once every phase ran.
    fn steps_of(&self, draft: &Draft, refs: Vec<Vec<Ref>>) -> Vec<Step> {
        draft
            .segs
            .iter()
            .zip(&draft.decisions)
            .zip(refs)
            .enumerate()
            .map(|(k, ((seg, decision), refs))| {
                let reflex = reflex_of(decision).cloned();
                let effect = reflex
                    .as_ref()
                    .and_then(|reflex| self.plan.active().get(reflex))
                    .map(|active| active.effect);
                Step {
                    n: k + 1,
                    text: seg.text.clone(),
                    end: seg.end,
                    decision: decision.clone(),
                    reflex,
                    effect,
                    refs,
                    repair: repair_of(draft, k),
                    shared: draft.shared.get(k).cloned().unwrap_or_default(),
                    beside: IndexMap::new(),
                    after: Vec::new(),
                    from: draft.origins[k]
                        .iter()
                        .map(|origin| origin.from.clone())
                        .collect(),
                    when: None,
                    typed: draft.typed[k].clone().filter(|typed| *typed != seg.text),
                }
            })
            .collect()
    }

    /// The request refused before anything is decided: no step, the split points as judged, one reason.
    fn refused_whole(&self, splits: Vec<Split>, because: Because) -> Weave {
        Weave {
            input: self.request.clone(),
            steps: Vec::new(),
            binds: Vec::new(),
            stages: Vec::new(),
            verdict: Verdict {
                outcome: Outcome::Refuse,
                because: vec![because],
            },
            exclusive: false,
            excluded: Vec::new(),
            folded: Vec::new(),
            asides: Vec::new(),
            splits,
            count: self.count(false),
        }
    }

    /// How many things the request asks, where it was asked, and whether it stands as one step for it.
    fn count(&self, as_one: bool) -> Option<Count> {
        let one = self.answers.judged.as_ref().and_then(reading::one_thing)?;
        Some(Count { one, as_one })
    }

    /// A playbook expanded in place. A segment decided as a playbook — a run, or a confirm — is replaced by its
    /// filled sentences, each a segment with the part's offsets, decided as any segment over the whole set; a
    /// sentence decided as a playbook expands in turn, to a depth of two; one that routes to a playbook on its own
    /// chain is refused, and so is a plan grown past the most steps it may hold. An ask stays: the verdict asks.
    fn expand(&self, draft: &mut Draft) -> Result<(), Need> {
        let mut k = 0;
        while k < draft.segs.len() {
            let decision = &draft.decisions[k];
            let Some((reflex, active)) = self.active_of(decision) else {
                k += 1;
                continue;
            };
            let reflex = reflex.clone();
            let Some(args) = complete(decision) else {
                k += 1;
                continue;
            };
            let id = draft.ids[k];
            if active.steps.is_empty() || draft.refusals.iter().any(|(i, _)| *i == id) {
                k += 1;
                continue;
            }
            let chain = &draft.origins[k];
            // A part of the person's own that picks a playbook whose plan already stands, every value it read
            // equal to that plan's, says the situation twice: it folds into the plan, as a repeated step does.
            if chain.is_empty()
                && let Some(into) = draft.expanded(&reflex, args)
            {
                draft.fold_into(k, into, Vec::new(), None);
                continue;
            }
            if let Some(refused) = refusal(chain, &reflex) {
                draft.refusals.push((id, refused));
                k += 1;
                continue;
            }
            let Some((filled, decisions)) = self.sentences(&reflex, active, args)? else {
                k += 1;
                continue;
            };
            let expansion = draft.expansions.len();
            let prompt = prompt_of(decision, active);
            let seg = draft.segs[k].clone();
            let chain = chain.clone();
            draft.expansions.push(Expansion {
                text: seg.text.clone(),
                playbook: reflex.clone(),
                args: args.clone(),
                prompt,
                effect: active.effect,
            });
            let (segs, origins): (Vec<Segment>, Vec<Vec<Origin>>) = filled
                .into_iter()
                .map(|(text, from, when)| {
                    let mut origins = chain.clone();
                    origins.push(Origin {
                        expansion,
                        from,
                        when,
                    });
                    (
                        Segment {
                            text,
                            start: seg.start,
                            end: seg.end,
                            left: None,
                        },
                        origins,
                    )
                })
                .unzip();
            draft.replace(k, segs, decisions, origins);
            if draft.segs.len() > MOST_STEPS {
                draft.refused.push(Because::TooLong {
                    playbook: reflex.clone(),
                    steps: draft.segs.len(),
                });
                break;
            }
        }
        // A part that says what not to do beside a plan a playbook wrote is a guess either way: refused whole.
        if let (Some(text), Some(expansion)) = (draft.excluded.first(), draft.expansions.first())
            && !draft
                .refused
                .iter()
                .any(|because| matches!(because, Because::Excluded { .. }))
        {
            draft.refused.push(Because::Excluded {
                text: text.clone(),
                playbook: expansion.playbook.clone(),
            });
        }
        Ok(())
    }

    /// A playbook's sentences filled from a decision's values and decided over the whole set: each filled text
    /// with where it came from, and its decision; the need when one is not decided yet; none when a sentence
    /// could not be filled, which never happens to a complete call.
    fn sentences(
        &self,
        reflex: &LocalName,
        active: &Active,
        args: &IndexMap<ArgName, Value>,
    ) -> Result<Option<(Filled, Vec<Decision>)>, Need> {
        let filled: Filled = active
            .steps
            .iter()
            .enumerate()
            .filter_map(|(i, sentence)| {
                let (text, slots) = sentence.filled(args)?;
                let from = From {
                    playbook: reflex.clone(),
                    step: i + 1,
                    slots,
                };
                Some((text, from, sentence.when().cloned()))
            })
            .collect();
        if filled.len() != active.steps.len() {
            return Ok(None);
        }
        let wanted: Vec<Asked> = filled
            .iter()
            .map(|(text, _, _)| self.segment(text))
            .collect();
        let mut missing: Vec<Asked> = Vec::new();
        for asked in &wanted {
            if self.decided(asked).is_none() && !missing.contains(asked) {
                missing.push(asked.clone());
            }
        }
        if !missing.is_empty() {
            return Err(Need::Decide { asked: missing });
        }
        let decisions: Vec<Decision> = wanted
            .iter()
            .map(|asked| {
                self.decided(asked)
                    .cloned()
                    .expect("every sentence is decided")
            })
            .collect();
        Ok(Some((filled, decisions)))
    }

    /// What the plan carries beside its steps, resolved once the segments stand: each expansion's review — its
    /// first step, its own line ending in the runner-up's step when that reflex is a step's, and the steps' worst
    /// effect when it is tighter than the claim — the refusals by step, and the folds.
    fn resolved(&self, draft: &Draft) -> Extra {
        let reviewed = draft
            .expansions
            .iter()
            .enumerate()
            .filter_map(|(e, expansion)| {
                let members: Vec<usize> = (0..draft.segs.len())
                    .filter(|&k| draft.origins[k].iter().any(|origin| origin.expansion == e))
                    .collect();
                let first = *members.first()?;
                let reach = members
                    .iter()
                    .filter_map(|&k| self.active_of(&draft.decisions[k]))
                    .map(|(_, active)| active.effect)
                    .max()
                    .filter(|worst| *worst > expansion.effect);
                Some(Because::Reviewed {
                    step: first + 1,
                    playbook: expansion.playbook.clone(),
                    text: expansion.text.clone(),
                    prompt: Prompt {
                        own: expansion.prompt.own.clone(),
                        reason: waits::reviewed(&expansion.prompt.reason, reach),
                        template: expansion.prompt.template.clone(),
                    },
                })
            })
            .collect();
        let mut refusals: Vec<Because> = draft
            .refusals
            .iter()
            .map(|(id, refused)| match refused {
                Refused::Nested(playbook) => Because::Nested {
                    step: draft.position(*id),
                    playbook: playbook.clone(),
                },
                Refused::TooDeep(playbook) => Because::TooDeep {
                    step: draft.position(*id),
                    playbook: playbook.clone(),
                },
                Refused::BranchIntoPlan(playbook) => Because::BranchIntoPlan {
                    step: draft.position(*id),
                    playbook: playbook.clone(),
                },
            })
            .collect();
        refusals.extend(draft.refused.iter().cloned());
        let folded = draft
            .folded
            .iter()
            .map(|fold| Folded {
                text: fold.text.clone(),
                into: draft.position(fold.into),
                gave: fold.gave.clone(),
                picked: fold.picked.clone(),
            })
            .collect();
        let spans = draft
            .segs
            .iter()
            .chain(&draft.apart)
            .map(|seg| (seg.start, seg.end))
            .chain(draft.out.iter().copied())
            .collect();
        let mut asides = by_words(&draft.apart);
        asides.extend(draft.asides.iter().cloned());
        Extra {
            reviewed,
            refusals,
            folded,
            asides,
            branches: branches_of(draft),
            spans,
            one: draft.one,
        }
    }

    /// Every split point, judged: a candidate a negation follows is taken without asking; a request the engine
    /// cannot be asked about, a control character among its words, is one step.
    fn judge(&self) -> Result<Result<Vec<Split>, Need>, Fault> {
        let mut all = reading::places(&self.request, true);
        // A stretch code sets apart by its words is cut at its ends without asking.
        let stretches = reading::stretches(&self.request, &all);
        all = reading::set_apart(&self.request, all, &stretches);
        if all.len() > MOST_SPLITS {
            return Ok(Ok(Vec::new()));
        }
        Ok(match reading::judging(&self.request, &all) {
            Err(Unclean) => Ok(Vec::new()),
            Ok(None) => Ok(all
                .iter()
                .map(|split| Split {
                    p: Some(SURE),
                    ..split.clone()
                })
                .collect()),
            Ok(Some((asked, judge))) => match &self.answers.judged {
                Some(raw) => Ok(reading::judged(&all, &asked, &judge, raw.clone())?),
                None => Err(Need::Judge {
                    request: judge,
                    ahead: self.ahead(&all),
                }),
            },
        })
    }

    /// What is decided ahead of the cut, beside its question: the whole request, the one step where nothing is cut;
    /// and, where a sign or a letter typed for «and» is among the places, the parts the places make, which are
    /// the steps where every place is cut. Each waits on no round of its own; nothing, once it is decided.
    fn ahead(&self, all: &[Split]) -> Vec<Asked> {
        let mut texts = vec![self.request.trim().to_owned()];
        if all.iter().any(|split| reading::joins(&split.word)) {
            let mut parts = reading::segments(&self.request, all);
            let stretches = reading::stretches(&self.request, all);
            reading::set_aside(&self.request, &mut parts, &stretches);
            for part in parts {
                if part.left.is_none() && !texts.contains(&part.text) {
                    texts.push(part.text);
                }
            }
        }
        texts
            .iter()
            .map(|text| self.segment(text))
            .filter(|asked| self.decided(asked).is_none())
            .collect()
    }

    /// The cut made again by the words: a list's items, a whole's parts, a fragment settled beside its
    /// neighbour. A request that asks one thing stands as its one step.
    fn recut(&self, draft: &mut Draft, judged: &[Split]) -> Result<(), Need> {
        if draft.one {
            return Ok(());
        }
        self.lists(draft, judged)?;
        self.items(draft, judged)?;
        self.continued(draft)?;
        self.repair(draft)
    }

    /// A part that continues the step before it. A part whose words are values alone — «2 of BOK-603» after
    /// «buy 3 of OUT-503» — names no action of its own, so its route was asked of words that hold none, and goes
    /// to whichever reflex takes such values, a coin toss between the step's reflex and a sibling. Whatever the
    /// engine made of it alone, the action it asks for is the step's: it is put in that step's words in place of
    /// the values they hold, «buy 2 of BOK-603», every word still the person's own, and decided narrowed to that
    /// step's reflex, as a fragment spliced is, so that the route is never the part's own toss. The reading
    /// stands where it reads every value the part holds into the arguments the step's values filled, and the
    /// part stays as it read alone otherwise. The step before a run of such parts is the one before the first
    /// of them; a step that picks a playbook is continued by none.
    fn continued(&self, draft: &mut Draft) -> Result<(), Need> {
        let mut wanted: Vec<(usize, usize, Asked, usize, Vec<Span>)> = Vec::new();
        let mut head: Option<usize> = None;
        for k in 0..draft.segs.len() {
            // A second verb's step stands between a step and its continuation, and is neither.
            if draft.verb(k) {
                continue;
            }
            let joined = k > 0 && split_before(&draft.taken, &draft.segs, k).is_some();
            let alone = if repair_of(draft, k).is_none() {
                values_alone(&draft.segs[k].text, Some(&draft.decisions[k]))
            } else {
                None
            };
            let (Some(h), Some(values)) = (head.filter(|_| joined), alone) else {
                head = Some(k);
                continue;
            };
            let Some(before) = reflex_of(&draft.decisions[h]) else {
                continue;
            };
            // A part that reads alone as a call of other values than the step before it takes is no
            // continuation of it; one that reads as nothing, or as any call of the same values, may be.
            if reflex_of(&draft.decisions[k]).is_some()
                && !self.takes_the_same(before, &draft.decisions[k])
            {
                continue;
            }
            if let Some((text, at)) = in_place(
                &draft.segs[h].text,
                &draft.decisions[h],
                &draft.segs[k].text,
            ) {
                wanted.push((k, h, Self::narrowed(&text, before, None), at, values));
            }
        }
        let mut missing: Vec<Asked> = Vec::new();
        for (_, _, asked, _, _) in &wanted {
            if self.decided(asked).is_none() && !missing.contains(asked) {
                missing.push(asked.clone());
            }
        }
        if !missing.is_empty() {
            return Err(Need::Decide { asked: missing });
        }
        let mut paid: Vec<(usize, usize)> = Vec::new();
        for (k, h, asked, at, values) in wanted {
            let decision = self
                .decided(&asked)
                .cloned()
                .expect("every part put in place is decided");
            if continues(&draft.decisions[h], &decision, &asked.text, at, &values) {
                draft.decisions[k] = decision;
                draft.segs[k].text.clone_from(&asked.text);
                draft.repaired.push((asked.text, Repair::Spliced));
                // A second verb of the step is the continuation's too: «buy and pay 3 of OUT-503 and 2 of
                // BOK-603» pays each purchase.
                let source = draft.ids[h];
                if let Some(j) = (0..draft.segs.len()).find(|&j| {
                    draft
                        .verbs
                        .iter()
                        .any(|(verb, of)| *verb == draft.ids[j] && *of == source)
                }) {
                    paid.push((k, j));
                }
            }
        }
        for (k, j) in paid.into_iter().rev() {
            let seg = Segment {
                text: draft.segs[j].text.clone(),
                start: draft.segs[k].start,
                end: draft.segs[k].end,
                left: None,
            };
            let decision = draft.decisions[j].clone();
            let of = draft.ids[k];
            let id = draft.insert(k, seg, decision);
            draft.verbs.push((id, of));
        }
        Ok(())
    }

    /// A second verb before the object of the first: «buy and pay 3 of OUT-503». The engine may keep the words
    /// whole — cut at the «and», «pay 3 of OUT-503» asks for nothing a reflex does — and the account then reads
    /// «buy and pay» as what the step does, so the second verb is lost; or it may cut them, and «pay 2 of
    /// HOM-403» reads the count as an order. Either way the object is the first verb's, and the second verb is a
    /// step of its own right after it, which takes from the first's result as «it» would, under its own gate as
    /// any step is. Kept whole, the words after the «and» in the run that says what to do are read alone; cut,
    /// the second part's object is put after the first's bare verb, «buy 2 of HOM-403», decided narrowed to its
    /// reflex, and the second part keeps its verb alone. A second verb that reads as the first's reflex, or as
    /// none, changes nothing.
    fn coordinated(&self, draft: &mut Draft) -> Result<(), Need> {
        let mut wanted: Vec<Verbs> = Vec::new();
        for k in 0..draft.segs.len() {
            if !draft.origins[k].is_empty() {
                continue;
            }
            if let Some((base, second)) = second_verb(&draft.segs[k].text, &draft.decisions[k]) {
                wanted.push(Verbs::Whole { k, base, second });
            } else if let Some((object, verb)) = self.cut_apart(draft, k) {
                wanted.push(Verbs::Cut { k, object, verb });
            }
        }
        let mut missing: Vec<Asked> = Vec::new();
        for verbs in &wanted {
            for asked in self.verbs_asked(draft, verbs) {
                if self.decided(&asked).is_none() && !missing.contains(&asked) {
                    missing.push(asked);
                }
            }
        }
        if !missing.is_empty() {
            return Err(Need::Decide { asked: missing });
        }
        // The later steps first, so that a step put in moves no earlier one.
        for verbs in wanted.into_iter().rev() {
            match verbs {
                Verbs::Whole { k, base, second } => {
                    let decision = self
                        .decide(self.segment(&second))
                        .expect("every second verb is decided");
                    let Some(first) = reflex_of(&draft.decisions[k]).cloned() else {
                        continue;
                    };
                    if reflex_of(&decision).is_none_or(|reflex| *reflex == first) {
                        continue;
                    }
                    // The step's words without the second verb, where they read as the step read with it: a
                    // continuation is then put in words that say one thing.
                    let without = self
                        .decide(Self::narrowed(&base, &first, None))
                        .expect("every step without its second verb is decided");
                    if alike(&draft.decisions[k], &without) {
                        let typed = draft.as_typed(k);
                        draft.typed[k] = Some(typed);
                        draft.decisions[k] = without;
                        draft.segs[k].text.clone_from(&base);
                        draft.repaired.push((base, Repair::First));
                    }
                    let seg = Segment {
                        text: second.clone(),
                        start: draft.segs[k].start,
                        end: draft.segs[k].end,
                        left: None,
                    };
                    let source = draft.ids[k];
                    let id = draft.insert(k, seg, decision);
                    draft.verbs.push((id, source));
                    draft.repaired.push((second, Repair::Verb));
                }
                Verbs::Cut { k, object, verb } => {
                    let Some(first) = reflex_of(&draft.decisions[k]).cloned() else {
                        continue;
                    };
                    let text = format!("{} {object}", draft.segs[k].text);
                    let whole = self
                        .decide(Self::narrowed(&text, &first, None))
                        .expect("every object put after its verb is decided");
                    let alone = self
                        .decide(self.segment(&verb))
                        .expect("every verb left alone is decided");
                    let at = draft.segs[k].text.chars().count() + 1;
                    let Some(values) = values_alone(&object, None) else {
                        continue;
                    };
                    let takes = reflex_of(&alone).is_some_and(|reflex| *reflex != first);
                    if !takes || !shares(&draft.decisions[k], &whole, &text, at, &values) {
                        continue;
                    }
                    draft.decisions[k] = whole;
                    draft.segs[k].text.clone_from(&text);
                    draft.repaired.push((text, Repair::Spliced));
                    draft.decisions[k + 1] = alone;
                    draft.segs[k + 1].text.clone_from(&verb);
                    draft.verbs.push((draft.ids[k + 1], draft.ids[k]));
                    draft.repaired.push((verb, Repair::Verb));
                }
            }
        }
        Ok(())
    }

    /// The texts a shared object asks to decide: the second verb alone; or the object after the first verb,
    /// narrowed to its reflex, and the second verb alone.
    fn verbs_asked(&self, draft: &Draft, verbs: &Verbs) -> Vec<Asked> {
        match verbs {
            Verbs::Whole { k, base, second } => {
                let Some(first) = reflex_of(&draft.decisions[*k]) else {
                    return Vec::new();
                };
                vec![Self::narrowed(base, first, None), self.segment(second)]
            }
            Verbs::Cut { k, object, verb } => {
                let Some(first) = reflex_of(&draft.decisions[*k]) else {
                    return Vec::new();
                };
                let text = format!("{} {object}", draft.segs[*k].text);
                vec![Self::narrowed(&text, first, None), self.segment(verb)]
            }
        }
    }

    /// A bare verb cut by «and» from a verb with the object: «buy» and «pay 2 of HOM-403». The first part reads
    /// as a reflex and asks for a value, holding none; the next part was cut from it by «and», reads as another
    /// reflex, and opens with words that say what to do, its values after them, typed values alone; and no
    /// word of the next part names an argument its call read, «pay order 1024», which keeps the object its
    /// own. The object, and the next part's verb.
    fn cut_apart(&self, draft: &Draft, k: usize) -> Option<(String, String)> {
        let next = k + 1;
        if next >= draft.segs.len() || !draft.origins[next].is_empty() {
            return None;
        }
        let split = split_before(&draft.taken, &draft.segs, next)?;
        if split.order != Order::And {
            return None;
        }
        let (first, theirs) = (reflex_of(&draft.decisions[k])?, &draft.decisions[next]);
        let second = reflex_of(theirs)?;
        if second == first || !matches!(draft.decisions[k], Decision::Ask { .. }) {
            return None;
        }
        // The first part: a bare verb, no value in its words and none read, and no word that points back.
        let bare = Input::new(&draft.segs[k].text).ok()?;
        if args_of(&draft.decisions[k]).is_some_and(|args| !args.is_empty())
            || !crate::propose::propose(&bare).is_empty()
            || reading::refers_back(&draft.segs[k].text)
        {
            return None;
        }
        let acts = |text: &str, decision: &Decision| {
            left_of(decision)
                .iter()
                .find(|run| {
                    run.does == Does::Action && run.p.get() >= SAYS && stands_in(text, &run.words)
                })
                .map(|run| run.words.clone())
        };
        acts(&draft.segs[k].text, &draft.decisions[k])?;
        // The next part: its verb first, its values after it.
        let verb = acts(&draft.segs[next].text, theirs)?;
        let (_, held) = held_by(&draft.segs[next].text, theirs)?;
        if verb.start() != 0
            || held.is_empty()
            || held.iter().any(|value| value.start() < verb.end())
        {
            return None;
        }
        let chars: Vec<char> = draft.segs[next].text.chars().collect();
        let object: String = chars[verb.end().min(chars.len())..].iter().collect();
        let object = object.trim().to_owned();
        // The object is values alone, typed: «2 of HOM-403»; words that hold a listed word only, «in
        // eu-west», are the next part's own.
        if object.is_empty() || values_alone(&object, None).is_none() {
            return None;
        }
        // A part that names what its call reads, «pay order 1024», keeps its object as its own.
        let names: Vec<String> = args_of(theirs)
            .into_iter()
            .flatten()
            .flat_map(|(arg, _)| names_of(self.plan, &QuestionId::Arg(second.clone(), arg.clone())))
            .collect();
        let lowered = draft.segs[next].text.to_lowercase();
        if names.iter().any(|name| occurrences(&lowered, name) > 0) {
            return None;
        }
        Some((object, verb.text().as_str().to_owned()))
    }

    /// The words of a step that count several things of an argument the call holds one of — «the last 2
    /// orders», «both orders». Where the step asks for the argument, the words point at what was said before,
    /// and the session recalls as many under the field the pick names, the step is one step per value, newest
    /// first, each standing on the words. Where the words count and the call takes one, the ask names them.
    fn recalled(&self, draft: &mut Draft) {
        let mut k = 0;
        while k < draft.segs.len() {
            let Some(found) = self.counted(draft, k) else {
                k += 1;
                continue;
            };
            let (arg, pick, pointer, values) = found;
            let Decision::Ask { asking, .. } = &draft.decisions[k] else {
                k += 1;
                continue;
            };
            // The call was held against the request without its value: that says nothing of a step that
            // holds one of the values the words count.
            let asking = crate::decide::Asking {
                whole: None,
                ..asking.clone()
            };
            let decisions: Vec<Decision> = values
                .iter()
                .take(pointer.count)
                .enumerate()
                .filter_map(|(i, text)| {
                    let Value::Pick { value, .. } = yielded(text, pick)? else {
                        return None;
                    };
                    let value = Value::Pick {
                        span: pointer.words.clone(),
                        value,
                        typed: Clean::new(text).ok(),
                    };
                    let stands = Basis::Recalled {
                        words: pointer.words.clone(),
                        place: i + 1,
                    };
                    Some(carry(
                        self.plan,
                        asking.clone(),
                        IndexMap::from([(arg.clone(), value)]),
                        IndexMap::from([(arg.clone(), stands)]),
                        self.gate,
                    ))
                })
                .collect();
            if pointer.points && decisions.len() == pointer.count {
                let segs = vec![draft.segs[k].clone(); pointer.count];
                draft.replace(k, segs, decisions, vec![Vec::new(); pointer.count]);
                draft
                    .recalled
                    .extend(draft.ids[k..k + pointer.count].iter().copied());
                k += pointer.count;
                continue;
            }
            // The call takes one: the ask names the words that count, so that nothing is dropped in silence.
            if let Decision::Ask { asking, missing } = &draft.decisions[k] {
                let named: Vec<crate::decide::Missing> = missing
                    .iter()
                    .cloned()
                    .map(|mut asked| {
                        if asked.arg == arg {
                            asked.words.get_or_insert_with(|| pointer.words.clone());
                        }
                        asked
                    })
                    .collect();
                if let Ok(missing) = crate::text::NonEmpty::try_from(named) {
                    draft.decisions[k] = Decision::Ask {
                        asking: asking.clone(),
                        missing,
                    };
                }
            }
            k += 1;
        }
    }

    /// The words of step `k` that count several things of an argument its call asks for: the argument, its
    /// recognizer, the words, and the values this session recalls for the ask. None where no words count.
    fn counted(
        &self,
        draft: &Draft,
        k: usize,
    ) -> Option<(ArgName, Recognizer, crate::pointer::Pointer, Vec<String>)> {
        if !draft.origins[k].is_empty() {
            return None;
        }
        let Decision::Ask { asking, missing } = &draft.decisions[k] else {
            return None;
        };
        let input = Input::new(&draft.segs[k].text).ok()?;
        let proposed = crate::propose::propose(&input);
        missing.iter().find_map(|asked| {
            let Choices::Pick { pick, recent, .. } = &asked.choices else {
                return None;
            };
            let id = QuestionId::Arg(asking.reflex.clone(), asked.arg.clone());
            let names = names_of(self.plan, &id);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let pointer = crate::pointer::pointed(&input, &proposed, &names)?;
            (pointer.count >= 2).then(|| {
                (
                    asked.arg.clone(),
                    *pick,
                    pointer,
                    recent.clone().unwrap_or_default(),
                )
            })
        })
    }

    /// A part left out that names a value the step beside it lacks — «…, not ireland, virginia» — says what
    /// the step is about as much as what it is not: it is read with that step, the two decided as one request,
    /// where the step is the person's own words as typed. A part that names no such value stays out, as what
    /// not to do.
    fn corrected(&self, draft: &mut Draft) -> Result<(), Need> {
        let mut at = 0;
        while at < draft.apart.len() {
            let part = draft.apart[at].clone();
            // A part set aside by its words names no value of a step; it stays aside.
            if matches!(part.left, Some(Left::Aside(_))) {
                at += 1;
                continue;
            }
            // The step its words follow, else the one they come before.
            let beside = draft
                .segs
                .iter()
                .rposition(|seg| seg.end <= part.start)
                .or_else(|| draft.segs.iter().position(|seg| seg.start >= part.end));
            let typed = |seg: &Segment| {
                let own: String = draft.chars
                    [seg.start.min(draft.chars.len())..seg.end.min(draft.chars.len())]
                    .iter()
                    .collect();
                own.trim() == seg.text
            };
            let Some(k) = beside.filter(|&k| {
                draft.origins[k].is_empty()
                    && typed(&draft.segs[k])
                    && self.names_a_value(&draft.decisions[k], &part.text)
            }) else {
                at += 1;
                continue;
            };
            let (start, end) = (
                draft.segs[k].start.min(part.start),
                draft.segs[k].end.max(part.end),
            );
            let whole = Segment {
                text: draft.chars[start..end.min(draft.chars.len())]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_owned(),
                start,
                end,
                left: None,
            };
            let decision = self.decide(self.segment(&whole.text))?;
            if matches!(decision, Decision::Abstain { .. }) {
                at += 1;
                continue;
            }
            draft
                .taken
                .retain(|split| split.start < start || split.end > end);
            draft.repaired.push((whole.text.clone(), Repair::Corrected));
            draft.merge(k, k, whole, decision);
            draft.apart.remove(at);
            if let Some(gone) = draft.excluded.iter().position(|text| *text == part.text) {
                draft.excluded.remove(gone);
            }
        }
        Ok(())
    }

    /// Whether a text names a value a decision's call may take and does not hold: a word of a list an argument
    /// of its reflex asks, or a typed value of an argument's kind.
    fn names_a_value(&self, decision: &Decision, text: &str) -> bool {
        let Some((reflex, active)) = self.active_of(decision) else {
            return false;
        };
        if !active.steps.is_empty() {
            return false;
        }
        let Ok(request) = crate::decide::request(
            self.plan,
            text,
            self.tags,
            Some(reflex),
            None,
            Scope::Full,
            &[],
        ) else {
            return false;
        };
        let held = args_of(decision);
        active.args.iter().any(|(arg, argument)| {
            if held.is_some_and(|held| held.contains_key(arg)) {
                return false;
            }
            let id = QuestionId::Arg(reflex.clone(), arg.clone());
            match &argument.kind {
                Kind::Flag => false,
                Kind::Value {
                    source: Source::Pick(_),
                    ..
                } => request
                    .questions
                    .get(&id)
                    .is_some_and(|question| match question {
                        Question::Choice(choice) => choice.options().keys().any(|key| {
                            Some(key) != choice.otherwise() && *key != crate::plan::none()
                        }),
                        Question::YesNo { .. } => false,
                    }),
                Kind::Value { .. } => request.listed.contains_key(&id),
            }
        })
    }

    /// Whether the request asks one thing where the split points cut it, or left a part of it out: *one
    /// thing* at its share.
    fn asks_one(&self, draft: &Draft) -> bool {
        (draft.segs.len() > 1 || !draft.apart.is_empty()) && self.says_one()
    }

    /// Whether the request asks one thing, by the answer to how many it asks.
    fn says_one(&self) -> bool {
        self.answers
            .judged
            .as_ref()
            .and_then(reading::one_thing)
            .is_some_and(|one| one.get() >= ONE)
    }

    /// One thing, one step. Where the request asks one thing and the split points cut it, the whole request
    /// decided as one text is the plan's one step, and no cut is made, by a split point or by a rule after it;
    /// the cut stands where the whole matches nothing, or where the whole or a part of it picks a playbook,
    /// whose plan is several things by its nature.
    fn one(&self, draft: &mut Draft) -> Result<(), Need> {
        if !self.asks_one(draft) {
            // One thing in one part already: no rule cuts it, where the part reads as a call.
            if let [decision] = draft.decisions.as_slice() {
                draft.one = self.says_one()
                    && !matches!(decision, Decision::Abstain { .. })
                    && !self.plays(decision);
            }
            return Ok(());
        }
        let text = self.request.trim();
        let whole = self.decide(self.segment(text))?;
        if matches!(whole, Decision::Abstain { .. })
            || self.plays(&whole)
            || draft.decisions.iter().any(|decision| self.plays(decision))
        {
            return Ok(());
        }
        let seg = Segment {
            text: text.to_owned(),
            start: 0,
            end: draft.chars.len(),
            left: None,
        };
        draft.shared.clear();
        match draft.segs.len().checked_sub(1) {
            Some(last) => draft.merge(0, last, seg, whole),
            None => draft.only(seg, whole),
        }
        draft.taken.clear();
        // Nothing of it is left out: what it rules out is read with what it asks.
        draft.excluded.clear();
        draft.apart.clear();
        draft.one = true;
        Ok(())
    }

    /// Whether a decision is a playbook's complete call, which its plan stands for.
    fn plays(&self, decision: &Decision) -> bool {
        complete(decision).is_some()
            && self
                .active_of(decision)
                .is_some_and(|(_, active)| !active.steps.is_empty())
    }

    /// Every segment decided, side by side; and with them the whole request, where it may be one step.
    fn segments(&self, draft: &mut Draft) -> Result<(), Need> {
        let wanted: Vec<Asked> = draft
            .segs
            .iter()
            .map(|seg| self.segment(&seg.text))
            .collect();
        let mut missing: Vec<Asked> = Vec::new();
        let whole = self
            .asks_one(draft)
            .then(|| self.segment(self.request.trim()));
        for asked in wanted.iter().chain(&whole) {
            if self.decided(asked).is_none() && !missing.contains(asked) {
                missing.push(asked.clone());
            }
        }
        if !missing.is_empty() {
            return Err(Need::Decide { asked: missing });
        }
        draft.decisions = wanted
            .iter()
            .map(|asked| {
                self.decided(asked)
                    .cloned()
                    .expect("every segment is decided")
            })
            .collect();
        Ok(())
    }

    /// A list. A segment still holding a comma or an `and` — «the logo, the poster and the banner», which the
    /// engine reads as one task at p ≈ 0.2 — is tried as one task over several items: the head decided alone,
    /// each item settled as the head's reflex the way a fan-out is, narrowed or spliced. All or nothing, and the
    /// decode settles it, not the judgment: a whole that decided with one item, or none, would have dropped the
    /// rest.
    fn lists(&self, draft: &mut Draft, judged: &[Split]) -> Result<(), Need> {
        let mut k = 0;
        while k < draft.segs.len() {
            let seg = draft.segs[k].clone();
            let inner: Vec<Split> = reading::places(&seg.text, true)
                .into_iter()
                .filter(|s| s.order == Order::And)
                .collect();
            if inner.is_empty() {
                k += 1;
                continue;
            }
            let base = index_of(&draft.chars, &seg.text, seg.start)
                .map_or(seg.start, |at| at.max(seg.start));
            let parts = reading::segments(&seg.text, &inner);
            // A condition inside a part is no item: the part stands as the engine read it.
            if parts
                .iter()
                .any(|part| part.left == Some(Left::Conditional))
            {
                k += 1;
                continue;
            }
            let kept: Vec<&Segment> = parts.iter().filter(|part| !part.excluded()).collect();
            if kept.len() < 2 {
                k += 1;
                continue;
            }
            // A second value the whole's decision already saw and no argument took — «10 minutes and 30
            // seconds» — is one task, not a list: the step confirms with its unused span at its turn, as the
            // foundation does.
            if let Some(chosen) = chosen_of(&draft.decisions[k])
                && chosen.unconsumed.iter().any(|span| {
                    kept.iter()
                        .any(|part| span.start() >= part.start && span.end() <= part.end)
                })
            {
                k += 1;
                continue;
            }
            let head = self.decide(self.segment(&kept[0].text))?;
            if matches!(head, Decision::Abstain { .. } | Decision::Ask { .. }) {
                k += 1;
                continue;
            }
            let mut items: Vec<(Segment, Decision, Option<Repair>)> = vec![(
                Segment {
                    text: kept[0].text.clone(),
                    start: base + kept[0].start,
                    end: base + kept[0].end,
                    left: None,
                },
                head,
                None,
            )];
            for part in &kept[1..] {
                let (prev_seg, prev_decision, _) = items.last().expect("the head is there");
                let Some(fan) = self.fanout(prev_seg, prev_decision, &part.text)? else {
                    items.clear();
                    break;
                };
                items.push((
                    Segment {
                        text: fan.text,
                        start: base + part.start,
                        end: base + part.end,
                        left: None,
                    },
                    fan.decision,
                    Some(fan.how),
                ));
            }
            if items.is_empty() {
                k += 1;
                continue;
            }
            k += listed(draft, k, base, &parts, items, &inner, judged);
        }
        Ok(())
    }

    /// Items of different reflexes. A segment the engine kept whole at a comma or an `and` — «invoices, card
    /// expenses», judged one thing at the comma — that abstained, asked or sat under the floor, whose parts each
    /// decide alone as a different reflex, is those steps: two nouns that each name a reflex of their own are two
    /// things, whatever the engine made of the joint. A whole decided firmly stays one, and so does one whose
    /// confirm carries a span no argument took — «10 minutes and 30 seconds» — as the list reading leaves it.
    fn items(&self, draft: &mut Draft, judged: &[Split]) -> Result<(), Need> {
        let mut k = 0;
        while k < draft.segs.len() {
            let seg = draft.segs[k].clone();
            let inner: Vec<Split> = reading::places(&seg.text, true)
                .into_iter()
                .filter(|s| s.order == Order::And)
                .collect();
            if inner.is_empty() || !unsettled(&draft.decisions[k]) {
                k += 1;
                continue;
            }
            let parts = reading::segments(&seg.text, &inner);
            if parts
                .iter()
                .any(|part| part.left == Some(Left::Conditional))
            {
                k += 1;
                continue;
            }
            let kept: Vec<&Segment> = parts.iter().filter(|part| !part.excluded()).collect();
            if kept.len() < 2 {
                k += 1;
                continue;
            }
            let wanted: Vec<Asked> = kept.iter().map(|part| self.segment(&part.text)).collect();
            let mut missing: Vec<Asked> = Vec::new();
            for asked in &wanted {
                if self.decided(asked).is_none() && !missing.contains(asked) {
                    missing.push(asked.clone());
                }
            }
            if !missing.is_empty() {
                return Err(Need::Decide { asked: missing });
            }
            let decided: Vec<Decision> = wanted
                .iter()
                .map(|asked| self.decided(asked).cloned().expect("every part is decided"))
                .collect();
            let mut reflexes: Vec<&LocalName> = Vec::new();
            let distinct = decided.iter().all(|decision| {
                reflex_of(decision).is_some_and(|reflex| {
                    let new = !reflexes.contains(&reflex);
                    reflexes.push(reflex);
                    new
                })
            });
            if !distinct {
                k += 1;
                continue;
            }
            let base = index_of(&draft.chars, &seg.text, seg.start)
                .map_or(seg.start, |at| at.max(seg.start));
            draft.excluded.extend(
                parts
                    .iter()
                    .filter(|part| part.excluded())
                    .map(|part| part.text.clone()),
            );
            let placed: Vec<Segment> = kept
                .iter()
                .map(|part| Segment {
                    text: part.text.clone(),
                    start: base + part.start,
                    end: base + part.end,
                    left: None,
                })
                .collect();
            for part in &placed {
                draft.repaired.push((part.text.clone(), Repair::Split));
            }
            let count = placed.len();
            // What the whole's call held is kept for each part: one that ends as the whole's reflex without a
            // value of it waits (`lacking`).
            let whole = read_of(&draft.decisions[k]);
            draft.replace(k, placed, decided, vec![Vec::new(); count]);
            if let Some((reflex, read)) = whole {
                for part in k..k + count {
                    draft.wholes.push(Whole {
                        part: draft.ids[part],
                        reflex: reflex.clone(),
                        read: read.clone(),
                    });
                }
            }
            for s in &inner {
                let start = base + s.start;
                draft.taken.push(Split {
                    start,
                    end: base + s.end,
                    word: s.word.clone(),
                    order: s.order,
                    p: judged.iter().find(|a| a.start == start).and_then(|a| a.p),
                    cut: false,
                });
            }
            draft.taken.sort_by_key(|s| s.start);
            k += count;
        }
        Ok(())
    }

    /// Repair. A segment that matches nothing on its own is one of three things. A second item of its
    /// neighbour's task — «gadgets» after «check stock for widgets» — is decided again narrowed to the
    /// neighbour's reflex, or spliced into the neighbour's own words in place of the value it stands for: a
    /// fan-out. Failing that, a fragment of its neighbour's words is merged back and decided again — and never
    /// runs unasked then, since the part it absorbed may have been a second task after all — unless the engine
    /// was firm that the request asks for two things there, or the fragment carries a pronoun: then it is an
    /// action nothing matches, and the request is refused rather than half done. A part that decided on its own
    /// as another reflex is tried as a fan-out too, and stays its own step when that fails, under a split the
    /// engine called one thing, or when it is a bare item — a determiner and one word, «the logo» — whatever the
    /// engine made of the split: such a part routes to the reflex whose example begins with it, weak evidence
    /// against the words' own shape. A part the items rule split is the step it is.
    fn repair(&self, draft: &mut Draft) -> Result<(), Need> {
        let mut k = 0;
        while k < draft.segs.len() && draft.segs.len() >= 2 {
            // A part split from a whole the engine kept together is a step by the items rule: never an item of
            // its neighbour's task, whatever the engine made of the joint.
            if repair_of(draft, k) == Some(Repair::Split) {
                k += 1;
                continue;
            }
            let Some(sibling) = draft.neighbour(k) else {
                k += 1;
                continue;
            };
            let split = split_before(&draft.taken, &draft.segs, k.max(1)).cloned();
            let sibling_reflex = reflex_of(&draft.decisions[sibling]).cloned();
            let doubted = split
                .as_ref()
                .is_some_and(|s| s.p.map_or(0.0, Prob::get) < DOUBT)
                && sibling_reflex.is_some();
            let bare = sibling_reflex.is_some() && bare(&draft.segs[k].text);
            let abstains = matches!(draft.decisions[k], Decision::Abstain { .. });
            let other = reflex_of(&draft.decisions[k]) != sibling_reflex.as_ref();
            if !(abstains || (doubted || bare) && other) {
                k += 1;
                continue;
            }
            // A part kept after a contrast's «not X» that matches nothing alone corrects the step before it: no
            // second task, and the place before it was code's, never the engine's to call two things.
            let corrects = abstains
                && k > 0
                && draft.ruled.iter().any(|&(start, end)| {
                    draft.segs[k - 1].end <= start && end <= draft.segs[k].start
                });
            // The fan-out first: for a fragment that matches nothing, and for a doubted part that decided as
            // another reflex — «the logo» as `render` beside «deadline for the flyer».
            if sibling_reflex.is_some()
                && !corrects
                && let Some(fan) = self.fanout(
                    &draft.segs[sibling],
                    &draft.decisions[sibling],
                    &draft.segs[k].text,
                )?
            {
                draft.decisions[k] = fan.decision;
                draft.segs[k].text.clone_from(&fan.text);
                draft.repaired.push((fan.text, fan.how));
                k += 1;
                continue;
            }
            // A doubted part the fan-out could not settle stays its own step, its own gate at its turn. Only a
            // fragment that matches nothing is merged back — and not when the engine was firm that it is a second
            // task, nor when it carries a pronoun — «pull up every one of them» — which makes it a step by the
            // words: the request then refuses rather than runs without it, whatever the engine made of the split.
            let firm = split
                .as_ref()
                .is_some_and(|s| s.p.map_or(0.0, Prob::get) >= FIRM);
            if !abstains || (firm && !corrects) || reading::refers_back(&draft.segs[k].text) {
                k += 1;
                continue;
            }
            let (a, b) = if k > 0 { (k - 1, k) } else { (k, k + 1) };
            // A second verb's step between two parts is merged into neither: the fragment stays as it is.
            if draft.verb(a) || draft.verb(b) {
                k += 1;
                continue;
            }
            let whole = Segment {
                text: draft.chars[draft.segs[a].start..draft.segs[b].end.min(draft.chars.len())]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_owned(),
                start: draft.segs[a].start,
                end: draft.segs[b].end,
                left: None,
            };
            let decision = self.decide(self.segment(&whole.text))?;
            if matches!(decision, Decision::Abstain { .. }) {
                k += 1;
                continue;
            }
            // What the merged words rule out is read with them: it is left out no more.
            let (start, end) = (whole.start, whole.end);
            let within = |part: &Segment| start <= part.start && part.end <= end;
            let ruled_out: Vec<String> = draft
                .apart
                .iter()
                .filter(|part| part.excluded() && within(part))
                .map(|part| part.text.clone())
                .collect();
            let how = if ruled_out.is_empty() {
                Repair::Merged
            } else {
                Repair::Corrected
            };
            draft.repaired.push((whole.text.clone(), how));
            draft.merge(a, b, whole, decision);
            if ruled_out.is_empty() {
                if let Some(split) = split {
                    draft.taken.retain(|s| *s != split);
                }
            } else {
                draft
                    .apart
                    .retain(|part| !(part.excluded() && within(part)));
                draft.excluded.retain(|text| !ruled_out.contains(text));
                draft
                    .taken
                    .retain(|split| split.start < start || split.end > end);
            }
            k = 0;
        }
        Ok(())
    }

    /// Whether a reflex that is no playbook takes the values a decision's call holds: for each value, an
    /// argument of its own from the same source as the argument that holds it — the same recognizer, the same
    /// vocabulary, the same options.
    fn takes_the_same(&self, reflex: &LocalName, decision: &Decision) -> bool {
        let source = |argument: &'a manifest::Argument| match &argument.kind {
            Kind::Value { source, .. } => Some(source),
            Kind::Flag => None,
        };
        let (Some((_, own)), Some(args), Some(before)) = (
            self.active_of(decision),
            args_of(decision),
            self.plan.active().get(reflex),
        ) else {
            return false;
        };
        let mut free: Vec<&Source> = before.args.values().filter_map(source).collect();
        before.steps.is_empty()
            && args
                .iter()
                .filter(|(_, value)| !matches!(value, Value::Flag))
                .all(|(arg, _)| {
                    let taken = own.args.get(arg).and_then(source).and_then(|source| {
                        free.iter().position(|other| same_source(source, other))
                    });
                    taken.map(|at| free.swap_remove(at)).is_some()
                })
    }

    /// What the plan asks of the request once its parts are decided, in one request: what each part that
    /// matches no reflex does, and whether a value one part states is another's. A value is another part's where a
    /// yes says so, and stands on that yes. A part that asks for something is then asked, in a request of its own,
    /// which reflex its words ask for, read in the whole request; where that gives one, the part is decided as it,
    /// on that route, its values its own words': a step so found takes the values stated once as any step does,
    /// and waits for a yes. A part that asks for nothing is set aside, a remark; where it may add a detail, the
    /// step beside it waits for a yes with the part's words.
    fn verify(&self, draft: &mut Draft) -> Result<(), Need> {
        let silent: Vec<usize> = (0..draft.segs.len())
            .filter(|&k| draft.segs.len() > 1 && draft.origins[k].is_empty())
            .filter(|&k| matches!(draft.decisions[k], Decision::Abstain { .. }))
            .collect();
        let offers = self.offers_of(draft);
        // A part or a value whose words cannot be asked about is asked nothing, and stays as it was read.
        let parts: Vec<(usize, (QuestionId, Question))> = silent
            .iter()
            .filter(|&&k| !crate::words::courtesy(&draft.segs[k].text))
            .filter_map(|&k| Some((k, reading::part(&draft.segs[k]).ok()?)))
            .collect();
        let shares = shares_of(draft, &offers);
        self.ask(
            parts.iter().map(|(_, asked)| asked.clone()).chain(
                shares
                    .iter()
                    .map(|(_, id, question)| (id.clone(), question.clone())),
            ),
        )?;
        self.give(draft, &shares);
        // Only a part that asks for something is read in the whole request, in a request after the first.
        let asking: Vec<(usize, (QuestionId, Question))> = parts
            .iter()
            .filter(|(_, (id, _))| self.asks(id))
            .filter_map(|(k, _)| Some((*k, self.span_of(&draft.segs[*k])?)))
            .collect();
        self.ask(asking.iter().map(|(_, asked)| asked.clone()))?;
        let found = self.found(draft, &asking)?;
        let steps: Vec<usize> = found.iter().map(|(k, _)| *k).collect();
        self.carried(draft, &steps)?;
        // A step only the whole request found never runs unasked.
        for (k, p) in found {
            let cap = Cap::Named {
                words: draft.as_typed(k),
                p,
            };
            draft.decisions[k] = held(self.plan, draft.decisions[k].clone(), cap);
        }
        let rest: Vec<usize> = silent.into_iter().filter(|k| !steps.contains(k)).collect();
        self.aside(draft, &rest);
        Ok(())
    }

    /// The plan's own questions of the request not answered yet, asked in one request; where its words cannot be
    /// asked about, nothing is, and the plan goes on with what it has.
    fn ask(&self, questions: impl Iterator<Item = (QuestionId, Question)>) -> Result<(), Need> {
        let missing: IndexMap<_, _> = questions.filter(|(id, _)| !self.answered(id)).collect();
        if !missing.is_empty()
            && let Ok(request) = reading::asking(&self.request, missing)
        {
            return Err(Need::Verify { request });
        }
        Ok(())
    }

    /// Whether the plan's own question about the request was answered.
    fn answered(&self, id: &QuestionId) -> bool {
        self.answers
            .verified
            .as_ref()
            .is_some_and(|raw| raw.0.contains_key(&id.to_string()))
    }

    /// The share an answer about the request gives a key of one of the plan's own questions; none reads as 0.
    fn said(&self, id: &QuestionId, key: &str) -> f64 {
        self.answers
            .verified
            .as_ref()
            .and_then(|raw| raw.0.get(&id.to_string()))
            .and_then(|answer| answer.get(key))
            .copied()
            .unwrap_or(0.0)
    }

    /// Each value another part states, given to the part a yes says it is and that holds none: it stands on that
    /// yes.
    fn give(&self, draft: &mut Draft, shares: &[Offered<'_>]) {
        for (offer, id, _) in shares {
            let held = args_of(&draft.decisions[offer.taker])
                .is_some_and(|args| args.contains_key(&offer.arg));
            let Some(yes) =
                Prob::new(self.said(id, "yes")).filter(|yes| yes.get() >= SHARE && !held)
            else {
                continue;
            };
            let stands = Basis::Shared {
                from: draft.segs[offer.giver].text.clone(),
                yes,
            };
            let given = given(
                self.plan,
                draft.decisions[offer.taker].clone(),
                (offer.arg.clone(), offer.value.clone()),
                stands,
                self.gate,
            );
            draft.decisions[offer.taker] = given;
        }
    }

    /// Which reflex a part's words ask for, read in the whole request: over the route's own options, as the route
    /// offers them for those words; none where they cannot be asked about.
    fn span_of(&self, seg: &Segment) -> Option<(QuestionId, Question)> {
        let request = crate::decide::request(
            self.plan,
            &seg.text,
            self.tags,
            None,
            None,
            Scope::Route,
            &[],
        )
        .ok()?;
        match request.questions.get(&QuestionId::Route) {
            Some(Question::Choice(route)) => reading::span(seg, route).ok(),
            _ => None,
        }
    }

    /// The parts the whole request reads as a reflex — each one that asks for something, whose words the whole
    /// request gives a reflex at the route's own floor — each decided as that reflex, side by side: its route the
    /// whole request's, never asked of its words alone, its values read from its own words. A part so decided is a
    /// step, unless its call lacks a listed word its words hold. The parts that became steps, each with the share
    /// the whole request gave its reflex.
    fn found(
        &self,
        draft: &mut Draft,
        parts: &[(usize, (QuestionId, Question))],
    ) -> Result<Vec<(usize, Prob)>, Need> {
        let wanted: Vec<(usize, Asked, Prob)> = parts
            .iter()
            .filter_map(|(k, span)| {
                let (reflex, named) = self.route_of(span)?;
                let asked = Self::narrowed(&draft.segs[*k].text, &reflex, Some(&named));
                Some((*k, asked, named.p))
            })
            .collect();
        let mut missing: Vec<Asked> = Vec::new();
        for (_, asked, _) in &wanted {
            if self.decided(asked).is_none() && !missing.contains(asked) {
                missing.push(asked.clone());
            }
        }
        if !missing.is_empty() {
            return Err(Need::Decide { asked: missing });
        }
        let mut found = Vec::new();
        for (k, asked, p) in wanted {
            let decision = self
                .decided(&asked)
                .cloned()
                .expect("every part named is decided");
            // A part a host decided as no reflex stays as it was; so does one whose call lacks a listed word its
            // words hold, «the same for Jo» read as a call that takes no person.
            if matches!(decision, Decision::Abstain { .. })
                || self.lacks(&draft.as_typed(k), &decision)
            {
                continue;
            }
            draft.decisions[k] = decision;
            draft.repaired.push((asked.text, Repair::Named));
            found.push((k, p));
        }
        Ok(found)
    }

    /// Whether a call lacks a listed word its part's own words hold: a word of a vocabulary some reflex of the
    /// plan takes, which no value of the call is. Such a call does not say what the words say.
    fn lacks(&self, text: &str, decision: &Decision) -> bool {
        let held: Vec<String> = args_of(decision)
            .into_iter()
            .flatten()
            .filter_map(|(_, value)| stated_of(value))
            .collect();
        self.plan.active().iter().any(|(reflex, active)| {
            active.args.iter().any(|(arg, argument)| {
                matches!(
                    &argument.kind,
                    Kind::Value {
                        source: Source::Vocab(_),
                        ..
                    }
                ) && words(self.plan, reflex, arg).keys().any(|word| {
                    occurrences(text, word.as_str()) > 0
                        && !held.contains(&word.as_str().to_lowercase())
                })
            })
        })
    }

    /// The route the whole request gives a part's words, by the answer about them: the reflex first offered with
    /// the highest share, and the judgment that names it, where it is a reflex at the route's own floor.
    fn route_of(&self, (id, question): &(QuestionId, Question)) -> Option<(LocalName, Judgment)> {
        let Question::Choice(choice) = question else {
            return None;
        };
        if !self.answered(id) {
            return None;
        }
        let answer: IndexMap<Key, Prob> = choice
            .options()
            .keys()
            .filter_map(|key| Some((key.clone(), Prob::new(self.said(id, key.as_str()))?)))
            .collect();
        let (top, p) = crate::decide::top(choice, &answer)?;
        let floored = self.gate.is_some_and(|gate| p < gate.route());
        if Some(&top) == choice.otherwise() || floored {
            return None;
        }
        let reflex = LocalName::new(top.as_str()).ok()?;
        let question = id.clone();
        Some((reflex, Judgment { question, top, p }))
    }

    /// The steps the whole request found take what carries a value stated once to a step that lacks it, as any
    /// step does: a word of a vocabulary the request states once, then a value another part states, by a yes.
    fn carried(&self, draft: &mut Draft, found: &[usize]) -> Result<(), Need> {
        if found.is_empty() {
            return Ok(());
        }
        self.reach(draft, found)?;
        let offers: Vec<Offer> = self
            .offers_of(draft)
            .into_iter()
            .filter(|offer| found.contains(&offer.taker))
            .collect();
        let shares = shares_of(draft, &offers);
        self.ask(
            shares
                .iter()
                .map(|(_, id, question)| (id.clone(), question.clone())),
        )?;
        self.give(draft, &shares);
        Ok(())
    }

    /// Whether the answer about what a part does says it asks for something.
    fn asks(&self, id: &QuestionId) -> bool {
        self.answered(id) && self.said(id, "asks") >= 1.0 - ASIDE
    }

    /// Whether the answer about what a part does sets it aside, and as what: a remark, `true`, or words that may
    /// add a detail, `false`; none where it asks for something, or was not answered, and it stays as it was read.
    fn set_aside(&self, id: &QuestionId) -> Option<bool> {
        (self.answered(id) && self.said(id, "asks") < 1.0 - ASIDE)
            .then(|| self.said(id, "aside") >= ASIDE)
    }

    /// The parts that match no reflex and ask for nothing, out of the plan: a part of courtesy alone is a
    /// remark by its words; any other by what the answer says it does. One step at least stays.
    fn aside(&self, draft: &mut Draft, silent: &[usize]) {
        for &k in silent.iter().rev() {
            if draft.segs.len() < 2 {
                break;
            }
            let text = draft.segs[k].text.clone();
            let (remark, does) = if crate::words::courtesy(&text) {
                (true, None)
            } else {
                let Ok((id, _)) = reading::part(&draft.segs[k]) else {
                    continue;
                };
                let Some(remark) = self.set_aside(&id) else {
                    continue;
                };
                let does = Prob::new(self.said(&id, "aside"))
                    .zip(Prob::new(self.said(&id, "detail")))
                    .zip(Prob::new(self.said(&id, "asks")))
                    .map(|((aside, detail), asks)| Parted {
                        aside,
                        detail,
                        asks,
                    });
                (remark, does)
            };
            if !remark {
                // The step beside it, before it first, waits for a yes with the part's words.
                let beside = [k.checked_sub(1), Some(k + 1)]
                    .into_iter()
                    .flatten()
                    .find(|&j| {
                        j < draft.segs.len()
                            && !draft.verb(j)
                            && reflex_of(&draft.decisions[j]).is_some()
                    });
                if let Some(j) = beside {
                    let cap = Cap::Detail {
                        words: text.clone(),
                    };
                    draft.decisions[j] = held(self.plan, draft.decisions[j].clone(), cap);
                }
            }
            draft.asides.insert(
                0,
                Aside {
                    text,
                    remark,
                    does,
                    by: None,
                },
            );
            draft.leave(k);
        }
    }

    /// Once every part is decided and folded: a step of the person's own whose words say it is wanted once more
    /// — «him too», «the same for the hall» — waits as words that ask for another thing do, whatever the engine
    /// answered of them, unless the plan holds another step of its reflex, which is the other time the words
    /// ask for.
    fn again(&self, draft: &mut Draft) {
        for k in 0..draft.segs.len() {
            let Some(reflex) = reflex_of(&draft.decisions[k]) else {
                continue;
            };
            let twice = (0..draft.segs.len())
                .any(|j| j != k && reflex_of(&draft.decisions[j]) == Some(reflex));
            if !twice && draft.origins[k].is_empty() {
                draft.decisions[k] = alone(self.plan, draft.decisions[k].clone());
            }
        }
    }

    /// Once every part is decided and every value given: a part the items rule cut from a whole, which reads as
    /// the whole's reflex without a value the whole's call held, was not read whole. The words of each such value
    /// may be its own, and it waits for a yes that names them. A value the part asks for is the person's to give.
    fn lacking(&self, draft: &mut Draft) {
        for k in 0..draft.segs.len() {
            let Some(whole) = draft.wholes.iter().find(|whole| whole.part == draft.ids[k]) else {
                continue;
            };
            if reflex_of(&draft.decisions[k]) != Some(&whole.reflex) {
                continue;
            }
            let own = args_of(&draft.decisions[k]);
            let asked = |arg: &ArgName| {
                matches!(&draft.decisions[k], Decision::Ask { missing, .. }
                    if missing.iter().any(|asked| asked.arg == *arg))
            };
            let words: Vec<String> = whole
                .read
                .iter()
                .filter(|(arg, _)| !own.is_some_and(|own| own.contains_key(arg)) && !asked(arg))
                .map(|(_, words)| words.clone())
                .collect();
            for words in words {
                draft.decisions[k] =
                    held(self.plan, draft.decisions[k].clone(), Cap::Detail { words });
            }
        }
    }

    /// Once every part is decided: two parts of the person's own that a sign typed for «and» cuts apart, on a yes
    /// the engine gave under `DOUBT` — a cut tried, from `LOW`, and not believed — may be one request. Each waits
    /// for a yes that names the other's words.
    fn doubted(&self, draft: &mut Draft) {
        for k in 1..draft.segs.len() {
            let unsure = split_before(&draft.taken, &draft.segs, k).is_some_and(|split| {
                reading::joins(&split.word)
                    && split.p.is_some_and(|p| (LOW..DOUBT).contains(&p.get()))
            });
            if !unsure
                || !draft.origins[k - 1].is_empty()
                || !draft.origins[k].is_empty()
                || draft.verb(k - 1)
                || draft.verb(k)
            {
                continue;
            }
            for (at, beside) in [(k - 1, k), (k, k - 1)] {
                // The other part as it was typed, where the plan wrote its words anew.
                let words = draft.as_typed(beside);
                draft.decisions[at] = held(
                    self.plan,
                    draft.decisions[at].clone(),
                    Cap::Detail { words },
                );
            }
        }
    }

    /// The values one part states that another part may take: for each part that reads as a reflex and lacks
    /// a value an argument of it takes, the values of that argument's kind the other parts hold, two at most —
    /// a word of the same vocabulary, a typed value of the same kind — where the part's words point at
    /// something, or the word's vocabulary is asked by several reflexes of the set.
    fn offers_of(&self, draft: &Draft) -> Vec<Offer> {
        let mut offers = Vec::new();
        for taker in 0..draft.segs.len() {
            let Some((reflex, active)) = self.active_of(&draft.decisions[taker]) else {
                continue;
            };
            if !draft.origins[taker].is_empty() || !active.steps.is_empty() {
                continue;
            }
            let points = pointing(&draft.segs[taker].text);
            let repeats = repeating(&draft.segs[taker].text);
            let held = args_of(&draft.decisions[taker]);
            for (arg, argument) in &active.args {
                if held.is_some_and(|held| held.contains_key(arg)) {
                    continue;
                }
                let allowed = match &argument.kind {
                    Kind::Value {
                        source: Source::Vocab(vocab),
                        ..
                    } => points || self.asked_by_several(vocab),
                    Kind::Value {
                        source: Source::Pick(_),
                        ..
                    } => points,
                    // An option and a switch are a call's own: a call of the same reflex gives them, where
                    // the part says that call is done again.
                    Kind::Value {
                        source: Source::Options(_),
                        ..
                    }
                    | Kind::Flag => repeats,
                };
                if !allowed {
                    continue;
                }
                let mut shown: Vec<String> = Vec::new();
                for giver in (0..draft.segs.len()).filter(|&giver| giver != taker) {
                    let Some((theirs, given)) = self.active_of(&draft.decisions[giver]) else {
                        continue;
                    };
                    for (name, value) in args_of(&draft.decisions[giver]).into_iter().flatten() {
                        let Some(said) =
                            self.shown((reflex, arg, argument), (theirs, name, given), value)
                        else {
                            continue;
                        };
                        if shown.len() == MOST_GIVERS || shown.contains(&said) {
                            continue;
                        }
                        shown.push(said.clone());
                        let no = match argument.kind {
                            Kind::Flag => reading::NOT_THIS,
                            Kind::Value { .. } => reading::ANOTHER,
                        };
                        offers.push(Offer {
                            taker,
                            giver,
                            reflex: reflex.clone(),
                            arg: arg.clone(),
                            ask: argument.ask.clone(),
                            shown: said,
                            no,
                            value: value.clone(),
                        });
                    }
                }
            }
        }
        offers
    }

    /// A value another part holds, as a part that lacks one of its kind is shown it: a word by its meaning, a
    /// typed value between marks, an option by its meaning, a switch by its yes. None where the value is not
    /// of the argument's kind: a word of another list, a typed value of another kind, an option or a switch
    /// of another reflex or argument.
    fn shown(
        &self,
        (reflex, arg, argument): (&LocalName, &ArgName, &manifest::Argument),
        (theirs, name, holder): (&LocalName, &ArgName, &Active),
        value: &Value,
    ) -> Option<String> {
        let other = holder.args.get(name)?;
        let own = theirs == reflex && name == arg;
        match (&argument.kind, &other.kind, value) {
            (
                Kind::Value {
                    source: Source::Vocab(vocab),
                    ..
                },
                Kind::Value {
                    source: Source::Vocab(of),
                    ..
                },
                _,
            ) if of == vocab => {
                let text = value.text()?;
                Some(
                    words(self.plan, theirs, name)
                        .get(text)
                        .map_or_else(|| text.to_owned(), ToString::to_string),
                )
            }
            (
                Kind::Value {
                    source: Source::Pick(pick),
                    ..
                },
                Kind::Value {
                    source: Source::Pick(of),
                    ..
                },
                _,
            ) if of.recognizer() == pick.recognizer() => {
                Some(format!("\u{ab}{}\u{bb}", value.text()?))
            }
            (
                Kind::Value {
                    source: Source::Options(options),
                    ..
                },
                _,
                Value::Option { key },
            ) if own => options.get(key).map(ToString::to_string),
            (Kind::Flag, _, Value::Flag) if own => Some(reading::THIS.to_owned()),
            _ => None,
        }
    }

    /// Whether more than one reflex of the set, playbooks aside, asks for a word of the vocabulary.
    fn asked_by_several(&self, vocab: &VocabName) -> bool {
        self.plan
            .active()
            .values()
            .filter(|active| active.steps.is_empty())
            .filter(|active| {
                active.args.values().any(|argument| {
                    matches!(
                        &argument.kind,
                        Kind::Value { source: Source::Vocab(of), .. } if of == vocab
                    )
                })
            })
            .count()
            > 1
    }

    /// One call said twice. A part of the person's own that reads as the reflex of a part before it, no value
    /// of the two differing, is that call: its values join the first's, what it asks that the first holds is
    /// asked no more, and its words are kept as folded. Where it could be either of several calls before it,
    /// it is the one its own words pick, «the second one», «the last»; where they pick none, it stays a step.
    fn same(&self, draft: &mut Draft) {
        // A playbook that still asks is a part as any other; one whose call is complete is its plan.
        let own = |draft: &Draft, k: usize| {
            draft.origins[k].is_empty()
                && !draft.verb(k)
                && !draft.refused_at(k)
                && self.active_of(&draft.decisions[k]).is_some()
                && !self.plays(&draft.decisions[k])
        };
        let mut j = 1;
        while j < draft.segs.len() {
            let could: Vec<usize> = (0..j)
                .filter(|&i| {
                    own(draft, i)
                        && own(draft, j)
                        && reflex_of(&draft.decisions[i]) == reflex_of(&draft.decisions[j])
                        && accord(&draft.decisions[i], &draft.decisions[j])
                })
                .collect();
            let first = match could.as_slice() {
                [] => None,
                [one] => Some((*one, None)),
                several => pointed(&draft.segs[j].text, several.len())
                    .map(|(k, word)| (several[k], Some(word))),
            };
            let Some((i, picked)) = first else {
                j += 1;
                continue;
            };
            let held = args_of(&draft.decisions[i]).cloned().unwrap_or_default();
            let joined = joined(
                self.plan,
                draft.decisions[i].clone(),
                &draft.decisions[j],
                self.gate,
            );
            let gave = args_of(&joined)
                .into_iter()
                .flatten()
                .map(|(arg, _)| arg)
                .filter(|arg| !held.contains_key(*arg))
                .cloned()
                .collect();
            draft.decisions[i] = joined;
            draft.fold_into(j, draft.ids[i], gave, picked);
        }
    }

    /// A playbook's own values. A playbook that still asks takes the values the parts beside it state for
    /// the arguments it lacks, by their names, from each part that reads as a reflex its steps route to; such a
    /// part is then a step the playbook writes, and folds into it: where it gave a value, or where it brings
    /// none the playbook lacks and asks for nothing the playbook does not hold or ask. A part that names
    /// another value than the playbook holds is a step of its own.
    fn fill(&self, draft: &mut Draft) -> Result<(), Need> {
        let mut p = 0;
        while p < draft.segs.len() {
            let Decision::Ask { asking, missing } = draft.decisions[p].clone() else {
                p += 1;
                continue;
            };
            let Some(active) = self
                .plan
                .active()
                .get(&asking.reflex)
                .filter(|active| !active.steps.is_empty())
            else {
                p += 1;
                continue;
            };
            let asks: Vec<&ArgName> = missing.iter().map(|asked| &asked.arg).collect();
            let beside: Vec<usize> = (0..draft.segs.len())
                .filter(|&k| k != p && draft.origins[k].is_empty() && !draft.refused_at(k))
                .filter(|&k| {
                    self.active_of(&draft.decisions[k])
                        .is_some_and(|(_, active)| active.steps.is_empty())
                })
                .collect();
            if beside.is_empty() {
                p += 1;
                continue;
            }
            // What the playbook would hold with every value the parts beside it state for what it asks.
            let mut held = asking.args.clone();
            for &k in &beside {
                for (arg, value) in args_of(&draft.decisions[k]).into_iter().flatten() {
                    if asks.contains(&arg) && !held.contains_key(arg) {
                        held.insert(arg.clone(), value.clone());
                    }
                }
            }
            let writes = self.writes(active, &held)?;
            let written = |k: &usize| {
                reflex_of(&draft.decisions[*k]).is_some_and(|reflex| writes.contains(reflex))
            };
            let mut given: IndexMap<ArgName, Value> = IndexMap::new();
            let mut stands: IndexMap<ArgName, Basis> = IndexMap::new();
            let mut gone: Vec<(usize, Vec<ArgName>)> = Vec::new();
            for &k in beside.iter().filter(|k| written(k)) {
                let theirs = args_of(&draft.decisions[k]).cloned().unwrap_or_default();
                let differs = theirs.iter().any(|(arg, value)| {
                    asking
                        .args
                        .get(arg)
                        .or_else(|| given.get(arg))
                        .is_some_and(|held| stated_of(held) != stated_of(value))
                });
                if differs {
                    continue;
                }
                let mut took: Vec<ArgName> = Vec::new();
                for (arg, value) in &theirs {
                    if asks.contains(&arg) && !given.contains_key(arg) {
                        given.insert(arg.clone(), value.clone());
                        stands.extend(
                            basis_of(&draft.decisions[k])
                                .and_then(|basis| basis.get(arg))
                                .map(|basis| (arg.clone(), basis.clone())),
                        );
                        took.push(arg.clone());
                    }
                }
                let brings = theirs
                    .keys()
                    .any(|arg| !asking.args.contains_key(arg) && !given.contains_key(arg));
                let covered = asked_of(&draft.decisions[k])
                    .iter()
                    .all(|arg| asks.contains(arg) || asking.args.contains_key(*arg));
                if !took.is_empty() || !brings && covered {
                    gone.push((k, took));
                }
            }
            if !given.is_empty() {
                draft.decisions[p] = carry(self.plan, asking.clone(), given, stands, self.gate);
            }
            let into = draft.ids[p];
            for (k, gave) in gone.into_iter().rev() {
                draft.fold_into(k, into, gave, None);
                if k < p {
                    p -= 1;
                }
            }
            p += 1;
        }
        Ok(())
    }

    /// The reflexes a playbook's steps route to, as far as the values go: each sentence decided as its words
    /// stand with the values at hand, a slot no value fills left out of them.
    fn writes(
        &self,
        active: &Active,
        args: &IndexMap<ArgName, Value>,
    ) -> Result<Vec<LocalName>, Need> {
        let wanted: Vec<Asked> = active
            .steps
            .iter()
            .map(|sentence| match sentence.filled(args) {
                Some((text, _)) => self.segment(&text),
                None => self.segment(&sentence.sketched(args)),
            })
            .collect();
        let mut missing: Vec<Asked> = Vec::new();
        for asked in &wanted {
            if self.decided(asked).is_none() && !missing.contains(asked) {
                missing.push(asked.clone());
            }
        }
        if !missing.is_empty() {
            return Err(Need::Decide { asked: missing });
        }
        Ok(wanted
            .iter()
            .filter_map(|asked| self.decided(asked).and_then(reflex_of).cloned())
            .collect())
    }

    /// The shared value. A word of a vocabulary the words state once — «checkout's», «in eu-west», «for
    /// september» — reaches every step that lacks an argument of that vocabulary. Stated: the word is in the
    /// request's own words and some step read it, from its own words, as an argument of that vocabulary, so the
    /// engine never chooses the carried value and code carries it. It reaches a step whose own words hold no word
    /// of the vocabulary: an optional argument takes the word its run states once, by a rewrite; a required one
    /// the one distinct word the whole request states, however often, filled as a person's answer to the step's
    /// ask would be. A merged part holding two words asks as before; picks and options are never carried.
    fn share(&self, draft: &mut Draft) -> Result<(), Need> {
        draft.shared = vec![IndexMap::new(); draft.segs.len()];
        if draft.segs.len() < 2 {
            return Ok(());
        }
        let everyone: Vec<usize> = (0..draft.segs.len()).collect();
        self.reach(draft, &everyone)
    }

    /// The shared value reaching the steps given, each that lacks an argument of its vocabulary: by a rewrite,
    /// then by a fill.
    fn reach(&self, draft: &mut Draft, takers: &[usize]) -> Result<(), Need> {
        // The words as the person typed them: a step's text a rewrite appended a word to states nothing more.
        let original: Vec<String> = (0..draft.segs.len()).map(|k| draft.as_typed(k)).collect();
        self.rewritten(draft, &original, takers)?;
        self.filled(draft, &original, takers);
        Ok(())
    }

    /// An optional vocabulary argument a step left unstated takes the word its run states once — the run being
    /// the steps joined by coordinating connectives, an ordering word ending it — by a rewrite: the word appended
    /// to the step's words, decided again narrowed to the step's reflex, on the route the whole request gave it
    /// where it gave one, kept only when the reflex holds, the word lands on that argument and no other argument
    /// moved; else the step stays. Every rewrite is decided side by side, one need.
    fn rewritten(
        &self,
        draft: &mut Draft,
        original: &[String],
        takers: &[usize],
    ) -> Result<(), Need> {
        let mut rewrites: Vec<(usize, IndexMap<ArgName, Word>, Asked)> = Vec::new();
        for run in runs_of(&draft.taken, &draft.segs) {
            for &k in &run {
                if !takers.contains(&k) || draft.refused_at(k) {
                    continue;
                }
                let Some((reflex, active)) = self.active_of(&draft.decisions[k]) else {
                    continue;
                };
                let mut carried: IndexMap<ArgName, Word> = IndexMap::new();
                for (arg, argument) in &active.args {
                    let Kind::Value {
                        source: Source::Vocab(vocab),
                        optional: true,
                    } = &argument.kind
                    else {
                        continue;
                    };
                    if args_of(&draft.decisions[k]).is_some_and(|args| args.contains_key(arg)) {
                        continue;
                    }
                    let vocabulary = words(self.plan, reflex, arg);
                    if holds(&original[k], &vocabulary) {
                        continue;
                    }
                    if let Some(word) = self.stated(draft, original, &run, vocab, &vocabulary, true)
                    {
                        carried.insert(arg.clone(), word);
                    }
                }
                if carried.is_empty() {
                    continue;
                }
                let appended: Vec<&str> = carried.values().map(Word::as_str).collect();
                let text = format!("{} {}", draft.segs[k].text, appended.join(" "));
                let asked = Self::narrowed(&text, reflex, named(&draft.decisions[k]));
                rewrites.push((k, carried, asked));
            }
        }
        let missing: Vec<Asked> = rewrites
            .iter()
            .filter(|(_, _, asked)| self.decided(asked).is_none())
            .map(|(_, _, asked)| asked.clone())
            .collect();
        if !missing.is_empty() {
            return Err(Need::Decide { asked: missing });
        }
        for (k, carried, asked) in rewrites {
            let again = self
                .decided(&asked)
                .cloned()
                .expect("every rewrite is decided");
            let Some(reflex) = reflex_of(&draft.decisions[k]) else {
                continue;
            };
            if !kept(&draft.decisions[k], &again, reflex, &carried) {
                continue;
            }
            let repair = repair_of(draft, k);
            draft.decisions[k] = again;
            if draft.typed[k].is_none() {
                draft.typed[k] = Some(draft.segs[k].text.clone());
            }
            draft.segs[k].text.clone_from(&asked.text);
            if let Some(repair) = repair {
                draft.repaired.push((asked.text, repair));
            }
            for (arg, word) in carried {
                let via = Via::Rewrite;
                draft.shared[k].insert(arg, Shared { word, via });
            }
        }
        Ok(())
    }

    /// A required vocabulary argument a step lacks takes the one distinct word of its vocabulary the whole
    /// request states, however often: the decision filled as `fill` fills a person's answer, no request.
    fn filled(&self, draft: &mut Draft, original: &[String], takers: &[usize]) {
        let everyone: Vec<usize> = (0..draft.segs.len()).collect();
        for &k in takers {
            if draft.refused_at(k) {
                continue;
            }
            let Decision::Ask { asking, missing } = &draft.decisions[k] else {
                continue;
            };
            let Some(active) = self.plan.active().get(&asking.reflex) else {
                continue;
            };
            let mut given: IndexMap<ArgName, Value> = IndexMap::new();
            let mut stands: IndexMap<ArgName, Basis> = IndexMap::new();
            let mut carried: IndexMap<ArgName, Word> = IndexMap::new();
            for m in missing.iter() {
                let Some(Kind::Value {
                    source: Source::Vocab(vocab),
                    ..
                }) = active.args.get(&m.arg).map(|argument| &argument.kind)
                else {
                    continue;
                };
                let vocabulary = words(self.plan, &asking.reflex, &m.arg);
                if holds(&original[k], &vocabulary) {
                    continue;
                }
                let stated = self.stated(draft, original, &everyone, vocab, &vocabulary, false);
                if let Some(word) = stated {
                    let value = Value::Word {
                        word: word.clone(),
                        value: None,
                    };
                    given.insert(m.arg.clone(), value);
                    stands.extend(
                        self.stood(draft, original, &everyone, vocab, &word)
                            .map(|basis| (m.arg.clone(), basis)),
                    );
                    carried.insert(m.arg.clone(), word);
                }
            }
            if given.is_empty() {
                continue;
            }
            let filled = carry(self.plan, asking.clone(), given, stands, self.gate);
            draft.decisions[k] = filled;
            for (arg, word) in carried {
                let via = Via::Fill;
                draft.shared[k].insert(arg, Shared { word, via });
            }
        }
    }

    /// The one word of a vocabulary the scope's words state: in the request's own words with no other word of
    /// the vocabulary beside it — once, for an optional argument; however often, for a required one — and read by
    /// some step of the scope, from its own words, as an argument of that vocabulary. None otherwise: two words
    /// stated, nothing is carried, and a word the person placed twice for two steps was placed. None either from
    /// a vocabulary one reflex alone asks for: that is the reflex's own object — a folder to open, a channel to
    /// tell — never another step's; a vocabulary several reflexes ask for is what the set is about, and one none
    /// asks for is a qualifier, and both are the sentence's to share. The words counted are the person's: their
    /// own segments, and the part each outermost playbook expanded from, once — never an author's sentence, which
    /// states its slot's word as often as the author wrote it.
    fn stated(
        &self,
        draft: &Draft,
        original: &[String],
        scope: &[usize],
        vocab: &VocabName,
        vocabulary: &IndexMap<Word, Clean>,
        once: bool,
    ) -> Option<Word> {
        if self.owned(vocab) {
            return None;
        }
        let members: Vec<(&str, &Decision)> = scope
            .iter()
            .map(|&k| (original[k].as_str(), &draft.decisions[k]))
            .collect();
        let mut expansions: Vec<usize> = Vec::new();
        let mut typed: Vec<&str> = Vec::new();
        for &k in scope {
            match draft.origins[k].first() {
                None => typed.push(original[k].as_str()),
                Some(origin) if !expansions.contains(&origin.expansion) => {
                    expansions.push(origin.expansion);
                    typed.push(draft.expansions[origin.expansion].text.as_str());
                }
                Some(_) => {}
            }
        }
        let counted: Vec<(&Word, usize)> = vocabulary
            .keys()
            .map(|word| {
                let n = typed
                    .iter()
                    .map(|text| occurrences(text, word.as_str()))
                    .sum();
                (word, n)
            })
            .filter(|(_, n)| *n > 0)
            .collect();
        let [(word, n)] = counted.as_slice() else {
            return None;
        };
        if once && *n != 1 {
            return None;
        }
        let read = members
            .iter()
            .any(|(text, decision)| self.read_as(text, decision, vocab, word).is_some());
        read.then(|| (*word).clone())
    }

    /// The argument a step read a word as, from its own words, when it read it as one of the vocabulary.
    fn read_as<'d>(
        &self,
        text: &str,
        decision: &'d Decision,
        vocab: &VocabName,
        word: &Word,
    ) -> Option<&'d ArgName> {
        let (_, active) = self.active_of(decision)?;
        let (arg, _) = args_of(decision)?.iter().find(|(arg, value)| {
            matches!(value, Value::Word { word: read, .. } if read == word)
                && matches!(
                    active.args.get(*arg).map(|argument| &argument.kind),
                    Some(Kind::Value { source: Source::Vocab(of), .. }) if of == vocab
                )
                && occurrences(text, word.as_str()) > 0
        })?;
        Some(arg)
    }

    /// What a word stated once stands on, where the first step of the scope read it from its own words.
    fn stood(
        &self,
        draft: &Draft,
        original: &[String],
        scope: &[usize],
        vocab: &VocabName,
        word: &Word,
    ) -> Option<Basis> {
        scope.iter().find_map(|&k| {
            let decision = &draft.decisions[k];
            let arg = self.read_as(&original[k], decision, vocab, word)?;
            basis_of(decision)?.get(arg).cloned()
        })
    }

    /// Whether exactly one active reflex asks for a word of the vocabulary as a required argument: the vocabulary
    /// is that reflex's own.
    fn owned(&self, vocab: &VocabName) -> bool {
        let asking = self
            .plan
            .active()
            .values()
            .filter(|active| {
                active.args.values().any(|argument| {
                    matches!(
                        &argument.kind,
                        Kind::Value { source: Source::Vocab(of), optional: false } if of == vocab
                    )
                })
            })
            .count();
        asking == 1
    }

    /// The reflex a decision is about and its manifest as compiled, when it is about one.
    fn active_of<'d>(&self, decision: &'d Decision) -> Option<(&'d LocalName, &'a Active)> {
        let reflex = reflex_of(decision)?;
        let active = self.plan.active().get(reflex)?;
        Some((reflex, active))
    }

    /// References: code finds the words; with two or more earlier steps, the engine says which one a word names.
    /// A noun may name a field of an earlier result.
    fn refer(&self, draft: &Draft) -> Result<Result<Vec<Vec<Ref>>, Need>, Fault> {
        let fields: Vec<Vec<String>> = draft
            .decisions
            .iter()
            .map(|decision| {
                reflex_of(decision)
                    .and_then(|reflex| self.plan.active().get(reflex))
                    .map_or_else(Vec::new, |active| field_names(&active.yields))
            })
            .collect();
        let mut refs: Vec<Vec<Ref>> = (0..draft.segs.len())
            .map(|k| {
                if k == 0 || draft.recalled.contains(&draft.ids[k]) {
                    Vec::new()
                } else {
                    reading::refs_by_code(&draft.segs, k, &fields)
                }
            })
            .collect();
        // A second verb's step names the step whose object it shares, by code.
        for (k, id) in draft.ids.iter().enumerate() {
            let Some((_, source)) = draft.verbs.iter().find(|(verb, _)| verb == id) else {
                continue;
            };
            let Some(from) = draft.position(*source).checked_sub(1) else {
                continue;
            };
            let text = draft.segs[k].text.clone();
            refs[k].push(Ref {
                span: Where {
                    start: 0,
                    end: text.chars().count(),
                    text,
                },
                from: vec![from],
                how: How::Shared,
                // Code's: a reference only when something takes it, so a first verb that yields nothing a
                // second takes holds nothing for a yes.
                weak: true,
                noun: None,
                many: false,
                p: None,
            });
        }
        if let Ok(Some(refer)) = reading::referring(&self.request, &draft.segs, &refs) {
            match &self.answers.referred {
                Some(raw) => reading::referred(&mut refs, &refer, raw.clone())?,
                None => return Ok(Err(Need::Refer { request: refer })),
            }
        }
        Ok(Ok(refs))
    }

    /// A fragment as another item of its neighbour's task: decided narrowed to that reflex, else spliced into the
    /// neighbour's words in place of the argument value it replaces — «gadgets» for «widgets» in «check stock for
    /// widgets», «the poster» for «the logo» — every word still the person's own.
    fn fanout(
        &self,
        sibling: &Segment,
        decided: &Decision,
        fragment: &str,
    ) -> Result<Option<Fan>, Need> {
        let Some(reflex) = reflex_of(decided) else {
            return Ok(None);
        };
        let narrowed = self.decide(Self::narrowed(fragment, reflex, None))?;
        if !matches!(narrowed, Decision::Abstain { .. }) {
            return Ok(Some(Fan {
                decision: narrowed,
                text: fragment.to_owned(),
                how: Repair::Narrowed,
            }));
        }
        let Some(args) = args_of(decided) else {
            return Ok(None);
        };
        let sibling_chars: Vec<char> = sibling.text.chars().collect();
        let lowered: Vec<char> = sibling.text.to_lowercase().chars().collect();
        let fragment_chars: Vec<char> = fragment.chars().collect();
        let fragment_lowered = fragment.to_lowercase();
        // A determiner the fragment carries replaces the one before the value: «the poster» over «the logo».
        let determiner = determined(&fragment_chars);
        let item = item_of(&fragment_chars);
        for (name, value) in args {
            // A text takes any words, so nothing could tell a fragment put in its place from a clause that reads
            // as nonsense there: a fragment is never spliced in place of a text.
            if matches!(
                value,
                Value::Pick {
                    value: PickValue::Quoted { .. },
                    ..
                }
            ) {
                continue;
            }
            let Some(text) = stated(value) else {
                continue;
            };
            let needle = text.to_lowercase();
            let Some(at) = index_of(&lowered, &needle, 0) else {
                continue;
            };
            let from = match determiner {
                Some(det)
                    if at >= det
                        && lowered[at - det..at]
                            == fragment_lowered.chars().take(det).collect::<Vec<_>>()[..] =>
                {
                    at - det
                }
                _ => at,
            };
            let spliced: String = sibling_chars[..from]
                .iter()
                .chain(fragment_chars.iter())
                .chain(
                    sibling_chars[(at + needle.chars().count()).min(sibling_chars.len())..].iter(),
                )
                .collect();
            let decision = self.decide(Self::narrowed(&spliced, reflex, None))?;
            if matches!(decision, Decision::Abstain { .. }) || reflex_of(&decision) != Some(reflex)
            {
                continue;
            }
            // The value put in place must be the fragment itself, its determiner aside — «the poster» is
            // «poster» — never a word the fragment happens to hold: a clause spliced where a value should stand
            // reads as nonsense, and may still decide.
            let after = args_of(&decision)
                .and_then(|args| args.get(name))
                .and_then(stated);
            if let Some(after) = after
                && Some(&after) != stated(value).as_ref()
                && after.to_lowercase() == item
            {
                return Ok(Some(Fan {
                    decision,
                    text: spliced,
                    how: Repair::Spliced,
                }));
            }
        }
        Ok(None)
    }

    /// Bindings, edges, stages and the verdict over decided steps, with what a playbook's expansion carries: its
    /// review among the reasons, so the plan confirms whatever its verdict; a refusal by step; the folds; the
    /// steps that may not run, bound to the step whose result picks each.
    fn finish(
        &self,
        mut splits: Vec<Split>,
        taken: &[Split],
        mut steps: Vec<Step>,
        excluded: Vec<String>,
        extra: Extra,
    ) -> Weave {
        let Extra {
            reviewed,
            refusals: mut refused,
            folded,
            asides,
            branches,
            spans,
            one,
        } = extra;
        // The request is cut where no step, and no part out of the plan, holds both sides.
        for split in &mut splits {
            split.cut = !spans
                .iter()
                .any(|(start, end)| *start <= split.start && split.end <= *end);
        }
        let mut after: Vec<Vec<usize>> = vec![Vec::new(); steps.len()];
        // An explicit `then` orders everything before it before everything after it.
        for split in taken.iter().filter(|split| split.order == Order::Then) {
            let before: Vec<usize> = steps
                .iter()
                .filter(|s| s.end <= split.start)
                .map(|s| s.n)
                .collect();
            let rest: Vec<usize> = steps
                .iter()
                .map(|s| s.n)
                .filter(|n| !before.contains(n))
                .collect();
            for b in &before {
                for r in &rest {
                    follow(&mut after, *r, *b);
                }
            }
        }
        let Bound {
            binds,
            asks,
            because,
            refusals,
        } = self.bind(&steps, &mut after);
        dated(self.plan, &mut steps, &binds);
        let branched = self.branches(&mut steps, &mut after, &binds, branches);
        for (step, mut edges) in steps.iter_mut().zip(after) {
            edges.sort_unstable();
            step.after = edges;
        }
        // The policy: writes never run beside anything; reads alone may.
        let writes = steps
            .iter()
            .filter(|s| s.effect.is_some_and(|e| e != Effect::Read))
            .count();
        let exclusive = writes > 0 && steps.len() > 1;
        let stages = schedule(&steps, exclusive);
        let mut verdict = verdict_of(&steps, &binds);
        // A whole result no step hands, or several do, is refused before anything runs: nothing answers it. So
        // is a step that routes to its own plan, a plan too deep or too long, a part left out beside a plan, and
        // a step that may not run on a field its step does not yield, or that a later step takes from.
        refused.extend(refusals);
        refused.extend(branched);
        if !refused.is_empty() {
            verdict.outcome = Outcome::Refuse;
            verdict.because.splice(0..0, refused);
        }
        if verdict.outcome != Outcome::Refuse && !asks.is_empty() {
            verdict.outcome = Outcome::Ask;
        }
        verdict.because.extend(asks);
        // A plan a playbook wrote asks one yes over the whole, whatever its verdict: the review is a reason.
        if verdict.outcome == Outcome::Run && !(because.is_empty() && reviewed.is_empty()) {
            verdict.outcome = Outcome::Confirm;
        }
        verdict.because.extend(because);
        verdict.because.extend(reviewed);
        Weave {
            input: self.request.clone(),
            steps,
            binds,
            stages,
            verdict,
            exclusive,
            excluded,
            folded,
            asides,
            splits,
            count: self.count(one),
        }
    }

    /// Every step that may not run bound to the step whose result picks it — the field one value that step's
    /// reflex yields, the value one the field's reader reads — and ordered after it. A source that is a plan of
    /// its own, lacks the field, yields it per record or runs once per record refuses the step; so does a value
    /// the field's reader never reads, which could never be chosen; and a step that takes anything from a step
    /// that may not run is refused too, since nothing may answer it.
    fn branches(
        &self,
        steps: &mut [Step],
        after: &mut [Vec<usize>],
        binds: &[Binding],
        branches: Vec<Branching>,
    ) -> Vec<Because> {
        let mut refusals = Vec::new();
        for Branching { step, when, source } in branches {
            let manifest::When { field, is } = when;
            // A source that matches nothing refuses the plan on its own.
            if source.is_some_and(|n| steps[n - 1].reflex.is_none()) {
                continue;
            }
            let yields = source
                .and_then(|n| steps[n - 1].reflex.as_ref())
                .and_then(|reflex| self.plan.active().get(reflex))
                .map(|active| &active.yields);
            let kind = match (source, yields.and_then(|yields| yields.get(&field))) {
                (Some(n), Some(Yield::Kind(kind))) if !per_record(binds, n) => *kind,
                _ => {
                    refusals.push(Because::NoField { step, field });
                    continue;
                }
            };
            // The value as a body would yield it: a relative day, `tomorrow`, could never be compared.
            if yielded(is.as_str(), kind).is_none() {
                refusals.push(Because::BadValue { step, field, is });
                continue;
            }
            let n = source.expect("the field was found on the source");
            steps[step - 1].when = Some(When { step: n, field, is });
            follow(after, step, n);
        }
        for binding in binds {
            if steps[binding.from - 1].when.is_some() {
                refusals.push(Because::MaybeSource {
                    step: binding.to,
                    name: binding.field.clone(),
                    source: binding.from,
                });
            }
        }
        refusals
    }

    /// Every whole result a step takes bound by its name to the one step before it whose reflex returns it —
    /// the words never choose it — then every reference bound to what a source yields, by kind, a noun naming
    /// the field; never guessed: several fields, or one record of several, are the layer's own questions. A
    /// pronoun or a demonstrative orders the steps whether or not anything binds; `the <noun>` that nothing
    /// takes is plain words.
    fn bind(&self, steps: &[Step], after: &mut [Vec<usize>]) -> Bound {
        let mut out = Bound {
            binds: Vec::new(),
            asks: Vec::new(),
            because: Vec::new(),
            refusals: Vec::new(),
        };
        for step in steps {
            let Some(reflex) = &step.reflex else {
                continue;
            };
            self.takes(step, reflex, steps, after, &mut out);
            let receivers = self.receiving(step);
            for r in &step.refs {
                let sources: Vec<&Step> = r
                    .from
                    .iter()
                    .filter_map(|j| steps.get(*j))
                    .filter(|s| s.reflex.is_some() && s.n < step.n)
                    .collect();
                let mut took: Vec<usize> = Vec::new();
                for source in &sources {
                    if self.take(step, r, source, &receivers, &mut out) {
                        took.push(source.n);
                        follow(after, step.n, source.n);
                    }
                }
                let bound = !took.is_empty();
                if !bound && r.weak {
                    continue;
                }
                for source in &sources {
                    follow(after, step.n, source.n);
                }
                // A plural over several sources: one that yields what the step takes and bound nothing is not
                // dropped in silence; the plan confirms, as it does for a reference nothing takes.
                let dropped: Vec<usize> = if bound && r.many {
                    sources
                        .iter()
                        .filter(|s| !took.contains(&s.n) && self.offers(s, &receivers))
                        .map(|s| s.n)
                        .collect()
                } else if !bound && !sources.is_empty() {
                    sources.iter().map(|s| s.n).collect()
                } else {
                    Vec::new()
                };
                if !dropped.is_empty() {
                    out.because.push(Because::TakesNothing {
                        step: step.n,
                        sources: dropped,
                    });
                }
            }
        }
        out
    }

    /// The whole results a step takes, each by the name its manifest gives, from the one step before it whose
    /// reflex returns that name: none, two, or one that runs once per record stops the plan before anything
    /// runs, since nothing a person could answer settles it.
    fn takes(
        &self,
        step: &Step,
        reflex: &LocalName,
        steps: &[Step],
        after: &mut [Vec<usize>],
        out: &mut Bound,
    ) {
        let Some(active) = self.plan.active().get(reflex) else {
            return;
        };
        for (arg, name) in &active.takes {
            let sources: Vec<usize> = steps
                .iter()
                .take_while(|s| s.n < step.n)
                .filter(|s| {
                    s.reflex
                        .as_ref()
                        .and_then(|reflex| self.plan.active().get(reflex))
                        .is_some_and(|active| active.returns.as_ref() == Some(name))
                })
                .map(|s| s.n)
                .collect();
            match sources.as_slice() {
                [] => out.refusals.push(Because::NoSource {
                    step: step.n,
                    name: name.clone(),
                }),
                [source] if !per_record(&out.binds, *source) => {
                    out.binds.push(Binding {
                        from: *source,
                        to: step.n,
                        arg: arg.clone(),
                        field: name.clone(),
                        kind: None,
                        via: Via::Takes,
                        each: None,
                    });
                    follow(after, step.n, *source);
                }
                _ => out.refusals.push(Because::SeveralSources {
                    step: step.n,
                    name: name.clone(),
                    sources,
                }),
            }
        }
    }

    /// One reference against one source: what the source yields that a free receiver of the step could take. A
    /// noun that names a field settles which: «the png» takes `png`; a bare pronoun over several is asked; one
    /// record of several is asked; a second reference to a source already bound is satisfied. Whether anything
    /// bound.
    fn take(
        &self,
        step: &Step,
        r: &Ref,
        source: &Step,
        receivers: &[Receiver],
        out: &mut Bound,
    ) -> bool {
        let taken_args: Vec<&ArgName> = out
            .binds
            .iter()
            .filter(|b| b.to == step.n)
            .map(|b| &b.arg)
            .collect();
        let free: Vec<&Receiver> = receivers
            .iter()
            .filter(|rc| !taken_args.contains(&&rc.arg))
            .collect();
        let yields = source
            .reflex
            .as_ref()
            .and_then(|reflex| self.plan.active().get(reflex))
            .map(|active| &active.yields);
        let candidates = takeable(yields, &free);
        let already: Vec<&Binding> = out
            .binds
            .iter()
            .filter(|b| b.from == source.n && b.to == step.n)
            .collect();
        if candidates.is_empty() {
            return !already.is_empty();
        }
        let named: Vec<&Candidate> = r.noun.as_ref().map_or_else(Vec::new, |noun| {
            candidates
                .iter()
                .filter(|c| crate::words::stem_of(c.field.as_str()) == *noun)
                .collect()
        });
        let chosen: Vec<&Candidate> = if named.is_empty() {
            candidates.iter().collect()
        } else {
            named
        };
        if chosen.len() == 1 && already.iter().any(|b| b.field == chosen[0].field) {
            return true;
        }
        // A source that runs once per record returns one result per round: several, where the step takes one.
        if per_record(&out.binds, source.n) {
            out.refusals.push(Because::SeveralSources {
                step: step.n,
                name: chosen[0].field.clone(),
                sources: vec![source.n],
            });
            return true;
        }
        if chosen.len() > 1 {
            out.asks.push(Because::Several {
                step: step.n,
                source: source.n,
                fields: chosen.iter().map(|c| c.field.clone()).collect(),
            });
            return true;
        }
        let c = chosen[0];
        if c.each.is_some() && !r.many {
            out.asks.push(Because::OneOfMany {
                step: step.n,
                source: source.n,
                field: c.field.clone(),
            });
            return true;
        }
        let receiver = free
            .iter()
            .find(|rc| rc.kind == c.kind && rc.required)
            .or_else(|| free.iter().find(|rc| rc.kind == c.kind))
            .expect("a candidate has a free receiver of its kind");
        out.binds.push(Binding {
            from: source.n,
            to: step.n,
            arg: receiver.arg.clone(),
            field: c.field.clone(),
            kind: Some(c.kind),
            via: if receiver.required {
                Via::Fill
            } else {
                Via::Rewrite
            },
            each: c.each.clone(),
        });
        true
    }

    /// Whether a source yields anything a receiver of the step could take, free or not.
    fn offers(&self, source: &Step, receivers: &[Receiver]) -> bool {
        let yields = source
            .reflex
            .as_ref()
            .and_then(|reflex| self.plan.active().get(reflex))
            .map(|active| &active.yields);
        let all: Vec<&Receiver> = receivers.iter().collect();
        !takeable(yields, &all).is_empty()
    }

    /// The arguments of a step a bound value may reach: every pick the words left unstated, the missing required
    /// ones marked — those `fill` answers; an optional one is rewritten into the words.
    fn receiving(&self, step: &Step) -> Vec<Receiver> {
        let Some(active) = step
            .reflex
            .as_ref()
            .and_then(|reflex| self.plan.active().get(reflex))
        else {
            return Vec::new();
        };
        let missing: Vec<&ArgName> = match &step.decision {
            Decision::Ask { missing, .. } => missing.iter().map(|m| &m.arg).collect(),
            _ => Vec::new(),
        };
        let stated: Vec<&ArgName> =
            args_of(&step.decision).map_or_else(Vec::new, |args| args.keys().collect());
        active
            .args
            .iter()
            .filter_map(|(name, argument)| match &argument.kind {
                Kind::Value {
                    source: Source::Pick(pick),
                    ..
                } if !stated.contains(&name) => Some(Receiver {
                    arg: name.clone(),
                    kind: pick.recognizer(),
                    required: missing.contains(&name),
                }),
                _ => None,
            })
            .collect()
    }
}

/// What binding found: the bindings, the layer's own questions, the references nothing takes, and what stops
/// the plan before anything runs.
struct Bound {
    binds: Vec<Binding>,
    asks: Vec<Because>,
    because: Vec<Because>,
    refusals: Vec<Because>,
}

/// A pick argument a bound value may reach.
struct Receiver {
    arg: ArgName,
    kind: Recognizer,
    required: bool,
}

/// What a result offers a step: a field of a receiver's kind, a record's field marked with its list.
struct Candidate {
    field: FieldName,
    kind: Recognizer,
    each: Option<FieldName>,
}

/// Whether a step runs once per record: a binding into it takes a field of a list's records.
fn per_record(binds: &[Binding], step: usize) -> bool {
    binds.iter().any(|b| b.to == step && b.each.is_some())
}

/// `later` follows `earlier`, once.
fn follow(after: &mut [Vec<usize>], later: usize, earlier: usize) {
    if !after[later - 1].contains(&earlier) {
        after[later - 1].push(earlier);
    }
}

/// The fields of a result some free receiver could take, a list's records opened.
fn takeable(yields: Option<&IndexMap<FieldName, Yield>>, free: &[&Receiver]) -> Vec<Candidate> {
    let mut out = Vec::new();
    let Some(yields) = yields else {
        return out;
    };
    for (field, yield_) in yields {
        match yield_ {
            Yield::Kind(kind) => {
                if free.iter().any(|r| r.kind == *kind) {
                    out.push(Candidate {
                        field: field.clone(),
                        kind: *kind,
                        each: None,
                    });
                }
            }
            Yield::Each(fields) => {
                for (sub, kind) in fields {
                    if free.iter().any(|r| r.kind == *kind) {
                        out.push(Candidate {
                            field: sub.clone(),
                            kind: *kind,
                            each: Some(field.clone()),
                        });
                    }
                }
            }
        }
    }
    out
}

/// The split point between segment `k - 1` and segment `k`, when one was taken there.
fn split_before<'s>(taken: &'s [Split], segs: &[Segment], k: usize) -> Option<&'s Split> {
    let (before, at) = (segs.get(k.checked_sub(1)?)?, segs.get(k)?);
    taken
        .iter()
        .find(|s| s.start >= before.end && s.end <= at.start)
}

/// Reads alone are layers by longest path, each layer run together; a write among the steps puts every step at
/// its turn in the words' order, alone — a step follows only steps before it, so that order keeps every edge, and
/// it is the order the plan prints.
fn schedule(steps: &[Step], exclusive: bool) -> Vec<Vec<usize>> {
    fn depth(steps: &[Step], n: usize, memo: &mut Vec<Option<usize>>) -> usize {
        if let Some(depth) = memo[n - 1] {
            return depth;
        }
        let d = steps[n - 1]
            .after
            .iter()
            .map(|m| depth(steps, *m, memo) + 1)
            .max()
            .unwrap_or(0);
        memo[n - 1] = Some(d);
        d
    }
    if exclusive {
        return steps.iter().map(|step| vec![step.n]).collect();
    }
    let mut memo = vec![None; steps.len()];
    let mut layers: Vec<Vec<usize>> = Vec::new();
    for step in steps {
        let d = depth(steps, step.n, &mut memo);
        if layers.len() <= d {
            layers.resize(d + 1, Vec::new());
        }
        layers[d].push(step.n);
    }
    layers
        .into_iter()
        .filter(|layer| !layer.is_empty())
        .collect()
}

/// Before anything runs: a request left with no step refuses; a step that matches nothing refuses the whole
/// request; a required argument no binding covers asks; else the plan stands, its confirms taken at their turn.
fn verdict_of(steps: &[Step], binds: &[Binding]) -> Verdict {
    if steps.is_empty() {
        return Verdict {
            outcome: Outcome::Refuse,
            because: vec![Because::NothingToDo],
        };
    }
    let mut because = Vec::new();
    for step in steps {
        match &step.decision {
            Decision::Abstain { .. } => because.push(Because::NoReflex { step: step.n }),
            Decision::Ask { missing, .. } => {
                for m in missing.iter() {
                    if !binds.iter().any(|b| b.to == step.n && b.arg == m.arg) {
                        because.push(Because::Needs {
                            step: step.n,
                            arg: m.arg.clone(),
                        });
                    }
                }
            }
            Decision::Run { .. } | Decision::Confirm { .. } => {}
        }
    }
    let outcome = if because
        .iter()
        .any(|b| matches!(b, Because::NoReflex { .. }))
    {
        Outcome::Refuse
    } else if because.is_empty() {
        Outcome::Run
    } else {
        Outcome::Ask
    };
    Verdict { outcome, because }
}

/// What a decision read as: its reflex, and each value its call holds in the words of the request that hold it,
/// or as it is typed where code knows none. None where it read as none.
fn read_of(decision: &Decision) -> Option<(LocalName, Vec<(ArgName, String)>)> {
    let basis = basis_of(decision);
    let read = args_of(decision)?
        .iter()
        .filter_map(|(arg, value)| {
            let words = match (value, basis.and_then(|basis| basis.get(arg))) {
                (Value::Pick { span, .. }, _) => span.text().as_str(),
                (_, Some(Basis::View { words, .. } | Basis::Words { words, .. })) => {
                    words.text().as_str()
                }
                (
                    _,
                    Some(Basis::Views {
                        anchored: Some(anchored),
                        ..
                    }),
                ) => anchored.words.text().as_str(),
                _ => value.text()?,
            };
            Some((arg.clone(), words.to_owned()))
        })
        .collect();
    Some((reflex_of(decision)?.clone(), read))
}

/// A day of the month a step's call holds alone, where the step takes from a step whose call holds a calendar
/// day, is read beside that day, as one is read beside today: in its month when on or after it, else in the next
/// month that has it. The value is shown as it is then typed, and the step records the step it was read beside.
fn dated(plan: &Plan, steps: &mut [Step], binds: &[Binding]) {
    for n in 1..=steps.len() {
        let (decision, read) = dated_at(plan, steps, binds, n, steps[n - 1].decision.clone());
        steps[n - 1].decision = decision;
        steps[n - 1].beside.extend(read);
    }
}

/// A decision of step `n`, made by the plan or again at the step's turn, with each day of the month it holds
/// alone read beside the calendar day of a step it takes from; and each argument so read, with that step.
pub(crate) fn dated_at(
    plan: &Plan,
    steps: &[Step],
    binds: &[Binding],
    n: usize,
    decision: Decision,
) -> (Decision, Vec<(ArgName, usize)>) {
    let mut read = Vec::new();
    let decision = binds
        .iter()
        .filter(|bind| bind.to == n)
        .filter_map(|bind| {
            let source = steps.get(bind.from.checked_sub(1)?)?;
            Some((bind.from, calendar_of(&source.decision)?))
        })
        .fold(decision, |decision, (from, named)| {
            let (decision, moved) = beside(plan, decision, &named);
            read.extend(moved.into_iter().map(|arg| (arg, from)));
            decision
        });
    (decision, read)
}

/// The calendar day a decision's call holds, when it holds exactly one.
fn calendar_of(decision: &Decision) -> Option<Day> {
    let mut days = args_of(decision)?.values().filter_map(|value| match value {
        Value::Pick {
            value: PickValue::Date { value: day },
            ..
        } if matches!(day, Day::Calendar { .. }) => Some(day.clone()),
        _ => None,
    });
    let day = days.next()?;
    days.next().is_none().then_some(day)
}

/// A decision with each day of the month its call holds alone read beside a named calendar day, and the
/// arguments so read; a confirm's prompt is made again where a day moved, since it shows the call.
fn beside(plan: &Plan, decision: Decision, named: &Day) -> (Decision, Vec<ArgName>) {
    let read = |args: &mut IndexMap<ArgName, Value>| {
        let mut moved = Vec::new();
        for (arg, value) in args.iter_mut() {
            if let Value::Pick {
                value: PickValue::Date { value: day },
                typed,
                span,
            } = value
                && let Some(read) = day.beside(named)
            {
                *typed =
                    crate::propose::typed_calendar(&pack::lexicon(span.text().as_str()), &read);
                *day = read;
                moved.push(arg.clone());
            }
        }
        moved
    };
    match decision {
        Decision::Run { mut chosen } => {
            let moved = read(&mut chosen.call.args);
            (Decision::Run { chosen }, moved)
        }
        Decision::Confirm {
            mut chosen,
            prompt,
            because,
        } => {
            let moved = read(&mut chosen.call.args);
            let prompt = match plan.active().get(&chosen.call.reflex) {
                Some(active) if !moved.is_empty() => {
                    let caps: Vec<Cap> = because.iter().cloned().collect();
                    Prompt::of(&chosen, active, &caps)
                }
                _ => prompt,
            };
            (
                Decision::Confirm {
                    chosen,
                    prompt,
                    because,
                },
                moved,
            )
        }
        Decision::Ask {
            mut asking,
            missing,
        } => {
            let moved = read(&mut asking.args);
            (Decision::Ask { asking, missing }, moved)
        }
        Decision::Abstain { .. } => (decision, Vec::new()),
    }
}

/// Whether the engine's reading of a whole left it unsettled: it abstained, asked, or sat under the floor — not
/// one decided firmly, nor one whose call leaves a typed span no argument took, which is one task as it stands.
fn unsettled(decision: &Decision) -> bool {
    match decision {
        Decision::Abstain { .. } | Decision::Ask { .. } => true,
        Decision::Confirm {
            chosen, because, ..
        } => {
            because
                .iter()
                .any(|cap| matches!(cap, Cap::UnderFloor { .. }))
                && chosen.unconsumed.is_empty()
        }
        Decision::Run { .. } => false,
    }
}

/// The complete call a decision holds, when it holds one.
fn chosen_of(decision: &Decision) -> Option<&crate::decide::Chosen> {
    match decision {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(chosen),
        Decision::Ask { .. } | Decision::Abstain { .. } => None,
    }
}

/// A list's items in place of the segment at `k`, the parts left out recorded, each repair with its text and
/// every inner split taken with the judgment it had; how many segments the list became.
fn listed(
    draft: &mut Draft,
    k: usize,
    base: usize,
    parts: &[Segment],
    items: Vec<(Segment, Decision, Option<Repair>)>,
    inner: &[Split],
    judged: &[Split],
) -> usize {
    draft.excluded.extend(
        parts
            .iter()
            .filter(|part| part.excluded())
            .map(|part| part.text.clone()),
    );
    let count = items.len();
    let (segs, decisions): (Vec<Segment>, Vec<Decision>) = items
        .into_iter()
        .map(|(seg, decision, how)| {
            if let Some(how) = how {
                draft.repaired.push((seg.text.clone(), how));
            }
            (seg, decision)
        })
        .unzip();
    draft.replace(k, segs, decisions, vec![Vec::new(); count]);
    for s in inner {
        let start = base + s.start;
        draft.taken.push(Split {
            start,
            end: base + s.end,
            word: s.word.clone(),
            order: s.order,
            p: judged.iter().find(|a| a.start == start).and_then(|a| a.p),
            cut: false,
        });
    }
    draft.taken.sort_by_key(|s| s.start);
    count
}

/// The parts code set aside by their words, in the request's order: each a remark, what it is by `by`, asked
/// nothing.
fn by_words(apart: &[Segment]) -> Vec<Aside> {
    apart
        .iter()
        .filter_map(|seg| match seg.left {
            Some(Left::Aside(by)) => Some(Aside {
                text: seg.text.clone(),
                remark: true,
                does: None,
                by: Some(by),
            }),
            _ => None,
        })
        .collect()
}

/// The fold. A part of the person's own that repeats a step a playbook wrote — the same reflex, every value it
/// read equal to that step's, every value it asks for held by that step — is removed and recorded in the
/// person's words, so the step runs once: the plan asks one yes over its steps, and the part's words stand
/// beside them. A part that picks a playbook whose plan already stands folds by the same rule where the playbook
/// would expand.
fn fold(draft: &mut Draft) {
    let mut k = 0;
    while k < draft.segs.len() {
        let asks = asked_of(&draft.decisions[k]);
        // A second verb's step is its own call, of the step whose object it shares: never one said again.
        if !draft.origins[k].is_empty() || draft.verb(k) {
            k += 1;
            continue;
        }
        let (Some(reflex), Some(read)) =
            (reflex_of(&draft.decisions[k]), args_of(&draft.decisions[k]))
        else {
            k += 1;
            continue;
        };
        // Only into a step the author wrote that always runs: the person asked unconditionally.
        let target = (0..draft.segs.len()).find(|&j| {
            j != k
                && !draft.verb(j)
                && draft.origins[j]
                    .last()
                    .is_some_and(|origin| origin.when.is_none())
                && reflex_of(&draft.decisions[j]) == Some(reflex)
                && read.iter().all(|(arg, value)| {
                    args_of(&draft.decisions[j])
                        .and_then(|theirs| theirs.get(arg))
                        .is_some_and(|theirs| stated(theirs) == stated(value))
                })
                // What the part asks, the step holds: the step answers it.
                && asks.iter().all(|arg| {
                    args_of(&draft.decisions[j]).is_some_and(|theirs| theirs.contains_key(*arg))
                })
        });
        match target {
            Some(j) => draft.fold_into(k, draft.ids[j], Vec::new(), None),
            None => k += 1,
        }
    }
}

/// A value one part states, offered to another part that lacks one of its kind: the two parts by their places,
/// the taker's reflex and argument with its ask, the value as the question shows it and what its no says, and
/// the value.
struct Offer {
    taker: usize,
    giver: usize,
    reflex: LocalName,
    arg: ArgName,
    ask: Clean,
    shown: String,
    no: &'static str,
    value: Value,
}

/// A value offered, with the question whether it is the taker's too.
type Offered<'o> = (&'o Offer, QuestionId, Question);

/// The question of each value offered: whether it is the taker's too; none for words that cannot be asked about.
fn shares_of<'o>(draft: &Draft, offers: &'o [Offer]) -> Vec<Offered<'o>> {
    offers
        .iter()
        .filter_map(|offer| {
            let (id, question) = reading::shared(
                (&draft.segs[offer.taker], &draft.segs[offer.giver]),
                (&offer.reflex, &offer.arg),
                &offer.ask,
                (&offer.shown, offer.no),
            )
            .ok()?;
            Some((offer, id, question))
        })
        .collect()
}

/// The most values of other parts one argument is offered.
const MOST_GIVERS: usize = 2;

/// At this share a yes gives a part the value another part states.
const SHARE: f64 = 0.5;

/// At this share a part is a remark; and under one less it, it asks for something.
const ASIDE: f64 = 0.5;

/// Whether a part's words point at something said elsewhere.
fn pointing(text: &str) -> bool {
    says(
        text,
        &pack::lexicon(text).phrases(|pack| &pack.refer.points),
    )
}

/// Whether a part's words say an earlier call is done again.
fn repeating(text: &str) -> bool {
    says(
        text,
        &pack::lexicon(text).phrases(|pack| &pack.refer.repeats),
    )
}

/// Whether a text holds one of the phrases, word for word.
fn says(text: &str, phrases: &[&str]) -> bool {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(text::fold)
        .collect();
    phrases.iter().any(|phrase| {
        let phrase: Vec<&str> = phrase.split(' ').collect();
        words
            .windows(phrase.len())
            .any(|window| window.iter().map(String::as_str).eq(phrase.iter().copied()))
    })
}

/// Whether two readings of one reflex do not differ: what one holds the other holds too, or lacks; and
/// neither holds a value for what the other's words state and no answer read.
fn accord(a: &Decision, b: &Decision) -> bool {
    let (Some(held), Some(other)) = (args_of(a), args_of(b)) else {
        return false;
    };
    let unread = |decision: &Decision, against: &IndexMap<ArgName, Value>| match decision {
        Decision::Ask { missing, .. } => missing.iter().any(|asked| {
            !matches!(asked.because, crate::decide::Why::Unstated)
                && against.contains_key(&asked.arg)
        }),
        _ => false,
    };
    !unread(a, other)
        && !unread(b, held)
        && held.iter().all(|(arg, value)| {
            other
                .get(arg)
                .is_none_or(|theirs| stated_of(theirs) == stated_of(value))
        })
}

/// The arguments a decision asks for.
fn asked_of(decision: &Decision) -> Vec<&ArgName> {
    match decision {
        Decision::Ask { missing, .. } => missing.iter().map(|asked| &asked.arg).collect(),
        _ => Vec::new(),
    }
}

/// A value as it is compared between two readings: its text, the case of its letters aside; a flag by itself.
fn stated_of(value: &Value) -> Option<String> {
    value.text().map(str::to_lowercase)
}

/// Which of several a part's own words pick, by the one place they name — the first, the second, the last,
/// as the pack lists the words that pick — with the word that names it as typed; none where they name none, two,
/// or one past the last.
fn pointed(words: &str, of: usize) -> Option<(usize, String)> {
    let lexicon = pack::lexicon(words);
    let mut places: Vec<(isize, &str)> = words
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|word| {
            let place = lexicon.value(|pack| &pack.ordinals.picks, &text::fold(word))?;
            Some((isize::try_from(place).ok()?, word))
        })
        .collect();
    places.sort_unstable_by_key(|(place, _)| *place);
    places.dedup_by_key(|(place, _)| *place);
    let [(place, word)] = places.as_slice() else {
        return None;
    };
    let of = isize::try_from(of).ok()?;
    let at = if *place < 0 { of + place } else { *place };
    usize::try_from(at)
        .ok()
        .filter(|_| at < of)
        .map(|at| (at, (*word).to_owned()))
}

/// Why a step that routes to a playbook opens no plan, when it does not: the step may not run, and a branch is
/// one step; the playbook is on the step's own chain; the chain is as deep as a plan goes.
fn refusal(chain: &[Origin], playbook: &LocalName) -> Option<Refused> {
    if chain.last().is_some_and(|origin| origin.when.is_some()) {
        Some(Refused::BranchIntoPlan(playbook.clone()))
    } else if chain.iter().any(|origin| origin.from.playbook == *playbook) {
        Some(Refused::Nested(playbook.clone()))
    } else if chain.len() >= HOPS {
        Some(Refused::TooDeep(playbook.clone()))
    } else {
        None
    }
}

/// Every step that may not run, once the segments stand: its number, what picks it, and the step whose result
/// picks it — the nearest before it in the same expansion without `when`; none when that step expanded into a
/// plan of its own, which yields nothing.
fn branches_of(draft: &Draft) -> Vec<Branching> {
    (0..draft.segs.len())
        .filter_map(|k| {
            let depth = draft.origins[k].len().checked_sub(1)?;
            let origin = &draft.origins[k][depth];
            let when = origin.when.clone()?;
            let source = (0..k)
                .rev()
                .find(|&j| {
                    draft.origins[j].get(depth).is_some_and(|before| {
                        before.expansion == origin.expansion && before.when.is_none()
                    })
                })
                .filter(|&j| draft.origins[j].len() == depth + 1)
                .map(|j| j + 1);
            Some(Branching {
                step: k + 1,
                when,
                source,
            })
        })
        .collect()
}

/// The arguments of a decision that is a complete call: a run, or a confirm.
fn complete(decision: &Decision) -> Option<&IndexMap<ArgName, Value>> {
    match decision {
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(&chosen.call.args),
        Decision::Ask { .. } | Decision::Abstain { .. } => None,
    }
}

/// The prompt of a playbook's decision: its own when it confirmed, else made over the run.
fn prompt_of(decision: &Decision, active: &Active) -> Prompt {
    match decision {
        Decision::Confirm { prompt, .. } => prompt.clone(),
        Decision::Run { chosen } => Prompt::of(chosen, active, &[]),
        Decision::Ask { .. } | Decision::Abstain { .. } => {
            unreachable!("a playbook expands from a complete call")
        }
    }
}

/// The runs of the plan: consecutive segments joined by coordinating connectives, an ordering word ending each.
fn runs_of(taken: &[Split], segs: &[Segment]) -> Vec<Vec<usize>> {
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for k in 0..segs.len() {
        let ordered = split_before(taken, segs, k).is_some_and(|s| s.order == Order::Then);
        match runs.last_mut() {
            Some(run) if !ordered => run.push(k),
            _ => runs.push(vec![k]),
        }
    }
    runs
}

/// How a segment's step came to be, when it was repaired.
fn repair_of(draft: &Draft, k: usize) -> Option<Repair> {
    draft
        .repaired
        .iter()
        .find(|(text, _)| *text == draft.segs[k].text)
        .map(|(_, how)| *how)
}

/// Whether a step decided again with shared words in its words took them: the reflex holds, each word landed on
/// its argument, and no argument the step had read moved.
fn kept(
    before: &Decision,
    again: &Decision,
    reflex: &LocalName,
    carried: &IndexMap<ArgName, Word>,
) -> bool {
    if reflex_of(again) != Some(reflex) {
        return false;
    }
    let Some(after) = args_of(again) else {
        return false;
    };
    carried.iter().all(|(arg, word)| {
        matches!(after.get(arg), Some(Value::Word { word: landed, .. }) if landed == word)
    }) && args_of(before).is_some_and(|read| {
        read.iter().all(|(arg, value)| {
            after
                .get(arg)
                .is_some_and(|moved| stated(moved) == stated(value))
        })
    })
}

/// Whether a text holds any word of a vocabulary: such a step states its own value, or asks for it itself.
fn holds(text: &str, vocabulary: &IndexMap<Word, Clean>) -> bool {
    vocabulary
        .keys()
        .any(|word| occurrences(text, word.as_str()) > 0)
}

/// How often a text holds a word, bounded as the research counted it and in any letter case: «checkout's» holds
/// `checkout`, «#incident» holds `#incident`, «eu-west» holds neither `eu` nor `west`.
fn occurrences(text: &str, word: &str) -> usize {
    let text: Vec<char> = text::fold(text).chars().collect();
    let word: Vec<char> = text::fold(word).chars().collect();
    if word.is_empty() || word.len() > text.len() {
        return 0;
    }
    let joins = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    (0..=text.len() - word.len())
        .filter(|&i| {
            text[i..i + word.len()] == word[..]
                && !(i > 0 && (joins(text[i - 1]) || text[i - 1] == '#'))
                && !text.get(i + word.len()).is_some_and(|c| joins(*c))
        })
        .count()
}

/// The runs of words a decision's values leave over, with what each does.
pub(crate) fn left_of(decision: &Decision) -> &[Run] {
    match decision {
        Decision::Abstain { .. } => &[],
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => &chosen.left,
        Decision::Ask { asking, .. } => &asking.left,
    }
}

/// The reflex a decision is about, if any.
pub(crate) fn reflex_of(decision: &Decision) -> Option<&LocalName> {
    match decision {
        Decision::Abstain { .. } => None,
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(&chosen.call.reflex),
        Decision::Ask { asking, .. } => Some(&asking.reflex),
    }
}

/// What each value a decision read stands on.
fn basis_of(decision: &Decision) -> Option<&IndexMap<ArgName, Basis>> {
    match decision {
        Decision::Abstain { .. } => None,
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(&chosen.basis),
        Decision::Ask { asking, .. } => Some(&asking.basis),
    }
}

/// The arguments a decision read, complete or partial.
pub(crate) fn args_of(decision: &Decision) -> Option<&IndexMap<ArgName, Value>> {
    match decision {
        Decision::Abstain { .. } => None,
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(&chosen.call.args),
        Decision::Ask { asking, .. } => Some(&asking.args),
    }
}

/// An article at the head of the words, and the whitespace after it: where the item begins.
fn determined(chars: &[char]) -> Option<usize> {
    let text: String = chars.iter().collect();
    let lexicon = pack::lexicon(&text);
    lexicon
        .phrases(|pack| &pack.words.articles)
        .into_iter()
        .find_map(|article| {
            let length = article.chars().count();
            let head = text::fold(&chars.iter().take(length).collect::<String>());
            let mut end = length;
            while end < chars.len() && chars[end].is_whitespace() {
                end += 1;
            }
            (head == article && end > length).then_some(end)
        })
}

/// The item the words name: what follows the determiner, lowered, the sentence's end mark aside.
fn item_of(chars: &[char]) -> String {
    let from = determined(chars).unwrap_or(0);
    chars[from..]
        .iter()
        .collect::<String>()
        .to_lowercase()
        .trim_end_matches(['.', '!', '?'])
        .trim()
        .to_owned()
}

/// The words of a text that hold its decision's values: a typed value's span; a listed word's by what it
/// stands on, else the word itself where the text holds it. None where the decision is no call, the text
/// cannot be read, or a listed word can be placed on no word of it.
fn held_by(text: &str, decision: &Decision) -> Option<(Input, Vec<Span>)> {
    let args = args_of(decision)?;
    let basis = basis_of(decision)?;
    let input = Input::new(text).ok()?;
    let tokens = crate::words::tokens(input.as_str());
    let mut held: Vec<Span> = Vec::new();
    for (arg, value) in args {
        let words = match (value, basis.get(arg)) {
            (Value::Flag, _) => continue,
            (Value::Pick { span, .. }, _) => span.clone(),
            (_, Some(Basis::View { words, .. } | Basis::Words { words, .. })) => words.clone(),
            (
                _,
                Some(Basis::Views {
                    anchored: Some(anchored),
                    ..
                }),
            ) => anchored.words.clone(),
            _ => {
                let word = value.text()?.to_lowercase();
                let token = tokens.iter().find(|token| token.plain == word)?;
                Span::of(&input, token.start, token.end)?
            }
        };
        held.push(words);
    }
    Some((input, held))
}

/// The words of a text that are values, where its words are values alone: every word is one a value the
/// decision read holds, one code proposes as a typed value, one that carries nothing, or one of `ONCE_MORE`,
/// with one value at least and no text in quotes. None otherwise. Code's candidates count beside the decision's
/// values, so that a part reads as values alone whatever the engine made of it: alone, «2 of BOK-603» may
/// read as nothing, or as a call that took the number or left it.
fn values_alone(text: &str, decision: Option<&Decision>) -> Option<Vec<Span>> {
    let input = Input::new(text).ok()?;
    if !crate::account::quotes(&input, &[]).is_empty() {
        return None;
    }
    let mut held: Vec<Span> = decision
        .and_then(|decision| held_by(text, decision))
        .map(|(_, held)| held)
        .unwrap_or_default();
    for proposed in crate::propose::propose(&input) {
        let within = held.iter().any(|value| {
            value.start() <= proposed.span.start() && proposed.span.end() <= value.end()
        });
        if !within {
            held.push(proposed.span);
        }
    }
    // A word by which the part says the step before it is wanted once more, «2 more of BOK-603», «1004 too»,
    // names no action.
    let lexicon = pack::lexicon(input.as_str());
    let idle = |word: &str| {
        word.is_empty()
            || crate::words::function(&lexicon, word)
            || lexicon.holds(|pack| &pack.refer.once_more, &text::fold(word))
    };
    let covered = crate::words::tokens(input.as_str()).iter().all(|token| {
        idle(&token.plain)
            || held
                .iter()
                .any(|value| token.start < value.end() && value.start() < token.end)
    });
    (!held.is_empty() && covered).then_some(held)
}

/// A part put in a step's words in place of the values they hold: the words from the step's first value to its
/// last replaced by the part's — «buy 2 of BOK-603» from «buy 3 of OUT-503» and «2 of BOK-603» — and where the
/// part begins in them. None where the step's words hold no value, or nothing but values, so that no word of
/// the step's would say what to do.
fn in_place(step: &str, decision: &Decision, part: &str) -> Option<(String, usize)> {
    let (input, held) = held_by(step, decision)?;
    let start = held.iter().map(Span::start).min()?;
    let end = held.iter().map(Span::end).max()?;
    let chars: Vec<char> = input.as_str().chars().collect();
    let (before, after): (String, String) = (
        chars[..start.min(chars.len())].iter().collect(),
        chars[end.min(chars.len())..].iter().collect(),
    );
    let spoken = |words: &str| {
        crate::words::tokens(words).iter().any(|token| {
            !token.plain.is_empty() && !crate::words::function(&pack::lexicon(words), &token.plain)
        })
    };
    (spoken(&before) || spoken(&after)).then(|| (format!("{before}{part}{after}"), start))
}

/// Whether a part put in a step's words reads as the step's call over the part's own values: the reading holds
/// a value for each argument the step's call holds and no other, asks for nothing the step did not ask, and
/// every value the part holds, standing at `at` in the words, is held by a value it read.
fn continues(step: &Decision, spliced: &Decision, text: &str, at: usize, values: &[Span]) -> bool {
    let (Some(before), Some(after)) = (args_of(step), args_of(spliced)) else {
        return false;
    };
    let asked = asked_of(step);
    let same = before.len() == after.len() && before.keys().all(|arg| after.contains_key(arg));
    let Some((_, held)) = held_by(text, spliced) else {
        return false;
    };
    same && asked_of(spliced).iter().all(|arg| asked.contains(arg))
        && values.iter().all(|value| {
            held.iter()
                .any(|read| read.start() <= value.start() + at && value.end() + at <= read.end())
        })
}

/// Whether an object put after a bare verb reads as that verb's call over the object's values: the reading
/// holds a value for every argument the verb alone asked, and every value the object holds, standing at `at` in
/// the words, is held by a value it read.
fn shares(verb: &Decision, whole: &Decision, text: &str, at: usize, values: &[Span]) -> bool {
    let Some(read) = args_of(whole) else {
        return false;
    };
    let Some((_, held)) = held_by(text, whole) else {
        return false;
    };
    asked_of(verb).iter().all(|arg| read.contains_key(*arg))
        && values.iter().all(|value| {
            held.iter()
                .any(|read| read.start() <= value.start() + at && value.end() + at <= read.end())
        })
}

/// A second verb before the object of the first, in a step's words: a run that says what to do holds a sign
/// for «and» between two words, the words after the sign carry a word and no value, and the run stands before
/// every value the call holds. The step's words without the sign and the second verb, and the second verb:
/// «buy 3 of OUT-503» and «pay» from «buy and pay 3 of OUT-503».
fn second_verb(text: &str, decision: &Decision) -> Option<(String, String)> {
    let held = held_by(text, decision)
        .map(|(_, held)| held)
        .unwrap_or_default();
    let chars: Vec<char> = text.chars().collect();
    left_of(decision)
        .iter()
        .filter(|run| run.does == Does::Action && run.p.get() >= SAYS)
        .filter(|run| stands_in(text, &run.words))
        .filter(|run| held.iter().all(|value| run.words.end() <= value.start()))
        .find_map(|run| {
            let words = run.words.text().as_str();
            let place = reading::places(words, false)
                .into_iter()
                .find(|split| split.order == Order::And)?;
            let own: Vec<char> = words.chars().collect();
            let before: String = own[..place.start.min(own.len())].iter().collect();
            let after: String = own[place.end.min(own.len())..].iter().collect();
            let (before, after) = (before.trim(), after.trim());
            // Two verbs, each a word or two that carry something and point at nothing: never the last two
            // items of a list, «the invoices and card expenses», «cards and payroll».
            if !verb_group(before) || !verb_group(after) {
                return None;
            }
            let input = Input::new(after).ok()?;
            if !crate::propose::propose(&input).is_empty() {
                return None;
            }
            let cut = (run.words.start() + place.start).min(chars.len());
            let rest = run.words.end().min(chars.len());
            let base: String = chars[..cut]
                .iter()
                .chain(chars[rest..].iter())
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            Some((base, after.to_owned()))
        })
}

/// Whether words are a verb group as a second verb is told by its shape: one word or two, each carrying
/// something, none a mark, and nothing that points back.
fn verb_group(words: &str) -> bool {
    let tokens = crate::words::tokens(words);
    !tokens.is_empty()
        && tokens.len() <= 2
        && tokens.iter().all(|token| {
            !token.plain.is_empty()
                && !crate::words::function(&pack::lexicon(words), &token.plain)
                && token.end - token.start == token.plain.chars().count()
        })
        && !reading::refers_back(words)
}

/// Whether a span stands in a text: its characters are the text's at its place.
fn stands_in(text: &str, span: &Span) -> bool {
    let own: String = text
        .chars()
        .skip(span.start())
        .take(span.end() - span.start())
        .collect();
    own == span.text().as_str()
}

/// Whether a step decided again in other words read as it did: the same reflex, the same arguments with the
/// same values, as the words state them, and the same arguments asked.
fn alike(before: &Decision, after: &Decision) -> bool {
    let (Some(held), Some(theirs)) = (args_of(before), args_of(after)) else {
        return false;
    };
    reflex_of(before) == reflex_of(after)
        && held.len() == theirs.len()
        && held.iter().all(|(arg, value)| {
            theirs
                .get(arg)
                .is_some_and(|other| stated_of(other) == stated_of(value))
        })
        && asked_of(before) == asked_of(after)
}

/// Whether two arguments take their values from the same source: the same recognizer, whatever its range, the
/// same vocabulary, or the same options.
fn same_source(a: &Source, b: &Source) -> bool {
    match (a, b) {
        (Source::Pick(a), Source::Pick(b)) => {
            std::mem::discriminant(a) == std::mem::discriminant(b)
        }
        _ => a == b,
    }
}

/// A bare item: a determiner and one word — «the logo» — the shape of a second item of the neighbour's task by the
/// words alone.
fn bare(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    determined(&chars).is_some() && {
        let item = item_of(&chars);
        !item.is_empty() && item.chars().all(char::is_alphabetic)
    }
}

/// A value as the words stated it: an option's key, a word, a pick's span; a flag has none.
fn stated(value: &Value) -> Option<String> {
    value.text().map(str::to_owned)
}

/// Where `needle` first occurs in `chars` at or after `from`, in characters.
fn index_of(chars: &[char], needle: &str, from: usize) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    if needle.is_empty() {
        return Some(from.min(chars.len()));
    }
    (from..chars.len().checked_sub(needle.len() - 1)?)
        .find(|&i| chars[i..i + needle.len()] == needle[..])
}
