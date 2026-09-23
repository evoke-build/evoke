<p align="center"><img src="assets/logo.png" alt="" width="240"></p>

# evoke

**Software, by reflex.** Say it, and the right small program runs when it is sure. A reflex is a recipe: written
once, shared, improved by everyone. The program and its arguments are chosen by [Jev](https://typesafe.ai),
TypeSafe AI's System One classifier, which answers closed questions with calibrated probabilities. The questions,
and the gate that decides whether the call runs, confirms or asks, are `evoke`'s.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

```bash
curl -fsSL https://evoke.build/install.sh | sh    # the CLI, on macOS and Linux
npm install @evoke-build/evoke                    # the SDK
```

One core, three ways in: a CLI you talk to, a package manager that installs reflexes from git, and a TypeScript
SDK that puts the same decisions inside your app.

## The idea

Saying what you want is easy. Trusting what runs is the hard part. An action taken on a guess costs more than one
that never ran, so trust needs three things. Every value comes from you: from what you said, or from a list you
own. The confidence is a number that means what it says, so you can set a bar on it. And what cannot be undone
asks first, every time.

That is the idea behind a reflex. Your words pick a program from the ones you installed. Jev answers closed questions
about your sentence, every answer a calibrated probability, and the weakest one decides. `evoke` is the first
implementation of this idea: one core, a CLI, a package manager and a TypeScript SDK, with Jev as its first
engine and the design bound to none. The whole idea, with real sessions, at
[evoke.build](https://evoke.build).

## It stops

The value of all this is what it refuses to do. Three real sessions where nothing ran on a guess:

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

Sure at 0.97, and it still asked: a destructive reflex always does. A value outside its range never reached a
program, and the range was on the line. A sentence that fits nothing ran nothing, and showed the ranking that
says why. `evoke try` puts every judgment on the table, and a bar is a line in a file you own.

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
| [spec/](spec/README.md)               | The executable spec — schemas, golden vectors, transcripts — and the tests of both hosts            |
| [reflexes/](reflexes/README.md)       | The first-party collection, published as `evoke-build/reflexes`                                     |
| [docs/manual/](https://evoke.build/manual/) | The manual, as published at evoke.build                                                       |
| [site/](https://evoke.build)          | evoke.build: the landing page, the install script, and the manual rendered by mdBook                |

## Developing

Tools and tasks are pinned by [mise](https://mise.jdx.dev). With git and a C compiler on the machine, run
`mise install`, then `mise run lint` and `mise run test`. Those two are what CI runs; `mise tasks` lists the rest.
Packaging a Linux release needs `musl-tools` as well. `bin/dx <command>` runs a command with the same toolchain
inside an OrbStack machine named `devbox`, for those who develop in one. The tests need no key.
To decide against Jev, export `TYPESAFE_API_KEY`. To decide without one, the `replay` adapter answers from a file
you write: [Testing](https://evoke.build/manual/sdk/testing.html#the-cli-on-a-recording).

## Licence

[Apache-2.0](LICENSE); the collection under [reflexes/](reflexes/README.md) is [MIT](reflexes/LICENSE).

**Trademarks.** Except for displaying the licence details and identifying us as the origin of the software, you have
no right under the licence to use our trademarks, trade names, service marks or product names: the name `evoke`, the
name evoke.build and the logo. A fork takes its own name and logo. [TRADEMARK.md](TRADEMARK.md).
