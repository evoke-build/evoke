// The adapters: Jev's settings, key and policy loop over a stand-in transport; replay over the spec's own
// recording, a miss as a fault, and recording a miss into a file the core reads back.

import { deepStrictEqual, equal, ok, rejects, throws } from "node:assert/strict"
import { mkdtempSync, readFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"

import type { Adapter } from "../src/adapter.ts"
import { answered } from "../src/adapter.ts"
import { DiagnosticError, FaultError } from "../src/errors.ts"
import { type Response, Transport } from "../src/https.ts"
import { type Post, over } from "../src/jev.ts"
import { replay } from "../src/testing.ts"
import type { Request } from "../src/types.ts"

const request = JSON.parse(readFileSync(new URL("../../spec/fixtures/request-kill-the-lights.json", import.meta.url), "utf8")) as Request
const answers = JSON.stringify({
  answers: {
    route: { type: "choice", probabilities: { lights: 0.9, timer: 0.02, volume: 0.02, none: 0.06 } },
    "fits.lights": { type: "noul", noul: 0.7 },
  },
})

test("jev declares its id, limits and the gate with overrides, and needs its key", () => {
  const adapter = over({ key: "k", gate: { write: 0.85 } }, async () => ({ status: 200, body: answers }))
  equal(adapter.id, "jev-1.13.0")
  deepStrictEqual(adapter.limits, { options: 255 })
  deepStrictEqual(adapter.gate, { route: 0.5, fits: 0.3, read: 0.6, write: 0.85 })
  throws(
    () => over({ key: "k", gate: { read: 0.9 } }, async () => ({ status: 200, body: answers })),
    (error: DiagnosticError) => error.message === "adapters.jev.gate: read 0.9 is above write 0.8  →  jev({ gate })",
  )
  const key = process.env.TYPESAFE_API_KEY
  delete process.env.TYPESAFE_API_KEY
  try {
    throws(
      () => over({}, async () => ({ status: 200, body: answers })),
      (error: DiagnosticError) => error.message === "jev needs TYPESAFE_API_KEY  →  export TYPESAFE_API_KEY=<value>",
    )
  } finally {
    if (key !== undefined) process.env.TYPESAFE_API_KEY = key
  }
})

test("the policy loop: once more after a connect error or a 5xx, never after a 4xx", async () => {
  const calls: string[] = []
  const sequence = (...replies: (Response | Transport)[]): Post => async (_url, bearer, body, _signal, timeout) => {
    calls.push(`${bearer}:${timeout}`)
    ok(body.includes('"model":"jev-1.13.0"'))
    const next = replies.shift()
    if (next === undefined) throw new Error("asked once too often")
    if (next instanceof Transport) throw next
    return next
  }
  const answer = { status: 200, body: answers }
  const { raw, trace } = await answered(over({ key: "k" }, sequence(new Transport("connect refused", false), answer)), request, 30_000, undefined, "decide()")
  deepStrictEqual(calls, ["k:1500", "k:1500"])
  deepStrictEqual(raw["fits.lights"], { yes: 0.7 })
  equal(trace.adapter, "jev-1.13.0")
  equal(trace.questions, Object.keys(request.questions).length)
  calls.length = 0
  await answered(over({ key: "k" }, sequence({ status: 503, body: "" }, answer)), request, 30_000, undefined, "decide()")
  equal(calls.length, 2)
  await rejects(
    answered(over({ key: "k" }, sequence({ status: 503, body: "" }, { status: 503, body: "" })), request, 30_000, undefined, "decide()"),
    (error: FaultError) => error.message === "the adapter answered 503  →  decide()",
  )
  await rejects(
    answered(over({ key: "k" }, async () => ({ status: 429, body: "" })), request, 30_000, undefined, "decide()"),
    (error: FaultError) => error.message === "the adapter answered 429  →  decide()",
  )
  await rejects(
    answered(over({ key: "k" }, async () => { throw new Transport("reset", true) }), request, 30_000, undefined, "decide()"),
    (error: FaultError) => error.message === "reset  →  decide()",
  )
})

test("an adapter that ignores the deadline is a transport fault; the caller's abort is its own reason", async () => {
  const slow: Adapter = { id: "slow", answer: () => new Promise(() => {}) }
  await rejects(
    answered(slow, request, 20, undefined, "decide()"),
    (error: FaultError) => error.message === "did not answer within 20 ms  →  decide()",
  )
  const controller = new AbortController()
  const pending = answered(slow, request, 30_000, controller.signal, "decide()")
  controller.abort(new Error("gone"))
  await rejects(pending, (error: Error) => error.message === "gone")
})

test("replay answers from the spec's recording and faults on a miss", async () => {
  const adapter = replay(new URL("../../spec/transcripts/use/answers.toml", import.meta.url))
  equal(adapter.id, "replay")
  deepStrictEqual(adapter.gate, { route: 0.5, fits: 0.3, read: 0.6, write: 0.8 })
  const raw = await adapter.answer(request.state, request.questions, AbortSignal.timeout(1000))
  deepStrictEqual(Object.keys(raw), ["route", "fits.lights", "lights.room", "lights.state", "fits.timer"])
  await rejects(
    adapter.answer({ request: "what time is it" }, request.questions, AbortSignal.timeout(1000)),
    (error: FaultError) => error.message.startsWith('"what time is it" is not recorded  →  replay("') && error.message.endsWith('answers.toml", { record: jev() })'),
  )
})

test("recording a miss writes the file, which reads back", async () => {
  const dir = mkdtempSync(join(tmpdir(), "evoke-"))
  const file = join(dir, "answers.toml")
  let asked = 0
  const source: Adapter = {
    id: "fake-1",
    gate: { route: 0.5, read: 0.6, write: 0.8 },
    answer: async (_state, questions) => {
      asked += 1
      return Object.fromEntries(Object.keys(questions).map(id => [id, id === "route" ? { lights: 1 } : { yes: 0.5 }]))
    },
  }
  const recorder = replay(file, { record: source })
  equal(recorder.id, "fake-1")
  const first = await recorder.answer(request.state, request.questions, AbortSignal.timeout(1000))
  deepStrictEqual(first.route, { lights: 1 })
  deepStrictEqual(await recorder.answer(request.state, request.questions, AbortSignal.timeout(1000)), first)
  equal(asked, 1)
  const text = readFileSync(file, "utf8")
  ok(text.includes('id = "fake-1"'))
  ok(text.includes('[answers."kill the lights"]'))
  const again = replay(file)
  equal(again.id, "fake-1")
  deepStrictEqual(await again.answer(request.state, request.questions, AbortSignal.timeout(1000)), first)
  throws(
    () => replay(join(dir, "missing.toml")),
    (error: DiagnosticError) => error.message.startsWith(`${join(dir, "missing.toml")}: there is no such file  →  replay(`),
  )
})

test("a fault from an adapter that could not name the call is rendered with it", async () => {
  await rejects(
    answered(over({ key: "k" }, async () => ({ status: 429, body: "" })), request, 30_000, undefined, 'decide("kill the lights")'),
    (error: FaultError) => error.message === 'the adapter answered 429  →  decide("kill the lights")',
  )
  const adapter = replay(new URL("../../spec/transcripts/use/answers.toml", import.meta.url))
  await rejects(
    answered(adapter, { ...request, state: { request: "what time is it" } }, 30_000, undefined, "decide()"),
    (error: FaultError) => error.message.startsWith('"what time is it" is not recorded  →  replay("') && error.message.endsWith('answers.toml", { record: jev() })'),
  )
})

test("a recording that answered fewer questions than are asked is repaired when recording", async () => {
  const dir = mkdtempSync(join(tmpdir(), "evoke-"))
  const file = join(dir, "answers.toml")
  let asked = 0
  const source: Adapter = {
    id: "fake-1",
    answer: async (_state, questions) => {
      asked += 1
      return Object.fromEntries(Object.keys(questions).map(id => [id, { yes: 1 }]))
    },
  }
  const recorder = replay(file, { record: source })
  const fewer = { route: request.questions.route! }
  deepStrictEqual(Object.keys(await recorder.answer(request.state, fewer, AbortSignal.timeout(1000))), ["route"])
  const more = await recorder.answer(request.state, request.questions, AbortSignal.timeout(1000))
  deepStrictEqual(Object.keys(more), Object.keys(request.questions))
  equal(asked, 2)
  deepStrictEqual(Object.keys(await replay(file).answer(request.state, request.questions, AbortSignal.timeout(1000))), Object.keys(request.questions))
})
