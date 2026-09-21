<p align="center"><img src="assets/logo.png" alt="" width="240"></p>

# evoke

**Software, by reflex.** `evoke` turns a sentence into a call of a small program — a *reflex* — chosen by a classifier
that never generates text, gated by confidence, and run only when the call is safe enough to run. One tool does three
jobs: a CLI you talk to, a package manager that installs reflexes from git, and a TypeScript SDK that puts the same
decisions inside your app. [Jev](https://typesafe.ai) is the first classifier; the design is bound to none.

```text
$ evoke "kill the lights in the den"
  lights room="den" state="off"  0.85
den lights off
```

Start with the [manual](docs/manual/README.md): [install](docs/manual/start/install.md), then
[the first ten minutes](docs/manual/start/first-run.md); [write a reflex](docs/manual/author/first-reflex.md);
[put decisions in your app](docs/manual/sdk/getting-started.md).

## Repository

| Directory                             | Holds                                                                                              |
| :------------------------------------ | :------------------------------------------------------------------------------------------------- |
| [crates/](Cargo.toml)                 | The Rust workspace: `evoke-core`, the rules with no engine and no I/O; `evoke-adapters`; `evoke-wasm`, the SDK's boundary; `evoke`, the CLI |
| [sdk/](sdk/README.md)                 | `@evoke-build/evoke`: TypeScript over the same core, compiled to WebAssembly                        |
| [spec/](spec/README.md)               | The executable spec — schemas, golden vectors, transcripts — and the tests of both hosts            |
| [reflexes/](reflexes/README.md)       | The first-party collection, published as `evoke-build/reflexes`                                     |
| [docs/manual/](docs/manual/README.md) | The manual                                                                                          |

## Developing

Tools and tasks are pinned by [mise](https://mise.jdx.dev): `mise install`, then `mise run lint` and `mise run test`,
the two CI runs; `mise tasks` lists the rest. `bin/dx <command>` runs a command with the same toolchain inside an
OrbStack machine named `devbox`, for those who develop in one. The tests need no key; to decide against Jev, export
`TYPESAFE_API_KEY`.

## Licence

[Apache-2.0](LICENSE); the collection under [reflexes/](reflexes/README.md) is [MIT](reflexes/LICENSE). The name and
the logo are trademarks: [TRADEMARK.md](TRADEMARK.md).
