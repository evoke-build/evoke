<p align="center"><img src="../../assets/logo.png" alt="" width="200" height="200"></p>

# The evoke manual

**Software, by reflex.** `evoke` turns a sentence into a call of a small program. That program is a *reflex*: a
recipe anyone can write, share and improve. A classifier chooses it. A confidence gate decides whether it runs.
A sentence can choose a program. It can never invent a value. The classifier is [Jev](https://typesafe.ai),
TypeSafe AI's System One model, the first adapter of a design bound to none.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

One tool does three jobs. A **CLI** you talk to. A **package manager** that installs reflexes from git. A
**TypeScript SDK** that puts the same decisions inside your app. You invoke a function; you evoke a reflex.

## Where to start

| You want to…                                        | Read                                                          |
| :-------------------------------------------------- | :------------------------------------------------------------ |
| Say something to your laptop and see it happen      | [Install](start/install.md), then [The first ten minutes](start/first-run.md) |
| Learn the words this manual uses                    | [Concepts](start/concepts.md)                                 |
| Write a reflex of your own                          | [Your first reflex](author/first-reflex.md)                   |
| Decide and run reflexes inside your application     | [The SDK](sdk/getting-started.md)                             |
| Look something up                                   | [Reference](reference/cli.md)                                 |

## The manual

### Start

| Page                                        | Read it when                                            |
| :------------------------------------------ | :------------------------------------------------------ |
| [Install](start/install.md)                 | Putting `evoke` and its SDK on a machine                |
| [The first ten minutes](start/first-run.md) | Your first key, your first reflexes, your first lesson  |
| [Concepts](start/concepts.md)               | The words, and the three steps of a decision            |

### Use: the CLI

| Page                                         | Read it when                                                     |
| :------------------------------------------- | :--------------------------------------------------------------- |
| [Saying things](use/saying-things.md)        | Bare input, the REPL, a pipe, `--json`, `--tag`, exit codes       |
| [Outcomes](use/outcomes.md)                  | Run, confirm, ask and abstain; `try`, `why`, `run`                |
| [Weaving](use/weaving.md)                    | Several things in one sentence: the plan, what a step takes from another |
| [Installing reflexes](use/installing.md)     | `add`, `remove`, `update`, `sync`, `trust`; refs, the lock, the store |
| [Tuning](use/tuning.md)                      | `teach`, overlays, vocabularies, `config`, `show`, `test`          |
| [Projects](use/projects.md)                  | Your home project, an app's project, `evoke.toml`, trust           |

### Author: writing reflexes

| Page                                          | Read it when                                                   |
| :-------------------------------------------- | :------------------------------------------------------------- |
| [Your first reflex](author/first-reflex.md)   | `new`, `check`, a try, a run: a reflex in ten minutes           |
| [The manifest](author/manifest.md)            | `reflex.toml`, key by key                                       |
| [Arguments](author/arguments.md)              | Options, vocabularies, picks and flags; ranges; renames         |
| [The body](author/body.md)                    | File bodies and argv bodies; context, results, the deadline     |
| [Examples and tests](author/records.md)       | Records, assertions, `evoke test`, lint                         |
| [Wording](author/wording.md)                  | Writing descriptions the classifier reads well                  |
| [Publishing](author/publishing.md)            | Tags, versions, the contract diff, collections, the Hub         |

### SDK: `@evoke-build/evoke`

| Page                                        | Read it when                                                       |
| :------------------------------------------ | :----------------------------------------------------------------- |
| [Getting started](sdk/getting-started.md)   | The first hour: one file, one reflex, one decision                 |
| [Projects in code](sdk/projects.md)         | `load()`, roots, reflexes as code, `with()`, generated types        |
| [Decisions](sdk/decisions.md)               | The `Decision` union, `fill`, `run`, `handle`; `steps` and `weave`   |
| [Adapters](sdk/adapters.md)                 | `jev()`, thresholds, and writing an adapter of your own             |
| [Testing](sdk/testing.md)                   | `replay()`: offline, deterministic tests over a recording           |
| [Errors](sdk/errors.md)                     | The three errors, and what each asks of you                         |

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

- A line beginning `$ ` is a command you type. The lines under it are what the terminal shows.
- What you asked to see goes to stdout: a reflex's result, or the answer of `show`, `try`, `why` and `test`.
  What `evoke` says about a run goes to stderr, indented two spaces.
- Every line that needs something from you ends with the command that does it: `  →  evoke trust`.
- macOS and Linux; Windows through WSL. The [collection](collection.md)'s programs are for macOS.

`evoke` is [Apache-2.0](https://github.com/evoke-build/evoke/blob/main/LICENSE); the collection is MIT. The name
and the logo are [trademarks](https://github.com/evoke-build/evoke/blob/main/TRADEMARK.md): except for displaying the
licence details and identifying us as the origin of the software, you have no right under the licence to use them.
