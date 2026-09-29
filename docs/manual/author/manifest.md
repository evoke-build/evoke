# The manifest

`reflex.toml` is what the classifier reads and what the body receives. It has no `name` and no `version`. A
reflex's name is whatever each user installs it as. Its version is the git tag it was fetched at.

This page says what each key is and what `evoke` does with it. [Rules for reflexes, playbooks and
collections](rules.md) says how to write each one so that what people type reaches the right reflex.

## The whole shape

```toml
reflex = 1                                   # the format's version; frozen

description = """
Turn the lights in one room on, off, or dim them.
Ceiling and lamp lights, through the Hue bridge; their colours and schedules stay as they are."""
not_for = ["colour scenes and schedules", "asking whether a light is on"]
tags    = ["home", "lighting"]
effect  = "write"                            # always written; absent means destructive
confirm = "Set the {room} lights {state}?"
run     = "lights.mts"                       # or argv: ["hue", "set", "{room}", "{state}"]

[needs]                                      # what the body touches; absent means its own directory and TMPDIR
hosts = ["*"]                                # the bridge is on the network

[config]
bridge = "Hue bridge address"
token  = { about = "Hue API key", secret = true }

[args.room]
ask   = "Which room of the house?"
vocab = "rooms"                              # options come from the user

[args.state]
ask = "What should the lights do?"
options.on  = "Switch on."                   # options come from the author
options.off = "Switch off."
options.dim = "Lower the brightness without switching off."

[args.brightness]
ask      = "How bright, in percent?"
pick     = "number"                          # options come from the input
range    = [1, 100]
optional = true

[yields]                                     # what the returned data holds, for a later step
level = "number"

[examples]                                   # sent to the classifier
"turn on the kitchen lights"   = { state = "on" }
"dim the office to 30 percent" = { state = "dim", brightness = "30 percent" }
"bedroom lights off"           = { state = "off" }

[tests]                                      # held out, never sent
"make it darker in here"           = { state = "dim", brightness = false }
"flip the hall lights"             = { state = false }
"is the porch light still on"      = false
"set the lounge to a sunset scene" = false
"light a candle"                   = false
```

After `reflex = 1`, the keys are written in the order `evoke show` prints them, which is the order of the table
below. Within an argument, `ask` comes first and its source next, then `range`, `recent`, `optional` and `was`.
The arguments with `takes` come after the others. In TOML a plain key written below a table header belongs to
that table, so the plain keys come first: [rule 36](rules.md#7-what-a-person-reads).

## Key by key

| Key           | Required | Meaning                                                                                                   |
| :------------ | :------- | :-------------------------------------------------------------------------------------------------------- |
| `reflex`      | yes      | `1`. The format is frozen. Keys are never removed or given new meanings, so every manifest ever tagged stays readable |
| `description` | yes      | What the reflex does. The first line is the **summary**, never empty. The classifier reads all of it, not the summary alone. The summary says the action and what it acts on, and the next line draws the boundary: [rule 11](rules.md#3-words-that-reach-the-right-reflex) |
| `not_for`     | no       | What it does *not* do, one line per entry. It is read as the *no* side of "does this reflex do what was asked?". It names the near neighbours: [rule 13](rules.md#3-words-that-reach-the-right-reflex) |
| `tags`        | no       | Words for `--tag` to narrow a decision by. No other meaning. `[a-z][a-z0-9_]*` |
| `effect`      | always   | `read`, `write` or `destructive`. It sets how sure `evoke` must be before a call runs unasked, and a destructive call asks for a yes every time. **Left out, it still means destructive**, and lint reports `effect is absent, which means destructive; write it`. It is the author's claim, and not trusted: a user may tighten it, never loosen it. A reflex has one effect. Claim the one the action has: [rule 33](rules.md#7-what-a-person-reads) |
| `confirm`     | yes      | The one-line question a person answers. A `{placeholder}` names a **required** argument. A pick shows its span. `evoke check` refuses a placeholder for an optional argument, a flag or a taken result. The confirm reads the call back: [rule 23](rules.md#4-words-that-read-each-value) |
| `run`         | one of   | The body: a path ending in `.mts` or `.mjs` inside the directory, or an argv. [The body](body.md) |
| `steps`       | one of   | A plan in place of a body: one sentence per step, `{slot}` for an argument, `[ words with a {slot}]` for an optional one, `{ say = "…", when = { field = "value" } }` for a step that runs only under what the step before it yields; at most 24. None of a body's keys with it. [Playbooks](playbooks.md) |
| `platforms`   | no       | Where the body runs, when not anywhere: `["macos"]`, `["linux"]`, or both. On another machine the reflex is inactive, and `add` and `show` say so: `runs on macOS only` |
| `returns`     | no       | The name of what the body's `data` is, whole, for a later step to take: `returns = "deploys"`. Not with an argv `run`. [A result another step takes whole](#a-result-another-step-takes-whole) |
| `[needs]`     | no       | What the body touches, held by the kernel: `reads`, `writes`, `hosts`, `runs`. Left out, the tightest declaration: its own directory and `TMPDIR`. [What the body touches](#what-the-body-touches) |
| `[config]`    | no       | Settings the user provides with `evoke config`: `key = "about"` or `key = { about, secret = true }`. A secret is only ever set from an environment variable |
| `[args.<name>]` | no     | The arguments: an `ask` and exactly one source, or `takes` alone. [Arguments](arguments.md) |
| `[yields]`    | no       | What the body's `data` holds, for a later step to take ([Weaving](../use/weaving.md)) or for a pick's `recent` to recall ([Arguments](arguments.md#a-value-recalled)): per field, the recognizer that reads it, `number`, `duration`, `email`, `url`, `quoted`, `date`, `time`, `amount` or `code`; or `{ each = { … } }` for a list of records. Declare each field a later step takes, by the noun a person would type for it: [rules 30 and 31](rules.md#6-how-results-connect) |
| `[examples]`  | no       | Utterances with what they assert, sent to the classifier. [Examples and tests](records.md) |
| `[tests]`     | no       | The same shape, held out: never sent, run by `evoke test` |

## A result another step takes whole

A body may name what it returns: `returns = "deploys"`. The name says what the body's `data` is, as a whole. A
later step whose argument has `takes = "deploys"` receives that `data` as the body returned it
([Arguments](arguments.md#a-whole-result-taken)). Two reflexes meet when their names are equal, as two arguments
share a vocabulary by naming it. Give each whole result one noun and one shape across the collection:
[rule 32](rules.md#6-how-results-connect). A reflex never returns a name it takes: a step that refines a result
returns a new name, so a later step has one source. An argv body returns no data, so a reflex whose `run` is an
argv names none, and declares no `[yields]` either.

## What the body touches

```toml
[needs]
reads  = ["{place}"]                         # the value of an argument or a config key
writes = ["~/Downloads", "{to}"]             # under the home; or absolute, like "/tmp/out"
hosts  = ["*"]                               # the network, all of it; absent means none
runs   = ["open", "/usr/bin/plutil"]         # by name on PATH, or by absolute path
```

Each key is a list, and leaving the table out is the tightest declaration: the body's own directory and its
private `TMPDIR`, nothing else. `reads` and `writes` name paths: `~/…` under the home, absolute, or `{name}` for
the value of an argument or a config key, so a folder the user names reaches the body and nothing beside it. A
path names a file, or a folder with everything under it, whichever is on the disk when the body runs. A path in
`writes` may be read too. **A declared path must exist when the body runs.** A body that makes a file declares
its folder, and a path the machine lacks stops the run before the body starts: `[needs] reads names ~/nowhere,
which is not there`; a folder in `writes` ends in the `mkdir -p` that makes it. `hosts` is all or nothing:
`["*"]` reaches every host, absent reaches none, and a name is refused at `check`. `runs` names programs, by name
on `PATH` or by absolute path; each of them runs held to the same declaration, and what it asks the system to do
happens outside it. The kernel holds the body to all of this: [Security](../security.md#what-runs-and-as-whom).

A `{name}` names an argument that carries a value, a word's or a typed one, or a config key; `evoke check`
refuses one that names neither, or a flag. An entry over an optional argument left unstated is dropped. A value
that is not a path, a word whose value is a name rather than a folder, refuses the run and points at the value's
source: `[needs] writes names {to}, whose value "desk" is not a path  →  evoke vocab places add desk "<meaning>"
--value <path>`.

A reach past the declaration ends the run with what was reached and the key, `~/secret.txt is not in [needs]
reads`, and a fix by where the declaration is written: this manifest's line for a local reflex; for a fetched
one, `evoke update --accept` when upstream's declaration already allows it, else `evoke remove`. `[needs]` is
contract: an overlay cannot touch it; upstream may narrow it at any tag, which is `same`; widening waits for the
user's `evoke update --accept`, and is `minor`: [Publishing](publishing.md).

## Text rules

- Every string is **clean text**: no control characters, except the line feed in `description`, and no bidi
  controls. `evoke` refuses the rest, so a manifest can never repaint a terminal.
- Names of arguments, tags, config keys and vocabularies match `[a-z][a-z0-9_]*`. An argument name is also never
  a JavaScript reserved word, so a body can destructure it.
- Option keys are one clean line each. `none` and `unstated` are reserved.
- Unknown keys are reported by `evoke check`, never fatal: `unknown  lights: warmth is not a key this evoke
  reads`. The schema at `https://evoke.build/schemas/reflex.json` is strict, so an editor flags a typo before
  `evoke check` does.

## What is contract, what is wording

| Contract: a user cannot override it, and changing it is a version bump | Wording: a user may override it, and you may improve it at any tag |
| :---------------------------------------------------------------- | :--------------------------------------------------------- |
| `run` or `steps`; `platforms`; `[needs]`; argument names and their sources; option keys; `range`; `recent`; `config` keys; `yields`; `returns`; what an argument takes | `description`, `not_for`, `tags`, `confirm`; every `ask`; the meaning of each option; examples and tests |

An argument may be renamed by declaring its former names, like `was = ["state"]`. Every user's overlay and call
then follows. [Publishing](publishing.md) says how `evoke check` diffs one tag against the next.

## Lint

At `evoke check` and `evoke add`, lint reports and never refuses. It flags a text or a list longer than its cap,
a phrase that addresses a model, a step a plan would misread, and what a reader of the file would miss, like
`effect` left out or fewer than three examples. Each finding names the key, and the text is yours to weigh.
[Diagnostics](../reference/diagnostics.md#what-lint-reports) lists every line.

**Next:** [Arguments](arguments.md).
