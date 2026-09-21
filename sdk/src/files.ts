// The project as files: the owned texts under a root — evoke.toml, evoke.lock, every overlay and vocabulary — a
// local reflex's directory with its manifest, and a remote reflex's store entry, every file re-hashed and the
// digest composed by the core so it runs only while it still hashes to the lock. Read whole, once; nothing here
// is written. In: a root, a path, a digest. Out: texts and directories, or their absence.

import { createHash } from "node:crypto"
import { readFileSync, readdirSync, statSync } from "node:fs"
import { homedir } from "node:os"
import { join } from "node:path"

import { call } from "./core.ts"

/** The owned texts as found: absent when the file is. */
export interface Snapshot {
  project?: string
  lock?: string
  overlays: Record<string, string>
  vocab: Record<string, string>
}

/** The owned files under the root. */
export function snapshot(root: string): Snapshot {
  const project = text(join(root, "evoke.toml"))
  const lock = text(join(root, "evoke.lock"))
  return {
    ...(project === undefined ? {} : { project }),
    ...(lock === undefined ? {} : { lock }),
    overlays: named(join(root, "overlays")),
    vocab: named(join(root, "vocab")),
  }
}

/** A file's text, or nothing when there is no such file. */
export function text(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8")
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined
    throw error
  }
}

/** Every `<name>.toml` in a directory by name, in name order; none when there is no directory. */
function named(dir: string): Record<string, string> {
  const files: Record<string, string> = {}
  let entries: string[]
  try {
    entries = readdirSync(dir)
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return files
    throw error
  }
  for (const entry of entries.sort()) {
    if (entry.endsWith(".toml")) files[entry.slice(0, -5)] = readFileSync(join(dir, entry), "utf8")
  }
  return files
}

/** The store: fetched trees under `$XDG_CACHE_HOME/evoke/store/<hex>/`. */
function store(): string {
  const base = process.env.XDG_CACHE_HOME ?? join(homedir(), ".cache")
  return join(base, "evoke", "store")
}

/** A store entry's directory when every file under it still hashes to the digest; else nothing — `evoke sync`. */
export function entry(h1: string): string | undefined {
  const dir = join(store(), h1.replace(/^h1:/, ""))
  try {
    if (!statSync(dir).isDirectory()) return undefined
  } catch {
    return undefined
  }
  const hashed: [string, string][] = []
  const walk = (at: string, prefix: string) => {
    for (const name of readdirSync(at).sort()) {
      const path = join(at, name)
      const relative = prefix === "" ? name : `${prefix}/${name}`
      if (statSync(path).isDirectory()) walk(path, relative)
      else hashed.push([relative, `h1:${createHash("sha256").update(readFileSync(path)).digest("hex")}`])
    }
  }
  walk(dir, "")
  return call("digest", { hashed }) === h1 ? dir : undefined
}
