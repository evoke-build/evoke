// note appends one dated line per call to the file config names — `~` and a bare name landing under your home —
// and makes the folder when it is not there.
import { equal, match } from "node:assert/strict"
import { mkdtemp, readFile, rm } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"

import note from "../note/note.mts"

const context = (file: string) => ({ input: "", config: { file }, signal: new AbortController().signal })

test("a note is one dated line, appended; the file and its folder are made under your home", async () => {
  const home = await mkdtemp(join(tmpdir(), "note-"))
  const was = process.env.HOME
  process.env.HOME = home
  try {
    equal(await note({ text: "buy milk" }, context("notes/today.txt")), 'noted "buy milk" in ~/notes/today.txt')
    equal(await note({ text: "call the dentist" }, context("~/notes/today.txt")), 'noted "call the dentist" in ~/notes/today.txt')
    const lines = (await readFile(join(home, "notes/today.txt"), "utf8")).split("\n")
    equal(lines.length, 3)
    match(lines[0] ?? "", /^\d{4}-\d{2}-\d{2}  buy milk$/)
    match(lines[1] ?? "", /^\d{4}-\d{2}-\d{2}  call the dentist$/)
    equal(lines[2], "")
  } finally {
    process.env.HOME = was
    await rm(home, { recursive: true, force: true })
  }
})
