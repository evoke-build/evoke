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
ask   = "Which room of the house?"
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
| `pick`    | The input, as typed          | `number` → the number; `duration` → whole seconds; `email`, `url`, `quoted`, `code` → the text; `date` → the day, `2026-05-05`; `time` → `17:30`; `amount` → `{ amount, currency }` | Typed freely, read by the recognizer; a `recent` field's values numbered before it |
| `flag`    | The input                    | `true`, or nothing                                           | Never              |
| `takes`   | An earlier step of the same request, whose reflex `returns` that name | That step's `data`, as it returned it | Never: the request stops before anything runs |

A value from an open set is a pick, a switch is a flag, options are for a closed set, and a value only the user
knows is a vocabulary: [rule 4](rules.md#1-one-reflex-one-action) and
[rules 19 and 20](rules.md#4-words-that-read-each-value).

**`ask`** is the question a person would be asked. The same line serves the classifier and the prompt. Each ask is
one question, as a person would ask it: [rule 17](rules.md#4-words-that-read-each-value).

**`optional = true`** means an unstated argument is omitted and the body's own default applies. A required
argument left unstated makes `evoke` ask. A flag is optional by nature and takes no `optional`.

## Options

The author's closed set. The key is what the body receives and what a call shows, as in `state="off"`. The
meaning is what the classifier reads. Keys are one clean line each, never `none` or `unstated`. Examples that
assert an option teach it: `"kill the lights" = { state = "off" }`. A key is a word a person knows at a glance,
and its meaning answers the ask in one short line: [rule 18](rules.md#4-words-that-read-each-value).

## Vocabularies

The user's closed set, by name. A reflex that names a vocabulary, `vocab = "<name>"`, reads the words the user put
in `vocab/<name>.toml`. A package never ships or writes one. An empty vocabulary makes the reflex inactive until a
word is there, when the argument is required. An optional argument over an empty vocabulary is never asked and
never stated, and the reflex stays active. A word's `value` is for the body only: a path, a URL, a device id.
Reflexes share a list only by naming it alike, so each kind of thing has one published name:
[Vocabulary names](rules.md#vocabulary-names).

A shipped manifest may not assert a vocabulary argument in its records, not even as unstated. The words are not
yours to know. A user's own overlay may.

## Picks

A pick reads a piece of the input, word for word. Nine recognizers exist:

| `pick`     | Recognizes                                      | Value             | `range` |
| :--------- | :---------------------------------------------- | :---------------- | :------ |
| `number`   | A number in digits or in words up to a hundred, unit words kept in the span: `30 percent`, `fifty percent`, `twenty-five`; a leading minus is the number's: `-5` | The number | yes |
| `duration` | One number and one unit, the number in digits or in words: `10 minutes`, `2 hours`, `twenty five minutes`; `an hour`, `half an hour`, `a quarter of an hour`, `an hour and a half`, `two and a half hours` | Whole seconds | yes |
| `email`    | An address                                      | The text          | no      |
| `url`      | A URL                                           | The text          | no      |
| `quoted`   | `"…"`, `“…”` or `‘…’`; a straight single quote is an apostrophe | The text between the quotes | no |
| `date`     | A day: `today`, `tomorrow`, `tonight`, `yesterday`, `the day after tomorrow`; a weekday, alone or after `next`, `this` or `last`; a day of the month: `on the 14th`, `friday the 14th`; a month and a day, with a year or not: `may fifth`, `March 6th 2017`, `the 22nd of march`; `in two weeks`, `three days from now`; `2026-05-05`; `27/03/2017` when one order alone is possible. Never a period, a month or a year alone, `every monday` or `now` | The day, `2026-05-05`, on your machine's calendar when the program runs | no |
| `time`     | A clock time with its half of the day: `5pm`, `7 a.m.`, `5:30 pm`, `seven thirty am`, `six in the morning`, `4 o'clock in the afternoon`, `noon`, `midnight`; or a two-digit hour on the 24-hour clock: `13:00`, `09:30`. An hour word alone is a number: `at three`. `5:30` and `4 o'clock` with no half read as nothing at all. Type `3pm`, `15:00` or `05:30`. `in ten minutes` is a duration | The time, `17:30` | no |
| `amount`   | A figure or number words with a currency: `$30`, `€1,200`, `£19.99`, `50 dollars`, `twelve hundred euros`, `1000 USD`. `$` is the US dollar. A currency alone is nothing; `pound` after a count is a weight, `pounds` is money | The number and the currency's code, `{ amount, currency }` | no |
| `code`     | An identifier as typed, in capitals or small letters: a version `4.12.0`, a ticket `INC-311`, a flight `tp1043`, a serial `C02XK1ABJG5M`; quoted or not. `order 4821` is a number, `10mins` a duration | The text | no |

- The classifier chooses among the candidates found. It never invents one. A pick with no candidate reads as
  unstated. If it is required, `evoke` asks for it.
- Text people type freely is never a `quoted` argument, and the body reads it from `input`:
  [rule 21](rules.md#4-words-that-read-each-value).
- Quotes hide what they enclose. A typed span hides the bare numbers inside it. In *timer for 3 minutes called
  "eggs"*, the duration is `3 minutes`, the quoted text is `eggs`, and `3` alone is never a number. A date, a
  time, an amount or a code hides its figures the same way: in *pay €1,200 on may fifth at 5pm* no bare number
  is a candidate.
- A span that a recognizer reads and no argument takes is an **unconsumed span**: it stops the call at confirm, a
  bare number excepted ([Outcomes](../use/outcomes.md)). A vocabulary word that the person typed is never one,
  even where a recognizer reads it too: `tomorrow` in a vocabulary of days is the word, not a date.
- A date is resolved when the program runs, against your machine's clock in its own time zone, or against
  `EVOKE_TODAY` when it is set ([Environment](../reference/environment.md)). Before that, everywhere `evoke` shows
  the value, it shows your words; in a plan file, `tomorrow` means the tomorrow of the run.
- `range = [min, max]` compares the **value**, in seconds for a duration. It is allowed on `number` and `duration`
  only. Out of range, `evoke` asks, with the reason: `150 percent is outside 0–100`.
- In a `confirm` template, the placeholder shows the span. In an argv, it shows the value's text; an amount
  cannot stand in an argv.

## A value recalled

```toml
[args.release]
ask    = "From which release?"
pick   = "code"
recent = "release"                           # what earlier results of this session hold under that field
```

A pick may name a field of `[yields]` with `recent`. When the words leave the argument unstated, `evoke` lists
the values the bodies of this session returned under that field, newest first and five at most, each one the
recognizer reads whole, and asks:

```text
> list the checkout deploys
  deploys service="checkout"  0.90
checkout: 2 deploys today, the last 4.12.0 at 13:58
> roll back the last deploy
  From which release?  [1] 4.12.0  [2] 4.11.3  > 1
  rollback release="4.12.0" · destructive · weakest: route 0.90
  Roll back 4.12.0?  [y]es [n]o [t]each > y
```

A number picks one; anything else is read as typed. A `number` pick names the values as hints instead, since a
number typed is its own answer. The session is one process: the REPL and the stdin filter remember every result,
a one-shot call remembers none, and in the SDK an application hands the results it keeps. A sentence of several
steps takes nothing from memory: its second step asks as it always did. The classifier is never asked about a
recalled value; you choose, and `evoke why` names what was offered. `recent` is contract, beside a pick only.

## Flags

`flag = true` is a yes/no switch. `evoke` reads the classifier's answer as yes at or above one half. The body
receives `true`, or nothing. A flag never appears in a `confirm` template, and it cannot be an argv element. A body
that needs one is a file. In records, a flag is asserted as `{ clipboard = true }`.

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
`default`, so a body can destructure it. When one kind of value plays two roles, each role is an argument of its
own, named by the role: [rule 3](rules.md#1-one-reflex-one-action). To rename an argument, declare the old name
and keep it declared forever:

```toml
[args.power]
ask = "What should the lights do?"
was = ["state"]
```

Every user's overlay and every call written with the old name follows the rename at merge time. `was` is flat
and cumulative. A retired name never returns as a live argument, and it never leaves the list. `evoke check`
enforces both against the previous tag. `update` tells each user that their records followed.

## In the confirm line and in an argv

`confirm = "Set the {room} lights {state}?"` names required arguments only, never one with `takes`. The confirm
reads the call back: [rule 23](rules.md#4-words-that-read-each-value). In an argv `run`, a placeholder is a whole
element, as in `["hue", "set", "{room}", "{state}"]`. It names an `options`, `vocab` or `pick` argument; a date
stands as `2026-05-05`, a time as `17:30`. An element whose optional argument is unstated is dropped. Literal
braces, flags, an amount and a taken result need a file body.

**Next:** [The body](body.md).
