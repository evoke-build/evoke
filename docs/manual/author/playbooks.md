<!-- description: A playbook is a reflex whose body is a plan of sentences with slots: how a short sentence reaches it, how each step is decided, what one yes covers. -->
# Playbooks

A playbook is a reflex whose body is a plan: `steps`, one sentence per step in the order they happen, and no
`run`. A person reaches it with a short sentence, routed by your `description`, `not_for` and examples like any
request. Each step is then decided over what that person has installed, as a typed sentence would be, and the
whole plan prints before anything runs.

```toml
reflex = 1

description = """
Handle a service outage.
Looks at the service's errors, deploys and logs, finds the release behind it, rolls it back, tells the incident channel and updates the status page."""
not_for = ["one lookup on its own", "an incident's postmortem", "a service that is healthy"]
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
ask   = "Which service?"
vocab = "services"

[args.region]
ask      = "In which region?"
vocab    = "regions"
optional = true

[examples]
"payments is down in us-east" = {}
"search is broken"            = {}

[tests]
"checkout is failing in eu-west" = {}
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
instead of `run`, and none of a body's keys, `[needs]`, `[config]`, `[yields]`, `returns` or `platforms`. A
plan holds at most 24 steps. Its `effect` is your claim about the whole; `add` says when a step reaches a
tighter one.

## Slots

A slot, `{service}`, is an argument of `[args]`: a vocabulary word, an option or a pick, never a flag. It is
filled from the sentence, or asked before the plan prints, and written into the step as the person typed it, an
option as its key. A word beyond the slots fills nothing. A value is never the playbook's own: a word the person
lacks joins their vocabulary with `[+] add one`. Words in square brackets go only with the slot inside them:
`[ in {region}]` is written when a region was stated and dropped when it was not. An optional argument stands
only in a bracket; a required one never does.

```text
$ evoke "we have an outage"
  1  outage · asks service
  Which service?  [1] checkout  [2] payments  [3] search  [+] add one  > 2
  1  errors service="payments"  0.90 · outage 1
  …
```

## Every step is decided when the plan is made

A step is decided by the classifier over what the person has installed, from its words alone, as a typed
sentence would be. So name no reflex: describe what should happen, one action per step. A word another team
would change, a channel or an address, is better described than written: `post to the incident channel` reaches
whatever channel their vocabulary holds for it, and `lint` flags a step that states one. A step may reach
another playbook, once. A step that reaches the playbook it stands in is refused, so word a step apart from your
own examples. A step no reflex matches refuses the whole plan before anything runs:

```text
  4  "find the release behind it" · no reflex
[2]
```

`evoke add` shows what each step reaches on the set it joins, and `evoke test` fails a step that reaches
nothing ([Examples and tests](records.md)).

## One yes over the whole plan

Nothing runs before the whole plan is printed and one yes is given over it: your `confirm` when the plan is your
playbook's alone, `Run the plan as it stands?` when the sentence holds more. A step's own confirm, a destructive
one's above all, is asked again at its turn, every time. `[t]each` at the whole-plan question teaches the route
to your playbook; at a step's turn it teaches the step's reflex from the filled sentence.

## What the person adds to the sentence

A part of the sentence that repeats a step of the plan folds into that step, and the plan says so:
`folded "show me the error rate" into 1`. A part that says what not to do beside a plan is refused, and so is a
sentence that opens with a condition, `if`, `unless` or `in case`: `evoke` judges no condition, so it asks for the
check first and the plan after.

## What is contract

`steps` is contract: an overlay may reword the description, the confirm and the asks, never a step. A step
added, removed, reworded or reordered is a major version, and `evoke check` says so: `steps  the steps moved`.
A slot added or renamed rewords a step, so it is major too.

## `add`, `test` and `show`

`add` decides every step of a newcomer playbook over the set it joins, runs nothing, and prints what each
reaches under its row; a step that reaches nothing installs and is reported, since the plan refuses when it is
made. `test` decides the playbook's records as any reflex's, then its steps, each filled from the first record
whose reading fills every slot it holds; a step no record fills is `untested`. `show` prints the steps after
`confirm`, numbered.

```text
$ evoke test outage
  outage  4 passed · 7 steps route
```

## In the SDK

`playbook(manifest)` hands a playbook as code, `steps` in its manifest and no body; `project.steps(input, {
ask })` answers a slot the sentence lacks before the plan is made ([Decisions](../sdk/decisions.md)). A playbook
is never called by name: `run`, `handle` and `evoke run <name>` refuse it with `say it in a sentence`.

**Next:** [Examples and tests](records.md).
