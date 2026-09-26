<!-- title: Calibrating: what the confidence meant -->
<!-- description: What the number beside a call meant on your own records: the whole call right by confidence, the wrong calls at or over each bar, the spread over repeats. -->
# Calibrating

Every call prints a number: `lights room="den" state="off"  0.85`. It claims that a call like this is right
about 85 times in 100. `evoke calibrate` checks the claim against the records you own, and prints what it found.
It needs the classifier's key, or a recording to answer from ([below](#a-recording-in-ci)).

## What the number claims

The number is the lowest of the probabilities the classifier gave the route and each of the winner's arguments
([Outcomes](outcomes.md#what-confidence-is)). So the one event it can be measured on is *the whole call is right*:
the route, and every argument the record names. A record is an example or a test of a reflex
([Examples and tests](../author/records.md)). `calibrate` decides every record of every active reflex once
more, never through the cache, against the whole installed set; a sentence that is several records is decided
once. It judges each record as `evoke test` judges it. Then it counts whole calls by the confidence they claimed.

## The report

Over the manual's own `lights` and `timer` records, on a recording, after three decisions were made and logged:

```text
$ evoke calibrate
  replay · 12 records over 2 reflexes · 12 inputs, decided once
  7 run · 3 ask · 2 abstain
  whole call right, by confidence
    0.60–0.80  1 call    right 0 ·   0% (0–79) · claimed 0.60 · thin
    0.80–1.00  9 calls   right 9 · 100% (70–100) · claimed 0.89 · thin
    abstained  2 inputs  right 2 · 100% (34–100)
  wrong at or over write 0.80   0 of 9 · 0 per thousand, at most 283 · thin
  near write 0.80   at 0.70 9 run, 0 wrong · at 0.75 9, 0 · at 0.80 9, 0 · at 0.85 8, 0 · at 0.90 7, 0
  each judgment on its own
    route   12 judgments  right 100% (76–100) · claimed 0.89
    options  3 judgments  right  67% (21–94) · claimed 0.80
    pick     9 judgments  right 100% (70–100) · claimed 0.91
  Brier 0.048 · reliability 0.046 · resolution 0.090
  misses
    "make it darker in here"  ask lights 0.60 · state: expected "dim", read "off"
  the log · 3 decisions under replay · 1 ran · 1 confirmed and stopped · 1 abstained
    stopped at confirm, by confidence   0.60–0.80 1
    3 were records · right 2 · 67% (21–94)
  thin: under 100 calls in every bin, nothing proven at any bar
```

Line by line:

- **The head** counts the records, the reflexes they belong to and the distinct inputs, and says how many times
  each input was decided. Then how many inputs ran, confirmed, asked or abstained; an outcome none reached is
  left out.
- **Whole call right, by confidence.** One line per bin: its range, the calls in it, how many were right, that
  share in percent with its 95 % interval, and the confidence the calls claimed on average, as a probability.
  The bins' edges sit at the bars, 0.50, 0.60 and 0.80, so no bin straddles one; a range with a few hundred
  calls splits into bins of about a hundred; a bin with no call is left out. `thin` marks a bin under a hundred
  calls: it is shown, and it proves nothing. `over-confident` marks a bin whose claim is above its interval. A
  record of `false`, which says *never this reflex*, routed to a reflex no record names is `unknown`, counted
  apart. The abstains have a line of their own: an abstain is right on a `false` record, wrong on any other.
- **Wrong at or over a bar.** For each effect with a call, the calls at or over its bar, how many were wrong,
  the rate per thousand, and the most it could be at 95 %. This is the number a script gates on. Under it, the
  neighbourhood: at the bar and two steps of 0.05 either side, how many calls would run and how many of those
  are wrong.
- **Each judgment on its own**: the route, then the arguments by the source of their values, options, a
  vocabulary, a pick or a flag, each judged by its own probability where a record names it.
- **Brier**, the mean squared distance between each claim and the truth, with its reliability, how far the
  claims sit from the shares, and its resolution, how far the bins' shares sit from the whole; `--json` carries
  the uncertainty too.
- **The misses**: each record a decision missed, what was decided, and where it missed, as `evoke test` says it.
- **The log**: every line of `~/.local/state/evoke/log.jsonl` under this adapter, counted by what became of it:
  ran, confirmed and ran, confirmed and stopped, asked and stopped, abstained, failed. The confidence of what
  stopped at a confirm, by range. The lines whose input is a record, judged by it.
- **The last line** says `thin` while every bin is under a hundred calls. With a hundred calls in some bin, it
  says whether any such bin is over-confident at 95 %.

Nothing is said by colour alone: `thin`, `unknown` and `over-confident` are words.

## What a record proves, and what the log does not

A record proves one call. An asserting record says whether the route and every argument it names are right; an
argument it does not name is not compared. A `false` record proves only *not this reflex*: routed to its own
reflex the call is wrong, abstained it is right, routed to a third reflex it proves nothing, unless another
record of the same sentence names that reflex. Examples are sent to the classifier, so a right example shows
less than a right test, which the classifier never saw. The report counts both.

The log's lines are weaker. A run that a person watched says nothing about the call. A confirm answered `yes`
says a person read the call and agreed. A confirm without a result says `no`, or no answer, or no terminal: the
line cannot tell. So the log's block counts what became of each decision, and never mixes into the bins above.
A line whose input is a record is judged by the record, which settles it.

## `--repeat`

`evoke calibrate --repeat 3` decides each input three times, a few at once, never through the cache. Every
number above still counts each input once, by its first decision: the repeats measure stability, not accuracy.
They add one line, and under it the inputs that moved most:

```text
  repeats 3 · winner flips 0 of 12 · verdict flips 0 · spread median 0.00, 90th 0.00, max 0.00 · straddling a bar 0 · wrong at or over a bar 0
```

A **winner flip** is an input whose route came out differently across repeats; a **verdict flip**, one whose
call was right in some repeats and wrong in others. The **spread** is the range of the confidence over the
repeats, per input. An input **straddles a bar** when some repeats sat under the bar of its effect and others at
or over it. **Wrong at or over a bar** counts the calls, in any repeat, that were wrong there. Under the line,
each input that moved: its confidence and route ranges, its winners and outcomes with their counts, and how many
repeats were wrong. A miss says how many of the repeats missed the same way. On a recording the spread is
nothing, as above; against a live engine, the report says how far the same sentence moves from one decision to
the next.

Ten repeats over a thousand records is ten thousand requests: minutes, and the key's cost.

## A recording, in CI

The report needs no key over a recording. Set `adapter = "replay"` in `evoke.toml` and the recording's path in
`EVOKE_ANSWERS`, as the SDK's [Testing](../sdk/testing.md#the-cli-on-a-recording) page shows, and the same
report runs offline, the same every time. That is how this manual's block above was made.

## `--json`

`evoke calibrate --json` prints the report as one object: `adapter`, `records`, `reflexes`, `inputs`, `repeats`,
`outcomes`, `bins`, `unknown`, `abstained`, `bars`, `questions`, `brier`, `misses`, `variance` under `--repeat`,
and `log`: [The JSON line](../reference/json.md#evoke-calibrate---json).

## The exit

`calibrate` exits 1 when a call was wrong at or over its bar, in any repeat, or when a bin of a hundred calls is
over-confident at 95 %, with the reason on its last line:

```text
  1 call wrong at or over its bar  →  evoke calibrate
[1]
```

Otherwise it exits 0, however thin the bins. Nothing to calibrate, no active reflex with examples or tests, says
so in one line. Nothing runs, and nothing is logged.

**Next:** [Projects](projects.md).
