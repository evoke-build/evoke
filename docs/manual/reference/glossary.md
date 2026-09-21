# Glossary

Every term `evoke` uses, one line each.

| Term             | Meaning                                                                                                   |
| :--------------- | :-------------------------------------------------------------------------------------------------------- |
| **Abstain**      | The outcome when *none* wins the route, or the winner is under the route floor. Exit 2                    |
| **Adapter**      | The classifier behind a decision, as an object answering typed questions with probabilities. Jev is the first; `replay` answers from a recording |
| **Argument**     | A question about the input and a value for the body: `options`, `vocab`, `pick` or `flag`                 |
| **Ask**          | The outcome when a required argument is missing: `evoke` asks the argument's own question                 |
| **Body**         | What `run` names: a `.mts`/`.mjs` file exporting a function, or a program with its arguments              |
| **Call**         | A reflex with its arguments filled, on one line: `lights room="den" state="off"`                          |
| **Cap**          | A reason a complete call stops at confirm: destructive, no gate, under floor, an unconsumed span, two things |
| **Collection**   | A repository of reflex directories                                                                        |
| **Confidence**   | The lowest top probability among the route and every argument question of the winner                     |
| **Confirm**      | The outcome when a call is complete but capped: `[y]es [n]o [t]each`                                      |
| **Contract**     | What a user cannot override: `run`, argument names and sources, option keys, ranges, config keys          |
| **Effect**       | What running a reflex does: `read`, `write` or `destructive`. Absent means destructive                    |
| **Effective manifest** | The shipped manifest with your overlay merged in; what `show <name>` prints                         |
| **Fits**         | The yes/no question per reflex: does it do what was asked? Its runner-up floor marks an input asking for two things |
| **Flag**         | A yes/no argument. The body receives `true` or nothing                                                    |
| **Floor**        | The bar a decision must clear. A threshold the adapter ships, meaning *the probability this is right*: `route`, `fits`, `read`, `write` |
| **Gate**         | The step that turns confidence and effect into an outcome                                                 |
| **h1**           | The content hash of a reflex directory, `h1:<sha256>`, in the lock and the store                          |
| **Home project** | `~/.config/evoke/`: the project used when no `evoke.toml` is found upward                                  |
| **Hub**          | Where reflexes are found; today the collection `evoke-build/reflexes`                                     |
| **Identity**     | How utterances are keyed: NFC, lower-cased, whitespace collapsed, trailing punctuation dropped            |
| **Inactive**     | A reflex left out of every decision until a problem is fixed: an empty vocabulary, an unset setting, an overlay that does not read |
| **Local name**   | The key under `[reflexes]`: what the classifier reads and what the overlay file is named after            |
| **Lock**         | `evoke.lock`: per remote reflex, the ref, tag, commit, `h1` and consented effect. Also the adapter         |
| **Manifest**     | `reflex.toml`: what the classifier reads and what the body receives                                       |
| **Overlay**      | `overlays/<name>.toml`: your wording for one reflex, merged over the shipped one                          |
| **Pick**         | An argument read from an exact span of the input: `number`, `duration`, `email`, `url`, `quoted`           |
| **Plan**         | The installed set compiled to questions. Its digest keys the cache, the baselines and every decision      |
| **Project**      | A directory of files you own: `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`                            |
| **Record**       | One line of `[examples]` or `[tests]`: `"utterance" = { assertions } | false`                              |
| **Recording**    | An `answers.toml`: an adapter's declaration and its answers by utterance identity                         |
| **Ref**          | Where a reflex comes from: `owner/repo[/dir][@tag]`, a git URL, or `./dir`                                |
| **Reflex**       | A directory: a manifest and the file or program it names. Plural: reflexes                                |
| **Route**        | The one choice over every installed reflex plus *none*                                                    |
| **Run**          | The outcome when a call clears its floor. Also the key naming the body                                    |
| **Runtime**      | What runs a body: Node for a file, nothing for an argv                                                    |
| **Span**         | An exact piece of the input, by character offsets                                                         |
| **Store**        | `~/.cache/evoke/store/<h1>/`: fetched code, re-hashed before every run                                    |
| **Tag**          | A word under `tags` for `--tag` to narrow by. Also a git version tag                                      |
| **Trust**        | A project's four owned paths bound to their content. Required outside home                                |
| **Unstated**     | The answer that an argument was not given in the input                                                    |
| **Utterance**    | A sentence a record is keyed by                                                                           |
| **Vocabulary**   | `vocab/<name>.toml`: your closed list of words, shared by every argument that names it                    |
| **Wording**      | What a user may override: descriptions, questions, option meanings, records                               |
