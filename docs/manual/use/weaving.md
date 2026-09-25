<!-- description: One sentence can ask for several reflexes. evoke shows the numbered plan before anything runs, and a step can take a value from an earlier step's result. -->
# Weaving

`evoke` reads every sentence for its steps. One step is decided as always. More than one is a *weave*: each
part decided on its own, in the order the words give, a result of one step threaded into a later one, and the
plan shown before anything runs.

```text
$ evoke "kill the lights in the den and start a 10 minute timer"
  1  lights room="den" state="off"  0.85
  2  timer duration="10 minute"  0.90
den lights off
10 minute timer started
```

## How a sentence is read

- A connective can separate two things: `and`, `then`, `but`, a comma, `after`, `before`. At each one the
  classifier is asked whether it does. "kill the lights in the den and start a 10 minute timer" is two things.
  "set a timer for 10 minutes and 30 seconds" is one. A sentence with more than two dozen such points, a pasted
  list, is decided as one input.
- Each part is decided as one input is: routed, gated, its arguments read. A part that matches nothing on its own
  is tried as another item of its neighbour's task first: "check stock for widgets and gadgets" is two stock
  checks. So is a part that is only a determiner and one word, "the office" in "kill the lights in the den and
  the office", whatever it would mean on its own: it stays a step of its own only when it fits no item of its
  neighbour's task. A part read as an item must be the value it stands in for, "gadgets" for "widgets", never a
  longer phrase that holds one. When the classifier was sure the two parts were separate things and one still
  matches nothing, the whole request is refused rather than half done. When it was not sure, the part is read
  with its neighbour as one request, and that step confirms before it runs: its line ends in `merged`.
- A part that begins with `not`, `don't`, `never` or `without` is left out, however the apostrophe is typed. What
  you ask evoke not to do is no step. One step beside such a part is decided as one input. A sentence that is only
  such parts is nothing to do, and one line says so.
- `then`, `after that` and `next` order the steps. `after you X, Y` and `Y after you X` read as `X, then Y`.
  `before you X, Y` and `Y before you X` read as `Y, then X`. When a write is among the steps, every step runs
  alone. A plan of reads may run them side by side.

## What a step takes from another

A body may declare what its result holds, under `[yields]` in its manifest ([The manifest](../author/manifest.md)).
A later step may refer to it with `it`, `them`, `that report` or `the address`. It takes a field of the right kind
into the argument it lacks:

```text
$ evoke "look up dana's address and email them"
  1  contact name="dana"  0.90
  2  mail · takes email from 1
dana <dana@example.com>
  2  mail to="dana@example.com"  0.88
drafted to dana@example.com
```

`mail` needed an address the words did not give. `contact` yields one, and `them` names it. A required argument
is filled with the value. An optional one is decided again with the value written into the words, so the
classifier assigns it, under the same gate as any words: a quoted value no argument takes is an unconsumed span,
and the step confirms. Nothing is guessed: a reference that several fields could satisfy, or one record of a
list, stops with a line naming them. A step that refers to another whose result it takes nothing from asks
before anything runs:

```text
$ evoke "look up dana's address and start a 10 minute timer for them"
  1  contact name="dana"  0.90
  2  timer duration="10 minute"  0.88 · after 1
  step 2 refers to step 1, but takes nothing from it
  Run the plan as it stands?  [y]es [n]o > y
dana <dana@example.com>
10 minute timer started
```

## Before anything runs

The plan is settled first. A step whose required argument no other step provides is asked for it up front, as
one input would be at its turn. Then the plan is shown, numbered, one line per step:

```text
$ evoke "kill the lights and start a 10 minute timer"
  1  lights state="off" · asks room
  Which room?  [1] den  [2] office  [+] add one  > 1
  1  lights room="den" state="off" · write · weakest: state 0.58
  2  timer duration="10 minute"  0.90
  1  lights room="den" state="off" · write · weakest: state 0.58
  Set the den lights off?  [y]es [n]o [t]each > y
den lights off
10 minute timer started
```

An answer out of range is asked again with the reason, as one input's would be. Each line of the plan is what
`evoke` would say of that step alone: the call and its confidence, or the confirm's own line with its weakest
judgment. After it: `asks <arg>` for what the step still needs, `takes <field> from <n>` where a result threads
in, `after <n>` where the words ordered it, `no reflex` where nothing matched.

## At each step's turn

Each step then goes through the same gate as one input: a run runs, a confirm prompts, an ask asks. A step that
differs from its plan line, a bound value now in place, prints again. A failure, a decline or a refusal ends the
weave after its stage. The steps after it are skipped, and say so:

```text
$ evoke "start a 25 minute timer and kill the lights in the den"
  1  timer duration="25 minute" · write · weakest: duration 0.70
  2  lights room="den" state="off"  0.85
  1  timer duration="25 minute" · write · weakest: duration 0.70
  Start a 25 minute timer?  [y]es [n]o [t]each > n
  2  lights room="den" state="off"  0.85 · skipped
[2]
```

A step decided again with a bound value in its words may match no reflex then. It is refused, and says so:

```text
  2  "copy sam@example.com on a note to dana@example.com" · no reflex
```

A part that matches nothing refuses the whole request before anything runs:

```text
$ evoke "kill the lights in the den and feed the cat"
  1  lights room="den" state="off"  0.85
  2  "feed the cat" · no reflex
[2]
```

The exit code is the worst step's. `0` every step ran. `1` a body failed, or a step yielded nothing the next
could take. `2` a step was declined, or a part matched nothing. `3` a step needed a terminal, or a question only
you can answer.

## Stopping it

`Ctrl-C` while a step runs stops the weave. The body running at that moment is told to stop and ended with
everything it started: a JavaScript body sees its `signal` abort and has a second to finish. That step and
every step after it read `skipped · cancelled`, on the terminal and in the log, so `why` and `teach` still see
the whole plan:

```text
$ evoke "wait a while and start a 10 minute timer"
  1  wait  0.90
  2  timer duration="10 minute"  0.90
waiting
^C
  1  wait  0.90 · skipped · cancelled
  2  timer duration="10 minute"  0.90 · skipped · cancelled
```

Then `evoke` ends as an interrupted program does, and the shell reports exit 130. Under `--json` every step's
line prints first, its `why` `{ "type": "cancelled" }`.

## `try` and `--json`

`evoke try` shows the plan, then every step's judgments under its number. `evoke try --json` prints the plan
whole, on one line, with every adapter call it took. `evoke --json` prints one line per step as it runs: the
line of one decision, with `step` and `steps` first, `bound` where a value came from another step, and `status`
at the end, with `why` when the step stopped ([The JSON line](../reference/json.md)). A plan stopped before any
step ran, refused, its question or its prompt declined, or with no terminal to ask, prints every step's line at
once, with what stopped it.

## Afterwards

Every step is logged under its number. `evoke why` shows each step of the last sentence with what became of it:
`ran`, `failed`, `declined`, `refused`, `skipped` or `unanswered`, and why, `cancelled` among the reasons. `evoke teach <call>` with no
utterance takes the step the lesson's reflex decided; when none or several did, it names the steps and asks you
to say which ([Tuning](tuning.md)).

**Next:** [Installing reflexes](installing.md).
