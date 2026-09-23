//! The plan: read, decide, repair, bind, order, judge — over the answers a host has gathered, stopping at the
//! first it lacks. In: a `Plan`, the request, the tags a decision is narrowed by, the `Answers` so far. Out: the
//! `Weave`, or what is needed next.

use indexmap::IndexMap;

use super::reading::{self, Order, Ref, SURE, Segment, Split, Unclean};
use super::{
    Answers, Asked, Because, Binding, Need, Outcome, Planning, Repair, Step, Verdict, Via, Weave,
    field_names,
};
use crate::adapter::{Fault, Prob};
use crate::call::Value;
use crate::decide::{Cap, Decision};
use crate::manifest::{Effect, Kind, Recognizer, Source, Yield};
use crate::name::{ArgName, FieldName, LocalName, Tag};
use crate::plan::Plan;

/// A split the engine judged below this is never tried.
const LOW: f64 = 0.35;
/// A split the engine called two things with at least this probability is never merged back.
const FIRM: f64 = 0.8;
/// Under this the engine doubted the split: a part that decided as another reflex tries the fan-out first.
const DOUBT: f64 = 0.5;
/// The most split points one request is asked about; past it — a pasted list, a hostile line — the request is
/// one input, since each question carries both sides of the sentence and the request would grow as its square.
const MOST_SPLITS: usize = 24;
/// The plan of a request over the answers so far: the weave, or what is needed next; a fault when an answer does
/// not validate against its request.
pub fn plan(plan: &Plan, input: &str, tags: &[Tag], answers: &Answers) -> Result<Planning, Fault> {
    let planner = Planner {
        plan,
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
    tags: &'a [Tag],
    answers: &'a Answers,
    /// The request in the words' own order.
    request: String,
}

/// The plan as it takes shape: the segments with their decisions, the split points taken, what was left out, and
/// how a segment that matched nothing was settled.
struct Draft {
    chars: Vec<char>,
    segs: Vec<Segment>,
    decisions: Vec<Decision>,
    taken: Vec<Split>,
    excluded: Vec<String>,
    repaired: Vec<(String, Repair)>,
}

/// A fragment settled as another item of its neighbour's task.
struct Fan {
    decision: Decision,
    text: String,
    how: Repair,
}

impl Planner<'_> {
    /// The decision a host made for a text, when it has.
    fn decided(&self, asked: &Asked) -> Option<&Decision> {
        self.answers
            .decided
            .iter()
            .find(|(a, _)| a == asked)
            .map(|(_, decision)| decision)
    }

    /// A segment's decision over the reflexes the tags allow.
    fn segment(&self, text: &str) -> Asked {
        Asked {
            text: text.to_owned(),
            tags: self.tags.to_vec(),
            only: None,
        }
    }

    /// A text decided narrowed to one reflex.
    fn narrowed(text: &str, reflex: &LocalName) -> Asked {
        Asked {
            text: text.to_owned(),
            tags: Vec::new(),
            only: Some(reflex.clone()),
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
        // A segment that begins with a negation is left out: what the person said not to do is no step.
        let segments = reading::segments(&self.request, &taken);
        let mut draft = Draft {
            chars: self.request.chars().collect(),
            segs: segments
                .iter()
                .filter(|seg| !seg.excluded)
                .cloned()
                .collect(),
            decisions: Vec::new(),
            taken,
            excluded: segments
                .iter()
                .filter(|seg| seg.excluded)
                .map(|seg| seg.text.clone())
                .collect(),
            repaired: Vec::new(),
        };
        if draft.segs.is_empty() {
            return Ok(Ok(self.finish(
                judged,
                &draft.taken,
                Vec::new(),
                draft.excluded,
            )));
        }
        let phases = self
            .segments(&mut draft)
            .and_then(|()| self.lists(&mut draft, &judged))
            .and_then(|()| self.repair(&mut draft));
        if let Err(need) = phases {
            return Ok(Err(need));
        }
        let refs = match self.refer(&draft)? {
            Ok(refs) => refs,
            Err(need) => return Ok(Err(need)),
        };
        let steps: Vec<Step> = draft
            .segs
            .iter()
            .zip(draft.decisions)
            .zip(refs)
            .enumerate()
            .map(|(k, ((seg, decision), refs))| {
                let reflex = reflex_of(&decision).cloned();
                let effect = reflex
                    .as_ref()
                    .and_then(|reflex| self.plan.active().get(reflex))
                    .map(|active| active.effect);
                let repair = draft
                    .repaired
                    .iter()
                    .find(|(text, _)| *text == seg.text)
                    .map(|(_, how)| *how);
                Step {
                    n: k + 1,
                    text: seg.text.clone(),
                    end: seg.end,
                    decision,
                    reflex,
                    effect,
                    refs,
                    repair,
                    after: Vec::new(),
                }
            })
            .collect();
        Ok(Ok(self.finish(judged, &draft.taken, steps, draft.excluded)))
    }

    /// Every split point, judged: a candidate a negation follows is taken without asking; a request the engine
    /// cannot be asked about, a control character among its words, is one step.
    fn judge(&self) -> Result<Result<Vec<Split>, Need>, Fault> {
        let all = reading::splits(&self.request, true);
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
                None => Err(Need::Judge { request: judge }),
            },
        })
    }

    /// Every segment decided, side by side.
    fn segments(&self, draft: &mut Draft) -> Result<(), Need> {
        let wanted: Vec<Asked> = draft
            .segs
            .iter()
            .map(|seg| self.segment(&seg.text))
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
            let inner: Vec<Split> = reading::splits(&seg.text, true)
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
            let kept: Vec<&Segment> = parts.iter().filter(|part| !part.excluded).collect();
            if kept.len() < 2 {
                k += 1;
                continue;
            }
            // A second value the whole's decision already saw and no argument took — «10 minutes and 30
            // seconds» — is one task, not a list: the step confirms with its unused span at its turn, as the
            // foundation does.
            if let Decision::Confirm { because, .. } = &draft.decisions[k]
                && because.iter().any(|cap| {
                    matches!(cap, Cap::UnconsumedSpan { span }
                        if kept.iter().any(|part| span.start() >= part.start && span.end() <= part.end))
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
                    excluded: false,
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
                        excluded: false,
                    },
                    fan.decision,
                    Some(fan.how),
                ));
            }
            if items.is_empty() {
                k += 1;
                continue;
            }
            draft.excluded.extend(
                parts
                    .iter()
                    .filter(|part| part.excluded)
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
            draft.segs.splice(k..=k, segs);
            draft.decisions.splice(k..=k, decisions);
            for s in &inner {
                let start = base + s.start;
                draft.taken.push(Split {
                    start,
                    end: base + s.end,
                    word: s.word.clone(),
                    order: s.order,
                    p: judged.iter().find(|a| a.start == start).and_then(|a| a.p),
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
    /// fan-out. Failing that, a fragment of its neighbour's words is merged back and decided again, unless the
    /// engine was firm that the request asks for two things there, or the fragment carries a pronoun: then it is
    /// an action nothing matches, and the request is refused rather than half done. A part that decided on its
    /// own as another reflex, under a split the engine called one thing, is tried as a fan-out too and stays its
    /// own step when that fails: a bare item routes to the reflex whose example begins with it, weak evidence
    /// against the engine's doubt.
    fn repair(&self, draft: &mut Draft) -> Result<(), Need> {
        let mut k = 0;
        while k < draft.segs.len() && draft.segs.len() >= 2 {
            let sibling = if k > 0 { k - 1 } else { k + 1 };
            let split = split_before(&draft.taken, &draft.segs, k.max(1)).cloned();
            let sibling_reflex = reflex_of(&draft.decisions[sibling]).cloned();
            let doubted = split
                .as_ref()
                .is_some_and(|s| s.p.map_or(0.0, Prob::get) < DOUBT)
                && sibling_reflex.is_some();
            let abstains = matches!(draft.decisions[k], Decision::Abstain { .. });
            let other = reflex_of(&draft.decisions[k]) != sibling_reflex.as_ref();
            if !(abstains || doubted && other) {
                k += 1;
                continue;
            }
            // The fan-out first: for a fragment that matches nothing, and for a doubted part that decided as
            // another reflex — «the logo» as `render` beside «deadline for the flyer».
            if sibling_reflex.is_some()
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
            if !abstains || firm || reading::refers_back(&draft.segs[k].text) {
                k += 1;
                continue;
            }
            let (a, b) = if k > 0 { (k - 1, k) } else { (k, k + 1) };
            let merged = Segment {
                text: draft.chars[draft.segs[a].start..draft.segs[b].end.min(draft.chars.len())]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_owned(),
                start: draft.segs[a].start,
                end: draft.segs[b].end,
                excluded: false,
            };
            let decision = self.decide(self.segment(&merged.text))?;
            if matches!(decision, Decision::Abstain { .. }) {
                k += 1;
                continue;
            }
            draft.repaired.push((merged.text.clone(), Repair::Merged));
            draft.segs.splice(a..=b, [merged]);
            draft.decisions.splice(a..=b, [decision]);
            if let Some(split) = split {
                draft.taken.retain(|s| *s != split);
            }
            k = 0;
        }
        Ok(())
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
                if k == 0 {
                    Vec::new()
                } else {
                    reading::refs_by_code(&draft.segs, k, &fields)
                }
            })
            .collect();
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
        let narrowed = self.decide(Self::narrowed(fragment, reflex))?;
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
        let determiner = ["the", "a", "an"].into_iter().find_map(|det| {
            let head: String = fragment_chars
                .iter()
                .take(det.len())
                .collect::<String>()
                .to_lowercase();
            let mut end = det.len();
            while end < fragment_chars.len() && fragment_chars[end].is_whitespace() {
                end += 1;
            }
            (head == det && end > det.len()).then_some(end)
        });
        for (name, value) in args {
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
            let decision = self.decide(Self::narrowed(&spliced, reflex))?;
            if matches!(decision, Decision::Abstain { .. }) || reflex_of(&decision) != Some(reflex)
            {
                continue;
            }
            // The new value must be the fragment's own words: a spliced sentence that reads as nonsense still
            // decides.
            let after = args_of(&decision)
                .and_then(|args| args.get(name))
                .and_then(stated);
            if let Some(after) = after
                && Some(&after) != stated(value).as_ref()
                && fragment_lowered.contains(&after.to_lowercase())
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

    /// Bindings, edges, stages and the verdict over decided steps.
    fn finish(
        &self,
        splits: Vec<Split>,
        taken: &[Split],
        mut steps: Vec<Step>,
        excluded: Vec<String>,
    ) -> Weave {
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
        } = self.bind(&steps, &mut after);
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
        if verdict.outcome != Outcome::Refuse && !asks.is_empty() {
            verdict.outcome = Outcome::Ask;
        }
        verdict.because.extend(asks);
        if verdict.outcome == Outcome::Run && !because.is_empty() {
            verdict.outcome = Outcome::Confirm;
        }
        verdict.because.extend(because);
        Weave {
            input: self.request.clone(),
            splits,
            steps,
            excluded,
            binds,
            exclusive,
            stages,
            verdict,
        }
    }

    /// Every reference bound to what a source yields, by kind, a noun naming the field; never guessed: several
    /// fields, or one record of several, are the layer's own questions. A pronoun or a demonstrative orders the
    /// steps whether or not anything binds; `the <noun>` that nothing takes is plain words.
    fn bind(&self, steps: &[Step], after: &mut [Vec<usize>]) -> Bound {
        let mut out = Bound {
            binds: Vec::new(),
            asks: Vec::new(),
            because: Vec::new(),
        };
        for step in steps {
            if step.reflex.is_none() {
                continue;
            }
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
                .filter(|c| reading::stem_of(c.field.as_str()) == noun)
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
            kind: c.kind,
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

/// What binding found: the bindings, the layer's own questions, and the references nothing takes.
struct Bound {
    binds: Vec<Binding>,
    asks: Vec<Because>,
    because: Vec<Because>,
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

/// Layers by longest path; a layer of reads runs together, any other one step at a time in the words' order.
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
    let mut memo = vec![None; steps.len()];
    let mut layers: Vec<Vec<usize>> = Vec::new();
    for step in steps {
        let d = depth(steps, step.n, &mut memo);
        if layers.len() <= d {
            layers.resize(d + 1, Vec::new());
        }
        layers[d].push(step.n);
    }
    let mut stages = Vec::new();
    for layer in layers.into_iter().filter(|layer| !layer.is_empty()) {
        let reads = layer
            .iter()
            .all(|n| steps[n - 1].effect == Some(Effect::Read));
        if !exclusive && reads {
            stages.push(layer);
        } else {
            stages.extend(layer.into_iter().map(|n| vec![n]));
        }
    }
    stages
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

/// The reflex a decision is about, if any.
pub(crate) fn reflex_of(decision: &Decision) -> Option<&LocalName> {
    match decision {
        Decision::Abstain { .. } => None,
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(&chosen.call.reflex),
        Decision::Ask { asking, .. } => Some(&asking.reflex),
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
