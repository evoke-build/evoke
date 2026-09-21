# The manifest

`reflex.toml` is what the classifier reads and what the body receives. It has no `name` and no `version`: a reflex's
name is what each user installs it as, and its version is the git tag it was fetched at.

## The whole shape

```toml
reflex = 1                                   # the format's version; frozen

description = """
Turn the lights in one room on, off, or dim them.
Ceiling and lamp lights only."""
not_for = ["colour scenes and schedules", "asking whether a light is on"]
tags    = ["home", "lighting"]
effect  = "write"                            # absent means destructive
confirm = "Set the {room} lights {state}?"
run     = "lights.mts"                       # or argv: ["hue", "set", "{room}", "{state}"]

[config]
bridge = "Hue bridge address"
token  = { about = "Hue API key", secret = true }

[args.room]
ask   = "Which room?"
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

[examples]                                   # sent to the classifier
"turn on the kitchen lights"   = { state = "on" }
"dim the office to 30 percent" = { state = "dim", brightness = "30 percent" }

[tests]                                      # held out, never sent
"make it darker in here" = { state = "dim" }
"light a candle"         = false
```

## Key by key

| Key           | Required | Meaning                                                                                                   |
| :------------ | :------- | :-------------------------------------------------------------------------------------------------------- |
| `reflex`      | yes      | `1`. The format is frozen; keys are never removed or re-meant, so every manifest ever tagged stays readable |
| `description` | yes      | What the reflex does, in the user's words. The first line is the **summary**, as in a git subject: it is what listings and wide rankings show. The rest sharpens the boundary |
| `not_for`     | no       | What it does *not* do — its near neighbours. Read as the *no* side of "does this reflex do what was asked?" |
| `tags`        | no       | Words for `--tag` to narrow a decision by. No other meaning; `[a-z][a-z0-9_]*` |
| `effect`      | no       | `read`, `write` or `destructive`. **Absent means destructive.** An author's claim, untrusted: a user may tighten it, never loosen it. A reflex has one effect, so grouped actions take the worst case |
| `confirm`     | yes      | The one-line question a person answers. A `{placeholder}` names a **required** argument; a pick shows its span. A placeholder for an optional argument or a flag is an error |
| `run`         | yes      | The body: a path ending in `.mts` or `.mjs` inside the directory, or an argv. [The body](body.md) |
| `[config]`    | no       | Settings the user provides with `evoke config`: `key = "about"` or `key = { about, secret = true }`. A secret is only ever set from an environment variable |
| `[args.<name>]` | no     | The arguments: `ask` and exactly one source. [Arguments](arguments.md) |
| `[examples]`  | no       | Utterances with what they assert, sent to the classifier. [Examples and tests](records.md) |
| `[tests]`     | no       | The same shape, held out: never sent, run by `evoke test` |

## Text rules

- Every string is **clean text**: no control characters but the line feed in `description`, no bidi controls.
  `evoke` refuses the rest, so a manifest can never repaint a terminal.
- Names — arguments, tags, config keys, vocabularies — match `[a-z][a-z0-9_]*`. An argument name is also never a
  JavaScript reserved word, so a body can destructure it.
- Option keys are one clean line each; `none` and `unstated` are reserved.
- Unknown keys are reported, never fatal.

## What is contract, what is wording

| Contract — a user cannot override; changing it is a version bump | Wording — a user may override; free to improve at any tag |
| :---------------------------------------------------------------- | :--------------------------------------------------------- |
| `run`; argument names and their sources; option keys; `range`; `config` keys | `description`, `not_for`, `tags`, `confirm`; every `ask`; the meaning of each option; examples and tests |

An argument may be renamed by declaring its former names — `was = ["state"]` — and every user's overlay and call
follows. [Publishing](publishing.md) says how `evoke check` diffs one tag against the next.

## Lint

At `add` and at `check`, lint reports and never refuses: a summary over 100 characters, a description over 1 000,
more than eight `not_for` entries, more than 24 options on one argument, more than 40 records in a table, an
utterance over 200 characters — and any phrase that addresses a model rather than describes an action: *ignore
previous*, *you must*, *always choose*, *as an AI*, *the classifier*. The text is yours to weigh; the finding
names the key.

**Next:** [Arguments](arguments.md).
