# Outcomes

Every input ends in one of four outcomes. This page shows each, what decides it, and the three commands that look
at a decision without changing anything: `try`, `why` and `run`.

## Run

Confidence cleared the floor for the winner's effect, every required argument was stated, nothing else asked for
attention. The call prints with its confidence, the body runs, the result prints.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

## Confirm

The call is complete, but something caps it at a question. `evoke`'s own line says what — the call, the effect, the
weakest judgment, and any further reason — then the reflex's one-line prompt, then three answers.

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

What caps a decision at confirm, in the order the line names them:

- **destructive** — a destructive reflex always confirms, however sure.
- **no gate** — the adapter shipped no thresholds, so nothing auto-runs.
- **under the floor** — the weakest judgment is under the floor for this effect.
- **an unconsumed span** — you typed something recognizable, a number or a URL, that no argument took.
- **two things** — a runner-up reflex fits well enough that the input may have asked for two things.

`[t]each` records only what the input stated: an argument filled at a prompt is not asserted. Read from the
terminal, never stdin; with no terminal, a confirm exits 3 with the command to run yourself.

## Ask

The winner is clear but a required argument is missing: the input never stated it, or a value fell outside its
range. `evoke` asks the argument's own question, offers what it may be, and gates again with your answer.

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

- A choice takes its number or its own text. A pick — a number, a duration, an address, a URL, a quoted phrase — takes
  what you type, read the same way the input is; a quoted argument takes the whole line when nothing is quoted.
- An answer that does not do is asked again with the reason on the line. An empty line asks again.
- `+` at a vocabulary's prompt asks `Word?` and `Meaning?`, writes the word to your vocabulary, and goes on.
- The end of input — `Ctrl-D` — declines, exit 2.

## Abstain

`none` won the route, or the winner is under the route floor. The ranking prints and nothing runs, exit 2.

```text
$ evoke "make it cosy"
  lights 0.45 · none 0.40 · timer 0.10 · volume 0.05 · route floor 0.50
[2]
```

## What confidence is

For each decision the adapter answers one question per reflex and per argument. **Confidence is the lowest top
probability among the route and every argument question of the winner**, unstated arguments and flags included.
The floors come from the adapter, calibrated so that each number means *the probability this is right*:

| Floor         | Jev  | Gates                                                          |
| :------------ | :--- | :------------------------------------------------------------- |
| `route`       | 0.5  | Under it, abstain                                              |
| `read`        | 0.6  | A `read` reflex runs at or above it                            |
| `write`       | 0.8  | A `write` reflex runs at or above it                           |
| `fits`        | 0.3  | A *runner-up* at or above it caps the outcome at confirm       |
| destructive   | —    | Always confirms                                                |

You may raise or lower them for your own machine under `[adapters.jev]` in `evoke.toml`; `read` may never exceed
`write`, and no number makes a destructive reflex skip its question. There is no `--yes`: unattended use is a
threshold in a file you own, not a flag on a pipeline.

```toml
[adapters.jev]
gate = { write = 0.85 }
```

## `try` — decide, and show the work

`evoke try "<input>"` decides without running and prints every judgment: the ranking, each argument's
distribution, each reflex's `fits`, then the outcome and the weakest judgment. It shares the cache with a real
decision and is never logged.

```text
$ evoke try "kill the lights"
  lights 0.90 · none 0.06 · timer 0.02 · volume 0.02
  room   unstated 0.75 · den 0.20 · office 0.05
  state  off 0.58 · on 0.30 · dim 0.10 · unstated 0.02
  fits   lights 0.70 · timer 0.05 · volume 0.05
  ask room · weakest: state 0.58
```

`try --json` prints the decision as one JSON line: [The JSON line](../reference/json.md).

## `why` — the last decision, explained

Every real decision is logged. `evoke why` renders the last one as `try` would have, from the log alone, then says
what became of it.

```text
$ evoke why
  "kill the lights in the den"
  lights 0.91 · none 0.06 · timer 0.02 · volume 0.01
  room   den 0.85 · unstated 0.10 · office 0.05
  state  off 0.88 · on 0.05 · dim 0.05 · unstated 0.02
  fits   lights 0.70 · timer 0.05 · volume 0.05
  ran lights room="den" state="off" · weakest: room 0.85 · replay, 6 questions
```

## `run` — by name, no classifier

`evoke run <call>` runs a call you write yourself. Nothing is decided, so nothing prints but the call; the effect
policy still holds — a destructive call confirms — and every value is checked as a decision's would be.

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

The call grammar is `name arg=value…`; a value with spaces is quoted, `duration="10 minutes"`; a flag is its bare
name. A word the vocabulary lacks, an option not offered, a number out of range, an argument the reflex does not
have — each is refused with the command that shows what is allowed. A call by name is not logged.

## The cache

The adapter's answers are cached by the installed set and the sentence — not the decision. Repeat an input and it
costs nothing; change a threshold and the same answers gate differently; `try` and `why` explain a cached decision
exactly as a fresh one. `evoke test` never uses it.

**Next:** [Installing reflexes](installing.md).
