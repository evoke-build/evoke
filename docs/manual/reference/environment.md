<!-- title: Environment variables and paths -->
# Environment

The variables `evoke` reads, and the paths it writes under. Nothing is read from a file it does not own, and
nothing is sent anywhere but the adapter you chose.

## Variables `evoke` reads

| Variable            | Read by                        | Means                                                                    |
| :------------------ | :----------------------------- | :----------------------------------------------------------------------- |
| `TYPESAFE_API_KEY`  | The `jev` adapter, when deciding | The classifier's key, from typesafe.ai. Never written to a file        |
| `OPENJEV_API_KEY`   | The `openjev` adapter, when deciding | The classifier's key, from openjev.sh. Never written to a file      |
| `EVOKE_ANSWERS`     | The `replay` adapter           | A recording to answer from, with `adapter = "replay"` in `evoke.toml`     |
| `<VAR>` of a `--env` setting | A body's run           | A config value, most often a secret, resolved for that run only          |
| `NO_COLOR`          | The terminal                   | Set and not empty: no colour. The spinner and the line editor stay        |
| `TERM`              | The terminal                   | Unset, empty or `dumb`: no colour, no spinner, no line editor            |
| `HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME` | Paths | Where the project and the machine-local state live |
| `PATH`              | `add`, `sync`, every run       | Where `node` is found and recorded, and where a declared program or an argv's program is found at each run; `git` too |
| `GIT_*`             | Fetching                       | Your git configuration and credentials apply; `GIT_TERMINAL_PROMPT=0` is set |
| `HTTPS_PROXY`, `NO_PROXY` | The classifier's connection | A proxy the connection goes through, in the CLI and the SDK, and the hosts that bypass it; `https_proxy` and `no_proxy` win over the upper-case names. An address that is not `http` or `https` is refused, never bypassed. The endpoint itself never moves |

## Paths

| Path                                             | Holds                                                        | Written by            |
| :----------------------------------------------- | :----------------------------------------------------------- | :-------------------- |
| `$XDG_CONFIG_HOME/evoke/` · `~/.config/evoke/`    | The home project                                             | you, `add`, `teach`, … |
| `$XDG_CACHE_HOME/evoke/store/<h1>/`              | Fetched reflex directories, by content hash                   | `add`, `sync`         |
| `$XDG_CACHE_HOME/evoke/answers/<plan>/`          | The adapter's answers, per installed set, utterance and questions asked | every decision, `try` |
| `$XDG_CACHE_HOME/evoke/baselines/<plan>.json`    | `evoke test`'s last verdicts per installed set                | `test`                |
| `$XDG_STATE_HOME/evoke/log.jsonl`                | One JSON line per decision; `why` reads the last              | every decision        |
| `$XDG_STATE_HOME/evoke/trust.toml`               | Trusted project roots and their content digests                | `trust`, every write  |
| `$XDG_STATE_HOME/evoke/runtime`                  | The real path of `node` a file body runs under                 | `add`, `sync`         |
| `$XDG_STATE_HOME/evoke/history`                  | The REPL's lines, the last 1 000                               | the REPL              |
| `$TMPDIR/evoke-…/`                               | A body's private temporary folder, for one run                 | every run; removed after it |

`$XDG_STATE_HOME` defaults to `~/.local/state`, and `$XDG_CACHE_HOME` to `~/.cache`. Everything under cache can
be deleted. `sync` and the next decision rebuild it. No key and no config value is ever written under either. The
log and the history hold what you typed and what a body returned. Only you can read them.

## What a body sees

A file body runs under a scrubbed environment of five variables: `PATH`, `HOME`, `TMPDIR`, `LANG`, `TERM`, with
`TMPDIR` a private folder made for the run. It receives its arguments, the input, its config and a signal
through the call, and touches what its manifest declares under `[needs]`: [The body](../author/body.md#the-run).
An argv body gets the same five, plus:

| Variable              | Holds                                          |
| :-------------------- | :--------------------------------------------- |
| `EVOKE_CONFIG_<KEY>`  | Each `[config]` key, upper-cased, as its value; a plain one under the home expanded |
| `EVOKE_INPUT`         | The sentence as typed                          |

Secrets reach a body only this way, for the length of one run.

## Time

One decision has 30 seconds, shared by the adapter's answer and the body's run. A prompt never counts. The `jev`
adapter gives a request 1.5 seconds once connected, and `openjev` 3 seconds, since OpenJEV forwards the request
onward. Each retries once after a connect error or a server error, and waits out a 429 that says how long to
wait, within the deadline.
