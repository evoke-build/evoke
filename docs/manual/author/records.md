# Examples and tests

Two tables, one shape. `[examples]` are sent to the classifier: they teach. `[tests]` are held out, never sent:
they check. One line —

```toml
"kill the lights" = { state = "off" }
```

— is documentation, tuning and test at once, and a user's overlay adds lines of the same shape.

## The shape

```toml
[examples]
"turn on the kitchen lights"                = { state = "on" }          # asserts the route and one argument
"lock the screen"                           = {}                        # asserts the route alone
'timer for 3 minutes called "eggs"'         = { duration = "3 minutes", label = "eggs" }
"grab part of the screen to the clipboard"  = { area = "selection", clipboard = true }
"take a screenshot"                         = { area = false }          # asserts that area is unstated

[tests]
"make it darker in here" = { state = "dim" }
"light a candle"         = false                                        # never this reflex
```

| Assertion            | Means                                                                          |
| :------------------- | :----------------------------------------------------------------------------- |
| `{}`                 | The input routes here; nothing said about arguments                            |
| `{ arg = "key" }`    | An option: its key                                                              |
| `{ arg = "span" }`   | A pick: the **verbatim span**, which must occur in the utterance; for `quoted`, the text between the quotes |
| `{ arg = false }`    | The argument is unstated                                                        |
| `{ flag = true }`    | The flag is raised                                                              |
| `false`              | Never this reflex. Such an example teaches the *no* side, alongside `not_for`   |

An argument a record does not name is not asserted. A shipped manifest may not assert a `vocab` argument — the
words are the user's — and a user's overlay may.

## Identity

Records are keyed by the utterance as written, and the classifier reads that spelling. Two records that are the
same utterance under **identity** — NFC, lower-cased, whitespace collapsed and trimmed, trailing `.` `!` `?` `…`
dropped — are one record, and duplicates in one file are refused. A user's line naming a shipped utterance
replaces it, so the highest layer decides each utterance's table and value.

## How many, and which

Each reflex in the collection carries at least three examples and three tests, one of the tests `false`. Cover
each option at least once; cover each pick; include one *unstated* case for an optional argument; and make the
`false` cases the near neighbours — what a person might say that this reflex must not take.

## `evoke test`

```text
$ evoke test
  lights  3 passed · 1 failed
    "make it darker in here"  state: expected "dim", read "off"
  timer   8 passed
  1 of 12 cases failed  →  evoke test
[1]
```

Every example and every test of every active reflex is decided over the whole installed set — the classifier
sees the other reflexes too, so a near neighbour installed beside yours is part of the test — never through the
cache and never logged. A case passes on its route and each asserted argument:

- a `false` case passes on abstain or on another reflex;
- an ask reads its missing arguments as unstated;
- a confirm is judged by its call, not by whether it would have run;
- an argument the record does not name is not compared.

A case that passed at the last run and fails now is decided twice more, and two of three failing marks it a
**regression**. `test` exits 1 when a case failed and never blocks an install.

## Lint

Records count against lint: at most 40 per table, and 200 characters per utterance, reported at `add` and `check`.

**Next:** [Wording](wording.md).
