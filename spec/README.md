# Spec

Behavior written down as checkable examples before it is implemented, and the tests of both hosts after: read by
three suites, owned by none. The seeds under [reflexes/](../reflexes/README.md) are its fixtures.

| Directory      | Holds                                                                  | Run by                                            |
| :------------- | :--------------------------------------------------------------------- | :------------------------------------------------ |
| `schemas/`     | JSON Schema for `reflex.toml`, overlays, vocabularies and `evoke.toml` | `mise run lint`, through `.taplo.toml`; editors   |
| `fixtures/`    | Wire values the vectors share                                          | —                                                 |
| `vectors/`     | One directory per core function, one case per file                     | the core natively; the SDK through the wasm build |
| `transcripts/` | One directory per flow: the terminal, word for word                    | the built binary, offline, on recorded answers    |

`mise run lint-spec` checks the shapes below; the rules themselves are checked by the suites that run the spec.

## Vectors

`vectors/<function>/<case>.json` is `{ "input": …, "expect": … }`. `input` holds the function's arguments by name, as
the op table in `crates/evoke-wasm/src/ops.rs` takes them, one op per function of the core's surface; `expect` is the
result, `{ "ok": … }` or `{ "err": … }` for a `Result`. Every op has a family but `version`, which answers the
build's own number; a family the runners do not list fails their check. Values take the [wire form](#wire);
`{ "$ref": "fixtures/<name>.json" }`, a path under `spec/`, stands for that file's value. A file goes in as
`{ "file": …, "toml": "…" }` or `{ "file": …, "json": … }`. A runner compares typed values, so `40` and `40.0` are
one number.

## Wire

The JSON every value takes, pinned by the vectors and mirrored by hand in `sdk/src/types.ts`:

- A struct is an object with snake_case keys; an `Option` is absent when none; a collection is present even when empty.
- A newtype is its value: a digest `"h1:<hex>"`, a version `"1.2.0"`, a question id `"lights.room"`, a probability
  `0.58`; a key path is an array of segments; a range is `[min, max]`.
- An enum without data is its variant in snake_case, `"write"`; one with data is an object tagged by `type`,
  `{ "type": "env", "var": "HUE_TOKEN", "set": true }`, a decision by `outcome`. A `Result` is `{ "ok": … }` or
  `{ "err": … }`; a question's text is a string or `{ what, not_for?, examples? }`.
- A manifest, an overlay and a vocabulary take their file form, normalized — `effect` explicit, lists and tables
  present, no `reflex` key — plus what reading found: `unknown` keys, an overlay's `renamed` and `orphaned`. A project
  and a lock are typed, their refs parsed.
- The decision's line, which `--json` prints: `input`, the decision's fields, `trace`, then `result` or `error`; the
  log keeps the same line with `answers` and `proposed` beside it, which `why` reads back.

## Transcripts

`transcripts/<flow>/` holds — the flow named for what it shows, never `vocab` or `overlays`, since the schema globs
and the seeds test read those directory names as owned files:

- `session.txt` — the terminal. A line starting with `$ ` is a command, run by `sh -c` in `$HOME` under a
  pseudo-terminal with `TERM=dumb` and `NO_COLOR=1`, so the plain text is what is compared; the lines under it are
  what the terminal showed, stdout and stderr as they came. A prompt ends with
  `> ` and what was typed follows on the same line; a prompt with nothing after it gets the end of input, so `>`
  alone ends a REPL. `[N]` alone on a line is a non-zero exit code; absent, the command exited 0. Lines compare
  exactly but for trailing spaces; a line that is JSON is compared as JSON, `ms` values aside. A line starting with
  `#` is a note for the reader; the note `# no tty` runs the flow's commands without a terminal.
- `home/` — the throwaway `$HOME`: the project under `.config/evoke/`, local reflexes, XDG state as the flow needs it.
  A flow without one uses [`transcripts/home/`](home/.config/evoke/evoke.toml). Paths under it print as `~/…`. The
  runner records the `node` on its `PATH` in the home's state, as `evoke sync` will on a machine.
- `answers.toml` — what the `replay` adapter answers, keyed by utterance identity, with the gate it declares; the
  runner names it in `EVOKE_ANSWERS`. A flow without one makes no decision.
- `remote/<owner>/<repo>/<tag>/` — a remote's tree at that tag. The runner builds a bare repository from the trees,
  one commit per tag in version order, author and committer `evoke spec <spec@evoke.build>` at
  `2026-09-19T00:00:00Z`, message the tag; then points `https://github.com/<owner>/<repo>` at it through git's
  `url.<path>.insteadOf`. A lock in `home/` carries the commit and `h1` that recipe yields.
