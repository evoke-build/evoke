# Outcomes

Every input ends in one of four outcomes. This page shows each one and what decides it. It also covers the three
commands that look at a decision without changing anything: `try`, `why` and `run`.

## Run

Confidence cleared the bar for the winner's effect. Every required argument was stated. Nothing else needed
attention. The call prints with its confidence, the program runs, and the result prints.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

## Confirm

The call is complete, but something holds it at a question. `evoke`'s own line says what: the call, the effect,
the weakest judgment, and any further reason. Then comes the reflex's one-line prompt, and three answers.

```text
$ evoke "dim the office"
  lights room="office" state="dim" · write · weakest: state 0.70
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
- **an unconsumed span**: you typed something recognizable, like a number or a URL, and no argument took it.
- **two things**: a runner-up reflex fits well enough that the input may have asked for two things.

`[t]each` records only what the input stated. An argument you filled in at a prompt is not recorded. The answer
is read from the terminal, never from stdin. With no terminal, a confirm exits 3 with the command to run yourself.

## Ask

The winner is clear, but a required argument is missing. Either the input never stated it, or the value fell
outside its range. `evoke` asks the argument's own question, offers what it may be, and gates again with your
answer.

```text
$ evoke "kill the lights"
  Which room?  [1] den  [2] office  [+] add one  > 1
  lights room="den" state="off" · write · weakest: state 0.58
  Set the den lights off?  [y]es [n]o [t]each > y
den lights off
$ evoke "set the volume to 150 percent"
  How loud, in percent?  150 percent is outside 0–100  > 40
  volume level="40"  0.93
volume set to 40%
```

- A choice takes its number or its own text. A pick takes what you type, read the same way as the input. A pick
  is a number, a duration, an address, a URL, or a quoted phrase. A quoted argument takes the whole line when
  nothing is quoted.
- An answer that does not fit is asked again, with the reason on the line. An empty line asks again.
- `+` at a vocabulary's prompt asks `Word?` and `Meaning?`, writes the word to your vocabulary, and goes on.
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
$ evoke "open my desktop"
  none 0.80 · lights 0.10 · timer 0.05 · volume 0.05
  open is inactive  →  evoke show
[2]
```

## What confidence is

For each decision, the adapter answers one question per reflex and one per argument. **Confidence is the lowest
top probability among the route and every argument question of the winner.** Unstated arguments and flags count
too, until you answer for them: an argument you typed at a prompt is settled, and its judgment leaves the gate.
The bars come from the adapter. They are calibrated, so each number means *the probability this is right*:

| Floor         | Jev  | Gates                                                          |
| :------------ | :--- | :------------------------------------------------------------- |
| `route`       | 0.5  | Under it, abstain                                              |
| `read`        | 0.6  | A `read` reflex runs at or above it                            |
| `write`       | 0.8  | A `write` reflex runs at or above it                           |
| `fits`        | 0.3  | A *runner-up* at or above it holds the outcome at confirm      |
| destructive   | —    | Always confirms                                                |

You may raise or lower them for your own machine, under `[adapters.jev]` in `evoke.toml`. `read` may never exceed
`write`. No number makes a destructive reflex skip its question. There is no `--yes`. To run unattended, set a bar
in a file you own, not a flag on a pipeline.

```toml
[adapters.jev]
gate = { write = 0.85 }
```

## `try`: decide, and show the work

`evoke try "<input>"` decides without running, and prints every judgment: the ranking, each argument's
distribution, each reflex's `fits`, then the outcome and the weakest judgment. Answers that would print as `0.00`
fold into a count, `8 more under 0.01`; `none` and `unstated` always show. It shares the cache with a real
decision. It is never logged.

```text
$ evoke try "kill the lights"
  lights 0.90 · none 0.06 · timer 0.02 · volume 0.02
  room   unstated 0.75 · den 0.20 · office 0.05
  state  off 0.58 · on 0.30 · dim 0.10 · unstated 0.02
  fits   lights 0.70 · timer 0.05 · volume 0.05
  ask room · weakest: state 0.58
```

`try --json` prints the decision as one JSON line: [The JSON line](../reference/json.md).

## `why`: the last decision, explained

Every real decision is logged. `evoke why` renders the last one as `try` would have, from the log alone, then says
what became of it. After a sentence of several steps, it shows each step under its number
([Weaving](weaving.md)).

```text
$ evoke why
  "kill the lights in the den"
  lights 0.91 · none 0.06 · timer 0.02 · volume 0.01
  room   den 0.85 · unstated 0.10 · office 0.05
  state  off 0.88 · on 0.05 · dim 0.05 · unstated 0.02
  fits   lights 0.70 · timer 0.05 · volume 0.05
  ran lights room="den" state="off" · weakest: room 0.85 · replay, 6 questions
```

## `run`: by name, no classifier

`evoke run <call>` runs a call you write yourself. Nothing is decided, so nothing prints but the call. The effect
policy still holds: a destructive call confirms. Every value is checked, just as a decision's would be.

```text
$ evoke run lights room=den state=off
  lights room="den" state="off"
den lights off
$ evoke run power action=restart
  power action="restart" · destructive
  Really restart now?  [y]es [n]o > y
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
