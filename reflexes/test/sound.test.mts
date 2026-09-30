// sound writes one integer into AppleScript source and nothing else: an amount that is no number is refused
// before anything runs, on any platform.
import { rejects } from "node:assert/strict"
import { test } from "node:test"

import sound from "../sound/sound.mts"

const context = { input: "", config: {}, signal: new AbortController().signal }

test("an amount that is no number never reaches AppleScript", async () => {
  await rejects(sound({ state: "up", by: Number.NaN }, context), { message: "by NaN is not a number" })
  await rejects(sound({ state: "down", by: Number.POSITIVE_INFINITY }, context), { message: "by Infinity is not a number" })
})
