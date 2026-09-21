<p align="center"><img src="assets/logo.png" alt="" width="240"></p>

# evoke

**Software, by reflex.** Say what you want, and `evoke` runs the small program that does it. The program and its
arguments are chosen by [Jev](https://typesafe.ai), TypeSafe AI's System One classifier, which answers closed
questions with calibrated probabilities and never generates a word; a confidence gate decides whether the call runs,
confirms or asks; and what runs is always a *reflex* — a small program someone wrote and you installed from git. One
tool does three jobs: a CLI you talk to, a package manager that installs reflexes from git, and a TypeScript SDK that
puts the same decisions inside your app. Jev is the first adapter; the design is bound to no engine.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

```bash
curl -fsSL https://evoke.build/install.sh | sh    # the CLI, on macOS and Linux
npm install @evoke-build/evoke                    # the SDK
```

Start with the [manual](docs/manual/README.md), also at [evoke.build](https://evoke.build):
[install](docs/manual/start/install.md), then [the first ten minutes](docs/manual/start/first-run.md);
[write a reflex](docs/manual/author/first-reflex.md); [put decisions in your app](docs/manual/sdk/getting-started.md).

## Repository

| Directory                             | Holds                                                                                              |
| :------------------------------------ | :------------------------------------------------------------------------------------------------- |
| [crates/](Cargo.toml)                 | The Rust workspace: `evoke-core`, the rules with no engine and no I/O; `evoke-adapters`; `evoke-wasm`, the SDK's boundary; `evoke`, the CLI |
| [sdk/](sdk/README.md)                 | `@evoke-build/evoke`: TypeScript over the same core, compiled to WebAssembly                        |
| [spec/](spec/README.md)               | The executable spec — schemas, golden vectors, transcripts — and the tests of both hosts            |
| [reflexes/](reflexes/README.md)       | The first-party collection, published as `evoke-build/reflexes`                                     |
| [docs/manual/](docs/manual/README.md) | The manual                                                                                          |
| [site/](site/install.sh)              | evoke.build: the landing page, the install script, and the manual rendered by mdBook                |

## Developing

Tools and tasks are pinned by [mise](https://mise.jdx.dev): `mise install`, then `mise run lint` and `mise run test`,
the two CI runs; `mise tasks` lists the rest. `bin/dx <command>` runs a command with the same toolchain inside an
OrbStack machine named `devbox`, for those who develop in one. The tests need no key; to decide against Jev, export
`TYPESAFE_API_KEY`.

## Licence

[Apache-2.0](LICENSE); the collection under [reflexes/](reflexes/README.md) is [MIT](reflexes/LICENSE). The name and
the logo are trademarks: [TRADEMARK.md](TRADEMARK.md).
