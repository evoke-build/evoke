<p align="center"><img src="assets/logo.png" alt="" width="240"></p>

# evoke

**Software, by reflex.** Natural-language commands for small programs you install. A reflex is a small program
you install and ask for in your words. `evoke` picks the reflex your sentence asks for from the ones you installed,
and fills its inputs from your words or your own lists. It runs when it is sure enough for what the program does,
asks when something is missing or unclear, and always asks before anything that cannot be undone.

A reflex is a recipe: written once, shared, improved by everyone. A classifier reads the sentence:
[Jev](https://typesafe.ai), TypeSafe AI's model. It answers closed questions and gives each answer a probability.
The questions, and the gate that decides whether the call runs, confirms or asks, are `evoke`'s.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

`lights` is an illustration from the tests. The reflexes you can install are in
[the collection](https://evoke.build/manual/collection.html): thirteen, for a Mac.

```bash
curl -fsSL https://evoke.build/install.sh | sh    # the CLI, on macOS and Linux
npm install @evoke-build/evoke                    # the SDK, for Node 24.5 or newer
```

One core, three ways in: a CLI where you type what you want, a package manager that installs reflexes from git,
and a TypeScript SDK that puts the same decisions inside your app.

## The idea

Typing what you want is easy. Trusting what runs is the hard part. An action taken on a guess costs more than one
that never ran, so trust needs three things. Every value comes from you: from what you typed, or from a list you
own. Every answer comes with a probability you can read, so you can set a bar on it. And what cannot be undone asks
first, every time.

That is the idea behind a reflex. Your words pick a program from the ones you installed. Jev answers closed
questions about your sentence, each answer with a probability, and the weakest one decides. The classifier's
provider trains those probabilities to be calibrated: across many answers, those given 0.85 should be right about
85 times in 100. `evoke` is the first implementation of this idea: one core with a CLI, a package manager and a
TypeScript SDK. Jev is its first engine, and the core names no engine. The whole idea is at
[evoke.build](https://evoke.build/idea.html).

## It stops

The value of all this is what it refuses to do. Here are three sessions from the test suite. Nothing ran under its
bar, and nothing that cannot be undone ran without a yes:

```text
$ evoke "restart the computer"
  power action="restart" · destructive · weakest: route 0.97
  Really restart now?  [y]es [n]o [t]each > n
[2]
$ evoke "set the volume to 150 percent"
  How loud, in percent?  150 percent is outside 0–100  > 40
  volume level="40"  0.93
volume set to 40%
$ evoke "make it cosy"
  lights 0.45 · none 0.40 · timer 0.10 · volume 0.05 · route floor 0.50
[2]
```

At 0.97 it was sure, and it still asked, because a reflex that cannot be undone always asks. A volume of 150 percent
never reached a program. `evoke` named the allowed range, 0 to 100, and asked for a value inside it. A sentence that
fits no reflex ran nothing, and the ranking shows why. `evoke try` shows every judgment, and one line in a file you
own moves a bar.

## Start

[Install](https://evoke.build/manual/start/install.html), then
[the first ten minutes](https://evoke.build/manual/start/first-run.html). Then
[write a reflex](https://evoke.build/manual/author/first-reflex.html), or
[put decisions in your app](https://evoke.build/manual/sdk/getting-started.html). The whole
[manual](https://evoke.build/manual/) is at [evoke.build](https://evoke.build).

## Repository

| Directory                             | Holds                                                                                              |
| :------------------------------------ | :------------------------------------------------------------------------------------------------- |
| [crates/](Cargo.toml)                 | The Rust workspace: `evoke-core`, the rules with no engine and no I/O; `evoke-adapters`; `evoke-wasm`, the SDK's boundary; `evoke`, the CLI |
| [sdk/](sdk/README.md)                 | `@evoke-build/evoke`: TypeScript over the same core, compiled to WebAssembly                        |
| [spec/](spec/README.md)               | The executable spec: schemas, golden vectors and transcripts, with the tests of both hosts          |
| [reflexes/](reflexes/README.md)       | The first-party collection, published as `evoke-build/reflexes`                                     |
| [docs/manual/](https://evoke.build/manual/) | The manual, as published at evoke.build                                                       |
| [site/](https://evoke.build)          | evoke.build: the landing page, the install script, and the manual rendered by mdBook                |

## Developing

Tools and tasks are pinned by [mise](https://mise.jdx.dev). With git and a C compiler on the machine, run
`mise install`, then `mise run lint` and `mise run test`. Those two are what CI runs; `mise tasks` lists the rest.
Packaging a Linux release needs `musl-tools` as well. `bin/dx <command>` runs a command with the same toolchain
inside an OrbStack machine named `devbox`, for those who develop in one. The tests need no key.
To decide against Jev, export `TYPESAFE_API_KEY` for the `jev` adapter, or `OPENJEV_API_KEY` for `openjev`. To
decide without one, the `replay` adapter answers from a file you write: [Testing](https://evoke.build/manual/sdk/testing.html#the-cli-on-a-recording).

## Licence

[Apache-2.0](LICENSE). The collection under [reflexes/](reflexes/README.md) is [MIT](reflexes/LICENSE).

**Trademarks.** Except for displaying the licence details and identifying us as the origin of the software, you have
no right under the licence to use our trademarks, trade names, service marks or product names: the name `evoke`, the
name evoke.build and the logo. A fork takes its own name and logo. [TRADEMARK.md](TRADEMARK.md).
