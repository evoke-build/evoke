// The layers around a body in the SDK: Node's permission flags on both systems, and on macOS `sandbox-exec -p`
// with the core's profile in front of the command; on Linux the kernel layer needs native code the SDK does not
// carry, so the status says so. The facts the core's rules take, gathered here: the runtime as it runs, each
// declared path's real form and kind, each program's place on PATH and the interpreters it runs through, a private
// temporary folder for the run. In: a policy, the body's directory. Out: the facts or what is missing, the
// command that runs a program under the layers, the status.

import { accessSync, closeSync, constants, mkdtempSync, openSync, readSync, realpathSync, rmSync, statSync } from "node:fs"
import { homedir, tmpdir } from "node:os"
import { join } from "node:path"

import { call } from "./core.ts"
import type { Contained, Executable, Facts, Found, NeedsKey, Place, Platform, Policy, Program } from "./types.ts"

const PLATFORM: Platform = process.platform === "linux" ? "linux" : "macos"
const SANDBOX_EXEC = "/usr/bin/sandbox-exec"

/** What the machine lacks that the declaration names: a path, or a program `PATH` does not hold. */
export type Missing = { place: Place; key: NeedsKey } | { program: Program }

/** The policy and the facts a body runs under. */
export interface Layers {
  policy: Policy
  facts: Facts
}

/** Whether this machine holds a whole declaration: macOS through Seatbelt, when `sandbox-exec` is there; Linux
 *  through Node's layer alone, which holds files and programs, not the network and not what a program reaches. */
export function status(): Contained {
  if (PLATFORM === "linux") {
    return { type: "partial", why: "Node holds files and programs; the network and what a program reaches are not held" }
  }
  return runs(SANDBOX_EXEC) ? { type: "full" } : { type: "none", why: "sandbox-exec is missing" }
}

/** A private temporary folder for one run, under the system's, readable by its owner alone, by its real path;
 *  removed after the run. */
export function scratch(): { path: string; remove(): void } {
  const path = realpathSync(mkdtempSync(join(tmpdir(), "evoke-")))
  return { path, remove: () => rmSync(path, { recursive: true, force: true }) }
}

/** The facts the core's rules need: each declared path's real form and kind, each program's place, the runtime
 *  for a file body; a path or a program the machine lacks is the miss, before anything runs. */
export function facts(policy: Policy, runtime: string | undefined, bodyDir: string, tmp: string): Facts | Missing {
  const found: Record<string, Found> = {}
  for (const [key, places] of [
    ["reads", policy.reads ?? []],
    ["writes", policy.writes ?? []],
  ] as const) {
    for (const place of places) {
      let stats
      try {
        stats = statSync(place.path)
      } catch {
        return { place, key }
      }
      found[place.path] = { real: realpathSync(place.path), dir: stats.isDirectory() }
    }
  }
  const programs: Record<string, Executable> = {}
  for (const program of policy.runs ?? []) {
    const path = locate(program)
    if (path === undefined) return { program }
    programs[program] = executable(path)
  }
  return {
    platform: PLATFORM,
    ...(runtime === undefined ? {} : { runtime: executable(runtime) }),
    body_dir: bodyDir,
    tmp,
    home: homedir(),
    ...(Object.keys(found).length === 0 ? {} : { found }),
    ...(Object.keys(programs).length === 0 ? {} : { programs }),
  }
}

/** Node's flags for a file body, from the core. */
export function flags(layers: Layers): string[] {
  return call("contain.node_flags", layers)
}

/** The command that runs a program under the layers: on macOS `sandbox-exec -p` with the core's profile in front;
 *  on Linux the program itself, Node's flags being the one layer the SDK has. */
export function under(program: string, args: string[], layers: Layers): { file: string; args: string[] } {
  if (PLATFORM === "macos") return { file: SANDBOX_EXEC, args: ["-p", call("contain.seatbelt", layers), program, ...args] }
  return { file: program, args }
}

/** A program by its absolute path, or the first executable of its name on `PATH`. */
function locate(program: string): string | undefined {
  if (program.startsWith("/")) return runs(program) ? program : undefined
  for (const dir of (process.env.PATH ?? "").split(":")) {
    if (dir === "") continue
    const candidate = join(dir, program)
    if (runs(candidate)) return candidate
  }
  return undefined
}

function runs(path: string): boolean {
  try {
    accessSync(path, constants.X_OK)
    return statSync(path).isFile()
  } catch {
    return false
  }
}

/** A program as the kernel runs it: its real path, and on macOS the interpreters a script runs through — its
 *  shebang's, and the program `env` hands over to, found on `PATH`. */
function executable(path: string): Executable {
  const real = realpathSync(path)
  const interpreters = PLATFORM === "macos" ? viaShebang(real) : []
  return { path: real, ...(interpreters.length === 0 ? {} : { interpreters }) }
}

function viaShebang(program: string): string[] {
  let first: string
  try {
    first = head(program)
  } catch {
    return []
  }
  if (!first.startsWith("#!")) return []
  const [interpreter, argument] = (first.slice(2).split("\n")[0] ?? "").trim().split(/\s+/)
  if (interpreter === undefined || !interpreter.startsWith("/")) return []
  const interpreters = [realpathSync(interpreter)]
  if (interpreter.endsWith("/env") && argument !== undefined && !argument.startsWith("-")) {
    const handed = locate(argument)
    if (handed !== undefined) interpreters.push(realpathSync(handed))
  }
  return interpreters
}

/** A program's first bytes, where a script names its interpreter: never the whole of a binary. */
function head(program: string): string {
  const fd = openSync(program, "r")
  try {
    const buffer = Buffer.alloc(256)
    const read = readSync(fd, buffer, 0, buffer.length, 0)
    return buffer.toString("latin1", 0, read)
  } finally {
    closeSync(fd)
  }
}
