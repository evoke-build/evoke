<!-- description: A playbook is a reflex whose body is a plan of sentences with slots: how a short sentence reaches it, how each step is decided, what one yes covers. -->
# Playbooks

A playbook is a reflex whose body is a plan: `steps`, one sentence per step, and no `run`. A person reaches it
with a short sentence that states the situation they face: «checkout is failing in eu-west». That sentence is
routed by your `description`, `not_for` and examples, like any request. Each step is then decided over what that
person has installed, as a typed sentence would be, and the whole plan prints before anything runs. The
playbook's summary and examples state the situation, and its `not_for` names the steps typed alone:
[rules 5 to 7](rules.md#2-when-to-write-a-playbook).

```toml
# outage/reflex.toml
reflex = 1

description = """
Handle a service that is down or failing.
Checks its errors, deploys and logs, finds the release behind the failure, rolls it back, tells the incident channel and updates the status page; the rollback asks again before it runs."""
not_for = [
  "a service's error rate on its own",
  "listing a service's deploys on its own",
  "pulling a service's logs on its own",
  "rolling back one release on its own",
  "posting to a channel or updating the status page on its own",
  "writing up an incident's postmortem",
  "asking whether a service is healthy",
  "a slow service that still answers",
]
tags = ["outage"]
effect = "destructive"
confirm = "Run the outage plan for {service}?"
steps = [
  "check {service}'s errors[ in {region}]",
  "list {service}'s deploys[ in {region}]",
  "search {service}'s logs[ in {region}]",
  "find the release behind it",
  "roll {service} back from that release[ in {region}]",
  "post to the incident channel",
  "update the status page for {service}",
]

[args.service]
ask   = "Which service is down?"
vocab = "services"

[args.region]
ask      = "In which region?"
vocab    = "regions"
optional = true

[examples]
"payments is down in us-east"    = {}
"search is broken"               = {}
"we have an outage on checkout"  = {}
"checkout is on fire in eu-west" = {}
"payments stopped working"       = {}

[tests]
"checkout is failing in eu-west"               = {}
"we lost search in us-east"                    = {}
"how bad are the errors on payments right now" = false
"downgrade search from 6.0.1"                  = false
"ping #ops that checkout recovered"            = false
"flip the status page to resolved for search"  = false
"prepare a retro on the checkout outage"       = false
"is the search service doing fine"             = false
"checkout feels sluggish this morning"         = false
```

```text
$ evoke "checkout is failing in eu-west"
  1  errors service="checkout" region="eu-west"  0.90 · outage 1
  2  deploys service="checkout" region="eu-west"  0.90 · outage 2
  3  logs service="checkout" region="eu-west"  0.90 · outage 3
  4  suspect  0.90 · takes errors from 1, deploys from 2, logs from 3 · outage 4
  5  rollback service="checkout" region="eu-west" · takes release from 4 · outage 5
  6  post channel="#incident"  0.90 · outage 6
  7  status component="checkout"  0.90 · outage 7 · service as component
  outage service="checkout" region="eu-west" · destructive · weakest: route 0.90 · also errors (fits 0.60), step 1
  Run the outage plan for checkout?  [y]es [n]o [t]each > y
```

Every line of the plan ends with the playbook and the step that wrote it. Then comes the sentence's own line,
the decision that picked the playbook, and one question over the whole plan. A step that takes a result waits
for the steps it takes from, as in any sentence of several steps ([Weaving](../use/weaving.md)).

## A plan of sentences

A playbook is fetched, locked, tested, overlaid and shown like any reflex. What differs is the body: `steps`
instead of `run`, and none of a body's keys, `[needs]`, `[config]`, `[yields]`, `returns`, `platforms` or an
argument's `takes`. `evoke check` refuses each of them. A plan holds at most 24 steps. It lists the situation's
steps in the order they happen: [rule 8](rules.md#2-when-to-write-a-playbook). Its `effect` is your claim about
the whole; `add` says when a step reaches a tighter one. The claim is the worst effect its steps reach:
[rule 10](rules.md#2-when-to-write-a-playbook).

`evoke new --playbook <name>` writes one to start from: `<name>/reflex.toml` alone, a plan of three steps from the
template, which checks clean.

## Slots

A slot, `{service}`, is an argument of `[args]`: a vocabulary word, an option or a pick, never a flag. It is
filled from the sentence, or asked before the plan prints, and written into the step as the person typed it, an
option as its key. A word beyond the slots fills nothing. A value is never the playbook's own: a word the person
lacks joins their vocabulary with `[+] add one`.

```text
$ evoke "we have an outage"
  1  outage · asks service
  Which service is down?  [1] checkout  [2] payments  [3] search  [+] add one  > 2
  1  errors service="payments"  0.90 · outage 1
  …
```

Words in square brackets go only with the slot inside them: `[ in {region}]` is written when a region was stated
and dropped when it was not. An optional argument stands only in a bracket, and a required one never does:
`evoke check` refuses a slot in the wrong place. It also refuses a slot that is a flag or no argument at all, and
an argument that no step names. A slot goes in every step whose reflex reads its value:
[rules 25 and 26](rules.md#5-how-to-word-a-step).

## Every step is decided when the plan is made

A step is decided by the classifier over what the person has installed, from its words alone, as a typed
sentence would be. So each step is one action, worded as a person would type it:
[rules 24 to 29](rules.md#5-how-to-word-a-step). `evoke check` refuses a step that reads as a call with a value,
`errors service={service}`. Lint reports a step that holds a connective or a comma, states a channel or an
address, or opens with `check that`. It also reports a step whose words are all the summary's or an example's.
[Diagnostics](../reference/diagnostics.md#what-lint-reports) lists each line.

A step may reach another playbook, once: a plan inside a plan inside a plan is refused. A step that reaches the
playbook it stands in is refused too. A step no reflex matches refuses the whole plan before anything runs:

```text
  7  "update the status page for checkout" · no reflex · outage 7
[2]
```

`evoke add` shows what each step reaches on the set it joins, and `evoke test` fails a step that reaches
nothing ([Examples and tests](records.md)).

## One yes over the whole plan

Nothing runs before the whole plan is printed and one yes is given over it: your `confirm` when the plan is your
playbook's alone, `Run the plan as it stands?` when the sentence holds more. A step's own confirm, a destructive
one's above all, is asked again at its turn, every time. `[t]each` at the whole-plan question teaches the route
to your playbook; at a step's turn it teaches the step's reflex from the filled sentence.

## A result picks among the steps you listed

A step may run only under a value an earlier step yields. Write it as a table, `say` the sentence and `when`
one field with the value it runs under, as text, and list the alternatives together right after the step
whose result picks among them:

```toml
steps = [
  "drain connections off the primary",
  "fail over to the replica",
  "check whether writes land on the new primary",
  { say = "tell the ops channel the failover is done", when = { landing = "yes" } },
  { say = "fail back to the old primary",              when = { landing = "no" } },
]
```

A result never writes a step: it picks among the steps you listed, and the whole list prints before anything
runs. The field is one that step's reflex declares under `[yields]` as one value, never a list. The plan refuses
before anything runs when the step reaches a reflex that yields no such field, or the value is one the field
cannot hold. `evoke check` refuses a `when` on step 1, where no step comes before it, and a `when` on two fields.

The step's line ends `then 4 on "yes", 5 on "no"`, and each alternative's `if 3 yields landing "yes"`. At that
step's turn the result is compared with each value as text. The step it picks runs, the others are skipped clean
and say so at the end, and a value no step lists picks nothing, the plan going on. A step without `when` runs
either way. A step that may not run is one step: a later step cannot take its result, and its words may not
reach another playbook. A check whose result picks is a reflex of its own that yields one field:
[rule 9](rules.md#2-when-to-write-a-playbook).

```text
$ evoke "drain the primary"
  1  drain  0.90 · runbook 1
  2  failover · destructive · weakest: route 0.90 · runbook 2
  3  verify  0.90 · runbook 3 · then 4 on "yes", 5 on "no"
  4  tell channel="#ops"  0.90 · runbook 4 · if 3 yields landing "yes"
  5  failback · destructive · weakest: route 0.90 · runbook 5 · if 3 yields landing "no"
  runbook · destructive · weakest: route 0.90 · also drain (fits 0.60), step 1
  Run the failover runbook?  [y]es [n]o [t]each > y
primary drained
  2  failover · destructive · weakest: route 0.90
  Promote the replica now?  [y]es [n]o [t]each > y
replica promoted
writes landing on the new primary
told #ops
  5  failback · destructive · weakest: route 0.90 · runbook 5 · if 3 yields landing "no" · skipped · not chosen: step 3 yielded landing "yes"
```

## What the person adds to the sentence

A part of the sentence that repeats a step of the plan folds into that step, and the plan says so:
`folded "show me the error rate" into 1`. A part that states the situation a second time, «payments keep timing
out, customers can't pay», folds into the plan the first part opened, so the plan holds each step once. Its
`folded` line names that plan's first step: `folded "customers can't pay" into 1`. A part that says what not to
do beside a plan is refused, and so is a part that opens with a condition, `if`, `unless` or `in case`. `evoke`
judges no condition, so the line it prints says to ask for the check first, then what to do.

## What is contract

`steps` is contract: an overlay may reword the description, the confirm and the asks, never a step. A step
added, removed, reworded or reordered is a major version, and `evoke check` says so: `steps  the steps moved`.
A slot added or renamed rewords a step, so it is major too, and so is a `when` added, moved or changed.

## `add`, `test` and `show`

`add` decides every step of a newcomer playbook over the set it joins, runs nothing, and prints what each
reaches under its row; a step that reaches nothing installs and is reported, since the plan refuses when it is
made. Two lines more may follow. `claims write; its steps reach destructive` says a step reaches a tighter effect
than the playbook claims. `step 3 reaches logs, which lacks the tag outage` says a request narrowed with
`--tag outage` would leave that step without its reflex. Every reflex the steps reach carries the playbook's
tag: [rule 40](rules.md#8-how-to-design-a-collection). `add` also prints `also fits` for the examples of the
reflexes the steps reach, since the plan does hold their action. Typed alone, such a sentence still reaches its
own reflex, and stops at confirm: [Exceptions](rules.md#exceptions).

`test` decides the playbook's records as any reflex's, then its steps, each filled from the first record whose
reading fills every slot it holds; a step no record fills is `untested`. Both say when the step before a branch
reaches a reflex that yields no field the branch waits on, and `test` reports a claim the steps pass too. `show`
prints the steps after `confirm`, numbered, a step that may not run ending in its `when`.

```text
$ evoke test outage
  outage  14 passed · 7 steps route
```

## In the SDK

`playbook(manifest)` hands a playbook as code, `steps` in its manifest and no body; `project.steps(input, {
ask })` answers a slot the sentence lacks before the plan is made ([Decisions](../sdk/decisions.md)). A playbook
is never called by name: `run`, `handle` and `evoke run <name>` refuse it with `say it in a sentence`.

**Next:** [Examples and tests](records.md).
