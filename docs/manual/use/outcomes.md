<!-- title: Outcomes: run, confirm, ask or abstain -->
<!-- description: Every decision ends in run, confirm, ask or abstain. What decides each one, the confidence bars, and the try, why and run commands that show the work. -->
# Outcomes

Every input ends in one of four outcomes. This page shows each one and what decides it. It also covers the three
commands that look at a decision without changing anything: `try`, `why` and `run`.

## Run

Confidence cleared the bar for the winner's effect. Every required argument was stated. Nothing else needed
attention. The call prints with its confidence, the program runs, and the result prints.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.92
den lights off
```

## Confirm

The call is complete, but something holds it at a question. `evoke`'s own line says what: the call, the effect,
the weakest judgment, and any further reason. Then comes the reflex's one-line prompt, and three answers.

```text
$ evoke "dim the office"
  lights room="office" state="dim" · write · weakest: state 0.85
  Set the office lights dim?  [y]es [n]o [t]each > y
group-7 lights dim
```

| Answer      | Does                                                                                           |
| :---------- | :--------------------------------------------------------------------------------------------- |
| `y`, `yes`  | Runs                                                                                           |
| `n`, `no`   | Declines, exit 2                                                                               |
| `t`, `teach`| Records what you said as an example of this call in your overlay, then runs                    |

These are the reasons a decision stops at confirm, in the order the line names them:

- **destructive**: a destructive reflex always confirms, however sure.
- **no gate**: the adapter shipped no thresholds, so nothing runs on its own.
- **under the floor**: the weakest judgment is under the bar for this effect.
- **read from your words**: a value from a list was taken from a word of the input, where the answers about it
  did not agree. The line names the argument and the word: `room from "snug"`.
- **spelled another way**: a value was read from words that do not spell it as it is typed, like a day misspelt
  or a code typed with a space. The line names the words: `sku from "hs 0409"`.
- **a text without quotes**: a text was read from words you typed without quotes. The line names the
  argument: `label without quotes`.
- **more words**: the input holds words that ask for another thing, which the call does not hold. The line
  shows them: `also "lock the door too"`.
- **less than you typed**: the call, read back against the input, leaves out part of what you typed. The line
  says how far the call holds it, and names the words it leaves out: `holds all you said 0.20, leaves out
  "in the morning"`.
- **a detail beside it**: a part of the sentence that is not in the plan may add a detail to this step. The
  line shows the part: `without "the ones since noon"` ([Weaving](weaving.md)).

`[t]each` records only what the input stated. An argument you filled in at a prompt is not recorded. The answer
is read from the terminal, never from stdin. With no terminal, a confirm exits 3 with the command to run yourself.

## Ask

The winner is clear, but an argument is missing. The input never stated a required one, or the value fell
outside its range, or the input names a value that is not among the argument's choices, or it names one that
could be read two ways. `evoke` asks the argument's own question, offers what it may be, and gates again with
your answer.

```text
$ evoke "kill the lights"
  Which room?  [1] den  [2] office  [+] add one  > 1
  lights room="den" state="off" · write · weakest: state 0.74
  Set the den lights off?  [y]es [n]o [t]each > y
den lights off
$ evoke "set the volume to 150 percent"
  How loud, in percent?  150 percent is outside 0–100  > 40
  volume level="40"  0.96
volume set to 40%
$ evoke "kill the lights in the garage"
  Which room?  "garage" is not on the list  [1] den  [2] office  [+] add one  > 2
  lights room="office" state="off"  0.95
group-7 lights off
```

The prompt names the words of the input that answer the question, where `evoke` found them.

- A choice takes its number or its own text. A pick takes what you type, read the same way as the input. A pick
  is a number, a duration, an address, a URL, a quoted phrase, a date, a time, an amount or a code. A quoted
  argument takes the whole line when nothing is quoted; a code takes a quoted one.
- A pick whose manifest names a `recent` field lists what the bodies of this session returned under it, `From
  which release?  [1] 4.12.0  [2] 4.11.3  > `, and a number picks one ([Arguments](../author/arguments.md#a-value-recalled)).
- An answer that does not fit is asked again, with the reason on the line. An empty line asks again.
- `+` at a vocabulary's prompt asks `Word?` and `Meaning?`, then `Path?` when a reflex's declaration takes the
  word's value as a path, writes the word to your vocabulary, and goes on.
- The end of input, `Ctrl-D`, declines with exit 2.

## Abstain

`none` won the route, or the winner is under the route bar. The ranking prints, nothing runs, and the exit code
is 2.

```text
$ evoke "make it cosy"
  lights 0.45 · none 0.40 · timer 0.10 · volume 0.05 · route floor 0.50
[2]
```

An inactive reflex is never in the ranking. When one is left out, an abstain names it on the next line, and
`evoke show` says why:

```text
$ evoke "what time is it"
  none 0.85 · timer 0.10 · lights 0.03 · volume 0.02
  open is inactive  →  evoke show
[2]
```

## What confidence is

For each decision, the adapter answers a question about the route and questions about each argument.
**Confidence is the lowest probability among the route and every value the call holds.** Each value counts by
what gave it: where two questions about it agree, the less sure of the two counts. A flag, and an argument the
input says nothing of, add nothing to it. An argument you typed at a prompt is settled, and its judgment leaves
the gate.

A call that clears the bar of its effect is then read back against the input, in the reflex's own words, and one
more question is asked: does this reading hold everything the input says? The call runs when the answer clears
a bar of its own, `whole`.

The bars come from the adapter. Both built-in adapters reach Jev and ship the same numbers. They are calibrated,
so each number means *the probability this is right*, as `evoke calibrate` measures it on your own records
([Calibrating](calibrating.md)):

| Floor         | Default | Gates                                                       |
| :------------ | :--- | :------------------------------------------------------------- |
| `route`       | 0.5  | Under it, abstain                                              |
| `read`        | 0.8  | A `read` reflex runs at or above it                            |
| `write`       | 0.9  | A `write` reflex runs at or above it                           |
| `whole`       | 0.3  | A call that would run does so when it holds all you typed at or above it |
| `fits`        | 0.3  | At `add`, a new reflex that fits another's example at or above it is named |
| destructive   | —    | Always confirms                                                |

A write changes something, so it asks for more than a read. `whole` is a second look at a call that already
cleared its bar, so it asks for less: it holds a call that leaves part of the input out, and lets through one
the answer only hesitates on.

You may raise or lower them for your own machine, under your adapter's table in `evoke.toml`, `[adapters.jev]` or
`[adapters.openjev]`. `read` may never exceed
`write`. No number makes a destructive reflex skip its question. There is no `--yes`. To run unattended, set a bar
in a file you own, not a flag on a pipeline.

```toml
[adapters.jev]
gate = { write = 0.95 }
```

## `try`: decide, and show the work

`evoke try "<input>"` decides without running, and prints every judgment: the ranking, each argument's
distribution, then the outcome and the weakest judgment. An argument the input says nothing
of prints no line, unless it is asked. Answers that would print as `0.00` fold into a count, `8 more under 0.01`;
`none` and `unstated` always show. On an argument's line, `none` means the input states a value that is not among
the choices. Under a value's line, a second line says what else the value stands on: what a second question
about it answered, the words of the input that hold it, and the yes that took it. The `words` line lists the
words of the input that no value holds, with what each does: `say what to do`, `answer` an argument, `ask for
nothing`, or `ask for another thing`. The `whole` line says how far the call holds all you typed, where the
call was read back against the input. It shares the cache with a real decision. It is never logged.

```text
$ evoke try "kill the lights"
  lights 0.95 · none 0.03 · timer 0.01 · volume 0.01
  room   unstated 0.87 · den 0.10 · office 0.03
  state  off 0.74 · on 0.19 · dim 0.06 · unstated 0.01
         asked a second way: off 0.74 · is it off? yes 0.95
  words  "kill the lights" say what to do 0.90
  ask room · weakest: state 0.74
```

`try --json` prints the decision as one JSON line: [The JSON line](../reference/json.md).

## `why`: the last decision, explained

Every real decision is logged. `evoke why` renders the last one as `try` would have, from the log alone, then says
what became of it. After a sentence of several steps, it shows each step under its number
([Weaving](weaving.md)).

```text
$ evoke why
  "kill the lights in the den"
  lights 0.95 · none 0.03 · timer 0.01 · volume 0.01
  room   den 0.92 · unstated 0.05 · office 0.03
         asked a second way: den 0.92
  state  off 0.94 · on 0.03 · dim 0.02 · unstated 0.01
         asked a second way: off 0.94 · is it off? yes 0.95
  words  "kill the lights" say what to do 0.90
  whole  holds all you said 0.90
  ran lights room="den" state="off" · weakest: room 0.92 · replay, 13 questions in 3 rounds
```

The last line counts the questions the adapter answered, and the rounds they were asked in when the answers to
the first opened more. When the answers came from the cache, it ends in `· cached` in place of the adapter and
its count.

## `run`: by name, no classifier

`evoke run <call>` runs a call you write yourself. Nothing is decided, so nothing prints but the call. The effect
policy still holds: a destructive call confirms. Every value is checked, just as a decision's would be.

```text
$ evoke run lights room=den state=off
  lights room="den" state="off"
den lights off
$ evoke run power action=restart
  power action="restart" · destructive
  Make the laptop restart now?  [y]es [n]o > y
restart in 5 seconds
$ evoke run lights state=off
  lights: lights needs room  →  evoke show lights
[3]
```

The call grammar is `name arg=value…`. A value with spaces is quoted: `duration="10 minutes"`. A flag is its bare
name. A word the vocabulary lacks, an option not offered, a number out of range, or an argument the reflex does
not have: each is refused, with the command that shows what is allowed. A call by name is not logged.
`evoke run --json <call>` prints the call and its result as one line, for a script: [The JSON
line](../reference/json.md).

## The cache

The adapter's answers are cached by the installed set, the sentence and the questions asked, not by the
decision. Repeat an input and it costs nothing. A decision narrowed with `--tag` asks fewer questions, so it has
an entry of its own. Change a threshold, and the same answers gate differently. `try` and `why` explain a
cached decision exactly like a fresh one. `evoke test` never uses the cache.

**Next:** [Weaving](weaving.md).
