<p align="center"><img src="../../assets/logo.png" alt="" width="200"></p>

# The evoke manual

**Software, by reflex.** `evoke` turns a sentence into a call of a small program — a *reflex* — chosen by a
classifier that never generates text, gated by confidence, and run only when the call is safe enough to run. The
classifier is [Jev](https://typesafe.ai), TypeSafe AI's System One model, the first adapter of a design bound to none.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

One tool does three jobs: a **CLI** you talk to, a **package manager** that installs reflexes from git, and a
**TypeScript SDK** that puts the same decisions inside your app. You invoke a function; you evoke a reflex.

## Where to start

| You want to…                                        | Read                                                          |
| :-------------------------------------------------- | :------------------------------------------------------------ |
| Say something to your laptop and see it happen      | [Install](start/install.md), then [The first ten minutes](start/first-run.md) |
| Understand the words this manual uses               | [Concepts](start/concepts.md)                                 |
| Write a reflex of your own                          | [Your first reflex](author/first-reflex.md)                   |
| Decide and run reflexes inside your application     | [The SDK](sdk/getting-started.md)                             |
| Look something up                                   | [Reference](reference/cli.md)                                 |

## The manual

### Start

| Page                                        | Read it when                                            |
| :------------------------------------------ | :------------------------------------------------------ |
| [Install](start/install.md)                 | Putting `evoke` and its SDK on a machine                |
| [The first ten minutes](start/first-run.md) | Your first key, your first reflexes, your first lesson  |
| [Concepts](start/concepts.md)               | You want the vocabulary and the three steps of a decision |

### Use — the CLI

| Page                                         | Read it when                                                     |
| :------------------------------------------- | :--------------------------------------------------------------- |
| [Saying things](use/saying-things.md)        | Bare input, the REPL, a pipe, `--json`, `--tag`, exit codes       |
| [Outcomes](use/outcomes.md)                  | What run, confirm, ask and abstain mean; `try`, `why`, `run`      |
| [Installing reflexes](use/installing.md)     | `add`, `remove`, `update`, `sync`, `trust`; refs, the lock, the store |
| [Tuning](use/tuning.md)                      | `teach`, overlays, vocabularies, `config`, `show`, `test`          |
| [Projects](use/projects.md)                  | Your home project, an app's project, `evoke.toml`, trust           |

### Author — writing reflexes

| Page                                          | Read it when                                                   |
| :-------------------------------------------- | :------------------------------------------------------------- |
| [Your first reflex](author/first-reflex.md)   | `new`, `check`, a try, a run — a reflex in ten minutes          |
| [The manifest](author/manifest.md)            | `reflex.toml`, key by key                                       |
| [Arguments](author/arguments.md)              | Options, vocabularies, picks and flags; ranges; renames         |
| [The body](author/body.md)                    | File bodies and argv bodies; context, results, the deadline     |
| [Examples and tests](author/records.md)       | Records, assertions, `evoke test`, lint                         |
| [Wording](author/wording.md)                  | Writing descriptions the classifier decides well on             |
| [Publishing](author/publishing.md)            | Tags, versions, the contract diff, collections, the Hub         |

### SDK — `@evoke-build/evoke`

| Page                                        | Read it when                                                       |
| :------------------------------------------ | :----------------------------------------------------------------- |
| [Getting started](sdk/getting-started.md)   | The first hour: one file, one reflex, one decision                 |
| [Projects](sdk/projects.md)                 | `load()`, roots, reflexes as code, `with()`, generated types        |
| [Decisions](sdk/decisions.md)               | The `Decision` union, `fill`, `run`, `handle`                       |
| [Adapters](sdk/adapters.md)                 | `jev()`, thresholds, and writing an adapter of your own             |
| [Testing](sdk/testing.md)                   | `replay()`: offline, deterministic tests over a recording           |
| [Errors](sdk/errors.md)                     | The three errors and what each asks of you                          |

### Reference

| Page                                             | Holds                                                       |
| :----------------------------------------------- | :---------------------------------------------------------- |
| [Commands](reference/cli.md)                     | Every command, flag and exit code                            |
| [Files](reference/files.md)                      | `evoke.toml`, `evoke.lock`, overlays, vocabularies, the types |
| [The JSON line](reference/json.md)               | What `--json` prints, field by field                          |
| [Environment](reference/environment.md)          | Variables `evoke` reads and the paths it writes               |
| [Diagnostics](reference/diagnostics.md)          | Every line that ends in a fix, and what the fix does          |
| [Glossary](reference/glossary.md)                | The words, in one place                                       |

### Around the tool

| Page                              | Holds                                                              |
| :-------------------------------- | :----------------------------------------------------------------- |
| [The collection](collection.md)   | The first-party reflexes: what a Mac does at a word                |
| [Security](security.md)           | What runs as you, what is guaranteed, and what is not              |

## Conventions

- A line beginning `$ ` is a command you type; the lines under it are what the terminal shows.
- `evoke` writes its own lines to stderr, indented two spaces, and only a reflex's result to stdout.
- Every line that needs something from you ends in the literal command that does it: `  →  evoke trust`.
- macOS and Linux; Windows through WSL. The [collection](collection.md)'s bodies are macOS's.

`evoke` is [Apache-2.0](https://github.com/evoke-build/evoke/blob/main/LICENSE); the collection is MIT. The name
and the logo are [trademarks](https://github.com/evoke-build/evoke/blob/main/TRADEMARK.md).
