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

The call is complete, but something holds it at a question. `evoke`'s own line names the call, the effect and
the weakest judgment; the line under it says why the call waits. Then comes the reflex's one-line prompt, and
three answers.

```text
$ evoke "dim the office"
  lights room="office" state="dim" · write · weakest: state 0.85
    a write runs at 0.90 or more
  Set the office lights dim?  [y]es [n]o [t]each > y
group-7 lights dim
```

| Answer      | Does                                                                                           |
| :---------- | :--------------------------------------------------------------------------------------------- |
| `y`, `yes`  | Runs                                                                                           |
| `n`, `no`   | Declines, exit 2                                                                               |
| `t`, `teach`| Records what you said as an example of this call in your overlay, then runs                    |

These are the reasons a decision stops at confirm, each with the line it prints; several print on one line,
separated by `;`.

- **destructive**: a destructive reflex always confirms, however sure. `it cannot be undone, so it always
  waits for a yes`.
- **no gate**: the adapter shipped no thresholds, so nothing runs on its own. `the adapter ships no bars, so
  nothing runs on its own`.
- **under the bar**: the weakest judgment is under the bar for this effect. `a write runs at 0.90 or more`.
- **read from your words**: a value from a list was taken from a word of the input, where the answers about it
  did not agree; or a value was read from words that do not spell it as it is typed, like a day misspelt, a
  code typed with a space, or a code said aloud that took its capitals or its dash from the reflex's examples.
  `room was read from "snug"`, `sku was read from "hs 0409"`.
- **a text without quotes**: a text was read from words you typed without quotes. `label was typed without
  quotes, which always waits for a yes`.
- **more words**: the input holds words that ask for another thing, which the call does not hold. `"lock the
  door too" asks for another thing`. Words that say the call is wanted once more, like `too` or `the same for`,
  count as these when the plan holds the call once.
- **part of a value**: words right after a value answer the same question, so the value may be cut short of
  them. `"30" may be part of the duration`.
- **a text in quotes**: the input holds a text in quotes that no value took, and the reflex takes one. `'tea' is
  in quotes and the call does not hold it`.
- **a text left over**: the reflex takes a text, the call holds none, and the input has words left that say what
  is wanted from the result. They may be that text. `"saying tea is ready" may be the label`.
- **less than you typed**: the call, read back against the input, leaves out part of what you typed. `it holds
  all you said at 0.20, and a call runs at 0.30 or more; it leaves out "in the morning"`. The words named are
  ones no value holds: words a value was read from are not among them. Words that say what you want from the
  result are named apart: `"newest first" says what is wanted from the result`.
- **a detail beside it**: a part of the sentence that is not in the plan may add a detail to this step, or the
  step was cut from a longer part read with a value its call lacks. `"the ones since noon" may add a detail this
  call does not hold` ([Weaving](weaving.md)).

`[t]each` records only what the input stated. An argument you filled in at a prompt is not recorded. The answer
is read from the terminal, never from stdin. With no terminal, a confirm exits 3 with the command to run yourself.

## Ask

The winner is clear, but an argument is missing. The input never stated a required one, or the value fell
outside its range, or the input names a value that is not among the argument's choices, or it names one that
could be read two ways. `evoke` asks the argument's own question, offers what it may be, and gates again with
your answer.

```text
$ evoke "kill the lights"
  Which room?  [1] den  [2] office  [+] add one  [0] none of these  > 1
  lights room="den" state="off" · write · weakest: state 0.74
    a write runs at 0.90 or more
  Set the den lights off?  [y]es [n]o [t]each > y
den lights off
$ evoke "set the volume to 150 percent"
  How loud, in percent?  150 percent is outside 0–100  > 40
  volume level="40"  0.96
volume set to 40%
$ evoke "kill the lights in the garage"
  Which room?  "garage" is not on the list  [1] den  [2] office  [+] add one  [0] none of these  > 2
  lights room="office" state="off"  0.95
group-7 lights off
```

The prompt names the words of the input that answer the question, where `evoke` found them.

- A choice takes its number or its own text. A pick takes what you type, read the same way as the input. A pick
  is a number, a duration, an address, a URL, a quoted phrase, a date, a time, an amount or a code. A quoted
  argument takes the whole line when nothing is quoted; a code takes a quoted one.
- A pick whose manifest names a `recent` field lists what the bodies of this session returned under it, `From
  which release?  [1] 4.12.0  [2] 4.11.3  > `, and a number picks one ([Arguments](../author/arguments.md#a-value-recalled)).
- Where one listed word is the likely answer, the prompt puts it ready before the choices: `Which colleague's
  laptop?  you wrote "Samuel's": sam?  [y]es, or  [1] sam  [2] ana  [3] jo  [+] add one  [0] none of these  > `.
  `y` takes it. The choices keep their order, and a number or a word still picks any of them
  ([Saying things](saying-things.md#a-name-typed-another-way)).
- Where your words say a pick's value aloud and can be read in more than one way, the prompt quotes them and
  offers what they read as: `Which product code?  you wrote "k d thirty one fifteen"  [1] KD-3115  [0] none of
  these  > `. A number picks one, and you can still type the value yourself. For a number argument the readings
  are named without a number, `you wrote "three eleven"  311  > `, since a number you type is the answer itself
  ([Saying things](saying-things.md#a-value-typed-the-way-you-say-it)).
- An answer that does not fit is asked again, with the reason on the line. An empty line asks again.
- `+` at a vocabulary's prompt asks `Word?` and `Meaning?`, then `Path?` when a reflex's declaration takes the
  word's value as a path, writes the word to your vocabulary, and goes on.
- `0`, none of these, declines a listed question, as the end of input, `Ctrl-D`, declines any: nothing runs,
  the line says so, and the exit code is 2.

## Abstain

`none` won the route, or the winner is under the route bar. The ranking prints, nothing runs, and the exit code
is 2.

```text
$ evoke "make it cosy"
  lights 0.45 · none of them 0.40 · timer 0.10 · volume 0.05 · a reflex is picked at 0.50 or more
[2]
```

An inactive reflex is never in the ranking. When one is left out, an abstain names it on the next line, and
`evoke show` says why:

```text
$ evoke "what time is it"
  none of them 0.85 · timer 0.10 · lights 0.03 · volume 0.02
  open is inactive  →  evoke show
[2]
```

## What confidence is

For each decision, the adapter answers a question about the route and questions about each argument.
**Confidence is the lowest probability among the route and every value the call holds.** Each value counts by
what gave it: where two questions about it agree, the less sure of the two counts. A flag, and an argument the
input says nothing of, add nothing to it. An argument you typed at a prompt is settled, and its judgment leaves
the gate.

A call that clears the bar of its effect is then read back against the input, in the reflex's own words: its
whole description, then each argument with its value. One more question is asked: does this reading hold
everything the input says? The call runs when the answer clears a bar of its own, `whole`. The words no value
took are read beside it, each with what the reflex does: they say what to do, answer an argument, say what is
wanted from the result, ask for nothing, or ask for another thing.

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

`evoke try "<input>"` decides without running, and prints the sentence as it was read. First the sentence,
then, where it matters, how the sentence was read as a whole: whether it asks one thing or several and where it
was cut, each part set aside or kept out of the plan, and what was left out because you said not to do it. A
part set aside by its own words says what it is: `"before I forget": your own action`, `"if you would": a
courtesy`. Then
each step, under its number when there are several, with a row a judgment:

- `which reflex`: the answers about the reflex, the most probable first, then what the reflex does.
- One row per argument the call holds, asks, or the input says something of: the answer, or the value where an
  answer did not give it, and under it what the value stands on — what a second question about it answered,
  the words of the input that hold it, the yes that took it, the step it was taken from, the part of the
  sentence that gave it, or your own answer at the prompt.
- `holds all you said`: how far the call holds all you typed, where the call was read back against the input;
  under the bar, the words it leaves out, and the words that say what is wanted from the result.
- What a rule did: `the same call`, a part that adds to this call; `read with`, how a part that matched nothing
  alone was settled; `the playbook`, the playbook that wrote the step; `takes`, the results it takes whole;
  `runs if`, what picks it.
- `→`: what would become of the call — `runs`, `waits for a yes`, `asks`, `refused` — with the call as the
  plan prints it, and under it why.

A last line says how many questions the adapter answered and in how many rounds, or that the cache did. An
answer's key prints in words: `not said` for a question the input says nothing to, `said, and not on the list`
for a value that is not among the choices, `none of them` for no reflex. `try` shares the cache with a real
decision. It is never logged.

```text
$ evoke try "kill the lights"
  "kill the lights"

  which reflex            lights 0.95   (none of them 0.03 · timer 0.01)
                          Turn the lights in one room on, off, or dim them.
  room                    not said 0.87   (den 0.10 · office 0.03)
  state                   off 0.74   (on 0.19 · dim 0.06)
                          asked a second way: off 0.74; the less sure of the two counts
                          is it off? yes 0.95
  → asks: lights state="off" room=?
    it needs room

  replay answered 11 questions in 2 rounds
```

`try --json` prints the decision as one JSON line: [The JSON line](../reference/json.md).

## `why`: the last sentence, explained

Every real decision is logged. `evoke why` prints the last sentence as `try` would have, from the log alone,
and says what became of each step: `ran`, `ran at your yes`, `declined`, `asked`, `refused`, `failed`,
`cancelled`, `skipped`. After a sentence of several steps, it shows each step under its number
([Weaving](weaving.md)).

```text
$ evoke why
  "kill the lights in the den"

  which reflex            lights 0.95   (none of them 0.03 · timer 0.01)
                          Turn the lights in one room on, off, or dim them.
  room                    den 0.92   (not said 0.05 · office 0.03)
                          asked a second way: den 0.92; the less sure of the two counts
  state                   off 0.94   (on 0.03 · dim 0.02)
                          asked a second way: off 0.94; the less sure of the two counts
                          is it off? yes 0.95
  holds all you said      0.90
  → ran: lights room="den" state="off"  0.92
    a write runs at 0.90 or more, and its least sure judgment is room, 0.92; it holds all you said at 0.90, and a call runs at 0.30 or more

  replay answered 13 questions in 3 rounds
```

The last line counts the questions the adapter answered, and the rounds they were asked in: the parts of a
sentence are asked about side by side, and the answers to one round may open the next. When every answer came
from the cache, it reads `answered from the cache`; a plan run from a file names the file.

## `run`: by name, no classifier

`evoke run <call>` runs a call you write yourself. Nothing is decided, so nothing prints but the call. The effect
policy still holds: a destructive call confirms. Every value is checked, just as a decision's would be.

```text
$ evoke run lights room=den state=off
  lights room="den" state="off"
den lights off
$ evoke run power action=restart
  power action="restart" · destructive
    it cannot be undone, so it always waits for a yes
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
