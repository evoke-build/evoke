<!-- description: One sentence can ask for several reflexes: the plan shown before anything runs, results threaded between steps, a plan saved as a file for someone else. -->
# Weaving

`evoke` reads every sentence for its steps. One step is decided as it always was, unless its reflex takes an
earlier step's result: then the request stops before anything runs. More than one is a *weave*: each part
decided on its own, in the order the words give, a result of one step threaded into a later one, and the plan
shown before anything runs.

```text
$ evoke "kill the lights in the den and start a 10 minute timer"
  1  lights room="den" state="off"  0.92
  2  timer duration="10 minute"  0.95
den lights off
10 minute timer started
```

## How a sentence is read

- A connective can separate two things: `and`, `then`, `but`, a comma, `after`, `before`. At each one the
  classifier is asked whether it does. "kill the lights in the den and start a 10 minute timer" is two things.
  "set a timer for 10 minutes and 30 seconds" is one. A sentence with more than two dozen such points, a pasted
  list, is decided as one input.
- With the connectives, the classifier is asked how many things the sentence asks for. Where it says one, the
  sentence is one step, and no connective cuts it: "check the errors for checkout, and only in eu-west".
- Each part is decided as one input is: routed, gated, its arguments read. A part that matches nothing on its own
  is tried as another item of its neighbour's task first: "check stock for widgets and gadgets" is two stock
  checks. So is a part that is only a determiner and one word, "the office" in "kill the lights in the den and
  the office", whatever it would mean on its own: it stays a step of its own only when it fits no item of its
  neighbour's task. A part read as an item must be the value it stands in for, "gadgets" for "widgets", never a
  longer phrase that holds one. When the classifier was sure the two parts were separate things and one still
  matches nothing, the whole request is refused rather than half done. When it was not sure, the part is read
  with its neighbour as one request, and `evoke why` says what its words do.
- A part that begins with `not`, `don't`, `never` or `without` is left out, however the apostrophe is typed. What
  you ask evoke not to do is no step. One step beside such a part is decided as one input. A sentence that is only
  such parts is nothing to do, and one line says so. A part that also names a value the step beside it lacks is
  read with that step: in "pull the payments logs, not in us-east, in eu-west" the logs are pulled in eu-west.
  And where the classifier says the sentence asks for one thing, nothing is left out: the sentence is read
  whole, what it rules out with what it asks.
- `then`, `after that` and `next` order the steps. `after you X, Y` and `Y after you X` read as `X, then Y`.
  `before you X, Y` and `Y before you X` read as `Y, then X`. When a write is among the steps, every step runs
  alone, in the order you wrote them. A plan of reads may run them side by side.
- A word the sentence says once for several steps reaches each of them. "check checkout's errors, deploys and
  logs in eu-west" names the service and the region once: every lookup takes both. A step that lacks a required
  word takes the one word of its kind the sentence names, as an answer would give it. A step that could take an
  optional word takes the one said once in its part of the sentence, before any `then`, written into its words
  and decided again under the same gate. A step whose own words name a word of that kind keeps its own, and a
  sentence that names two, "checkout" and "payments", carries neither. Only a word from a vocabulary is carried:
  never a number, a date, an address, a quoted value or an option. And only a word no one reflex owns: a vocabulary one
  reflex alone asks for, the folder `open` opens, is that reflex's own and never reaches another step. The plan
  shows the word on each step it reached, and `evoke why` says which words were shared.
- A value one part states can be another part's where the classifier says so. It is asked only for a part whose
  words point at something, "it", "there", "the same", or for a word from a vocabulary that several reflexes
  ask for: in "check the errors for checkout in eu-west, then pull its logs there", the logs are pulled in
  eu-west. An option or a flag of a call reaches a part that asks for the same again, "the same", "likewise",
  "as before": in "take a screenshot of the window to the clipboard, then the same for the whole screen", both
  go to the clipboard. `evoke why` says where the value came from.
- Two parts that read as the same call, no value of one differing from the other's, are one step: "check the
  errors for checkout, and the errors in eu-west" is one check, of checkout in eu-west. Where a part could repeat
  either of two calls, its own words pick one, "the second one", "the last", or it stays a step of its own. The
  plan names the part: `folded "the errors in eu-west" into 1`.
- A part that matches no reflex and asks for nothing is set aside, and the plan runs without it: `set aside
  "thanks a lot for this"`. Where the classifier says the part may add a detail to the step beside it, that step
  confirms before it runs, its line ending in `without "the ones since noon"`, and the plan names the part: `not in
  the plan "the ones since noon"`. A part that asks for something and matches nothing still refuses the whole
  request.
- Two items the classifier read as one thing, "invoices, card expenses", are two steps when each is a reflex of
  its own: the joint is split, and a word the sentence says once reaches both. A joint it read firmly as one
  task stays one.
- A sentence reads most surely when it asks for a few steps. The plan is settled before anything runs, so the
  longer a sentence grows, the more often one of its steps stops to ask.

## What a step takes from another

A body may declare what its result holds, under `[yields]` in its manifest ([The manifest](../author/manifest.md)).
A later step may refer to it with `it`, `them`, `that report` or `the address`. It takes a field of the right kind
into the argument it lacks:

```text
$ evoke "look up dana's address and email them"
  1  contact name="dana"  0.95
  2  mail · takes email from 1
dana <dana@example.com>
  2  mail to="dana@example.com"  0.94
drafted to dana@example.com
```

`mail` needed an address the words did not give. `contact` yields one, and `them` names it. A required argument
is filled with the value. An optional one is decided again with the value written into the words, so the
classifier assigns it, under the same gate as any words. A date a step yields reaches the next as a day,
`2026-05-06`, never as `tomorrow`: a relative day is nothing to take. Nothing is guessed: a reference that
several fields could satisfy, or one record of a list, stops with a line naming them. Within one sentence a step refers to another; across sentences, in one
session, a pick with `recent` recalls what earlier bodies returned, at its ask
([Arguments](../author/arguments.md#a-value-recalled)). A step that refers to another whose result it takes
nothing from asks before anything runs:

```text
$ evoke "look up dana's address and start a 10 minute timer for them"
  1  contact name="dana"  0.95
  2  timer duration="10 minute"  0.94 · after 1
  step 2 refers to step 1, but takes nothing from it
  Run the plan as it stands?  [y]es [n]o > y
dana <dana@example.com>
10 minute timer started
```

## A step that takes whole results

A step may take earlier steps' results whole. Its manifest names what it takes with `takes`; each earlier reflex
names what it returns with `returns` ([The manifest](../author/manifest.md#a-result-another-step-takes-whole)).
The plan binds them by that name before anything runs, whatever the words refer to, and the step waits for the
steps it takes from:

```text
$ evoke "check the errors for checkout, list the checkout deploys and pull the checkout logs, then find the suspect"
  1  errors service="checkout"  0.95
  2  deploys service="checkout"  0.95 · with 1
  3  logs service="checkout"  0.95 · with 1, 2
  4  suspect  0.95 · takes errors from 1, deploys from 2, logs from 3
checkout: 8.4% errors since 14:02
checkout: 1 deploy today, 4.12.0 at 13:58
checkout: 412 timeouts calling payments
4.12.0 at 13:58, four minutes before 8.4% errors and 412 timeouts
```

Steps 1 to 3 run side by side; step 4 receives their three results as its arguments, beside its decision and
never in its words, an empty list as a list. A result over 1 MiB is not handed: the step is skipped, and the
weave fails as it does when a source yields nothing. When no step before it returns what a step takes, or two
steps do, the request stops before anything runs, and says which; so does a field taken from a step that runs
once per record:

```text
$ evoke "check the errors for checkout, then find the suspect"
  1  errors service="checkout"  0.95
  2  suspect  0.95 · takes errors from 1
  step 2 takes deploys, which no step before it returns  →  evoke show deploys
  step 2 takes logs, which no step before it returns  →  evoke show logs
[2]
```

## A plan a playbook wrote

A short sentence can reach a playbook, a reflex whose body is a plan of sentences
([Playbooks](../author/playbooks.md)). Its steps stand in the plan as if you had typed them, each decided over
what you have installed, and every line ends with the playbook and the step that wrote it. Under the plan comes
the sentence's own line, then one question over the whole. Your yes covers every step, and a step's own confirm
is asked again at its turn.

```text
$ evoke "checkout is failing in eu-west"
  1  errors service="checkout" region="eu-west"  0.95 · outage 1
  2  deploys service="checkout" region="eu-west"  0.95 · outage 2
  3  logs service="checkout" region="eu-west"  0.95 · outage 3
  4  suspect  0.95 · takes errors from 1, deploys from 2, logs from 3 · outage 4
  5  rollback service="checkout" region="eu-west" · takes release from 4 · outage 5
  6  post channel="#incident"  0.95 · outage 6
  7  status component="checkout"  0.95 · outage 7 · service as component
  outage service="checkout" region="eu-west" · destructive · weakest: route 0.95
  Run the outage plan for checkout?  [y]es [n]o [t]each > y
```

A slot the sentence does not state is asked before the plan prints, unless a part of the sentence states it for
a step the playbook writes: in "we have an outage, check the errors for payments", the service is payments. A
part of your sentence that repeats a step of the plan folds into that step, and the plan says so: `folded "show
me the error rate" into 1`. A part that
says what not to do beside such a plan is refused, since the step it may have meant is already there. A sentence
that opens with a condition, `if`, `unless` or `in case`, is refused whatever follows: `evoke` judges no
condition, so ask for the check first, then type what to do. A step of the plan that matches nothing here, or
that reaches a plan inside a plan inside a plan, refuses the whole plan, and the line names the step. When the
sentence holds more than the playbook, the question is `Run the plan as it stands?`.

A playbook may list steps that run only under a value an earlier step yields, a branch. The plan prints every
one before anything runs: the line of the step whose result picks ends `then 4 on "yes", 5 on "no"`, and each
alternative's `if 3 yields landing "yes"`. At that step's turn the value picks. The step it picks runs, the
others are skipped clean and say so at the end, `skipped · not chosen: step 3 yielded landing "yes"`, and a
value no step lists picks nothing, `which no step lists`, the plan going on ([Playbooks](../author/playbooks.md)).

## Before anything runs

The plan is settled first. A step whose required argument no other step provides is asked for it up front, as
one input would be at its turn. Then the plan is shown, numbered, one line per step:

```text
$ evoke "kill the lights and start a 10 minute timer"
  1  lights state="off" · asks room
  Which room?  [1] den  [2] office  [+] add one  > 1
  1  lights room="den" state="off" · write · weakest: state 0.74
  2  timer duration="10 minute"  0.95
  1  lights room="den" state="off" · write · weakest: state 0.74
  Set the den lights off?  [y]es [n]o [t]each > y
den lights off
10 minute timer started
```

An answer out of range is asked again with the reason, as one input's would be. Each line of the plan is what
`evoke` would say of that step alone: the call and its confidence, or the confirm's own line with its weakest
judgment. After it: `asks <arg>` for what the step still needs, `takes <name> from <n>` where a result threads
in, the name a field's or a whole result's, `after <n>` where the words ordered it, `with <n>` where the step
runs beside earlier ones, `<playbook> <n>` where a playbook wrote the step, `then <n> on "<value>"` where the
step's result picks among later steps, `if <n> yields <field> "<value>"` where the step runs only under that
value, `no reflex` where nothing matched.
Under the plan, `folded "…" into <n>` names a part that repeats another step, `set aside "…"` a part that asks
for nothing, `not in the plan "…"` a part that may add a detail to a step, and `left out "…"` a part that said
what not to do.

## At each step's turn

Each step then goes through the same gate as one input: a run runs, a confirm prompts, an ask asks. A step that
differs from its plan line, a bound value now in place, prints again. In the SDK a step's confirm can wait for
someone else: the decision travels as plain data to whoever answers it, and the weave goes on when they do
([Decisions](../sdk/decisions.md#stepsinput--tags-signal--and-weaveinput--confirm-ask-proceed-tags-signal-)).
A failure, a decline or a refusal ends the weave after its stage. The steps after it are skipped, and say so:

```text
$ evoke "start a 25 minute timer and kill the lights in the den"
  1  timer duration="25 minute" · write · weakest: duration 0.85
  2  lights room="den" state="off"  0.92
  1  timer duration="25 minute" · write · weakest: duration 0.85
  Start a 25 minute timer?  [y]es [n]o [t]each > n
  2  lights room="den" state="off"  0.92 · skipped
[2]
```

A step decided again with a bound value in its words may match no reflex then. It is refused, and says so:

```text
  2  "copy sam@example.com on a note to dana@example.com" · no reflex
```

A part that matches nothing refuses the whole request before anything runs:

```text
$ evoke "kill the lights in the den and feed the cat"
  1  lights room="den" state="off"  0.92
  2  "feed the cat" · no reflex
[2]
```

The exit code is the worst step's. `0` every step ran. `1` a body failed, or a step yielded nothing the next
could take, or more than it can be handed. `2` a step was declined, a part matched nothing, or a step takes a
result no step before it returns, or two do. `3` a step needed a terminal, or a question only you can answer.

## Stopping it

`Ctrl-C` while a step runs stops the weave. The body running at that moment is told to stop and ended with
everything it started: a JavaScript body sees its `signal` abort and has a second to finish. That step and
every step after it read `skipped · cancelled`, on the terminal and in the log, so `why` and `teach` still see
the whole plan:

```text
$ evoke "wait a while and start a 10 minute timer"
  1  wait  0.95
  2  timer duration="10 minute"  0.95
waiting
^C
  1  wait  0.95 · skipped · cancelled
  2  timer duration="10 minute"  0.95 · skipped · cancelled
```

Then `evoke` ends as an interrupted program does, and the shell reports exit 130. Under `--json` every step's
line prints first, its `why` `{ "type": "cancelled" }`.

## `try` and `--json`

`evoke try` shows the plan, then every step's judgments under its number, the sentence's own block first as
step 0 when a playbook wrote the plan. `evoke try --json` prints the plan whole, on one line, with every adapter
call it took. `evoke --json` prints one line per step as it runs: the line of one decision, with `step` and
`steps` first, `bound` where a value came from another step, `from` where a playbook wrote it, and `status` at
the end, with `why` when the step stopped; the sentence's own line prints first, as step 0, with no status
([The JSON line](../reference/json.md)). A plan stopped before any
step ran, refused, its question or its prompt declined, or with no terminal to ask, prints every step's line at
once, with what stopped it.

## Saving a plan, and running it for someone else

`evoke try --save <file> "<sentence>"` writes the plan as a file, and runs nothing. The file holds the sentence as
you typed it, the plan as `try --json` prints it, and every answer the classifier gave, in clear; and the pins the
plan was decided under: the `evoke` version, the adapter and its gate, the installed set's digest, each reflex and
vocabulary by its own hash, a remote reflex by the lock's `h1`. It carries no time and no name. A plan that asks
cannot be saved: a file holds no one's answers, so the sentence must say the value, `kill the lights in the den`
rather than `kill the lights`. The name `<name>.plan.json` is recommended, not required.

```text
$ evoke try --save ~/month.plan.json "pull september's bank transactions, invoices, card expenses and payroll, reconcile them, post the closing entries to the ledger, then send the report to cfo@example.com"
  1  bank month="september"  0.95
  2  invoices month="september"  0.95
  3  cards month="september"  0.95
  4  payroll month="september"  0.95
  5  reconcile  0.95 · takes transactions from 1, invoices from 2, expenses from 3, payroll from 4
  6  ledger month="september" · write · weakest: route 0.89
  7  send to="cfo@example.com"  0.95 · after 1, 2, 3, 4, 5, 6
+ ~/month.plan.json
```

`evoke run <file>` runs it, on your machine or another with the same project: the same reflexes at the same
content, the same words, the same adapter and gate. `run` tells a file from a call by its first word, which is no
reflex name. The plan is made again from the file's answers alone, so the classifier is asked for nothing, and a
plan that does not read the same as the file is refused. The plan prints, one yes over the whole plan is asked at
your terminal, whatever its verdict, then each step goes through the gate at its turn as any sentence's does: a
confirm asks you, a value bound at run time is decided again by your adapter under the same gate. Every step's line
in your log names the file, and `evoke why` shows it as `from ~/month.plan.json`.

```text
$ evoke run ~/month.plan.json
  1  bank month="september"  0.95
  …
  7  send to="cfo@example.com"  0.95 · after 1, 2, 3, 4, 5, 6
  the plan of ~/month.plan.json
  Run the plan as it stands?  [y]es [n]o > y
september: 214 transactions, 18 204.55 at the close
…
  6  ledger month="september" · write · weakest: route 0.89
  Post september to the ledger?  [y]es [n]o [t]each > y
september posted to the ledger
sent to cfo@example.com
```

A pin that moved refuses the file with one line naming it, exit 3, and the command that mends it: a plan written
by another `evoke`, decided by another engine or under another gate, a reflex not installed here or inactive here,
a remote reflex whose body moved, a reflex or a vocabulary whose wording moved, a reflex installed here the plan
never saw, or the project's files differing at all. A file edited by hand does not read the same as its answers,
and is refused too. The fix is to make the plan again here and read it again:

```text
$ evoke run ~/month.plan.json
  months differs here from the plan's  →  evoke try --save ~/month.plan.json "pull september's …"
[3]
```

In the SDK, `steps` returns the same object and `weave` takes it ([Decisions](../sdk/decisions.md#stepsinput--tags-signal--and-weaveinput--confirm-ask-proceed-tags-signal-)).

## Afterwards

Every step is logged under its number. `evoke why` shows each step of the last sentence with what became of it,
the sentence's own block first, as step 0, when a playbook wrote the plan:
`ran`, `failed`, `declined`, `refused`, `skipped` or `unanswered`, and why, `cancelled` among the reasons, and the
words the sentence shared into it, `shared service = checkout`. `evoke teach <call>` with no
utterance takes the step the lesson's reflex decided; when none or several did, it names the steps and asks you
to say which ([Tuning](tuning.md)).

**Next:** [Installing reflexes](installing.md).
