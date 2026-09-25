// The spec's body under a declaration: the text appended to the file the setting names, then, on request, one
// thing the declaration does not name — a read, a write, a program, the network — so what refused it is what
// prints. No frames: the refusal's line is the host's.
import { execFileSync } from "node:child_process"
import { appendFileSync, readFileSync, writeFileSync } from "node:fs"
import { homedir } from "node:os"

Error.stackTraceLimit = 0

export default async ({ text, also }: { text: string; also?: string }, { config }: { config: { file: string } }) => {
  const home = homedir()
  appendFileSync(config.file.replace(/^~/, home), `${text}\n`)
  if (also === "read") readFileSync(`${home}/secret.txt`)
  if (also === "write") writeFileSync(`${home}/pwned.txt`, "x")
  if (also === "spawn") execFileSync("env")
  if (also === "fetch") await fetch("https://example.com/")
  return `noted "${text}"`
}
