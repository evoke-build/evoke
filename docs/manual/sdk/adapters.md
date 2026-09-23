# Adapters

An adapter is the classifier behind a decision. It is an object that answers typed questions with probabilities.
The core names no engine. A project names an adapter, and only your machine resolves the name.

## `jev()`: the first adapter

```ts
import { jev } from "@evoke-build/evoke/jev"

const project = await load({ reflexes, adapter: jev() })
const project = await load({ reflexes, adapter: jev({ key, gate: { write: 0.85 } }) })
```

| Option | Meaning                                                                                       |
| :----- | :-------------------------------------------------------------------------------------------- |
| `key`  | The API key. Absent: `TYPESAFE_API_KEY` from the environment, read when `jev()` is called      |
| `gate` | Floors over the defaults of `route` 0.5, `fits` 0.3, `read` 0.6 and `write` 0.8. The same as `[adapters.jev] gate` in `evoke.toml` |

When `load` resolves `jev` by name from `evoke.toml`, it is built under the file's `[adapters.jev]` table. An
adapter passed in replaces both. The transport keeps one connection alive for the process, through the proxy
`HTTPS_PROXY` names when one is set; a proxy needs Node 24.5 or newer, and `jev()` says so on an older one. It
retries once after a connect error or a server error, and never after a client error. No key is a
`DiagnosticError` ending in `export TYPESAFE_API_KEY=<value>`.

## The contract

```ts
interface Adapter {
  id: string                                           // opaque; changes whenever answers could; compared, never parsed
  limits?: { options?: number; tokens?: number }       // per-call ceilings; a plan over them is refused at load
  gate?: { route: number; fits?: number; read: number; write: number }   // each number means P(correct)
  plan?: string                                        // a recording's plan digest; another plan refuses it
  answer(state: { request: string }, questions: Record<string, Question>, signal: AbortSignal): Promise<Raw>
}

type Question =
  | { type: "choice"; ask: string; options: Record<string, Text>; otherwise?: string }   // the key meaning "none of these"
  | { type: "yesno";  ask: string; yes: Text; no: Text }                                 // answers { yes: p }
type Text = string | { what: string; not_for?: string[]; examples?: string[] }
type Raw  = Record<string, Record<string, number>>    // per question, a number per key
```

- **State is only `{ request }`.** An adapter sees the input and the questions, nothing else.
- **A choice answer is a distribution** over the keys offered, summing to 1. An omitted key reads as 0. A yes/no
  answers `{ yes: p }`. The core validates every answer and fails closed. A missing question is a fault.
- **`otherwise`** marks the sentinel, `none` or `unstated`, in the structure. So an engine may abstain its own
  way.
- **`gate`** is the adapter's own calibration. The core never rescales. Without one, every decision confirms.
  `gate.fits` is the runner-up's floor. A second reflex clearing it turns a run into a confirm.
- **`answer` is stateless** and may be called with any subset of a plan's questions. `signal` aborts it at the
  deadline. An adapter that ignores its signal is raced against it anyway.
- **`id`** covers everything that could shift answers: model, version, prompt rendering, calibration. It is
  pinned in the lock with the project's adapter name.

## Writing one

```ts
const mine: Adapter = {
  id: "my-classifier-2",
  gate: { route: 0.5, read: 0.6, write: 0.8 },
  async answer(state, questions, signal) {
    const raw: Raw = {}
    for (const [id, q] of Object.entries(questions)) raw[id] = await classify(state.request, q, signal)
    return raw
  },
}
const project = await load({ root, adapter: mine })
```

Any engine that can answer a closed choice with probabilities fits. That could be a hosted model that spells them
out, a local model with log-probabilities, or a zero-shot classifier. An embedding router cannot fill arguments
on its own. A throw that is not `evoke`'s own becomes a transport `FaultError`. The caller's abort passes through
as its own reason.

**Next:** [Testing](testing.md).
