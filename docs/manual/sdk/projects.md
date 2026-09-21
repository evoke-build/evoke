# Projects

`load()` reads a project once — the owned files under a root, the reflexes handed as code, and who answers — and
compiles it. It never fetches and never writes.

## `load(options)`

```ts
const project = await load<Reflexes>({ root, reflexes, adapter })
```

| Option     | Meaning                                                                                                     |
| :--------- | :---------------------------------------------------------------------------------------------------------- |
| `root`     | The project directory: `evoke.toml`, `evoke.lock`, `overlays/`, `vocab/`, local reflexes. Absent: no files at all. A root without `evoke.toml` is the default project, `adapter = "jev"`, nothing installed |
| `reflexes` | Reflexes handed as code, by local name. A name `evoke.toml` also lists is an error. Overlays under `root` apply to them too |
| `adapter`  | Who answers. Absent: the adapter `evoke.toml` names, resolved from the SDK's own subpath and built under `[adapters.<name>]`. Required when there is no root |

Remote reflexes come from the store by the lock's content hash, re-hashed at load; a missing entry fails with
`evoke sync`. The SDK never checks trust — the root is yours to choose — and keeps no cache.

## Reflexes as code

```ts
const timer = reflex({
  description: "Start a countdown timer.",
  effect: "write",                                                   // absent means destructive
  confirm: "Start a {duration} timer?",                              // required
  args: { duration: { ask: "How long?", pick: "duration" } },
  examples: { "timer for ten minutes": { duration: "ten minutes" } },
}, async ({ duration }, { input, signal }) => startTimer(duration))
```

The manifest is the file's shape without `run` and `config`: a body closes over what it needs. Its types are
inferred from the literal — an option key union, a word or a quoted pick as `string`, a number or a duration as
`number`, a flag as `true`, an optional argument optional — so the body's `args` are typed with nothing generated.
A manifest that does not read throws at `load`, since it is your own code, its problems ending in `reflex(<name>)`.
The body runs in-process: it is your application's code, unscrubbed, and nothing can end it from outside, so it
honours the deadline's `signal`.

## Generated types

`evoke add`, `update` and `remove` write `evoke.d.ts` beside `evoke.toml`: one `Reflexes` interface, a member per
installed reflex, its arguments as a decision carries them. Hand it to `load`:

```ts
import type { Reflexes } from "./evoke.d.ts"
const project = await load<Reflexes>({ root: import.meta.dirname })
```

Now `d.reflex` narrows `d.args` and `d.values`. A project with reflexes both installed and handed as code writes
`load<Reflexes & ReflexesOf<typeof own>>({ root, reflexes: own })`, since TypeScript infers all type arguments or
none.

## What a project knows

```ts
project.reflexes   // per name: { active: true, effect, runs: "inline" | "file" | "argv" } | { active: false, problems }
project.plan       // the digest of the compiled set, "h1:…"; every decision carries it
```

An inactive reflex — an empty vocabulary, a setting unset, an overlay that does not parse — is left out of every
decision, and its `problems` each end in the command that fixes it.

## `with({ vocab })` — a tenant's words

A vocabulary binds a project, not a call. `with` compiles the same set over vocabularies of the call's own, in
the file's form, synchronously, in milliseconds:

```ts
const tenant = project.with({ vocab: { rooms: await db.rooms(user) } })
const d = await tenant.decide(input)
if (d.outcome === "run") await tenant.run(d)
```

`decide`, `fill` and `run` on that project cannot disagree about the plan the decision was made under, so a
multi-tenant server stays correct: a decision made on one tenant's project is refused by another's.

## Running file bodies from the SDK

A reflex installed from git runs in a child process through the SDK's own loader, under the SDK's Node; an argv
reflex is spawned directly. Nothing is warmed at `decide`. The body's deadline is 30 seconds less what the adapter
call took.

**Next:** [Decisions](decisions.md).
