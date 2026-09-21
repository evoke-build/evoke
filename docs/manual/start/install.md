# Install

`evoke` is one small native binary. Reflexes that run a JavaScript file need Node beside it; nothing else is
installed on your behalf.

## What you need

| For                                   | Needed                                                                   |
| :------------------------------------ | :----------------------------------------------------------------------- |
| The binary                            | macOS or Linux; Windows through WSL                                      |
| Installing reflexes (`add`, `update`, `sync`) | `git` on your `PATH`; your own git configuration and credentials apply |
| Reflexes that run a `.mts` or `.mjs` file | [Node](https://nodejs.org) 24 or newer on your `PATH` when you run `evoke add` or `evoke sync` |
| Deciding                              | A key for the classifier: `TYPESAFE_API_KEY` for Jev, the first adapter |

A reflex whose body is a program with arguments — an *argv* reflex — needs no runtime at all.

## The binary

Binaries on GitHub releases, a Homebrew tap and the mise registry come with release 0.1. Until then, build from
source with a current Rust toolchain:

```bash
git clone https://github.com/evoke-build/evoke && cd evoke && cargo install --path crates/evoke
```

Then:

```bash
evoke --version
```

## The SDK

`@evoke-build/evoke` reaches npm with release 0.1. Until then, build it from the same clone: the core as
WebAssembly, then the TypeScript.

```bash
rustup target add wasm32-unknown-unknown
cargo build -p evoke-wasm --target wasm32-unknown-unknown --profile wasm
mkdir -p sdk/runtime
cp target/wasm32-unknown-unknown/wasm/evoke_wasm.wasm sdk/core.wasm
cp crates/evoke/runtime/loader.mjs sdk/runtime/loader.mjs
cd sdk && npm ci && npm run build
```

The package needs Node 24 or newer and is ES modules only. The SDK is its own manual section:
[Getting started](../sdk/getting-started.md).

## The key

Jev is [TypeSafe AI](https://typesafe.ai)'s classifier. Put its key in your shell's environment; `evoke` reads it
at the moment it decides and nowhere else, and never writes it to a file:

```bash
export TYPESAFE_API_KEY=<value>
```

Without it, the first decision tells you exactly this line and exits 3.

**Next:** [The first ten minutes](first-run.md).
