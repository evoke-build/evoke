# Concepts

Seven words, three steps, and one idea that makes the rest follow.

## The idea

The classifier `evoke` uses is never trained. It answers closed questions — *which of these? is this stated? yes
or no?* — with calibrated probabilities, and everything it knows about a reflex comes from the reflex's own
description, in the request. So the wording **is** the tuning, and one line —

```toml
"kill the lights" = { state = "off" }
```

— is documentation, tuning, test and customization at once. A reflex is a package of such wording plus the
program it describes; `evoke` fetches, versions and refines that wording, and runs the program.

## The words

| Concept        | Meaning                                                                                                   |
| :------------- | :-------------------------------------------------------------------------------------------------------- |
| **Reflex**     | A directory: `reflex.toml` and the file or program it names. Its identity is its location — `owner/repo[/dir]` at a git tag. |
| **Manifest**   | `reflex.toml`: what the classifier reads and what the body receives. Its *wording* is yours to override; its *contract* — argument names, option keys, sources, ranges, what it runs — is not. |
| **Project**    | A directory of files you own: `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`. Your home project lives in `~/.config/evoke/`; an application has its own at its root. |
| **Overlay**    | Your wording for one reflex, one file, merged over the shipped manifest. Adds and replaces; never deletes. |
| **Vocabulary** | Your `word = "meaning"` list, shared by every argument that names it. A package never ships or writes one. |
| **Adapter**    | The classifier behind a decision. Jev is the first; the tool names no engine in its files or its core. |
| **Runtime**    | What runs a body: Node for a `.mts` or `.mjs` file, nothing for an argv. Chosen from the manifest; invisible to authors. |

Two more words recur everywhere. A **call** is a reflex with its arguments filled, on one line:
`lights room="den" state="off"`. An **effect** is what running a reflex does to the world — `read`, `write` or
`destructive` — and it decides how sure `evoke` must be before running without asking.

A **tag** is a word a manifest carries so `--tag` can narrow a decision. A **collection** is a repository of
reflex directories. The **Hub** is where reflexes are found: for now, the first-party collection
`evoke-build/reflexes`.

## The three steps

```text
evoke "mute the office lights"
```

1. **Decide.** Every active reflex's manifest becomes questions: which reflex does the input ask for, and for each
   argument, which value — or was it left unstated? One request carries them all; the adapter answers each with a
   probability.
2. **Gate.** Confidence is the weakest of those answers. Against the floor for the winner's effect it settles the
   outcome: **run**, **confirm**, **ask** for a missing argument, or **abstain**. A destructive reflex always
   confirms. Code stays in control: the classifier proposes, the gate disposes.
3. **Run.** The reflex's body runs with its arguments, a scrubbed environment and a deadline, and its one line of
   text comes back.

## What the classifier can and cannot do

It selects; it never writes. An argument's value is one of the author's options, one of your vocabulary words, or a
verbatim span of what you typed — a number, a duration, an email address, a URL, a quoted phrase — checked against
its range. So a sentence can choose a call, but it can never invent a value, and what it chooses still passes the
gate. That is the shape of every guarantee in [Security](../security.md).

**Next:** [Saying things](../use/saying-things.md).
