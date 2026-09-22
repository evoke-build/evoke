<p align="center"><img src="assets/logo.png" alt="" width="240"></p>

# evoke

**Software, by reflex.** Say what you want, and `evoke` runs the small program that does it. The program and its
arguments are chosen by [Jev](https://typesafe.ai), TypeSafe AI's System One classifier. Jev answers closed
questions with calibrated probabilities. A confidence gate decides whether the call runs, confirms or asks. What
runs is always a *reflex*: a recipe someone wrote, and you installed from git. Anyone can write one, share it,
and improve it. One tool does three jobs. A CLI you talk to. A package manager that installs reflexes from git.
A TypeScript SDK that puts the same decisions inside your app. Jev is the first adapter; the design is bound to
no engine.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

```bash
curl -fsSL https://evoke.build/install.sh | sh    # the CLI, on macOS and Linux
npm install @evoke-build/evoke                    # the SDK
```

Start with the [manual](https://evoke.build/manual/) at [evoke.build](https://evoke.build).
[Install](https://evoke.build/manual/start/install.html), then
[the first ten minutes](https://evoke.build/manual/start/first-run.html). Then
[write a reflex](https://evoke.build/manual/author/first-reflex.html), or
[put decisions in your app](https://evoke.build/manual/sdk/getting-started.html).

## The idea

Saying what you want is easy. Trusting what runs is the hard part. An action taken on a guess costs more than one
that never ran. So trust needs three things. Every value comes from you: from what you said, or from a list you
own. The confidence is a number that means what it says, so you can set a bar on it. And what cannot be undone
asks first, every time.

That is the idea behind a reflex. Your words pick a program. They never write one. The classifier answers closed
questions, every answer a calibrated probability, and the weakest one decides. A reflex is a recipe: written
once, shared, and improved by everyone who installs it.

`evoke` is the first implementation of this idea: one core, a CLI, a package manager and a TypeScript SDK, with
Jev as its first engine and the design bound to none. The whole idea in ten minutes, with real sessions:
[evoke.build/idea.html](https://evoke.build/idea.html).

## Repository

| Directory                             | Holds                                                                                              |
| :------------------------------------ | :------------------------------------------------------------------------------------------------- |
| [crates/](Cargo.toml)                 | The Rust workspace: `evoke-core`, the rules with no engine and no I/O; `evoke-adapters`; `evoke-wasm`, the SDK's boundary; `evoke`, the CLI |
| [sdk/](sdk/README.md)                 | `@evoke-build/evoke`: TypeScript over the same core, compiled to WebAssembly                        |
| [spec/](spec/README.md)               | The executable spec — schemas, golden vectors, transcripts — and the tests of both hosts            |
| [reflexes/](reflexes/README.md)       | The first-party collection, published as `evoke-build/reflexes`                                     |
| [docs/manual/](https://evoke.build/manual/) | The manual, as published at evoke.build                                                       |
| [site/](https://evoke.build)          | evoke.build: the landing page, the install script, and the manual rendered by mdBook                |

## Developing

Tools and tasks are pinned by [mise](https://mise.jdx.dev). Run `mise install`, then `mise run lint` and
`mise run test`. Those two are what CI runs; `mise tasks` lists the rest. `bin/dx <command>` runs a command with
the same toolchain inside an OrbStack machine named `devbox`, for those who develop in one. The tests need no key.
To decide against Jev, export `TYPESAFE_API_KEY`. To decide without one, the `replay` adapter answers from a file
you write: [Testing](https://evoke.build/manual/sdk/testing.html#the-cli-on-a-recording).

## Licence

[Apache-2.0](LICENSE); the collection under [reflexes/](reflexes/README.md) is [MIT](reflexes/LICENSE). The name and
the logo are trademarks: [TRADEMARK.md](TRADEMARK.md).
