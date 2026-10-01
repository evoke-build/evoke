<!-- description: A reflex's examples teach the classifier and its held-out tests check it. How to write both, and how evoke test reports each miss and each regression. -->
# Examples and tests

Two tables, one shape. `[examples]` are sent to the classifier: they teach. `[tests]` are held out, never sent:
they check. One line is documentation, tuning and test at once:

```toml
"kill the lights" = { state = "off" }
```

A user's overlay adds lines of the same shape.

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
| `{ arg = "span" }`   | A pick: the **exact span**, which must occur in the utterance. For `quoted`, the text between the quotes |
| `{ arg = false }`    | The argument is unstated                                                        |
| `{ flag = true }`    | The flag is raised                                                              |
| `false`              | Never this reflex. Such an example teaches the *no* side, alongside `not_for`   |

An argument a record does not name is not asserted. A shipped manifest may not assert a `vocab` argument,
because the words are the user's. A user's overlay may.

## Identity

Records are keyed by the utterance as written, and the classifier reads that spelling. Two records can still be
the same utterance under **identity**. Identity means NFC, lower-cased, whitespace collapsed and trimmed, and a
trailing `.` `!` `?` or `…` dropped. Such records are one record, and duplicates in one file are refused. A
user's line naming a shipped utterance replaces it. So the highest layer decides each utterance's table and
value.

## Which records to write

[The rules](rules.md) say which records to write, and why:

- At least three examples, as people type the request: [rule 14](rules.md#3-words-that-reach-the-right-reflex).
- Each near neighbour, a sentence that sounds like the reflex's request and is not, as a `false` test in words no
  example uses: [rule 15](rules.md#3-words-that-reach-the-right-reflex).
- For each option or pick argument, one record that leaves it out, `{ area = false }`:
  [rule 15](rules.md#3-words-that-reach-the-right-reflex).
- No record that opens with a negation or a condition: [rule 15](rules.md#3-words-that-reach-the-right-reflex).
- For a playbook, records that state the situation it handles, never one of its steps:
  [rule 6](rules.md#2-when-to-write-a-playbook).

## `evoke test`

```text
$ evoke test
  lights  3 passed · 1 failed
    "make it darker in here"  state: expected "dim", read "off"
  timer   8 passed · 1 held
    "ping me in 2 hours"  held: the call holds all of it at 0.20, and a call runs at 0.30 or more
  1 of 12 cases failed  →  evoke test
[1]
```

Every example and every test of every active reflex is decided over the whole installed set. The classifier
sees the other reflexes too, so a near neighbour installed next to yours is part of the test. It never goes
through the cache, and it is never logged. A case passes on its route and on each asserted argument:

- A `false` case passes on abstain, or on another reflex.
- An ask reads its missing arguments as unstated.
- A confirm is judged by its call, not by whether it would have run.
- An argument the record does not name is not compared.

A case that passed at the last run and fails now is decided twice more. Two of three failing marks it a
**regression**. `test` exits 1 when a case failed. It never blocks an install.

A case that passes while its call would wait for a yes, because the call holds less than the case's own words,
is named under its reflex as `held`, with how far the call holds them. It fails nothing. The cure is the
summary: write it so that the case reads as what the reflex does
([rule 11](rules.md#3-words-that-reach-the-right-reflex)).

A playbook's steps are tested after its records ([Playbooks](playbooks.md)): each filled from the first record
whose reading fills every required slot it holds, decided over the whole set, and passed when it reaches a reflex
other than its own playbook. Its line counts them, `outage  14 passed · 7 steps route`, and a step that reaches
nothing, or its own plan, fails and prints under it. These print under it too, and never fail:

- A step no record fills, marked untested.
- The step before a branch, when its reflex yields no field the branch waits on.
- A step that reaches a tighter effect than the playbook claims.

## Lint

Records count against lint: at most 40 per table, and 200 characters per utterance. Lint also reports fewer than
three examples that are not `false`, an option or pick argument that no record leaves out, and a `quoted` argument
that no example shows in quotes. Lint reports at `add` and `check`.

**Next:** [The rules](rules.md).
