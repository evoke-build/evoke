// The files box: the spec's home project read whole, and a store entry that stands only while it hashes.

import { deepStrictEqual, equal, throws } from "node:assert/strict"
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

import { call } from "../src/core.ts"
import { FailureError } from "../src/errors.ts"
import { entry, snapshot } from "../src/files.ts"

test("the owned files under a root, absent ones absent", () => {
  const home = fileURLToPath(new URL("../../spec/transcripts/home/.config/evoke/", import.meta.url))
  const found = snapshot(home)
  equal(found.project?.startsWith('adapter = "replay"'), true)
  equal(found.lock, undefined)
  deepStrictEqual(Object.keys(found.overlays), [])
  deepStrictEqual(Object.keys(found.vocab), ["rooms"])
  deepStrictEqual(snapshot(join(home, "nowhere")), { overlays: {}, vocab: {} })
})

test("a store entry stands while its files hash to the lock", () => {
  const cache = mkdtempSync(join(tmpdir(), "evoke-cache-"))
  process.env.XDG_CACHE_HOME = cache
  const files: [string, string][] = [["reflex.toml", "reflex = 1\n"], ["lights.mts", "export default () => 'x'\n"]]
  const hashed = files.map(([path, text]) => [path, `h1:${sha(text)}`] as [string, string])
  const h1 = call("digest", { hashed })
  const dir = join(cache, "evoke", "store", h1.slice(3))
  mkdirSync(dir, { recursive: true })
  for (const [path, text] of files) writeFileSync(join(dir, path), text)
  equal(entry(h1), dir)
  writeFileSync(join(dir, "lights.mts"), "changed\n")
  equal(entry(h1), undefined)
  equal(entry("h1:" + "0".repeat(64)), undefined)
})

function sha(text: string): string {
  return createHash("sha256").update(text).digest("hex")
}
import { createHash } from "node:crypto"

test("a file that will not read is a failure naming it; a stem that is no name is not an owned file", () => {
  const root = mkdtempSync(join(tmpdir(), "evoke-root-"))
  mkdirSync(join(root, "vocab"))
  writeFileSync(join(root, "vocab", "rooms.toml"), 'den = "The den."\n')
  writeFileSync(join(root, "vocab", "Bad Name.toml"), "not = 'read'\n")
  writeFileSync(join(root, "vocab", "notes.txt"), "")
  deepStrictEqual(Object.keys(snapshot(root).vocab), ["rooms"])
  mkdirSync(join(root, "evoke.toml"))
  throws(
    () => snapshot(root),
    (error: FailureError) => error instanceof FailureError && error.what === `reading ${join(root, "evoke.toml")}` && error.command === "load()",
  )
})
