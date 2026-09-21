// Download a URL into your downloads folder or the place named — a path, `~` allowed, a bare word under your home —
// as the file the URL names, never over one already there; a download the deadline cuts short leaves nothing.
import { open, rm, stat } from "node:fs/promises"
import { homedir } from "node:os"
import { join, resolve } from "node:path"
import { Readable } from "node:stream"
import { pipeline } from "node:stream/promises"
import type { Reflex } from "./reflex.d.ts"

export default (async ({ url, to }, { signal }) => {
  const dir = to === undefined ? join(homedir(), "Downloads") : expand(to)
  const name = fileName(url)
  const path = join(dir, name)
  const response = await fetch(url, { signal })
  if (!response.ok) throw new Error(`${new URL(url).host} answered ${response.status}`)
  if (response.body === null) throw new Error(`${new URL(url).host} sent nothing`)
  const file = await open(path, "wx").catch((error: NodeJS.ErrnoException) => {
    if (error.code === "EEXIST") throw new Error(`${name} is already in ${shown(dir)}`)
    if (error.code === "ENOENT") throw new Error(`${shown(dir)} does not exist`)
    throw error
  })
  try {
    await pipeline(Readable.fromWeb(response.body), file.createWriteStream(), { signal })
  } catch (error) {
    await rm(path, { force: true })
    throw signal.aborted ? signal.reason : error
  }
  const { size } = await stat(path)
  return `saved ${name} to ${shown(dir)} (${describe(size)})`
}) satisfies Reflex

/** The file the URL's path names, decoded, or `download` when it names none. */
function fileName(url: string): string {
  const last = new URL(url).pathname.split("/").filter(Boolean).at(-1) ?? ""
  let name: string
  try {
    name = decodeURIComponent(last)
  } catch {
    name = last
  }
  name = name.replaceAll("/", "_")
  return name === "" || name === "." || name === ".." ? "download" : name
}

/** A path as a person writes it: `~` is your home, and so is where a bare word lands. */
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

/** Bytes as a person reads them: `340 B`, `1.2 MB`, `12 GB`. */
function describe(bytes: number): string {
  const units = ["B", "kB", "MB", "GB", "TB"]
  let value = bytes
  let unit = 0
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000
    unit += 1
  }
  return `${unit === 0 || value >= 10 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`
}
