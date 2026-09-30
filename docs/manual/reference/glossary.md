# Glossary

Every term `evoke` uses, one line each: the outcomes of a decision, the parts of a reflex, the files of a project, the
adapter, and the words of a weave.

| Term             | Meaning                                                                                                   |
| :--------------- | :-------------------------------------------------------------------------------------------------------- |
| **Abstain**      | The outcome when *none* wins the route, or the winner is under the route floor. Exit 2                    |
| **Adapter**      | The classifier behind a decision, as an object answering typed questions with probabilities. `jev` and `openjev` reach Jev, through TypeSafe AI's API or through OpenJEV; `replay` answers from a recording |
| **Argument**     | A question about the input and a value for the body: `options`, `vocab`, `pick` or `flag`                 |
| **Ask**          | The outcome when a required argument is missing: `evoke` asks the argument's own question                 |
| **Binding**      | A value of one step's result taken by a later step, by kind: filled into a required argument, or written into the words for an optional one; or the whole result, by the name its source returns, handed beside the decision |
| **Body**         | What `run` names: a `.mts`/`.mjs` file exporting a function, or a program with its arguments; a playbook has `steps` instead |
| **Branch**       | Steps of a playbook that run only under a value an earlier step yields, `{ say = "…", when = { landing = "yes" } }`, listed right after that step; the plan prints every one, the result picks, the rest are skipped clean |
| **Call**         | A reflex with its arguments filled, on one line: `lights room="den" state="off"`                          |
| **Cap**          | A reason a complete call stops at confirm: destructive, no gate, under the floor, a value read from your words another way, words that ask for more, a call that holds less than you typed |
| **Change**       | A reflex whose `effect` is `write` or `destructive`: it changes something, where a lookup only reads     |
| **Collection**   | A repository of reflex directories                                                                        |
| **Confidence**   | The lowest probability among the route and every value the call holds                                    |
| **Confirm**      | The outcome when a call is complete but capped: `[y]es [n]o [t]each`                                      |
| **Contract**     | What a user cannot override: `run` or `steps`, argument names and sources, option keys, ranges, config keys, what a reflex yields, returns and takes |
| **Effect**       | What running a reflex does: `read`, `write` or `destructive`. Absent means destructive, so a manifest always writes it |
| **Effective manifest** | The shipped manifest with your overlay merged in; what `show <name>` prints                         |
| **Fits**         | The yes/no question asked when a reflex is added: does it do what another reflex's example asks? At or above its floor, `add` names the example |
| **Flag**         | A yes/no argument. The body receives `true` or nothing                                                    |
| **Floor**        | The bar a decision must clear. A threshold the adapter ships, meaning *the probability this is right*: `route`, `read`, `write`, `whole`, and `fits` at `add` |
| **Fold**         | A part of a sentence that repeats a step a playbook wrote: removed from the plan and named under it, `folded "…" into <n>` |
| **Gate**         | The step that turns confidence and effect into an outcome                                                 |
| **h1**           | The content hash of a reflex directory, `h1:<sha256>`, in the lock and the store                          |
| **Home project** | `~/.config/evoke/`: the project used when no `evoke.toml` is found upward                                  |
| **Hub**          | Where reflexes are found; today the collection `evoke-build/reflexes`                                     |
| **Identity**     | How utterances are keyed: NFC, lower-cased, whitespace collapsed, trailing punctuation dropped            |
| **Inactive**     | A reflex left out of every decision until a problem is fixed: an empty vocabulary, an unset setting, an overlay that does not read |
| **Local name**   | The key under `[reflexes]`: what the classifier reads and what the overlay file is named after            |
| **Lock**         | `evoke.lock`: per remote reflex, the ref, tag, commit, `h1` and consented effect. Also the adapter         |
| **Lookup**       | A reflex whose `effect` is `read`: it finds or shows something, and changes nothing                      |
| **Manifest**     | `reflex.toml`: what the classifier reads and what the body receives                                       |
| **Near neighbour** | What a person might type that sounds like a reflex's request and is not: named under `not_for`, and written as a `false` test |
| **Overlay**      | `overlays/<name>.toml`: your wording for one reflex, merged over the shipped one                          |
| **Pick**         | An argument read from an exact span of the input: `number`, `duration`, `email`, `url`, `quoted`, `date`, `time`, `amount`, `code` |
| **Recalled**     | A value an earlier body of this session returned under the field a pick's `recent` names, listed at the ask and chosen by you |
| **Pin**          | What a plan file is checked against before it runs: the `evoke` version, the adapter's id and gate, the set's digest, each reflex and vocabulary by hash, a remote reflex's `h1` |
| **Plan**         | The installed set compiled to questions. Its digest keys the cache, the baselines and every decision      |
| **Plan file**    | A weave saved by `try --save`: the sentence, the classifier's answers and the pins it was decided under; `run <file>` runs it exactly, or names the pin that moved |
| **Playbook**     | A reflex whose body is a plan: `steps`, one sentence per step with `{slots}`, reached by a short sentence and decided step by step over the installed set |
| **Project**      | A directory of files you own: `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`                            |
| **Record**       | One line of `[examples]` or `[tests]`: `"utterance" = { assertions } | false`                              |
| **Recording**    | An `answers.toml`: an adapter's declaration and its answers by utterance identity                         |
| **Ref**          | Where a reflex comes from: `owner/repo[/dir][@tag]`, a git URL, or `./dir`                                |
| **Reflex**       | A directory: a manifest and the file or program it names. Plural: reflexes                                |
| **Returns**      | The name a body's whole `data` goes by, `returns = "deploys"`, for a later step of one sentence to take    |
| **Route**        | The one choice over every installed reflex plus *none*                                                    |
| **Run**          | The outcome when a call clears its floor. Also the key naming the body                                    |
| **Runtime**      | What runs a body: Node for a file, nothing for an argv                                                    |
| **Situation**    | What a person faces, typed in one sentence, a problem or a routine: *checkout is failing in eu-west*, *close the month*. A playbook is reached by its situation |
| **Slot**         | An argument named in a step of a playbook, `{service}`: filled from the sentence or asked before the plan prints; in square brackets, written only when stated |
| **Span**         | An exact piece of the input, by character offsets                                                         |
| **Step**         | One part of a weave, decided as one input is, a playbook's sentences among them; numbered as the plan prints it, run at its turn |
| **Store**        | `~/.cache/evoke/store/<h1>/`: fetched code, re-hashed before every run                                    |
| **Tag**          | A word under `tags` for `--tag` to narrow by. Also a git version tag                                      |
| **Takes**        | An argument an earlier step's whole result fills by that name, `takes = "deploys"`: never asked, never stated, the plan alone hands it |
| **Trust**        | A project's four owned paths bound to their content. Required outside home                                |
| **Unstated**     | The answer that an argument was not given in the input                                                    |
| **Utterance**    | A sentence a record is keyed by                                                                           |
| **Vocabulary**   | `vocab/<name>.toml`: your closed list of words, shared by every argument that names it                    |
| **Weave**        | A sentence read as several steps: each decided on its own, ordered by the words, a result threaded into a later step, the plan shown before anything runs |
| **Wording**      | What a user may override: descriptions, questions, option meanings, records                               |
| **Yield**        | A field of a result's `data` a later step may take, declared under `[yields]` with the kind that reads it |
