# The body

The body is what `run` names. It is either a JavaScript file with a function as its default export, or a program
with its arguments. `evoke` calls it once per decision, with the arguments filled in, a context, and a deadline.

## A file body

```toml
run = "lights.mts"
```

```ts
import type { Reflex } from "./reflex.d.ts"     // Args, Config, Context, Result, Reflex — written by evoke check

export default (async ({ room, state, brightness = 30 }, { config, signal }) => {
  await hue(config, room, { on: state !== "off", bri: state === "dim" ? brightness : 100 }, signal)
  return `${room} lights ${state}`
}) satisfies Reflex
```

- The file ends in `.mts` or `.mjs` and lives inside the reflex directory. It is a module, whatever a nearby
  `package.json` says. It runs on Node 24 or newer, which strips types on its own. There is no build step.
- It is **self-contained**. Web APIs and `node:` builtins are there, and `evoke` installs no package for it. The
  body is one file, with anything else bundled into it: [rule 44](rules.md#9-what-a-body-must-do).
- `satisfies Reflex` types the arguments from the manifest and checks the return. `evoke check` writes
  `reflex.d.ts` next to the body whenever the arguments change. Commit it.

### What the body receives

```ts
(args: Args, context: { input: string; config: Config; signal: AbortSignal })
```

| Value        | Is                                                                                             |
| :----------- | :--------------------------------------------------------------------------------------------- |
| `args`       | Per argument: an option's key; a vocabulary word's value, or the word; a pick's value, meaning the number, the seconds, or the text; `true` for a flag; for an argument with `takes`, the whole `data` of the earlier step that returned it, as it was returned. An optional argument left unstated is absent |
| `input`      | The person's own words, for a typed sentence. When a plan runs the reflex as a step, the step's words. Empty when `evoke run` calls it by name. Free text, a reminder's message, is read from here: [rule 21](rules.md#4-words-that-read-each-value) |
| `config`     | Each `[config]` key as a string, secrets resolved from the environment for this run only        |
| `signal`     | Aborted at the deadline, on `Ctrl-C` or `SIGTERM`, when an app cancels the run, and when `evoke` ends. The body then stops: [rule 44](rules.md#9-what-a-body-must-do) |

### What it returns

A string: the text a person reads. Or `{ text, data? }`, where `data` is anything an app might use: a pid, a
time, a path. A field of `data` that a later step of one request may take is declared under `[yields]` in the
manifest, and the whole of `data`, under a name, with `returns`:
[The manifest](manifest.md#a-result-another-step-takes-whole).

Throwing is failure: the message prints on one line, and the exit code is 1. Anything else returned is a failure
too. A thrown error's frames are kept for its author: after a typed sentence, `evoke why` shows them under its
decision, and `--json` carries them as `frames`.

The text says what happened, with the value that lets a person check it: [rule 35](rules.md#7-what-a-person-reads).
A failure says what went wrong, in the body's own words: [rule 44](rules.md#9-what-a-body-must-do).

### The run

- **Environment.** Scrubbed: `PATH`, `HOME`, `TMPDIR`, `LANG` and `TERM`, nothing else. `TMPDIR` is a private
  folder, made for the run and removed after it. Secrets reach the body through `config`, never the environment.
- **Reach.** The body touches what `[needs]` declares, and nothing else:
  [The manifest](manifest.md#what-the-body-touches). Its own directory and `TMPDIR` are always there. A read
  past the declaration throws in the body, and the run ends with what was reached and the key, `~/secret.txt is
  not in [needs] reads`, and the fix the manifest page gives by where the declaration is written. A word's value
  or a plain setting that begins with `~/` reaches the body as a path under the home, in `config`, in an argv and
  in `EVOKE_CONFIG_<KEY>`, so the value, the argv and the declaration name one path.
- **Deadline.** 30 seconds per decision, shared with the classifier's answer. At the deadline `signal` aborts. A
  body still running a second later fails. What must outlive the run, like a timer or `caffeinate`, detaches:
  [rule 44](rules.md#9-what-a-body-must-do).
- **Output.** The result is what the function returns. The body's own `stdout` and `console` go to stderr, so a
  stray `console.log` never corrupts a result.
- **Process.** The runtime starts once the decision is made, in the body's directory, held by the kernel. Its
  life is bounded by `evoke`'s. On `Ctrl-C`, and for a body that will not end, the process group is ended:
  `SIGTERM`, a short grace, then `SIGKILL`.
- **Platform.** On a system `platforms` leaves out, the reflex is inactive, and `add` and `show` say so: `runs on
  macOS only`. When the body runs on one system alone, name it in `platforms`:
  [rule 43](rules.md#9-what-a-body-must-do).

## An argv body

```toml
run = ["networksetup", "-setairportpower", "en0", "{state}"]
```

A program and its arguments, run directly, never through a shell. The first element is a literal: the program.
It is a name found on `PATH`, or an absolute path. It is never a path relative to wherever `evoke` runs. A
placeholder is a whole element. It names an `options`, `vocab` or `pick` argument, and is replaced by the option
key, the word's value or the word, or the pick's value: the number, the seconds, or the text. A date stands as
`2026-05-05`, a time as `17:30`. An element whose optional argument is unstated is dropped. A value that would
start with `-` is refused. Config arrives as `EVOKE_CONFIG_<KEY>`, and the input as `EVOKE_INPUT`. Stdout is the
result. A non-zero exit is failure. No runtime is needed. The program is the body, so it runs without being named
in `runs`, held to what `[needs]` declares.

Flags, an amount, a taken result and literal braces cannot be expressed in an argv. Write a file instead.

## The generated types

```ts
// reflex.d.ts — generated by evoke check; do not edit.
export interface Args {
  /** How long? */
  duration: number
  /** What is the timer for? */
  label?: string
}
export interface Config {}
export interface Context { input: string; config: Config; signal: AbortSignal }
export type Result = string | { text: string; data?: unknown }
export type Reflex = (args: Args, context: Context) => Result | Promise<Result>
```

Option keys become a union: `"on" | "off" | "dim"`. A word, a quoted text, an address, a URL and a code are
`string`. A number and a duration are `number`. A date is a `string`, `2026-05-05`, resolved against the day the
body runs; a time a `string`, `17:30`; an amount `{ amount: number; currency: string }`. A flag is `true`. A taken
result is `unknown`, since `evoke` checks no shape.
An optional argument is marked `?`. Each member carries its `ask` or `about`, so an editor's hover shows the
question. The file imports nothing.

## Testing a body

A body is a function. Import it and call it. The collection's own tests do exactly that, under `node --test`,
with a context built by hand: `{ input: "", config: { file }, signal: new AbortController().signal }`. Nothing of
`evoke` is needed to unit-test a reflex.

**Next:** [Playbooks](playbooks.md).
