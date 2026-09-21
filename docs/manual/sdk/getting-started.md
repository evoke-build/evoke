# Getting started with the SDK

`@evoke-build/evoke` loads a project of reflexes, decides what an input asks for, and runs it. It does this inside
your application, with the same rules as the CLI. The rules run in `core.wasm`, the CLI's own core. The SDK adds
files, the network and a process.

Six things to learn, in order: `reflex()` · `load()` · `handle()` · a `Decision` · `run()` · `replay()`.

## Install

```bash
npm install @evoke-build/evoke
```

You need Node 24 or newer, and ES modules. The package ships three entries, so the main one imports no engine:

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

- `reflex()` takes the manifest as an object, and the body. The object is the file's shape, without `run` and
  `config`. The body's argument types are inferred from the manifest literal.
- `load()` builds a project. Here it builds one from code alone, with the adapter passed in.
- `handle()` runs the whole loop with your two handlers: decide, ask until nothing is missing, confirm, run. It
  returns `ran`, `abstained`, `declined` or `unanswered`.

## With files

Next to an app, `evoke add evoke-build/reflexes` writes `evoke.toml`, `evoke.lock` and `evoke.d.ts`, exactly as
it does in your home project. Overlays and vocabularies under the same root tune the wording. That includes a
reflex handed as code.

```ts
import type { Reflexes } from "./evoke.d.ts"

const project = await load<Reflexes>({ root: import.meta.dirname })       // the adapter evoke.toml names
const d = await project.decide("kill the lights in the den")
if (d.outcome === "run" && d.reflex === "lights") console.log(d.values.room, d.values.state)   // typed by evoke.d.ts
```

A `Decision` is a union. Narrow it by `outcome`, one of `run`, `confirm`, `ask` and `abstain`, then by `reflex`.

## Tests, offline

```ts
import { replay } from "@evoke-build/evoke/testing"

const record = process.env.RECORD ? jev() : undefined                     // RECORD=1 asks Jev once and writes the file
const project = await load({ reflexes: { timer }, adapter: replay("answers.toml", { record }) })
```

Every run after the first is offline, and always gives the same answers.

**Next:** [Projects in code](projects.md).
