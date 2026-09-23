# Install

`evoke` is one small native binary. Reflexes that run a JavaScript file need Node next to it. Nothing else is
installed for you.

## What you need

| For                                   | Needed                                                                   |
| :------------------------------------ | :----------------------------------------------------------------------- |
| The binary                            | macOS or Linux; Windows through WSL                                      |
| Installing reflexes (`add`, `update`, `sync`) | `git` on your `PATH`; your own git configuration and credentials apply |
| Reflexes that run a `.mts` or `.mjs` file | [Node](https://nodejs.org) 24 or newer on your `PATH` when you run `evoke add` or `evoke sync` |
| Deciding                              | A key for the classifier: `TYPESAFE_API_KEY` for Jev, the first adapter |

Some reflexes run a program with arguments instead of a JavaScript file. Those are *argv* reflexes. They need no
runtime at all.

## The binary

On macOS or Linux, use the script. It fetches the release built for your machine, checks it against the
release's `SHA256SUMS`, and puts `evoke` in `~/.local/bin`. Set `EVOKE_INSTALL` to choose another directory. Set
`EVOKE_VERSION` to pick a version.

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
evoke 0.4.0
```

The archives are on [GitHub releases](https://github.com/evoke-build/evoke/releases). Each is named
`evoke-<target>.tar.gz`, for `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-musl` or
`x86_64-unknown-linux-musl`. The Linux binaries are static. Each archive holds `evoke`, `LICENSE` and `NOTICE`.
`SHA256SUMS` sits beside them, and so does a build attestation, which
`gh attestation verify <archive> --repo evoke-build/evoke` checks. To update, run the script again, or
`mise upgrade`.

## The SDK

```bash
npm install @evoke-build/evoke
```

The package needs Node 24.5 or newer, and it is ES modules only. It carries the core as WebAssembly, so it has no
build step and no dependencies. The SDK has its own manual section: [Getting started](../sdk/getting-started.md).

## The key

Jev is [TypeSafe AI](https://typesafe.ai)'s classifier. Put its key in your shell's environment. `evoke` reads it
only at the moment it decides, and never writes it to a file:

```bash
export TYPESAFE_API_KEY=<value>
```

Without the key, the first decision prints this line and exits 3.

**Next:** [The first ten minutes](first-run.md).
