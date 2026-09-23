// volume writes one integer into AppleScript source and nothing else: a level that is no number is refused
// before anything runs, on any platform.
import { rejects } from "node:assert/strict"
import { test } from "node:test"

import volume from "../volume/volume.mts"

const context = { input: "", config: {}, signal: new AbortController().signal }

test("a level that is no number never reaches AppleScript", async () => {
  await rejects(volume({ level: Number.NaN }, context), { message: "level NaN is not a number" })
  await rejects(volume({ level: Number.POSITIVE_INFINITY }, context), { message: "level Infinity is not a number" })
})
