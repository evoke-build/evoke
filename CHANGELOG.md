# Changelog

What changed for a user, release by release, newest first. Written under *Unreleased* as changes land; `mise run
release` dates it. The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), the versions
[semantic](https://semver.org).

## [Unreleased]

- `evoke help`, `evoke add --help` and `-V` are what they say. A command word written after a flag, like
  `evoke --json try …`, is refused with the way to write it, never decided. The help names the manual, explains a
  call, and lays its columns by the terminal's width.
- A spinner turns while a repository is listed or fetched.
- A second reflex can be added before the key is set: the theft test reports that it did not finish, and the add
  stands.
- `evoke update` says `up to date` when nothing moved; `update`, `sync` and `test` with nothing installed say so.
- A failure names a path under your home as `~/…`, without the OS error number; a missing `git` or `node` is
  named as such; a repository GitHub hides behind a credential prompt is reported as not found or private; the
  line to run again never wraps a line already made, and never repeats an input over the cap.
- A thrown error's frames print without repeating its message; the `fits` line lists the best fit first; an
  answer a prompt refuses is quoted.
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
