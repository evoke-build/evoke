// note appends one dated line per call to the file config names, by the path evoke hands it, a file that exists,
// as its declaration requires.
import { equal, match, rejects } from "node:assert/strict"
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"

import note from "../note/note.mts"

const context = (file: string) => ({ input: "", config: { file }, signal: new AbortController().signal })

test("a note is one dated line, appended to the file under your home; a file that is not there is not made", async () => {
  const home = await mkdtemp(join(tmpdir(), "note-"))
  const was = process.env.HOME
  process.env.HOME = home
  try {
    const file = join(home, "notes/today.txt")
    await rejects(note({ text: "buy milk" }, context(file)), { code: "ENOENT" })
    await mkdir(join(home, "notes"))
    await writeFile(file, "")
    equal(await note({ text: "buy milk" }, context(file)), 'noted "buy milk" in ~/notes/today.txt')
    equal(await note({ text: "call the dentist" }, context(file)), 'noted "call the dentist" in ~/notes/today.txt')
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

test("a note is one line whatever was typed, and never joins a last line that lacks its line feed", async () => {
  const home = await mkdtemp(join(tmpdir(), "note-"))
  const was = process.env.HOME
  process.env.HOME = home
  try {
    await mkdir(join(home, "notes"), { recursive: true })
    await writeFile(join(home, "notes/today.txt"), "a line someone left open")
    equal(await note({ text: "buy\n  milk" }, context(join(home, "notes/today.txt"))), 'noted "buy milk" in ~/notes/today.txt')
    const lines = (await readFile(join(home, "notes/today.txt"), "utf8")).split("\n")
    equal(lines[0], "a line someone left open")
    match(lines[1] ?? "", /^\d{4}-\d{2}-\d{2}  buy milk$/)
    equal(lines[2], "")
  } finally {
    process.env.HOME = was
    await rm(home, { recursive: true, force: true })
  }
})
