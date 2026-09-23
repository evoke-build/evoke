// Every vector through the built module: spec/vectors/<family>/<case>.json — the input in, and the reply must equal
// expect, wrapped in ok when the function cannot fail. Numbers compare as numbers; key order is free.

import { deepStrictEqual } from "node:assert/strict"
import { readFileSync, readdirSync } from "node:fs"
import { describe, it } from "node:test"

import { reply } from "../src/core.ts"

const spec = new URL("../../spec/", import.meta.url)

/** The families whose function returns a Result, so their expect is already { ok } | { err }. */
const RESULTS = new Set([
  "manifest",
  "overlay",
  "vocabulary",
  "project",
  "name",
  "compile",
  "request",
  "read",
  "teach",
  "argv",
  "call",
  "by_name",
  "set_config",
  "reference",
  "lock",
  "weave.plan",
  "systemone.settings",
  "systemone.answers",
  "replay.recording",
  "replay.answer",
])

/** A JSON file under spec/, its { "$ref": … } objects replaced by what they name. */
function load(path: URL): unknown {
  return resolve(JSON.parse(readFileSync(path, "utf8")))
}

function resolve(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(resolve)
  if (value === null || typeof value !== "object") return value
  const entries = Object.entries(value)
  if (entries.length === 1 && entries[0]?.[0] === "$ref" && typeof entries[0][1] === "string") {
    return load(new URL(entries[0][1], spec))
  }
  return Object.fromEntries(entries.map(([key, inner]) => [key, resolve(inner)]))
}

for (const family of readdirSync(new URL("vectors/", spec)).sort()) {
  describe(family, () => {
    for (const file of readdirSync(new URL(`vectors/${family}/`, spec)).sort()) {
      it(file, () => {
        const { input, expect } = load(new URL(`vectors/${family}/${file}`, spec)) as { input: object; expect: unknown }
        deepStrictEqual(reply(family, input), RESULTS.has(family) ? expect : { ok: expect })
      })
    }
  })
}
