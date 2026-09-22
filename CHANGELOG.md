# Changelog

What changed for a user, release by release, newest first. Written under *Unreleased* as changes land; `mise run
release` dates it. The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), the versions
[semantic](https://semver.org).

## [Unreleased]

- `evoke kill the lights` needs no quotes: the words after `evoke` are one input, as the manual said. Every
  argument error now ends in `evoke --help`.
- A decision no longer lists the inactive reflexes first. An abstain names them once, with `evoke show`.
- `evoke add ./dir` installs a local reflex, written to `evoke.toml` relative to that file.
- `ssh://git@github.com/owner/repo` is a ref: a git URL may name its user.
- `sync`, `update` and `add` list a repository's tags and fetch a tag once, however many reflexes come from it.
- `test` and the theft test at `add` decide a few cases at a time. A theft test the classifier cannot finish is
  reported with `evoke test`, and the add stands.
- `show` prints a pinned ref once, not with its tag beside it.

## [0.1.0] - 2026-09-21

The first release: the CLI, the package manager and the SDK.

- `evoke "<sentence>"` decides among the installed reflexes and runs, confirms, asks or abstains; a REPL, a pipe
  and `--json` for scripts.
- `add`, `remove`, `update`, `sync` and `trust`: reflexes fetched from git at a tag, pinned by content in
  `evoke.lock`.
- `teach`, overlays, vocabularies and `config`: wording tuned in files you own, never touched by an update.
- `new`, `check` and `test`: a reflex written, typed and tested in ten minutes.
- `@evoke-build/evoke`: the same decisions inside a Node application, over the same core as WebAssembly.
- `evoke-build/reflexes`: thirteen reflexes for what a Mac does at a word.
