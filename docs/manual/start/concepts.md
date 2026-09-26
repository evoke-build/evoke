<!-- description: The words the evoke manual uses, from reflex and manifest to overlay and adapter, and the three steps of every decision: decide, gate and run. -->
# Concepts

Seven words, three steps, and one idea that makes the rest follow.

## The idea

The classifier `evoke` uses answers closed questions: *which of these? is this stated? yes or no?* It answers
with calibrated probabilities. Everything it knows about a
reflex comes from the reflex's own description, sent in the request. So the wording **is** the tuning, and one
line is documentation, tuning, test and customization at once:

```toml
"kill the lights" = { state = "off" }
```

A reflex is a package of such wording plus the program it describes. `evoke` fetches, versions and refines that
wording, and runs the program.

## The words

| Concept        | Meaning                                                                                                   |
| :------------- | :-------------------------------------------------------------------------------------------------------- |
| **Reflex**     | A directory: `reflex.toml` and the file or program it names. Its identity is its location, `owner/repo[/dir]` at a git tag. |
| **Manifest**   | `reflex.toml`: what the classifier reads and what the program receives. Its *wording* is yours to override. Its *contract* is not: argument names, option keys, sources, ranges, what it runs. |
| **Project**    | A directory of files you own: `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`. Your home project lives in `~/.config/evoke/`. An application has its own at its root. |
| **Overlay**    | Your wording for one reflex, in one file, laid over the shipped manifest. It adds and replaces. It never deletes. |
| **Vocabulary** | Your `word = "meaning"` list, shared by every argument that names it. A package never ships or writes one. |
| **Adapter**    | The classifier behind a decision, by name. `jev` and `openjev` both reach Jev, through TypeSafe AI's API or through OpenJEV. The core names no engine. |
| **Runtime**    | What runs a program: Node for a `.mts` or `.mjs` file, nothing for an argv. Chosen from the manifest. Authors never see it. |

Two more words come up everywhere. A **call** is a reflex with its arguments filled in, on one line:
`lights room="den" state="off"`. An **effect** is what running a reflex does to the world: `read`, `write` or
`destructive`. The effect decides how sure `evoke` must be before it runs without asking.

A **tag** is a word a manifest carries, so `--tag` can narrow a decision. A **collection** is a repository of
reflex directories. The **Hub** is where reflexes are found. For now, that is the first-party collection,
`evoke-build/reflexes`.

## The three steps

```text
evoke "mute the office lights"
```

1. **Decide.** Every active reflex's manifest becomes questions. Which reflex does the input ask for? For each
   argument, which value, or was it left unstated? One request carries all the questions. The adapter answers
   each with a probability.
2. **Gate.** Confidence is the weakest of those answers. Each effect has a bar. Against that bar, the outcome is
   settled: **run**, **confirm**, **ask** for a missing argument, or **abstain**. A destructive reflex always
   confirms. Code stays in control: the classifier proposes, the gate disposes.
3. **Run.** The reflex's program runs with its arguments, a clean environment and a deadline. Its one line of text
   comes back.

## What the classifier can and cannot do

It selects. An argument's value is one of five things: one of the author's options, one of
your vocabulary words, a piece of what you typed, taken word for word, or, in a sentence of several steps, a field
of an earlier step's result or that step's whole result, which the plan shows. That piece can be a number, a
duration, an email address, a URL, or a quoted phrase, and it is checked against its range. So a sentence can
choose a call, but it can never invent a value. And what it chooses still passes the gate. That is the shape of
every guarantee in [Security](../security.md).

**Next:** [Typing a request](../use/saying-things.md).
