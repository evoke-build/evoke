# Install

`evoke` is one small native binary. Reflexes that run a JavaScript file need Node next to it. Nothing else is
installed for you.

## What you need

| For                                   | Needed                                                                   |
| :------------------------------------ | :----------------------------------------------------------------------- |
| The binary                            | macOS or Linux; Windows through WSL                                      |
| Installing reflexes (`add`, `update`, `sync`) | `git` on your `PATH`; your own git configuration and credentials apply |
| Reflexes that run a `.mts` or `.mjs` file | [Node](https://nodejs.org) 24 or newer on your `PATH` when you run `evoke add` or `evoke sync` |
| Deciding                              | A key for the classifier: `TYPESAFE_API_KEY` for the `jev` adapter, `OPENJEV_API_KEY` for `openjev` |

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
evoke 0.6.0
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

Jev is [TypeSafe AI](https://typesafe.ai)'s classifier. `evoke` reaches it through one of two adapters. Each reads
its key from your shell's environment, only at the moment it decides, and never writes it to a file.

The `jev` adapter is the default. It posts to TypeSafe AI's own API, under a key from
[typesafe.ai](https://typesafe.ai):

```bash
export TYPESAFE_API_KEY=<value>
```

The `openjev` adapter posts to [OpenJEV](https://openjev.sh), an independent service that forwards the request
to Jev and returns its answers. OpenJEV is not TypeSafe AI's. `evoke` is affiliated with neither, and vouches for
neither. A key from OpenJEV is on OpenJEV's terms, and whether it may offer Jev is a matter between OpenJEV and
TypeSafe AI. As of this release it publishes no terms and no privacy policy, so weigh that before you choose it.
Name it in `evoke.toml` and export its key:

```toml
adapter = "openjev"
```

```bash
export OPENJEV_API_KEY=<value>
```

Without the key, the first decision prints a line naming the one it needs, and exits 3. Both adapters ask the
same questions and ship the same bars, so a project moves from one to the other with that one line.

**Next:** [The first ten minutes](first-run.md).
