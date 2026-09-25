<!-- title: Decisions in the SDK -->
# Decisions

One input decided is a `Decision`. It is a union, narrowed by `outcome` and then by `reflex`. `decide` makes one.
`fill` answers an ask. `run` runs a chosen call. `handle` does all of it.

## `decide(input, { tags?, signal? })`

```ts
const d = await project.decide("kill the lights in the den", { tags: ["home"] })
```

Asks the adapter, reads its answers, and gates them. Runs nothing. `tags` offers only the reflexes carrying one
of them. `signal` aborts the adapter call, and `decide` rejects with the signal's own reason.

## The four shapes

| `outcome`   | Carries                                                                                           |
| :---------- | :------------------------------------------------------------------------------------------------ |
| `"run"`     | `reflex`, `args`, `values`, `call`, `effect`, `confidence`, `weakest`, `judgments`, `contenders`, `runner_up?` |
| `"confirm"` | All of the above, plus `prompt: { own, template }` and `because: Cap[]`: every reason it stopped, in order |
| `"ask"`     | `reflex`, partial `args` and `values`, `unconsumed`, `missing: Missing[]`, and the judgments so far |
| `"abstain"` | `contenders`, the ranking, and `judgments`                                                        |

Every decision also carries three more things. `input` is the sentence as decided. `plan` is the digest it was
decided under. `trace` has one entry per adapter call: `{ adapter, questions, ms }`.

```ts
switch (d.outcome) {
  case "run":     await project.run(d); break
  case "confirm": if (await ui.confirm(d.prompt.template)) await project.run(d, { confirmed: true }); break
  case "ask":     project.fill(d, await ui.pick(d.missing)); break
  case "abstain": ui.say(d.contenders); break
}
```

## `args` and `values`

`args` is what the classifier read, typed: `{ type: "option", key }`, `{ type: "word", word, value? }`,
`{ type: "pick", span, value }`, `{ type: "flag" }`. `values` is what the body receives: the key, the word's
value or else the word, the number or text, `true`. With `Reflexes` from `evoke.d.ts`, or a manifest handed as
code, both are typed per reflex once `d.reflex` is known:

```ts
if (d.outcome === "run" && d.reflex === "lights") {
  d.values.room    // string
  d.values.state   // "on" | "off" | "dim"
}
```

An app that wants a reflex's wording with a body of its own calls `decide`, switches on `d.reflex`, and reads
`d.values`.

## Confirm

`prompt.own` is `evoke`'s line: the call, the effect, the weakest judgment, and each cap that names itself.
`prompt.template` is the reflex's own question, filled in. `because` lists why it stopped: `destructive`,
`no_gate`, `under_floor`, `unconsumed_span`, `two_things`. A confirm decision runs only with
`run(d, { confirmed: true })`.

## Ask and `fill(d, given)`

`missing` says, per argument, its `ask`, why it is missing, and what it may be. Missing means never stated, or a
pick out of range. What it may be is the options, the vocabulary's words, or which recognizer reads it. `fill`
takes what a person answered, by argument name: an option's key, a word, or the text a pick reads. It types the
answer and gates again, synchronously:

```ts
const filled = project.fill(d, { room: "den" })       // a Decision again: run, confirm, or still an ask
```

A text that does not read is a `DiagnosticError` naming the argument and what it may be:
`room: "attic" is not one of den, office`. A flag is never asked.

## `run(d, { signal? })`

Runs the chosen call's body and resolves to `{ text, data? }`. A reflex handed as code runs in-process. A file
runs in a child. An argv is spawned. A body's failure is a `FailureError`. `signal` aborts the body, and `run`
rejects with the signal's reason. A decision made under another plan is refused as misuse, with a `TypeError`.

## `handle(input, { confirm?, ask?, tags?, signal? })`

The whole loop: decide, then ask and fill until nothing is missing, then confirm, then run.

```ts
const handled = await project.handle(input, {
  confirm: d => ui.confirm(d.prompt.template),   // true runs, false declines
  ask: d => ui.pick(d.missing),                  // answers by name, or undefined to decline
})
```

| `handled.outcome` | When                                                                                          |
| :---------------- | :-------------------------------------------------------------------------------------------- |
| `"ran"`           | The body ran: `decision` and `result`                                                         |
| `"abstained"`     | `none` won, or the route was under its floor                                                  |
| `"declined"`      | `confirm` returned `false`, `ask` returned `undefined`, or an answer changed nothing           |
| `"unanswered"`    | A confirm or an ask was reached and no handler was given. The decision itself is the answer   |

An answer that does not read is asked again once. Twice is a decline. An empty input is refused before the
adapter is asked. Only a diagnostic, a fault or a failure throws.

## `steps(input, { tags?, signal? })` and `weave(input, { confirm?, ask?, proceed?, tags?, signal? })`

A sentence that asks for several things is a weave ([Weaving](../use/weaving.md)). `steps` reads one into its
plan and runs nothing:

```ts
const plan = await project.steps("look up dana's address and email them")
plan.steps       // [{ n: 1, text, decision, reflex: "contact", effect: "read", ... }, { n: 2, ... }]
plan.binds       // [{ from: 1, to: 2, arg: "to", field: "email", kind: "email", via: "fill" }]
plan.stages      // [[1], [2]]: a stage's steps run together; stages run in order
plan.verdict     // { outcome: "run" | "ask" | "confirm" | "refuse", because?: [...] }
```

Each step carries its `decision`, the `Decision` `decide` would have made of its words alone. `weave` settles
what the plan asks first, then runs every step under `handle`'s handlers, each told which step and round asks,
a `Turn`:

```ts
const woven = await project.weave(input, {
  ask: (d, turn) => ui.pick(d.missing, turn.step),   // before anything runs, for a required argument no step provides
  confirm: (d, turn) => ui.confirm(d.prompt.template), // at the step's turn
  proceed: plan => ui.confirm("Run the plan as it stands?"),  // a step refers to another it takes nothing from
})
woven.status     // the worst step's: "ran", "failed", "declined", "refused", "unanswered"; or why nothing ran
woven.steps      // per step: { step, status, why?, bound, rounds: [{ round, input, decision, status, result? }] }
```

A body's failure is the step's, with `status: "failed"`, never a throw. A step after one that stopped is
`skipped`, with `why: { type: "earlier_step" }`. A step bound to a list of records runs once per record, one
round each.

**Next:** [Adapters](adapters.md).
