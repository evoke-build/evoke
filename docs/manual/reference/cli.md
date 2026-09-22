# Commands

```text
evoke 0.1.0 · you invoke a function; you evoke a reflex

use
  evoke "<input>"                         decide, gate, run
  evoke                                   the REPL; a piped line is one input
  evoke try "<input>"                     decide only, and show every judgment
    --json                                one JSON line per input, for a filter
    --tag <tag>                           only the reflexes carrying the tag
    --                                    the rest is input, even a command word
  evoke why                               the last decision, explained
  evoke run <call>                        by name, without the classifier

install
  evoke add <ref>… [--as <name>]          fetch, lint, lock, install
  evoke remove <name>
  evoke update [<name>]                   each remote reflex to its newest tag
  evoke update --accept <name>            a moved contract or effect, accepted
  evoke sync                              the lock realised on this machine
  evoke trust                             this project, trusted at its content

tune
  evoke show [<name>]                     the installed reflexes, or one as used
  evoke teach ["<utterance>"] <call>      the utterance means this call
  evoke teach ["<utterance>"] not <name>  the utterance is not this reflex
    ["<utterance>"] omitted means the last input
  evoke vocab <name>                      the words and their meanings
  evoke vocab <name> add <word> "<meaning>" [--value <v>]
  evoke vocab <name> remove <word>
  evoke config <name> <key> <value>       a setting; a secret as --env <VAR>
  evoke test [<name>]                     every example and test, judged

author
  evoke new <name>                        a working reflex from the template
  evoke check                             lint, types, contract against its tag

exit  0 ran · 1 failed · 2 declined · 3 needs a human · 4 adapter failed
```

## Grammar

- The first argument selects a command only when it is exactly a command word. Otherwise every argument is the
  input, bare words joined by one space. `--` forces input. Lines from stdin are always input.
- `--help`, `-h` and `--version` are flags in the first place. They print to stdout and exit 0.
- Commands validate every argument before acting. Arguments that spell no command end in `evoke --help`. The
  reserved words `edit`, `search`, `publish`, `adapter` and `calibrate` are refused by name until they exist.
- A **call** is `name arg=value…`. A value is bare or a JSON string, like `duration="10 minutes"`. A flag is its
  bare name. `run` and `teach` take a call as one argument or as separate words.

## Use

| Command                | Does                                                                                          | Exit |
| :--------------------- | :-------------------------------------------------------------------------------------------- | :--- |
| `evoke "<input>"`      | Decides, gates, runs. Prints the call and confidence on stderr, and the result on stdout. Logged | 0 · 2 · 3 · 1 · 4 |
| `evoke`                | On a terminal, the REPL: `> `, line editing, history. Piped, a filter: one input per line, and the first non-zero exit is kept | as each line |
| `evoke try "<input>"`  | Decides only: the ranking, each argument's distribution, each `fits`, the outcome and the weakest judgment. Never logged | 0 · 4 |
| `evoke why`            | The last logged decision, shown as `try` would show it, and what became of it                  | 0 · 3 |
| `evoke run <call>`     | Runs the call by name. No classifier. The effect policy is kept, so a destructive call confirms with `[y]es [n]o`. Not logged | 0 · 2 · 3 · 1 |

| Flag          | With            | Does                                                                     |
| :------------ | :-------------- | :----------------------------------------------------------------------- |
| `--json`      | input, `try`    | One JSON line per input: [The JSON line](json.md)                        |
| `--tag <tag>` | input, `try`    | Only reflexes carrying the tag; repeatable                                |
| `--`          | input           | The rest is input                                                        |

## Install

| Command                                | Does                                                                                  |
| :------------------------------------- | :------------------------------------------------------------------------------------ |
| `evoke add <ref>… [--as <name>]`       | Fetches each ref at its pin or newest tag, once per repository; reads a local ref, `./dir`, where it is. Lints, tests for stolen phrases, writes `evoke.toml`, the lock and `evoke.d.ts`, and records the runtime. `--as` names a single ref |
| `evoke remove <name>`                  | Drops the reflex from `evoke.toml` and the lock. Keeps your overlay, vocabularies, settings and the store's copy |
| `evoke update [<name>]`                | Moves each unpinned remote reflex, or one, to its newest tag. A pinned one moves to its pin. Reports. Never prompts or rewrites your files |
| `evoke update --accept <name>`         | Takes on an effect upstream loosened, at the current tag                              |
| `evoke sync`                           | Places every locked reflex in the store at its locked tag, and records the runtime. Never changes the lock |
| `evoke trust`                          | Trusts the project here, at the content of its four owned paths                       |

A ref is `owner/repo[/dir][@tag]` on GitHub, `<git url>[#dir][@tag]` over `https` or `ssh`, or `./dir` for a
local reflex: [Installing reflexes](../use/installing.md#refs).

## Tune

| Command                                                   | Does                                                                        |
| :-------------------------------------------------------- | :-------------------------------------------------------------------------- |
| `evoke show`                                              | Every installed reflex: name, ref and tag or `./dir`, effect, what it runs. Then the inactive lines |
| `evoke show <name>`                                       | The effective manifest as TOML, with `+` in the gutter of every line that is yours. Then its inactive lines |
| `evoke teach "<utterance>" <call>`                        | Writes the example to `overlays/<name>.toml`, with only what the call asserts |
| `evoke teach "<utterance>" not <name>`                    | Writes `"<utterance>" = false` to the reflex's overlay                       |
| `evoke teach <call>` · `evoke teach not <name>`           | The same, for the last input the log holds                                  |
| `evoke vocab <name>`                                      | Lists the words. Empty exits 3, with the add line                            |
| `evoke vocab <name> add <word> "<meaning>" [--value <v>]` | Adds or replaces a word                                                     |
| `evoke vocab <name> remove <word>`                        | Removes one. A word that is not there is refused                            |
| `evoke config <name> <key> <value>`                       | Sets a declared setting under `[config.<name>]`                             |
| `evoke config <name> <key> --env <VAR>`                   | Names the variable a setting is read from. The only way to set a secret     |
| `evoke test [<name>]`                                     | Decides every example and test of every active reflex, or of one, without the cache. Exits 1 when a case failed |

None of these needs the classifier's key.

## Author

| Command            | Does                                                                                                       |
| :----------------- | :--------------------------------------------------------------------------------------------------------- |
| `evoke new <name>` | Writes `./<name>/reflex.toml`, `<name>.mts` and `reflex.d.ts` from the template. Refuses a directory that exists |
| `evoke check`      | In a reflex directory: the manifest's lines to fix, the body found and loaded, lint, `reflex.d.ts` rewritten when it changed, and the contract diffed against the repository's newest tag |

## Exit codes

| Code | Meaning                                                                                  |
| :--- | :--------------------------------------------------------------------------------------- |
| 0    | Ran, or the command did what it said                                                     |
| 1    | A body or the machine failed. Also `evoke test` with a failing case                       |
| 2    | Abstained, or declined at a prompt                                                       |
| 3    | Needs a human: a missing key, nothing installed, an untrusted project, a prompt without a terminal, a line to fix |
| 4    | The adapter failed                                                                       |
