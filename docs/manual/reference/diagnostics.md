# Diagnostics

Every line that needs something from you has one shape: what is wrong, then the literal command that fixes it.
It exits 3:

```text
  lights: vocabulary "rooms" is empty  →  evoke vocab rooms add <word> "<meaning>"
  ~/app is not trusted                  →  evoke trust
```

Under `--json`, the decision line stands for a prompt that could not be shown. In the SDK, the same lines are the
message of a `DiagnosticError`. Each problem carries its `fix` and `command`.

## The fixes

The set of fixing commands is closed. Each line ends in one of these:

| Fix                                        | The problem it answers                                                     |
| :----------------------------------------- | :------------------------------------------------------------------------- |
| `export <VAR>=<value>`                     | A key or a `--env` setting's variable is not set                            |
| `evoke add evoke-build/reflexes`           | Nothing is installed                                                        |
| `evoke add <ref> --as <name>`              | A local name is taken, or a `[reflexes]` line names something not locked    |
| `evoke vocab <name> add <word> "<meaning>"`| A vocabulary is empty, or a word is not in it                               |
| `evoke vocab <name> add <word> "<meaning>" --value <path>` | A word's value is not the path a declaration takes it for |
| `evoke vocab <name> remove <word>`         | A vocabulary offers more words than the classifier takes in one question    |
| `evoke vocab <name>`                       | A word to remove is not in the vocabulary                                   |
| `evoke add <ref>…`                         | The rest of a collection one taken name refused; a ref without a tag it has not, or without `--as` |
| `evoke config <reflex> <key> <value>`      | A declared setting is not set                                               |
| `evoke config <reflex> <key> --env <VAR>`  | A secret is not set, or was set plain                                       |
| `evoke show [<reflex>]`                    | Something answered by looking: an unknown argument, an option not offered, an undeclared key, a reflex not installed |
| `evoke teach "<phrase>" not <reflex>`      | A newcomer steals a phrase an installed reflex claims, or fits it over the floor |
| `evoke update [<reflex>]`                  | A name is already locked, a tag moved or vanished, or a retired adapter     |
| `evoke update --accept <reflex>`           | An effect upstream loosened, or a declaration it widened, since you consented |
| `evoke sync`                               | A reflex is not in the store, or no runtime is recorded                     |
| `evoke trust`                              | The project is not trusted, or changed since it was                          |
| `evoke remove <reflex>`                    | A local reflex's directory or manifest is missing; a fetched reflex reached past its declaration |
| `evoke check`                              | A body does not exist, does not load, or exports no function                |
| `evoke new <name>`                         | No `reflex.toml` here, or the directory already exists                      |
| `evoke test`                               | The theft test at `add` did not finish; the install stands                  |
| `mkdir -p <path>`                          | A folder a body's declaration writes into is not there                      |
| `evoke --help`                             | The arguments spell no command                                              |
| `<file>:<line>:<column>`                   | A line of an owned file to edit: the schema, a contract key in an overlay, a loosening effect, a local reflex's declaration to widen, or one naming a path that is not there |
| the command you ran                        | Try again once the reason on the line is addressed: a prompt with no terminal, a value out of range |

A body refused past its declaration, or a declared path the machine lacks, is a failure, exit 1, whose line
still ends in its fix: the manifest's line for a local reflex, `evoke remove <reflex>` or
`evoke update --accept <reflex>` for a fetched one, the value's source for a `{name}`, `mkdir -p` for a folder
the body writes into.

## What lint reports

At `evoke check` and `evoke add`, lint reports and never refuses. Each line starts with `lint`, names the reflex
and says what to change. The text is yours to weigh:

| Line                                                                          | Means                                                                  |
| :---------------------------------------------------------------------------- | :--------------------------------------------------------------------- |
| `the summary is 120 characters; the cap is 100`                               | A size passed its cap: the summary, the description, `not_for`, the options, the records, a sentence or a step |
| `description addresses the model: "you must"`                                 | A phrase speaks to a model instead of describing the action            |
| `effect is absent, which means destructive; write it`                         | The manifest leaves `effect` out, and a reader cannot see what that means |
| `confirm reads no value back; name {release}`                                 | The reflex has a required argument, and the confirm names none         |
| `confirm asks "Are you sure"; say what the call will do`                      | The confirm asks for a yes without saying what it covers               |
| `args.state.options.on repeats the ask; a meaning answers it`                 | An option's meaning is the question again                              |
| `args.label reads text in quotes, and no example shows them`                  | A `quoted` argument, and no example holds a quotation mark             |
| `args.level has no record that leaves it out; assert level = false once`      | No example or test asserts the option or the pick unstated             |
| `examples holds 2 requests; write three at least`                             | Fewer than three examples state a request of the reflex                |
| `step 4 holds "then"; one step is one action`                                 | A step holds a connective or a comma                                   |
| `step 6 states "#incident"; a word another team would change is a slot`       | A step writes in a channel or an address                               |
| `step 3 says "check that writes"; a step refers by "that <noun>": say "check whether"` | A step opens with `check that`                                |
| `step 1's words are the summary's or an example's; word a step apart from the situation` | Every word of a step that says something is in the playbook's own summary or examples |

`evoke add` prints two lines more for a playbook, once its steps are decided over the set it joins:

| Line                                                        | Means                                                             |
| :---------------------------------------------------------- | :---------------------------------------------------------------- |
| `claims write; its steps reach destructive`                 | A step reaches a reflex with a tighter effect than the playbook claims |
| `step 3 reaches logs, which lacks the tag outage`           | A request narrowed with `--tag outage` would leave that step without its reflex |

## Why a reflex is inactive

An inactive reflex is left out of every decision, and `show` lists why, one line each:

| Line                                             | Means                                                    |
| :----------------------------------------------- | :------------------------------------------------------- |
| `runs on macOS only`                             | The manifest's `platforms` leave this machine out        |
| `vocabulary "rooms" is empty`                    | An argument names a vocabulary with no words             |
| `config "bridge" is not set`                     | A declared setting has no value                          |
| `config "token" is a secret; it is set from a variable` | A secret was given plain                           |
| `HUE_TOKEN is not set`                           | A setting names a variable the environment lacks         |
| an overlay line, with its file and position      | A contract key, a loosening effect, or a parse error in your overlay |
| a manifest line                                  | The shipped manifest does not read                       |

A missing store entry or runtime is not inactive but a stop. The fix is `evoke sync`. An effect upstream loosened
is not inactive either. The reflex runs under the effect you consented to, until you accept.

## Failures and faults

A body that throws, exits non-zero, or overruns the deadline is a **failure**. Its message prints, exit 1. A
thrown error's frames are kept for its author: `evoke why` shows them under the decision, and `--json` carries
them as `frames`. An adapter that cannot be reached, returns an error status,
or answers something the core cannot validate is a **fault**, exit 4. A key the classifier refuses is one, and its
line ends in the `export` that replaces the key. The core fails closed rather than guess. Both name what to do
next. A path under your home prints as `~/…`.
