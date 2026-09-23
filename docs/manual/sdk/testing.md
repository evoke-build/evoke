# Testing

`replay()` is an adapter over a recording. The recording is the CLI's own `answers.toml`, read and written through
the core. With `record`, an utterance the file lacks is asked of a real adapter once, and written back. So one run
records a test suite. Every run after is offline, and always gives the same answers.

## `replay(file, { record? })`

```ts
import { load, reflex } from "@evoke-build/evoke"
import { jev } from "@evoke-build/evoke/jev"
import { replay } from "@evoke-build/evoke/testing"

const record = process.env.RECORD ? jev() : undefined
const project = await load({ reflexes: { timer }, adapter: replay(new URL("answers.toml", import.meta.url), { record }) })

const d = await project.decide("timer for 10 minutes")
assert.equal(d.outcome, "run")
```

- `RECORD=1 node --test` asks Jev for every utterance the file lacks. It writes the file whole, through a rename,
  so a reader never sees half a recording.
- Without `record`, a missing utterance is a `FaultError` naming it, with the call that records it.
- The recording carries the adapter's declaration: `id`, `limits`, `gate`. So the gate in tests is the gate in
  production. A `plan` line, when present, pins the installed set the answers were recorded against. A project
  over another set refuses it.

## The file

You can write it by hand. Answers are keyed by **utterance identity**: lower-cased, whitespace collapsed,
trailing punctuation dropped. Each question is keyed by its id: `route`, `fits.<reflex>`, `<reflex>.<argument>`.

```toml
id = "replay"

[gate]
route = 0.5
fits  = 0.3
read  = 0.6
write = 0.8

[answers."kill the lights in the den"]
route          = { lights = 0.91, timer = 0.02, volume = 0.01, none = 0.06 }
"fits.lights"  = { yes = 0.7 }
"fits.timer"   = { yes = 0.05 }
"lights.room"  = { den = 0.85, office = 0.05, unstated = 0.1 }
"lights.state" = { off = 0.88, on = 0.05, dim = 0.05, unstated = 0.02 }
```

A pick's candidates are keyed `<start>-<end>` by their character offsets in the input.

## The CLI on a recording

The same file drives the CLI. Set `adapter = "replay"` in `evoke.toml`, and the file's path in `EVOKE_ANSWERS`.
That is how `evoke`'s own transcripts run offline. It is also how a CI job can exercise a project without a key.

```bash
EVOKE_ANSWERS=answers.toml evoke try "kill the lights in the den"
```

## Testing bodies

A body is a function. Import it and call it with a context of your own. No adapter is involved:

```ts
import note from "../note/note.mts"
const context = { input: "", config: { file: "notes/today.txt" }, signal: new AbortController().signal }
assert.equal(await note({ text: "buy milk" }, context), 'noted "buy milk" in ~/notes/today.txt')
```

**Next:** [Errors](errors.md).
