// Append one dated line to the notes file `config.file` names — `~` allowed, a bare name under your home — making
// the file and its folder when they are not there: `2026-09-20  buy milk`.
import { appendFile, mkdir } from "node:fs/promises"
import { homedir } from "node:os"
import { dirname, join, resolve } from "node:path"
import type { Reflex } from "./reflex.d.ts"

export default (async ({ text }, { config }) => {
  const file = expand(config.file)
  const line = text.replace(/\s*\n\s*/g, " ")
  await mkdir(dirname(file), { recursive: true })
  await appendFile(file, `${today()}  ${line}\n`)
  return `noted "${line}" in ${shown(file)}`
}) satisfies Reflex

/** The day as `2026-09-20`, local time. */
function today(): string {
  const now = new Date()
  const two = (n: number) => String(n).padStart(2, "0")
  return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`
}

/** A path as a person writes it: `~` is your home, and so is where a bare name lands. */
function expand(path: string): string {
  const home = homedir()
  if (path === "~") return home
  if (path.startsWith("~/")) return join(home, path.slice(2))
  return resolve(home, path)
}

/** A path under your home as `~/…`. */
function shown(path: string): string {
  const home = homedir()
  return path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path
}
