// The loader's own contract: an op the table lacks, or an input that does not read, comes back as a bug naming the
// op — never as a throw, never as an ok.

import { deepStrictEqual } from "node:assert/strict"
import { test } from "node:test"

import { reply } from "../src/core.ts"

test("an unknown op is a bug naming it", () => {
  deepStrictEqual(reply("decide", {}), { bug: "decide: no such op" })
})

test("an input that does not read is a bug naming the argument", () => {
  deepStrictEqual(reply("identity", {}), { bug: "identity: text is missing or not a string" })
})

test("two calls share one instance and free what they used", () => {
  const before = reply("identity", { text: "Kill the lights!" })
  const after = reply("identity", { text: "Kill the lights!" })
  deepStrictEqual(before, { ok: "kill the lights" })
  deepStrictEqual(after, before)
})
