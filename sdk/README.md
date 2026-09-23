# @evoke-build/evoke

You invoke a function; you evoke a reflex. A reflex is a small program a classifier selects and calls. The SDK
loads a project of reflexes, decides what an input asks for, and runs it. The classifier is
[Jev](https://typesafe.ai), TypeSafe AI's System One model. It answers closed questions with calibrated
probabilities. `jev()` reaches it through TypeSafe AI's own API and `openjev()` through OpenJEV's public door; the
core is bound to neither. The rules run in `core.wasm`, the same core as the `evoke` CLI. The SDK adds files, the network
and a process.

Six things to learn, in order: `reflex()` · `load()` · `handle()` · a `Decision` · `run()` · `replay()`. Each is
explained in full in [the manual](https://evoke.build/manual/sdk/getting-started.html).

## The first hour

```ts
// app.ts · npm i @evoke-build/evoke · export TYPESAFE_API_KEY=… · node app.ts "timer for 10 minutes"
import { load, reflex } from "@evoke-build/evoke"
import { jev } from "@evoke-build/evoke/jev"

const timer = reflex({
  description: "Start a countdown timer.",
  effect: "write",
  confirm: "Start a {duration} timer?",
  args: { duration: { ask: "How long?", pick: "duration" } },
  examples: { "timer for 10 minutes": { duration: "10 minutes" } },
}, async ({ duration }) => {                                   // duration: number — seconds, typed from the manifest
  setTimeout(() => console.log("ring"), duration * 1000)
  return `ringing in ${duration} s`
})

const project = await load({ reflexes: { timer }, adapter: jev() })

const handled = await project.handle(process.argv[2] ?? "", {
  confirm: async d => { console.log(d.prompt.template); return true },   // "Start a 10 minutes timer?"
  ask: async d => ({ duration: "10 minutes" }),                           // d.missing says what is missing and why
})
console.log(handled.outcome === "ran" ? handled.result.text : handled.outcome)
```

A `Decision` is a union. Narrow it by `outcome`, one of `run`, `confirm`, `ask` and `abstain`, then by `reflex`.
It carries `args` as the classifier read them, `values` as the body receives them, `call` as one line,
`confidence`, the `prompt` of a confirm, the `missing` of an ask, and the `trace` of every adapter call.

## With files

`evoke add evoke-build/reflexes` next to the app writes `evoke.toml`, `evoke.lock` and `evoke.d.ts`. Overlays and
vocabularies under the same root tune the wording. That includes a reflex handed as code.

```ts
import type { Reflexes } from "./evoke.d.ts"

const project = await load<Reflexes>({ root: import.meta.dirname })       // the adapter evoke.toml names
const tenant = project.with({ vocab: { rooms: await db.rooms(user) } })   // a tenant's words, a tenant's plan; milliseconds
const d = await tenant.decide("kill the lights in the den")
if (d.outcome === "run" && d.reflex === "lights") console.log(d.values.room, d.values.state)   // typed by evoke.d.ts
```

## Tests

```ts
import { replay } from "@evoke-build/evoke/testing"

const record = process.env.RECORD ? jev() : undefined                     // RECORD=1 asks Jev once and writes the file
const project = await load({ reflexes: { timer }, adapter: replay("answers.toml", { record }) })
```

Every run after the first is offline, and always gives the same answers. The file is the CLI's own `answers.toml`.

## Errors

Every error ends in what to do: `lights: vocabulary "rooms" is empty  →  evoke vocab rooms add <word> "<meaning>"`.
`DiagnosticError` is something a person fixes. `FaultError` is the adapter. `FailureError` is a body or the
machine. Each carries the structured value, and the fixing line as `command`. Aborts reject with the signal's own
reason.
