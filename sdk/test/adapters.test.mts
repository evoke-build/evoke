// The adapters: Jev's settings, key and policy loop over a stand-in transport; replay over the spec's own
// recording, a miss as a fault, and recording a miss into a file the core reads back.

import { deepStrictEqual, equal, ok, rejects, throws } from "node:assert/strict"
import { mkdtempSync, readFileSync } from "node:fs"
import { constants, tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"

import type { Adapter } from "../src/adapter.ts"
import { answered } from "../src/adapter.ts"
import { DiagnosticError, FaultError } from "../src/errors.ts"
import { type Response, Transport, describe } from "../src/https.ts"
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
      (error: DiagnosticError) => error.message === "jev needs TYPESAFE_API_KEY, a key from typesafe.ai  →  export TYPESAFE_API_KEY=<value>",
    )
    throws(
      () => over({ key: "" }, async () => ({ status: 200, body: answers })),
      (error: DiagnosticError) => error.message === "jev needs TYPESAFE_API_KEY, a key from typesafe.ai  →  export TYPESAFE_API_KEY=<value>",
    )
  } finally {
    if (key !== undefined) process.env.TYPESAFE_API_KEY = key
  }
})

test("a question of evoke's own, weave.<name>, travels through jev and replay like any other", async () => {
  const own: Request = {
    state: { request: "check stock and look up order 4821" },
    questions: { "weave.split_0": { type: "yesno", ask: "At «and», two things?", yes: "Two things.", no: "One thing." } },
    proposed: [],
  }
  const bodies: string[] = []
  const jev = over({ key: "k" }, async (_url, _bearer, body) => {
    bodies.push(body)
    return { status: 200, body: JSON.stringify({ answers: { "weave.split_0": { type: "noul", noul: 0.73 } } }) }
  })
  deepStrictEqual((await answered(jev, own, 30_000, undefined, "decide()")).raw, { "weave.split_0": { yes: 0.73 } })
  equal((JSON.parse(bodies[0] ?? "{}") as { questions: Record<string, { type: string }> }).questions["weave.split_0"]?.type, "noul")
  const dir = mkdtempSync(join(tmpdir(), "evoke-weave-"))
  const file = join(dir, "answers.toml")
  const recorded = replay(file, { record: jev })
  deepStrictEqual((await answered(recorded, own, 30_000, undefined, "decide()")).raw, { "weave.split_0": { yes: 0.73 } })
  ok(readFileSync(file, "utf8").includes('"weave.split_0" = { yes = 0.73 }'))
  deepStrictEqual((await answered(replay(file), own, 30_000, undefined, "decide()")).raw, { "weave.split_0": { yes: 0.73 } })
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
    answered(over({ key: "k" }, async () => ({ status: 401, body: "" })), request, 30_000, undefined, "decide()"),
    (error: FaultError) => error.message === "the key in TYPESAFE_API_KEY was refused  →  export TYPESAFE_API_KEY=<value>" && error.fault.type === "refused",
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

test("a transport failure is said in plain words, the host named, the system's own cause", () => {
  const refused = Object.assign(new Error("connect ECONNREFUSED 127.0.0.1:443"), {
    code: "ECONNREFUSED",
    errno: -constants.errno.ECONNREFUSED,
    syscall: "connect",
  })
  equal(describe(refused, "api.typesafe.ai", false), "could not connect to api.typesafe.ai: connection refused")
  const several = Object.assign(new AggregateError([refused], ""), { code: "ECONNREFUSED" })
  equal(describe(several, "api.typesafe.ai", false), "could not connect to api.typesafe.ai: connection refused")
  const unknown = Object.assign(new Error("getaddrinfo ENOTFOUND proxy.example"), {
    code: "ENOTFOUND",
    errno: -3008,
    syscall: "getaddrinfo",
    hostname: "proxy.example",
  })
  equal(describe(unknown, "api.typesafe.ai", false), "could not resolve proxy.example")
  equal(describe(new Error("no answer within 1.5 s"), "api.typesafe.ai", true), "api.typesafe.ai: no answer within 1.5 s")
  equal(describe(Object.assign(new Error("boom"), { code: "EPIPE" }), "api.typesafe.ai", true), "api.typesafe.ai: EPIPE")
})
