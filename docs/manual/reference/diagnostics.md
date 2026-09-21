# Diagnostics

Every line that needs something from you has one shape — what is wrong, then the literal command that fixes it —
and exits 3:

```text
  lights: vocabulary "rooms" is empty  →  evoke vocab rooms add <word> "<meaning>"
  ~/app is not trusted                  →  evoke trust
```

Under `--json`, the decision line stands for a prompt that could not be shown. In the SDK the same lines are the
message of a `DiagnosticError`, each problem carrying its `fix` and `command`.

## The fixes

The set of fixing commands is closed. Each line ends in one of these:

| Fix                                        | The problem it answers                                                     |
| :----------------------------------------- | :------------------------------------------------------------------------- |
| `export <VAR>=<value>`                     | A key or a `--env` setting's variable is not set                            |
| `evoke add evoke-build/reflexes`           | Nothing is installed                                                        |
| `evoke add <ref> --as <name>`              | A local name is taken, or a `[reflexes]` line names something not locked    |
| `evoke vocab <name> add <word> "<meaning>"`| A vocabulary is empty, or a word is not in it                               |
| `evoke config <reflex> <key> <value>`      | A declared setting is not set                                               |
| `evoke config <reflex> <key> --env <VAR>`  | A secret is not set, or was set plain                                       |
| `evoke show [<reflex>]`                    | Something answered by looking: an unknown argument, an option not offered, an undeclared key, a reflex not installed |
| `evoke teach "<phrase>" not <reflex>`      | A newcomer steals a phrase an installed reflex claims                       |
| `evoke update [<reflex>]`                  | A name is already locked; a tag moved or vanished; a retired adapter        |
| `evoke update --accept <reflex>`           | Upstream loosened an effect you had consented to                            |
| `evoke sync`                               | A reflex is not in the store, or no runtime is recorded                     |
| `evoke trust`                              | The project is not trusted, or changed since it was                          |
| `evoke remove <reflex>`                    | A local reflex's directory or manifest is missing                           |
| `evoke check`                              | A body does not exist, does not load, or exports no function                |
| `evoke new <name>`                         | No `reflex.toml` here; or the directory already exists                      |
| `<file>:<line>:<column>`                   | A line of an owned file to edit: the schema, a contract key in an overlay, a loosening effect |
| the command you ran                        | Try again once the reason on the line is addressed: a prompt with no terminal, a value out of range |

## Why a reflex is inactive

An inactive reflex is left out of every decision, and `show` lists why, one line each:

| Line                                             | Means                                                    |
| :----------------------------------------------- | :------------------------------------------------------- |
| `vocabulary "rooms" is empty`                    | An argument names a vocabulary with no words             |
| `config "bridge" is not set`                     | A declared setting has no value                          |
| `config "token" is a secret; it is set from a variable` | A secret was given plain                           |
| `HUE_TOKEN is not set`                           | A setting names a variable the environment lacks         |
| an overlay line, with its file and position      | A contract key, a loosening effect, or a parse error in your overlay |
| a manifest line                                  | The shipped manifest does not read                       |

A missing store entry or runtime is not inactive but a stop: `evoke sync`. An effect upstream loosened is not
inactive either: the reflex runs under the effect you consented to until you accept.

## Failures and faults

A body that throws, exits non-zero, or overruns the deadline is a **failure**: its message prints, exit 1. An
adapter that cannot be reached, returns an error status, or answers something the core cannot validate is a
**fault**, exit 4; the core fails closed rather than guess. Both name what to do next.
