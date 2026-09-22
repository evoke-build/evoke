# Changelog

What changed for a user, release by release, newest first. Written under *Unreleased* as changes land; `mise run
release` dates it. The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), the versions
[semantic](https://semver.org).

## [Unreleased]

- The SDK's transport connects through the proxy `HTTPS_PROXY` names, as the CLI's does; `NO_PROXY` excludes
  hosts. The manual's environment page lists both.
- A connection that fails is said plainly: `could not connect to api.typesafe.ai: connection refused`, the proxy
  named when it carries the connection, in place of the library's line with an OS error number.

## [0.2.0] - 2026-09-22

- `evoke kill the lights` needs no quotes: the words after `evoke` are one input, as the manual said. Every
  argument error ends in `evoke --help`.
- `evoke help`, `evoke add --help` and `-V` are what they say. A command word written after a flag, like
  `evoke --json try …`, is refused with the way to write it, never decided. The help names the manual, explains
  a call, and lays its columns by the terminal's width.
- `show`, `vocab`, `why`, `try`, `test` and `check` print their answer on stdout, so `evoke show | grep timer`
  finds the row; what `evoke` says about a run stays on stderr. An answer is coloured where stdout is a terminal.
- `evoke add ./dir` installs a local reflex, written to `evoke.toml` relative to that file. A git URL may name
  its user: `ssh://git@github.com/owner/repo` is a ref. `sync`, `update` and `add` list a repository's tags and
  fetch a tag once, however many reflexes come from it, with a spinner while they do.
- The theft test at `add` decides a few cases at a time, counts them in the spinner, and reports what stopped
  it — the classifier, or a key not yet set — with `evoke test`; the add stands either way. `test` decides and
  counts the same way.
- `evoke teach dim lights state=dim` teaches the one word `dim`: a first word followed by a call is the
  utterance, unless it names an installed reflex.
- `evoke update` and `evoke sync` say `up to date` when nothing moved; `update`, `sync` and `test` with nothing
  installed say so.
- A decision no longer lists the inactive reflexes first. An abstain names them once, with `evoke show`.
- `try` and `why` set the top answer of every judgment in bold, list the best `fits` first, and fold the
  answers that would print as `0.00` into a count. `show` prints a pinned ref once, not with its tag beside it.
- The first line without a key says where one comes from: `jev needs TYPESAFE_API_KEY, a key from typesafe.ai`.
- A failure names a path under your home as `~/…`, without the OS error number; a missing `git` or `node` is
  named as such; a repository GitHub hides behind a credential prompt is reported as not found or private; a
  thrown error's frames print without repeating its message; an answer a prompt refuses is quoted; the line to
  run again never wraps a line already made, and never repeats an input over the cap.

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
