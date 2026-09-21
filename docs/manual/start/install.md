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

With [Homebrew](https://brew.sh), on macOS or Linux — the tap once, then the name:

```bash
brew tap evoke-build/tap
brew install evoke
```

With the script, which fetches the release built for your machine, checks it against the release's `SHA256SUMS`
and puts `evoke` in `~/.local/bin` — `EVOKE_INSTALL` names another directory, `EVOKE_VERSION` picks a version:

```bash
curl -fsSL https://evoke.build/install.sh | sh
```

With [mise](https://mise.jdx.dev):

```bash
mise use --global github:evoke-build/evoke
```

From source, with a current Rust toolchain:

```bash
cargo install --locked --git https://github.com/evoke-build/evoke evoke
```

Every way ends the same:

```text
$ evoke --version
evoke 0.1.0
```

The archives are on [GitHub releases](https://github.com/evoke-build/evoke/releases): `evoke-<target>.tar.gz` for
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl` — the
Linux binaries static — each holding `evoke`, `LICENSE` and `NOTICE`, with `SHA256SUMS` beside them and a build
attestation `gh attestation verify <archive> --repo evoke-build/evoke` checks. To update: `brew upgrade evoke`, the
script again, or `mise upgrade`.

## The SDK

```bash
npm install @evoke-build/evoke
```

The package needs Node 24 or newer and is ES modules only; it carries the core as WebAssembly, so it has no build
step and no dependencies. The SDK is its own manual section: [Getting started](../sdk/getting-started.md).

## The key

Jev is [TypeSafe AI](https://typesafe.ai)'s classifier. Put its key in your shell's environment; `evoke` reads it
at the moment it decides and nowhere else, and never writes it to a file:

```bash
export TYPESAFE_API_KEY=<value>
```

Without it, the first decision tells you exactly this line and exits 3.

**Next:** [The first ten minutes](first-run.md).
