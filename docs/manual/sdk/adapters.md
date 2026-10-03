# Adapters

An adapter is the classifier behind a decision. It is an object that answers typed questions with probabilities.
The core names no engine. A project names an adapter, and only your machine resolves the name. Two built-in
adapters reach Jev: `jev()`, through TypeSafe AI's own API, and `openjev()`, through OpenJEV, an independent
service. They send the same questions and ship the same bars. Two more reach Clef, Cloudflare's model: `clef()`
and `clefFlash()`. They send the same questions and ship no bars.

## `jev()`: the first adapter

```ts
import { jev } from "@evoke-build/evoke/jev"

const project = await load({ reflexes, adapter: jev() })
const project = await load({ reflexes, adapter: jev({ key, gate: { write: 0.95 } }) })
```

| Option | Meaning                                                                                       |
| :----- | :-------------------------------------------------------------------------------------------- |
| `key`  | The API key. Absent: `TYPESAFE_API_KEY` from the environment, read when `jev()` is called      |
| `gate` | Floors over the defaults of `route` 0.5, `fits` 0.3, `read` 0.8, `write` 0.9 and `whole` 0.3. The same as `[adapters.jev] gate` in `evoke.toml` |

When `load` resolves `jev` by name from `evoke.toml`, it is built under the file's `[adapters.jev]` table. An
adapter passed in replaces both. The transport keeps one connection alive for the process, through the proxy
`HTTPS_PROXY` names when one is set; a proxy needs Node 24.5 or newer, and `jev()` says so on an older one. It
retries once after a connect error or a server error, and never after a client error, except a 429 that says
how long to wait: that wait is waited out, and the request sent again, within the deadline. No key is a
`DiagnosticError` ending in `export TYPESAFE_API_KEY=<value>`.

## `openjev()`: the same model, through OpenJEV

```ts
import { openjev } from "@evoke-build/evoke/openjev"

const project = await load({ reflexes, adapter: openjev() })
const project = await load({ reflexes, adapter: openjev({ key, gate: { write: 0.95 } }) })
```

| Option | Meaning                                                                                       |
| :----- | :-------------------------------------------------------------------------------------------- |
| `key`  | The API key. Absent: `OPENJEV_API_KEY` from the environment, read when `openjev()` is called   |
| `gate` | Floors over the same defaults as `jev()`. The same as `[adapters.openjev] gate` in `evoke.toml` |

[OpenJEV](https://openjev.sh) is an independent service, not TypeSafe AI's. It forwards each request to Jev and
returns Jev's answers. `evoke` is affiliated with neither, and vouches for neither. A key from OpenJEV is on
OpenJEV's terms, and whether it may offer Jev is a matter between OpenJEV and TypeSafe AI. As of this release it
publishes no terms and no privacy policy, so weigh that before you choose it for an application. The transport
is `jev()`'s, with 3 seconds once connected instead of 1.5, since the service forwards the request onward. It
names the model by version, `jev-1.13.0`, as `jev()` does, so its `id` is Jev's own. No key is a
`DiagnosticError` ending in `export OPENJEV_API_KEY=<value>`.

## `clef()` and `clefFlash()`: Clef, on Cloudflare's Workers AI

```ts
import { clef, clefFlash } from "@evoke-build/evoke/clef"

const project = await load({ reflexes, adapter: clef() })
const project = await load({ reflexes, adapter: clefFlash({ key, account, gate }) })
```

| Option    | Meaning                                                                                              |
| :-------- | :--------------------------------------------------------------------------------------------------- |
| `key`     | The API token. Absent: `CLOUDFLARE_API_TOKEN` from the environment, read when the adapter is called    |
| `account` | The account's id. Absent: `CLOUDFLARE_ACCOUNT_ID` from the environment, read at the same time          |
| `gate`    | The floors `route`, `read` and `write`, with `fits` and `whole` if you want them. The same as `[adapters.clef] gate` or `[adapters.clef_flash] gate` in `evoke.toml` |

[Clef](https://developers.cloudflare.com/workers-ai/models/clef/) and Clef-flash are Cloudflare's models, with
weights published under Apache-2.0. `clef()` reaches Clef and `clefFlash()` the smaller Clef-flash. Both run on
Cloudflare's Workers AI, in the account the token belongs to. Cloudflare's terms cover the token and each request.

These adapters ship no floors, so every decision confirms. To let calls run, pass a `gate` with `route`, `read`
and `write`, and measure the numbers you choose on your own records: [Calibrating](../use/calibrating.md). A
`gate` that lacks one of the three is a `DiagnosticError`.

The account's id goes into the address. So it must be 32 hex digits in lowercase, and anything else is refused
before a request is made. The transport is `jev()`'s, with 6 seconds once connected for `clef()` and 3 for
`clefFlash()`. Workers AI takes 64 questions a request, so a longer request is sent in parts and answered as one.
It names no version of a model, so each `id` carries a date: `clef-2026-10-03` and `clef-flash-2026-10-03`. No
token or no account is a `DiagnosticError` ending in the `export` line for its variable.

## The contract

```ts
interface Adapter {
  id: string                                           // opaque; changes whenever answers could; compared, never parsed
  limits?: { options?: number; tokens?: number }       // options: a plan over it is refused at load; tokens: declared, not enforced
  gate?: { route: number; fits?: number; read: number; write: number; whole?: number }   // each number means P(correct)
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
  `gate.whole` is the floor of a call read back against the input; without it no call is read back.
  `gate.fits` is read when a reflex is added: a new reflex that fits another's example at or above it is named.
- **`answer` is stateless** and may be called with any subset of a plan's questions. `signal` aborts it at the
  deadline. An adapter that ignores its signal is raced against it anyway.
- **`id`** covers everything that could shift answers: model, version, prompt rendering, calibration. It is
  pinned in the lock with the project's adapter name.

## Writing one

```ts
const mine: Adapter = {
  id: "my-classifier-2",
  gate: { route: 0.5, read: 0.8, write: 0.9, whole: 0.3 },
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
