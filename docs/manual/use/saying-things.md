# Saying things

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
ends the session with exit 0.

```text
$ evoke
> kill the lights in the den
  lights room="den" state="off"  0.85
den lights off
> what time is it
  none 0.70 · timer 0.20 · lights 0.05 · volume 0.05
>
```

**A filter.** Pipe lines into `evoke`, and every line is one input, answered in order. A line that needs a
prompt, a confirm or an ask, cannot be answered without a terminal. That line exits 3 and names the command to run
yourself. The filter still answers every other line, and exits with the first non-zero code.

```text
$ printf 'kill the lights in the den\nstart a timer\n' | evoke
  lights room="den" state="off"  0.85
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
  lights room="den" state="off"  0.85          ← stderr: the call, and the confidence it ran on
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
help  try  why  run  add  remove  update  sync  trust  show  teach  vocab  config  test  new  check
```

Five more words are reserved for later: `edit`, `search`, `publish`, `adapter`, `calibrate`. Reserving them now
means adding one later never changes what a sentence means. Lines from stdin are always input, whatever they begin
with.

## Exit codes

| Code | Meaning                                                                   |
| :--- | :------------------------------------------------------------------------ |
| 0    | Ran. Also `--help`, `--version`, and every command that did what it said |
| 1    | The program, or the machine, failed. Also `evoke test` with a failing case |
| 2    | Abstained, or you declined                                                 |
| 3    | Needs a human: a missing key, an untrusted project, a prompt with no terminal, a line to fix |
| 4    | The adapter failed                                                         |

`Ctrl-C` ends `evoke` at once, wherever it is. A program it started ends with it. The shell reports the interrupt
as exit 130.

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
