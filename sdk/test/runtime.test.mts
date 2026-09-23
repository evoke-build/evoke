// The runtime box: a function body in-process, a file body through the loader, a program by argv; a body that
// does not settle is a failure that names the wait; config comes from the environment and a missing variable is
// the failure it names.

import { deepStrictEqual, ok, rejects } from "node:assert/strict"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

import { FailureError } from "../src/errors.ts"
import { child, inline, program } from "../src/runtime.ts"
import type { Envelope } from "../src/types.ts"

process.env.EVOKE_TEST_TOKEN = "secret"

const envelope = (over: Partial<Envelope> = {}): Envelope => ({
  reflex: "timer",
  args: { duration: 600 },
  input: "set a timer for 10 minutes",
  config: {},
  deadline: 2000,
  ...over,
})

test("a function body receives plain values and the context", async () => {
  const seen: unknown[] = []
  const result = await inline(
    "running timer",
    (args, context) => {
      seen.push(args, context.input, context.config, context.signal.aborted)
      return { text: "ok", data: { n: 1 } }
    },
    envelope({ config: { token: { type: "env", var: "EVOKE_TEST_TOKEN" } } }),
    undefined,
  )
  deepStrictEqual(result, { text: "ok", data: { n: 1 } })
  deepStrictEqual(seen, [{ duration: 600 }, "set a timer for 10 minutes", { token: "secret" }, false])
})

test("a function body that returns text alone, or the wrong shape", async () => {
  deepStrictEqual(await inline("running timer", () => "done", envelope(), undefined), { text: "done" })
  await rejects(
    inline("running timer", () => [1] as never, envelope(), undefined),
    (error: FailureError) => error.message === "running timer: the body returned an array, not text or { text, data }  →  run(d)",
  )
})

test("a function body that never settles is abandoned after the deadline and a grace", async () => {
  await rejects(
    inline("running timer", (_args, { signal }) => new Promise<string>(resolve => signal.addEventListener("abort", () => setTimeout(() => resolve("late"), 5000))), envelope({ deadline: 50 }), undefined),
    (error: FailureError) => error instanceof FailureError && error.message === "running timer: did not finish within 50 ms  →  run(d)",
  )
})

test("an unset variable is the failure it names", async () => {
  await rejects(
    inline("running lights", () => "x", envelope({ config: { token: { type: "env", var: "EVOKE_TEST_UNSET" } } }), undefined),
    (error: FailureError) => error.message === "running lights: EVOKE_TEST_UNSET is not set  →  export EVOKE_TEST_UNSET=<value>",
  )
})

test("a file body runs in a child through the loader", async () => {
  const dir = fileURLToPath(new URL("../../spec/transcripts/home/.config/evoke/timer/", import.meta.url))
  const result = await child("running timer", dir.slice(0, -1), { ...envelope(), run: "timer.mts" }, undefined)
  deepStrictEqual(result, { text: "10 minute timer started" })
})

test("a program runs by argv with the input in its environment", async () => {
  const result = await program("running echo", ["/bin/sh", "-c", "echo \"$EVOKE_INPUT|$EVOKE_CONFIG_BRIDGE\""], envelope({ config: { bridge: { type: "plain", value: "10.0.0.2" } } }), undefined)
  deepStrictEqual(result, { text: "set a timer for 10 minutes|10.0.0.2" })
  await rejects(
    program("running false", ["/bin/sh", "-c", "exit 3"], envelope(), undefined),
    (error: FailureError) => error.message === "running false: exited 3  →  run(d)",
  )
})

test("a signal already aborted is the caller's own reason, before anything runs", async () => {
  const reason = new Error("gone")
  const signal = AbortSignal.abort(reason)
  await rejects(inline("running timer", () => "x", envelope(), signal), (error: Error) => error === reason)
  await rejects(program("running echo", ["/bin/echo", "x"], envelope(), signal), (error: Error) => error === reason)
})

test("a program that cannot start says why; a grandchild holding stdout does not hold the run", async () => {
  await rejects(
    program("running nothing", ["/nonexistent/program"], envelope(), undefined),
    (error: FailureError) => error.message.startsWith("running nothing: spawn /nonexistent/program ENOENT"),
  )
  const started = performance.now()
  const result = await program("running sh", ["/bin/sh", "-c", "(sleep 3 &); echo hi"], envelope({ deadline: 5000 }), undefined)
  deepStrictEqual(result, { text: "hi" })
  ok(performance.now() - started < 2500, "settled at exit plus a grace, not at the grandchild's end")
})

test("a body's own error is the failure's cause", async () => {
  await rejects(
    inline("running timer", () => { throw new RangeError("boom") }, envelope(), undefined),
    (error: FailureError) => error.message === "running timer: boom  →  run(d)" && error.cause instanceof RangeError,
  )
})
