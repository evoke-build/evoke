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

- Where a connective could separate two things — `and`, `then`, `but`, a comma, `after`, `before` — the
  classifier is asked whether it does. "kill the lights in the den and start a 10 minute timer" is two things.
  "set a timer for 10 minutes and 30 seconds" is one.
- Each part is decided as one input is: routed, gated, its arguments read. A part that matches nothing on its own
  is tried as another item of its neighbour's task first: "check stock for widgets and gadgets" is two stock
  checks. Failing that, the whole request is refused rather than half done.
- A part that begins with `not`, `don't`, `never` or `without` is left out. What you said not to do is no step.
- `then`, `after that` and `next` order the steps. `before you X, Y` and `Y after you X` both read as `X, then Y`.
  Two writes never run side by side. Reads may.

## What a step takes from another

A body may declare what its result holds, under `[yields]` in its manifest ([The manifest](../author/manifest.md)).
A later step that refers to it — `it`, `them`, `that report`, `the address` — takes a field of the right kind into
the argument it lacks:

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

Each line is what `evoke` would say of that step alone: the call and its confidence, or the confirm's own line
with its weakest judgment. After it: `asks <arg>` for what the step still needs, `takes <field> from <n>` where
a result threads in, `after <n>` where the words ordered it, `no reflex` where nothing matched.

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

## `try` and `--json`

`evoke try` shows the plan, then every step's judgments under its number. `evoke try --json` prints the plan
whole, on one line, with every adapter call it took. `evoke --json` prints one line per step as it runs: the line of one decision, with `step`
and `steps` first, `bound` where a value came from another step, and `status` at the end, with `why` when the
step stopped ([The JSON line](../reference/json.md)).

**Next:** [Installing reflexes](installing.md).
