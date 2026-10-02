<!-- description: How a request gets into evoke and what comes out: a sentence as the argument, the REPL or a pipe, and --json, --tag, exit codes and limits. -->
# Typing a request

`evoke` takes one sentence and does three things with it: decide, gate, run. This page is about how the sentence
gets in and how the answer comes out.

## Three ways in

```text
evoke "kill the lights in the den"     one input, as the argument
evoke                                  the REPL, on a terminal
echo "kill the lights" | evoke         a filter: one input per line of stdin
```

**As the argument.** Everything after `evoke` is the input, unless the first word is exactly a command name. Bare
words are joined by one space, so `evoke kill the lights` and `evoke "kill the lights"` are the same call. Quote a
sentence that holds a character the shell would take, such as `?` or `*`.

**The REPL.** Run `evoke` alone on a terminal. It prompts `> ` and decides each line as if you had typed it as the
argument. It skips blank lines. It keeps one warm connection to the adapter. You can edit the line, and the arrow
keys recall earlier ones from `~/.local/state/evoke/history`. `Ctrl-C` cancels the line. `Ctrl-D` on an empty line
ends the session with exit 0. Under `TERM=dumb` the line is read plain: no editing, no history, and `Ctrl-C`
ends the session.

```text
$ evoke
> kill the lights in the den
  lights room="den" state="off"  0.92
den lights off
> what time is it
  none of them 0.85 · timer 0.10 · lights 0.03 · volume 0.02
>
```

The session remembers what its reflexes returned. A pick whose author named a `recent` field offers those values
back when a sentence leaves the argument out, so after a listing, «roll back the last deploy» asks with the
releases numbered; words that point at one, «buy it» after a product shown, «pay my last order», take the newest
without asking ([Arguments](../author/arguments.md#a-value-recalled)):

```text
> list the checkout deploys
checkout: 2 deploys today, the last 4.12.0 at 13:58
> roll back the last deploy
  From which release?  [1] 4.12.0  [2] 4.11.3  > 1
  Roll back 4.12.0?  [y]es [n]o [t]each > y
```

A day in a sentence, `tomorrow`, `next friday`, `the 14th`, is read as you said it and becomes a date when the
program runs, against your machine's clock ([Arguments](../author/arguments.md#picks)).

## Words the reading takes as they are

A value is always one of your own words. A text an argument wants between quotes, like a timer's label, is read
from the words you typed without them: in `set a timer for 10 minutes called tea` the label is `tea`, and the call
waits for your yes, since the words could have been cut otherwise. A word that stands for a value from a list but
is not on it — the `snug` for a room called `den`, `garage` for no room at all — is read where the list's meanings
say what it is, and the call waits for a yes; where they do not, `evoke` asks, and names the word:
`"garage" is not on the list`. `evoke why` shows each of these under the value it gave
([Outcomes](outcomes.md#why-the-last-sentence-explained)).

## A value typed the way you say it

You can type a value the way you would say it aloud. `evoke` reads an address, a URL, a code, a number, a day
or a length of time this way: `dana dot weiss at example dot org`, `k d oh three one five`, `seven oh`, `an hour
and twenty minutes`. The call shows the value as it would be typed.

A code takes the shape of the reflex's own examples. Where every example reads like `HS-0409`, your letters
become capitals and the dash is added. The call then waits for your yes, with your words on the line:

```text
$ evoke "earmark k d oh three one five"
  reserve sku="KD-0315" · write · weakest: route 0.95
    sku was read from "k d oh three one five"
  Reserve every unit of KD-0315?  [y]es [n]o [t]each > n
```

Some words can be read in more than one way. `ops twenty at example dot com` may be `ops20@example.com` or
`20@example.com`, and `thirty one fifteen` may be the start of a longer code. `evoke` does not choose for you.
It asks, and offers what your words read as ([Outcomes](outcomes.md#ask)):

```text
$ evoke "email the update to ops twenty at example dot com"
  To which email address?  you wrote "ops twenty at example dot com"  [1] ops20@example.com  [2] 20@example.com  [0] none of these  > 1
  email to="ops20@example.com" · destructive · weakest: route 0.95
    it cannot be undone, so it always waits for a yes
  Email the shipment update to ops20@example.com?  [y]es [n]o [t]each > n
```

## A name typed another way

A word of your vocabulary may be typed in a form that is not on the list. `evoke` reads it only where your
words leave no doubt about which listed word they name.

- A word cut short or typed with a slip is read where your own words name the listed word: `sept` for
  `september`, `eu-wwest` for `eu-west`. `evoke why` shows the check under the value, as `asked of "sept"
  alone`.
- A longer name, a pet name or a name one letter away may be another person: `Samuel`, `Sammy` or `Sal` beside
  a colleague listed as `sam`. `evoke` never reads such a name as the listed word. It asks, and puts the nearest
  word ready. A `y` takes it:

```text
$ evoke "where is Samuel's laptop"
  Which colleague's laptop?  you wrote "Samuel's": sam?  [y]es, or  [1] sam  [2] ana  [3] jo  [+] add one  [0] none of these  > y
  device person="sam"  0.95
sam: serial C02XK1ABJG5M, last seen 07:41
```

**A filter.** Pipe lines into `evoke`, or into `evoke try`, and every line is one input, answered in order. A line that needs a
prompt, a confirm or an ask, cannot be answered without a terminal. That line exits 3 and names the command to run
yourself. The filter still answers every other line, and exits with the first non-zero code.

```text
$ printf 'kill the lights in the den\nstart a timer\n' | evoke
  lights room="den" state="off"  0.92
den lights off
  an ask needs a terminal  →  evoke "start a timer"
[3]
```

## What comes out

What you asked to see goes to **stdout**: a reflex's result, and the answer of `show`, `vocab`, `why`, `try`,
`test` and `check`. What `evoke` says about a run goes to **stderr**, indented two spaces: the call and its
confidence, a prompt, the line of a write, the ranking after an abstain, and every diagnostic. So
`evoke "…" > out.txt` captures the result alone, and `evoke show | grep timer` finds the row. A script reads the
rest from the exit code.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.92          ← stderr: the call, and the confidence it ran on
den lights off                                 ← stdout: what the reflex returned
```

## Flags

| Flag          | Does                                                                                   |
| :------------ | :------------------------------------------------------------------------------------- |
| `--json`      | One JSON line per input instead of the lines above. It holds the input, the decision, the trace and the result. This is the filter format: see [The JSON line](../reference/json.md). |
| `--tag <tag>` | Offers only the reflexes carrying the tag. Repeat it to widen: `--tag home --tag sound`. |
| `--`          | The rest is input, even when it begins with a command word: `evoke -- test the alarm`.  |

`--help`, `-h` or `help`, and `--version` or `-V`, only count in the first place. They print to stdout.
`evoke add --help` prints the help too, as does `-h` right after any command word. To say the word `help` as a
sentence, put `--` before it.

A command word written after a flag, like `evoke --json try "lock it"`, is refused rather than decided. The line
names the command and how to write it. Nothing runs by accident.

## Command words

The first argument selects a command only when it is *exactly* one of these words:

```text
help  try  why  run  add  remove  update  sync  trust  show  teach  vocab  config  test  calibrate  new  check
```

Four more words are reserved for later: `edit`, `search`, `publish`, `adapter`. Reserving them now
means adding one later never changes what a sentence means. Lines from stdin are always input, whatever they begin
with.

## Exit codes

| Code | Meaning                                                                   |
| :--- | :------------------------------------------------------------------------ |
| 0    | Ran. Also `--help`, `--version`, and every command that did what it said |
| 1    | The program, or the machine, failed. Also `evoke test` with a failing case, and `evoke calibrate` with a call wrong at or over its bar |
| 2    | Abstained, or you declined                                                 |
| 3    | Needs a human: a missing key, an untrusted project, a prompt with no terminal, a line to fix |
| 4    | The adapter failed                                                         |

`Ctrl-C` while a body runs ends the body and everything it started, a JavaScript body's `signal` aborting first;
then `evoke` ends as an interrupted program does, and the shell reports exit 130. In a weave, every step that
did not finish reads `skipped · cancelled` ([Weaving](weaving.md#stopping-it)). Anywhere else, `Ctrl-C` ends
`evoke` at once.

## Limits and time

An input over 2 000 characters is refused. One decision has 30 seconds. The classifier's answer and the program's
run share them. A prompt waiting for you never counts. When the deadline passes, `evoke` stops and says so. It
never guesses.

## Colour

On a terminal, `evoke` colours what matters. The reflex's name in a call. The top answer of every judgment `try`
shows. The effect: green for `read`, yellow for `write`, red for `destructive`. The weakest judgment, dimmed. The
`→` of a fix. The `+` and `-` of a write. A spinner turns while a request or a fetch is in flight, and counts the
cases of a `test`. Piped, logged, under `NO_COLOR`, or with `TERM=dumb`, the same words print plain. On a
terminal narrower than the help, `--help` puts each description under its command.

**Next:** [Outcomes](outcomes.md).
