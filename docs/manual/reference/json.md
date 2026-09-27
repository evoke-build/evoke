<!-- title: JSON output -->
<!-- description: What --json prints: one line per input, with the decision, every judgment, each call to the classifier, and the result or the error, field by field. -->
# The JSON line

`--json` prints one line per input. The line holds the input, the decision's fields flattened next to it, the
adapter calls made, and the result or the error when a body ran. It is the filter format. The log holds the same
line, with the adapter's raw answers and the input's candidates next to it. `why` reads that. The SDK's
`Decision` is the same object, plus `values` and `plan`.

## Shape by outcome

| Field        | run | confirm | ask | abstain | Holds                                                          |
| :----------- | :-: | :-----: | :-: | :-----: | :------------------------------------------------------------- |
| `input`      | ●   | ●       | ●   | ●       | The sentence as decided                                        |
| `outcome`    | ●   | ●       | ●   | ●       | `"run"`, `"confirm"`, `"ask"`, `"abstain"`                      |
| `reflex`     | ●   | ●       | ●   |         | The winner's local name                                        |
| `args`       | ●   | ●       | ●   |         | Per argument, a typed value; partial for an ask                 |
| `call`       | ●   | ●       |     |         | The call on one line: `lights room="den" state="off"`           |
| `effect`     | ●   | ●       |     |         | `"read"`, `"write"`, `"destructive"`                            |
| `confidence` | ●   | ●       | ●   |         | The weakest judgment's probability                              |
| `weakest`    | ●   | ●       | ●   |         | `{ question, top, p }`                                          |
| `judgments`  | ●   | ●       | ●   | ●       | Every choice read: `{ question, top, p }`                       |
| `contenders` | ●   | ●       | ●   | ●       | The ranking: `{ reflex, route, fits? }`                         |
| `runner_up`  | ○   | ○       | ○   |         | The second reflex, when there is one                            |
| `prompt`     |     | ●       |     |         | `{ own, template }`: `evoke`'s line and the reflex's question   |
| `because`    |     | ●       |     |         | Why it stopped, in order: `destructive`, `no_gate`, `under_floor`, `unconsumed_span`, `two_things`, `merged` |
| `unconsumed` |     |         | ●   |         | Typed spans no argument took                                    |
| `missing`    |     |         | ●   |         | Per missing argument: `arg`, `ask`, `because`, `choices`        |
| `trace`      | ●   | ●       | ●   | ●       | One entry per adapter call: `{ adapter, questions, ms }`. Empty when the answers came from the cache |
| `result`     | ○   |         |     |         | `{ text, data? }` when the body ran                             |
| `contained`  | ○   |         |     |         | Whenever the body ran, with `result` or `error`: whether this machine held it to its declaration, `{ "type": "full" }`, or `"partial"` or `"none"` with `why`, one sentence |
| `error`      | ○   |         |     |         | The failure's message when it did not. The line is all that prints; the exit is 1 |
| `frames`     | ○   |         |     |         | With `error`, the frames of the error a JavaScript body threw, from its first: `at body (throws.mts:1:1)` |
| `cancelled`  | ○   |         |     |         | `true` when `Ctrl-C` ended the body; a step of a weave says it as `status` and `why` instead |

A sentence read as several steps ([Weaving](../use/weaving.md)) prints one line per step, as each runs, the
line of its decision with five fields more: `step` and `steps` first, the step's number and the count; `bound`
after `trace`, where a value came from another step, `[{ arg, from, field, value }]`, `value` absent for a whole
result, which stays on its source's line as `result.data`; `shared`, where the sentence said a word once for
several steps and this one took it, `{ "<arg>": { "word", "via" } }`, `via` `fill` for a required argument and
`rewrite` for an optional one written into the step's words; and `status` last, with `why` when the step stopped.
A request that is only what not to do prints one `abstain` line over the whole input, with no judgments and no
contenders. A plan stopped before any step ran prints every step's line at once, its `status` `refused`,
`skipped`, `declined` or `unanswered`. A step run from a plan file carries `pinned` too, `{ "file", "id" }`: the
file as shown, `~/month.plan.json`, and the SHA-256 of what was read, `h1:…`. `evoke try --json` prints the plan
whole instead, on one line: `input`, `steps` with each step's decision, `binds`, `stages`, `verdict`, `exclusive`,
`excluded`, `splits`, and `trace`, every adapter call the plan took.

| Field    | Holds                                                                                                   |
| :------- | :------------------------------------------------------------------------------------------------------ |
| `status` | `"ran"`, `"failed"`, `"declined"`, `"refused"`, `"skipped"`, `"unanswered"`                              |
| `why`    | `{ "type": "earlier_step" }`, `{ "type": "nothing_to_take", "from" }` and `{ "type": "too_large", "from" }` with the source step, `found_nothing`, `no_reflex`, `cancelled`, `{ "type": "read_as", "reflex" }`, or `{ "type": "said", "message" }`: a prompt's own line, a failure, a question no one answered |

In the plan `evoke try --json` prints, a step's `refs` count the steps they may name from 0; `step`, `after`,
`stages` and `binds` count from 1. A step carries `shared` as the line does, and `repair` where it was not one
part decided on its own: `narrowed` or `spliced`, an item of its neighbour's task; `merged`, a part read with
its neighbour; `split`, a part of a joint the classifier read as one thing. A binding's `via` is `fill`,
`rewrite` or `takes`, a whole result by the name its source returns, with no `kind`; the verdict's `because`
names `no_source` and `several_sources` where such a result stops the plan.

A call by name, `evoke run --json`, prints the same line with nothing judged: `input` is empty, there is no
`confidence`, `weakest`, `judgments` or `contenders`, `trace` is empty, and `result` or `error` says what the body
did. A destructive call carries `prompt` and `because` like any confirm. Nothing was decided.

## The plan file

`evoke try --save <file>` writes the plan whole as JSON, readable without a tool
([Files](files.md#a-plan-file)): `note`, fixed text; `plan`, `1`; `input`, the sentence as typed; `tags`, when
any; `weave`, the plan as `try --json` prints it less `trace`; `evoke`, the version that wrote it; `adapter`, the
lock's `{ name, id }`; `gate`, the adapter's floors, absent when it declares none; `set`, the plan digest;
`reflexes`, per active reflex `item`, the hash of its wording, consent and settings, with `path` for a local
one, or `ref`, `tag` and `h1` for a remote one, or nothing more for one handed to the SDK as code; `vocab`, each
vocabulary's hash; `answers`, every classifier answer the plan took, `{ text, raw }` each, the whole sentence's
own questions first. `run <file>` refuses the file when any of these differs here, with the line and the fix.

## Values

```json
{ "type": "option", "key": "off" }
{ "type": "word", "word": "office", "value": "group-7" }
{ "type": "pick", "span": { "start": 18, "end": 28, "text": "10 minutes" }, "value": { "type": "duration", "value": 600 } }
{ "type": "flag" }
```

A pick's `value` is what the body receives: the number, the seconds, the text. Its `span` is where it was read,
in character offsets of the input.

## Examples

A run, from `evoke try --json`:

```json
{"input":"kill the lights in the den","outcome":"run","reflex":"lights","args":{"room":{"type":"word","word":"den"},"state":{"type":"option","key":"off"}},"call":"lights room=\"den\" state=\"off\"","effect":"write","confidence":0.85,"weakest":{"question":"lights.room","top":"den","p":0.85},"judgments":[{"question":"route","top":"lights","p":0.91},{"question":"lights.room","top":"den","p":0.85},{"question":"lights.state","top":"off","p":0.88}],"runner_up":{"reflex":"timer","route":0.02,"fits":0.05},"contenders":[{"reflex":"lights","route":0.91,"fits":0.7},{"reflex":"timer","route":0.02,"fits":0.05},{"reflex":"volume","route":0.01,"fits":0.05}],"trace":[]}
```

An ask, from a filter with no terminal. The line stands for the prompt that could not be shown, exit 3:

```json
{"input":"kill the lights","outcome":"ask","reflex":"lights","args":{"state":{"type":"option","key":"off"}},"unconsumed":[],"confidence":0.58,"weakest":{"question":"lights.state","top":"off","p":0.58},"judgments":[{"question":"route","top":"lights","p":0.9},{"question":"lights.room","top":"unstated","p":0.75},{"question":"lights.state","top":"off","p":0.58}],"runner_up":{"reflex":"timer","route":0.02,"fits":0.05},"contenders":[{"reflex":"lights","route":0.9,"fits":0.7},{"reflex":"timer","route":0.02,"fits":0.05},{"reflex":"volume","route":0.02,"fits":0.05}],"missing":[{"arg":"room","ask":"Which room?","because":{"type":"unstated"},"choices":{"type":"vocab","words":{"den":"The TV room downstairs; also 'the snug'.","office":"The upstairs study."}}}],"trace":[]}
```

## Questions

A question id is `route`, `fits.<reflex>`, `<reflex>.<argument>`, or `weave.<name>`: a question `evoke` asks on
its own account, beside a reflex's, and no reflex is named `weave`. A choice's keys are option keys, vocabulary
words, `<start>-<end>` for a pick's candidates, `yes` and `no` for a flag, local names for the route, and the
sentinels `none` and `unstated`.

## `evoke calibrate --json`

One object, the report of [Calibrating](../use/calibrating.md): `adapter`; `records`, `reflexes`, `inputs`,
`repeats`; `outcomes` with `run`, `confirm`, `ask`, `abstain`; `bins`, each `{ lo, hi, calls, right, interval,
claimed, thin, over_confident }`, a bin with no call left out; `unknown`; `abstained` as `{ count, right,
interval }`; `bars` with `read` and `write` where the effect has a call, each `{ bar, wrong, calls,
per_thousand, at_most, near }` and `near` a list of `{ at, run, wrong }`; `questions`, each `{ kind, judgments,
right, interval, claimed }` with `kind` one of `route`, `options`, `vocab`, `pick`, `flag`; `brier` with
`brier`, `reliability`, `resolution`, `uncertainty`, absent without a call; `misses`, each `{ case, outcome,
reflex?, confidence?, mismatch, wrong }`; under `--repeat`, `variance` with `flips`, `verdict_flips`, `spread`
as `{ median, p90, max }`, `straddling`, `wrong_at_bar`, and `moved`, each `{ utterance, confidence?, route,
winners, outcomes, wrong }`; and `log` with `adapter`, `decisions`, `ran`, `confirmed_ran`,
`confirmed_stopped`, `asked`, `abstained`, `failed`, `skipped`, `stopped_by_confidence` as `[{ lo, hi, count }]`,
`records` as `{ count, right, interval }`, and `unread`. An interval is `[low, high]`; a rate is per thousand.

## Rules of the wire

Keys are `snake_case`. An absent optional is omitted, never `null`. A tagged value carries `type`. A decision
carries `outcome`. Numbers are numbers: `0.85` and `0.850` are one value.
