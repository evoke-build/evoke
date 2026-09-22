# The manifest

`reflex.toml` is what the classifier reads and what the body receives. It has no `name` and no `version`. A
reflex's name is whatever each user installs it as. Its version is the git tag it was fetched at.

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

[yields]                                     # what the returned data holds, for a later step
level = "number"

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
| `reflex`      | yes      | `1`. The format is frozen. Keys are never removed or given new meanings, so every manifest ever tagged stays readable |
| `description` | yes      | What the reflex does, in the user's words. The first line is the **summary**, like a git subject line. Listings and wide rankings show it. The rest sharpens the boundary |
| `not_for`     | no       | What it does *not* do: its near neighbours. It is read as the *no* side of "does this reflex do what was asked?" |
| `tags`        | no       | Words for `--tag` to narrow a decision by. No other meaning. `[a-z][a-z0-9_]*` |
| `effect`      | no       | `read`, `write` or `destructive`. **Absent means destructive.** It is the author's claim, and not trusted: a user may tighten it, never loosen it. A reflex has one effect, so grouped actions take the worst case |
| `confirm`     | yes      | The one-line question a person answers. A `{placeholder}` names a **required** argument. A pick shows its span. A placeholder for an optional argument or a flag is an error |
| `run`         | yes      | The body: a path ending in `.mts` or `.mjs` inside the directory, or an argv. [The body](body.md) |
| `[config]`    | no       | Settings the user provides with `evoke config`: `key = "about"` or `key = { about, secret = true }`. A secret is only ever set from an environment variable |
| `[args.<name>]` | no     | The arguments: `ask` and exactly one source. [Arguments](arguments.md) |
| `[yields]`    | no       | What the body's `data` holds, for a later step to take ([Weaving](../use/weaving.md)): per field, the kind that reads it, `number`, `duration`, `email`, `url` or `quoted`; or `{ each = { … } }` for a list of records |
| `[examples]`  | no       | Utterances with what they assert, sent to the classifier. [Examples and tests](records.md) |
| `[tests]`     | no       | The same shape, held out: never sent, run by `evoke test` |

## Text rules

- Every string is **clean text**: no control characters, except the line feed in `description`, and no bidi
  controls. `evoke` refuses the rest, so a manifest can never repaint a terminal.
- Names of arguments, tags, config keys and vocabularies match `[a-z][a-z0-9_]*`. An argument name is also never
  a JavaScript reserved word, so a body can destructure it.
- Option keys are one clean line each. `none` and `unstated` are reserved.
- Unknown keys are reported, never fatal.

## What is contract, what is wording

| Contract: a user cannot override it, and changing it is a version bump | Wording: a user may override it, and you may improve it at any tag |
| :---------------------------------------------------------------- | :--------------------------------------------------------- |
| `run`; argument names and their sources; option keys; `range`; `config` keys; `yields` | `description`, `not_for`, `tags`, `confirm`; every `ask`; the meaning of each option; examples and tests |

An argument may be renamed by declaring its former names, like `was = ["state"]`. Every user's overlay and call
then follows. [Publishing](publishing.md) says how `evoke check` diffs one tag against the next.

## Lint

At `add` and at `check`, lint reports and never refuses. It flags a summary over 100 characters, a description
over 1 000, more than eight `not_for` entries, more than 24 options on one argument, more than 40 records in a
table, or an utterance over 200 characters. It also flags any phrase that addresses a model instead of describing
an action: *ignore previous*, *you must*, *always choose*, *as an AI*, *the classifier*. The text is yours to
weigh. The finding names the key.

**Next:** [Arguments](arguments.md).
