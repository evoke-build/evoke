# Getting started with the SDK

`@evoke-build/evoke` loads a project of reflexes, decides what an input asks for, and runs it — inside your
application, with the same rules as the CLI. The rules run in `core.wasm`, the CLI's own core; the SDK adds files,
the network and a process.

Six things to learn, in order: `reflex()` · `load()` · `handle()` · a `Decision` · `run()` · `replay()`.

## Install

```bash
npm install @evoke-build/evoke
```

Node 24 or newer, ES modules. The package ships three entries, so the main one imports no engine:

| Import                          | Holds                                                   |
| :------------------------------ | :------------------------------------------------------ |
| `@evoke-build/evoke`            | `load`, `reflex`, the types, the errors                  |
| `@evoke-build/evoke/jev`        | `jev()`, the first adapter                               |
| `@evoke-build/evoke/testing`    | `replay()`, the recorded adapter for tests               |

## The first hour

One file. A reflex handed as code, a project with nothing on disk, and the whole loop in one call:

```ts
// app.ts · export TYPESAFE_API_KEY=… · node app.ts "timer for ten minutes"
import { load, reflex } from "@evoke-build/evoke"
import { jev } from "@evoke-build/evoke/jev"

const timer = reflex({
  description: "Start a countdown timer.",
  effect: "write",
  confirm: "Start a {duration} timer?",
  args: { duration: { ask: "How long?", pick: "duration" } },
  examples: { "timer for ten minutes": { duration: "ten minutes" } },
}, async ({ duration }) => {                                   // duration: number — seconds, typed from the manifest
  setTimeout(() => console.log("ring"), duration * 1000)
  return `ringing in ${duration} s`
})

const project = await load({ reflexes: { timer }, adapter: jev() })

const handled = await project.handle(process.argv[2] ?? "", {
  confirm: async d => { console.log(d.prompt.template); return true },   // "Start a ten minutes timer?"
  ask: async d => ({ duration: "10 minutes" }),                           // d.missing says what is missing and why
})
console.log(handled.outcome === "ran" ? handled.result.text : handled.outcome)
```

- `reflex()` takes the manifest as an object — the file's shape, without `run` and `config` — and the body. The
  body's argument types are inferred from the manifest literal.
- `load()` builds a project: here from code alone, with the adapter passed in.
- `handle()` runs the whole loop — decide, ask until nothing is missing, confirm, run — with your two handlers, and
  returns `ran`, `abstained`, `declined` or `unanswered`.

## With files

Beside an app, `evoke add evoke-build/reflexes` writes `evoke.toml`, `evoke.lock` and `evoke.d.ts` exactly as it
does in your home project; overlays and vocabularies under the same root tune wording, including a reflex handed
as code.

```ts
import type { Reflexes } from "./evoke.d.ts"

const project = await load<Reflexes>({ root: import.meta.dirname })       // the adapter evoke.toml names
const d = await project.decide("kill the lights in the den")
if (d.outcome === "run" && d.reflex === "lights") console.log(d.values.room, d.values.state)   // typed by evoke.d.ts
```

A `Decision` is a union narrowed by `outcome` — `run`, `confirm`, `ask`, `abstain` — then by `reflex`.

## Tests, offline

```ts
import { replay } from "@evoke-build/evoke/testing"

const record = process.env.RECORD ? jev() : undefined                     // RECORD=1 asks Jev once and writes the file
const project = await load({ reflexes: { timer }, adapter: replay("answers.toml", { record }) })
```

Every run after the first is offline and deterministic.

**Next:** [Projects](projects.md).
