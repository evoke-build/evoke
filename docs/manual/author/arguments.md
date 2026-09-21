# Arguments

An argument is a question the classifier answers about the input and a value the body receives. Each has an
`ask`, and exactly one **source** for its values: the author, the user, or the input.

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

## The four sources

| Source    | Values come from             | The body receives                                            | Asked when missing |
| :-------- | :--------------------------- | :----------------------------------------------------------- | :----------------- |
| `options` | The author: `key = "meaning"` | The key                                                     | A numbered choice  |
| `vocab`   | The user: `vocab/<name>.toml` | The word's `value` if set, else the word                    | A numbered choice, plus `[+] add one` |
| `pick`    | The input, verbatim          | `number` → the number; `duration` → whole seconds; `email`, `url`, `quoted` → the text | Typed freely, read by the recognizer |
| `flag`    | The input                    | `true`, or nothing                                           | Never              |

**`ask`** is the question a person would be asked. The same line serves the classifier and the prompt, so write it
as you would say it: *Which room?*, *How long?*.

**`optional = true`** means an unstated argument is omitted and the body's own default applies. A required
argument left unstated makes `evoke` ask. A flag is optional by nature and takes no `optional`.

## Options

The author's closed set. The key is what the body receives and what a call shows — `state="off"` — so keep keys
short and stable; the meaning is what the classifier reads, so make it precise. Keys are one clean line each,
never `none` or `unstated`. Examples that assert an option teach it: `"kill the lights" = { state = "off" }`.

## Vocabularies

The user's closed set, by name. A reflex that names a vocabulary reads whatever the user put in
`vocab/<name>.toml`; a package never ships or writes one, and an empty vocabulary makes the reflex inactive until
the user adds a word. Names are a convention the collection sets — `rooms`, `places`, `sites` — so that two reflexes
asking for a place share one list. A word's `value` is for the body only: a path, a URL, a device id.

A shipped manifest may not assert a vocabulary argument in its records, not even as unstated: the words are not
yours to know. A user's own overlay may.

## Picks

A pick reads a verbatim span of the input. Five recognizers exist:

| `pick`     | Recognizes                                      | Value             | `range` |
| :--------- | :---------------------------------------------- | :---------------- | :------ |
| `number`   | A bare number, unit words kept in the span: `30 percent` | The number  | yes     |
| `duration` | `10 minutes`, `2 hours`, `90 seconds`, `1 hour 30 minutes` | Whole seconds | yes  |
| `email`    | An address                                      | The text          | no      |
| `url`      | A URL                                           | The text          | no      |
| `quoted`   | `"…"`, `“…”` or `‘…’`; a straight single quote is an apostrophe | The text between the quotes | no |

- The classifier chooses among the candidates found — it never invents one. A pick with no candidate reads
  unstated; required, it is asked for.
- Quotes hide what they enclose, and a typed span masks the bare numbers inside it: in *timer for 3 minutes called
  "eggs"*, the duration is `3 minutes`, the quoted text `eggs`, and `3` alone is never a number.
- `range = [min, max]` compares the **value** — seconds for a duration — and is allowed on `number` and `duration`
  only. Out of range, `evoke` asks with the reason: `150 percent is outside 0–100`.
- In a `confirm` template the placeholder shows the span; in an argv, the value's text.

## Flags

`flag = true` is a yes/no switch. The classifier reads it as yes at or above one half; the body receives `true` or
nothing. A flag never appears in a `confirm` template and cannot be an argv element; a body that needs one is a
file. In records, a flag is asserted as `{ clipboard = true }`.

## Names and renames

An argument name matches `[a-z][a-z0-9_]*` and is not a JavaScript reserved word — `for`, `class`, `default` —
so a body can destructure it. To rename one, declare the old name and keep it declared forever:

```toml
[args.power]
ask = "What should the lights do?"
was = ["state"]
```

Every user's overlay and every call written with the old name follows the rename at merge time. `was` is flat
and cumulative; a retired name never returns as a live argument and never leaves the list. `evoke check`
enforces both against the previous tag, and `update` tells each user their records followed.

## In the confirm line and in an argv

`confirm = "Set the {room} lights {state}?"` names required arguments only. In an argv `run`, a placeholder is a
whole element — `["hue", "set", "{room}", "{state}"]` — and names an `options`, `vocab` or `pick` argument; an
element whose optional argument is unstated is dropped. Literal braces and flags need a file body.

**Next:** [The body](body.md).
