// The project as files: the owned texts under a root — evoke.toml, evoke.lock, every overlay and vocabulary — a
// local reflex's directory with its manifest, and a remote reflex's store entry, every file re-hashed and the
// digest composed by the core so it runs only while it still hashes to the lock. Read whole, once; nothing here
// is written. In: a root, a path, a digest. Out: texts and directories, or their absence.

import { createHash } from "node:crypto"
import { readFileSync, readdirSync, statSync } from "node:fs"
import { homedir } from "node:os"
import { join } from "node:path"

import { call, misnamed } from "./core.ts"
import { FailureError } from "./errors.ts"

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
    overlays: named(join(root, "overlays"), "local"),
    vocab: named(join(root, "vocab"), "vocab"),
  }
}

/** A file's text, or nothing when there is no such file; one that will not read is a failure naming it. */
export function text(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8")
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined
    throw unreadable(path, error)
  }
}

/** Every `<name>.toml` in a directory by name, in name order; none when there is no directory. A file whose stem
 *  is no name is not an owned file and is left alone, as the CLI leaves it. */
function named(dir: string, kind: "local" | "vocab"): Record<string, string> {
  const files: Record<string, string> = {}
  let entries: string[]
  try {
    entries = readdirSync(dir)
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return files
    throw unreadable(dir, error)
  }
  for (const entry of entries.sort()) {
    if (!entry.endsWith(".toml")) continue
    const stem = entry.slice(0, -5)
    if (misnamed(stem, kind) !== undefined) continue
    const path = join(dir, entry)
    try {
      files[stem] = readFileSync(path, "utf8")
    } catch (error) {
      throw unreadable(path, error)
    }
  }
  return files
}

/** A path that exists and will not read — a directory where a file should be, a permission missing. */
function unreadable(path: string, error: unknown): FailureError {
  const why = error instanceof Error ? error.message : String(error)
  return new FailureError(`reading ${path}`, why, { type: "rerun" }, "load()")
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
  try {
    walk(dir, "")
  } catch (error) {
    throw unreadable(dir, error)
  }
  return call("digest", { hashed }) === h1 ? dir : undefined
}
