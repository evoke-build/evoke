# Arguments

An argument is a value the body receives. Most are also a question the classifier answers about the input: each
of those has an `ask`, and exactly one **source** for its values: the author, the user, or the input. An
argument with `takes` has a fifth source, an earlier step's result, and no `ask`: nothing is asked.

```toml
[args.state]
ask = "What should the lights do?"          # the question a person would be asked; sent verbatim
options.on  = "Switch on."                   # the author's closed set
options.off = "Switch off."

[args.room]
ask   = "Which room?"
vocab = "rooms"                              # the user's closed set: vocab/rooms.toml

[args.brightness]
ask      = "How bright, in percent?"
pick     = "number"                          # a span of the input, read by a built-in recognizer
range    = [1, 100]
optional = true

[args.clipboard]
ask  = "To the clipboard instead of a file?"
flag = true                                  # a yes/no switch
```

## The five sources

| Source    | Values come from             | The body receives                                            | Asked when missing |
| :-------- | :--------------------------- | :----------------------------------------------------------- | :----------------- |
| `options` | The author: `key = "meaning"` | The key                                                     | A numbered choice  |
| `vocab`   | The user: `vocab/<name>.toml` | The word's `value` if set, else the word                    | A numbered choice, plus `[+] add one` |
| `pick`    | The input, as typed          | `number` → the number; `duration` → whole seconds; `email`, `url`, `quoted` → the text | Typed freely, read by the recognizer |
| `flag`    | The input                    | `true`, or nothing                                           | Never              |
| `takes`   | An earlier step of the same request, whose reflex `returns` that name | That step's `data`, as it returned it | Never: the request stops before anything runs |

**`ask`** is the question a person would be asked. The same line serves the classifier and the prompt, so write it
as you would say it: *Which room?*, *How long?*.

**`optional = true`** means an unstated argument is omitted and the body's own default applies. A required
argument left unstated makes `evoke` ask. A flag is optional by nature and takes no `optional`.

## Options

The author's closed set. The key is what the body receives and what a call shows, as in `state="off"`. So keep
keys short and stable. The meaning is what the classifier reads, so make it precise. Keys are one clean line
each, never `none` or `unstated`. Examples that assert an option teach it: `"kill the lights" = { state = "off" }`.

## Vocabularies

The user's closed set, by name. A reflex that names a vocabulary reads whatever the user put in
`vocab/<name>.toml`. A package never ships or writes one. An empty vocabulary makes the reflex inactive until a word is there, when the argument is required. An optional argument over an empty vocabulary is never asked and never stated, and the reflex stays active. The names are a convention the collection sets: `rooms`, `places`, `sites`. That way, two
reflexes asking for a place share one list. A word's `value` is for the body only: a path, a URL, a device id.

A shipped manifest may not assert a vocabulary argument in its records, not even as unstated. The words are not
yours to know. A user's own overlay may.

## Picks

A pick reads a piece of the input, word for word. Five recognizers exist:

| `pick`     | Recognizes                                      | Value             | `range` |
| :--------- | :---------------------------------------------- | :---------------- | :------ |
| `number`   | A number in digits or in words up to a hundred, unit words kept in the span: `30 percent`, `fifty percent`, `twenty-five`; a leading minus is the number's: `-5` | The number | yes |
| `duration` | One number and one unit, the number in digits or in words: `10 minutes`, `2 hours`, `twenty five minutes`; `an hour`, `half an hour`, `a quarter of an hour`, `an hour and a half`, `two and a half hours` | Whole seconds | yes |
| `email`    | An address                                      | The text          | no      |
| `url`      | A URL                                           | The text          | no      |
| `quoted`   | `"…"`, `“…”` or `‘…’`; a straight single quote is an apostrophe | The text between the quotes | no |

- The classifier chooses among the candidates found. It never invents one. A pick with no candidate reads as
  unstated. If it is required, `evoke` asks for it.
- Quotes hide what they enclose. A typed span hides the bare numbers inside it. In *timer for 3 minutes called
  "eggs"*, the duration is `3 minutes`, the quoted text is `eggs`, and `3` alone is never a number.
- `range = [min, max]` compares the **value**, in seconds for a duration. It is allowed on `number` and `duration`
  only. Out of range, `evoke` asks, with the reason: `150 percent is outside 0–100`.
- In a `confirm` template, the placeholder shows the span. In an argv, it shows the value's text.

## Flags

`flag = true` is a yes/no switch. The classifier reads it as yes at or above one half. The body receives `true`,
or nothing. A flag never appears in a `confirm` template, and it cannot be an argv element. A body that needs one
is a file. In records, a flag is asserted as `{ clipboard = true }`.

## A whole result taken

```toml
[args.deploys]
takes = "deploys"                            # an earlier step's whole result, by the name its manifest returns
```

A `takes` argument is required, and stands alone: no `ask`, no `optional`, no `range`, no `was`. Its reflex runs
only after a step that returns the name, in one sentence ([Weaving](../use/weaving.md#a-step-that-takes-whole-results)):
alone, when no step before it returns the name, or when two do, the request stops before anything runs and says
which. The argument's own name is the author's; the plan and `evoke why` print the result's. The body receives the
`data` as its source returned it, and checks its shape itself: `reflex.d.ts` types it `unknown`. A confirm cannot
show a whole result, so a reflex that runs destructive takes none: it takes what it acts on as a field of
`[yields]`, which its confirm names. `evoke run` cannot give a `takes` argument, and no sentence states one.

## Names and renames

An argument name matches `[a-z][a-z0-9_]*`. It is never a JavaScript reserved word, like `for`, `class` or
`default`, so a body can destructure it. To rename one, declare the old name and keep it declared forever:

```toml
[args.power]
ask = "What should the lights do?"
was = ["state"]
```

Every user's overlay and every call written with the old name follows the rename at merge time. `was` is flat
and cumulative. A retired name never returns as a live argument, and it never leaves the list. `evoke check`
enforces both against the previous tag. `update` tells each user that their records followed.

## In the confirm line and in an argv

`confirm = "Set the {room} lights {state}?"` names required arguments only, never one with `takes`. In an argv
`run`, a placeholder is a whole element, as in `["hue", "set", "{room}", "{state}"]`. It names an `options`, `vocab`
or `pick` argument. An element whose optional argument is unstated is dropped. Literal braces, flags and a taken
result need a file body.

**Next:** [The body](body.md).
