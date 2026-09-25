// Append one dated line to the notes file `config.file` names — a path under your home, which evoke hands over
// expanded, or absolute — a file that exists, since the declaration names it: `2026-09-20  buy milk`. A file
// whose last line has no line feed gets one first, so the note never joins onto it.
import { appendFile, open, stat } from "node:fs/promises"
import { homedir } from "node:os"
import type { Reflex } from "./reflex.d.ts"

export default (async ({ text }, { config }) => {
  const file = config.file
  const line = text.replace(/\s*\n\s*/g, " ")
  const lead = (await endsWithLineFeed(file)) ? "" : "\n"
  await appendFile(file, `${lead}${today()}  ${line}\n`)
  return `noted "${line}" in ${shown(file)}`
}) satisfies Reflex

/** Whether the file ends where a new line may start: a line feed, or nothing yet. */
async function endsWithLineFeed(file: string): Promise<boolean> {
  const size = await stat(file).then(stats => stats.size, () => 0)
  if (size === 0) return true
  const handle = await open(file, "r")
  try {
    const { bytesRead, buffer } = await handle.read(Buffer.alloc(1), 0, 1, size - 1)
    return bytesRead === 1 && buffer[0] === 0x0a
  } finally {
    await handle.close()
  }
}

/** The day as `2026-09-20`, local time. */
function today(): string {
  const now = new Date()
  const two = (n: number) => String(n).padStart(2, "0")
  return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`
}

/** A path under your home as `~/…`. */
function shown(path: string): string {
  const home = homedir()
  return path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path
}
