// A body run, three ways: a function in-process under the deadline's signal; a file body in a child through the
// same loader.mjs the CLI embeds, the envelope on its stdin held open until it exits; a program spawned with an
// argv, config as EVOKE_CONFIG_<KEY> and the input as EVOKE_INPUT. Config is resolved from the environment here
// and only here; a secret reaches the body and nothing else. In: the envelope, where the body lives, a signal.
// Out: what the body returned, or a FailureError; the caller's abort passes through as its own reason.

import { type ChildProcess, spawn } from "node:child_process"
import { fileURLToPath } from "node:url"

import { command } from "./core.ts"
import { FailureError } from "./errors.ts"
import type { Envelope, Fix, Setting } from "./types.ts"

/** What a body returns: its text, and data when it gives some. */
export interface Result {
  text: string
  data?: unknown
}

/** What a body receives beside its arguments. */
export interface Context<Config> {
  input: string
  config: Config
  signal: AbortSignal
}

/** A body's own type: the function a file exports by default, and what `reflex` takes. */
export type Reflex<Args, Config = Record<never, string>> = (
  args: Args,
  context: Context<Config>,
) => Result | string | Promise<Result | string>

/** The loader beside the package: the CLI's own file. */
const LOADER = fileURLToPath(new URL("../runtime/loader.mjs", import.meta.url))

/** What the scrubbed environment keeps. */
const KEPT = ["PATH", "HOME", "TMPDIR", "LANG", "TERM"]

/** What a body gets to settle after its signal aborts, and a group after SIGTERM before SIGKILL, in milliseconds. */
const GRACE = 1000

/** A function body, in-process: its arguments as the envelope carries them, the deadline as its signal. */
export async function inline(
  what: string,
  body: Reflex<Record<string, unknown>, Record<string, string>>,
  envelope: Envelope,
  signal: AbortSignal | undefined,
): Promise<Result> {
  signal?.throwIfAborted()
  const config = resolved(what, envelope.config)
  // Referenced timers: a body that hangs holding no handle cannot let the process exit before the deadline.
  const timeout = new AbortController()
  const timer = setTimeout(() => timeout.abort(new DOMException(`no answer within ${envelope.deadline} ms`, "TimeoutError")), envelope.deadline)
  const own = signal === undefined ? timeout.signal : AbortSignal.any([signal, timeout.signal])
  let grace: NodeJS.Timeout | undefined
  const abandoned = new Promise<{ abandoned: true }>(resolve => {
    const later = () => (grace = setTimeout(() => resolve({ abandoned: true }), GRACE))
    if (own.aborted) later()
    else own.addEventListener("abort", later, { once: true })
  })
  const settled = Promise.resolve()
    .then(() => body(envelope.args, { input: envelope.input, config, signal: own }))
    .then(returned => ({ ok: returned }) as const, (error: unknown) => ({ threw: error }) as const)
  try {
    const outcome = await Promise.race([settled, abandoned])
    if (signal?.aborted) throw signal.reason
    if ("abandoned" in outcome) throw failed(what, `did not finish within ${envelope.deadline} ms`)
    if ("threw" in outcome) throw failed(what, outcome.threw instanceof Error ? outcome.threw.message : String(outcome.threw))
    return result(what, outcome.ok)
  } finally {
    clearTimeout(timer)
    if (grace !== undefined) clearTimeout(grace)
  }
}

/** A file body, in a child: the loader fed the envelope with the body's path under its directory. */
export async function child(
  what: string,
  dir: string,
  envelope: Envelope & { run: string },
  signal: AbortSignal | undefined,
): Promise<Result> {
  signal?.throwIfAborted()
  const fed = {
    run: `${dir}/${envelope.run}`,
    args: envelope.args,
    input: envelope.input,
    config: resolved(what, envelope.config),
    deadline: envelope.deadline,
  }
  // The loader warms Node's type stripper through an API still marked experimental: its warning is off, so a
  // body's stderr is the body's.
  const started = spawn(process.execPath, ["--disable-warning=ExperimentalWarning", LOADER], {
    detached: true,
    env: scrubbed(),
    stdio: ["pipe", "pipe", "inherit"],
  })
  // A loader gone before it read is reported by its exit, not by the pipe.
  started.stdin?.on("error", () => {})
  started.stdin?.write(`${JSON.stringify(fed)}\n`)
  // Stdin stays open until the child is gone: a body's life is bounded by its parent's.
  const { output, code, timedOut, error } = await collected(started, envelope.deadline + 2 * GRACE, signal)
  if (signal?.aborted) throw signal.reason
  if (error !== undefined) throw failed(what, error.message)
  if (timedOut) throw failed(what, `did not finish within ${envelope.deadline} ms`)
  const line = output.split("\n")[0] ?? ""
  let parsed: unknown
  try {
    parsed = JSON.parse(line)
  } catch {
    throw failed(what, line === "" ? `${ended(code)} without a result` : `the result line is not JSON: ${line}`)
  }
  if (parsed !== null && typeof parsed === "object" && "error" in parsed) throw failed(what, String(parsed.error))
  return result(what, parsed)
}

/** A program with its argv: config and the input in its environment, its stdout the text. */
export async function program(
  what: string,
  argv: readonly string[],
  envelope: Envelope,
  signal: AbortSignal | undefined,
): Promise<Result> {
  signal?.throwIfAborted()
  const [file, ...rest] = argv
  if (file === undefined) throw failed(what, "the argv is empty")
  const env = scrubbed()
  for (const [key, value] of Object.entries(resolved(what, envelope.config))) {
    env[`EVOKE_CONFIG_${key.toUpperCase()}`] = value
  }
  env.EVOKE_INPUT = envelope.input
  const started = spawn(file, rest, { detached: true, env, stdio: ["ignore", "pipe", "inherit"] })
  const { output, code, timedOut, error } = await collected(started, envelope.deadline, signal)
  if (signal?.aborted) throw signal.reason
  if (error !== undefined) throw failed(what, error.message)
  if (timedOut) throw failed(what, `did not finish within ${envelope.deadline} ms`)
  if (code !== 0) throw failed(what, ended(code))
  return { text: output.endsWith("\n") ? output.slice(0, -1) : output }
}

/** Each config setting as its value; a variable that is not set is the failure it names. */
function resolved(what: string, config: Record<string, Setting>): Record<string, string> {
  const values: Record<string, string> = {}
  for (const [key, setting] of Object.entries(config)) {
    if (setting.type === "plain") {
      values[key] = setting.value
      continue
    }
    const value = process.env[setting.var]
    if (value === undefined) {
      throw failed(what, `${setting.var} is not set`, { type: "export_key", var: setting.var })
    }
    values[key] = value
  }
  return values
}

/** The environment a body runs under: five variables, nothing else. */
function scrubbed(): Record<string, string> {
  const env: Record<string, string> = {}
  for (const name of KEPT) {
    const value = process.env[name]
    if (value !== undefined) env[name] = value
  }
  return env
}

interface Collected {
  output: string
  code: number | null
  timedOut: boolean
  error?: Error
}

/** Everything the child writes until it exits — a grace for what is still in the pipe, so a grandchild that keeps
 *  stdout cannot hold the run — then its code; past the wait, or on abort, the group is ended: SIGTERM, a grace,
 *  SIGKILL. A child that could not start is the error it raised. */
function collected(started: ChildProcess, wait: number, signal: AbortSignal | undefined): Promise<Collected> {
  return new Promise(resolve => {
    let output = ""
    let timedOut = false
    let error: Error | undefined
    let done = false
    let drain: NodeJS.Timeout | undefined
    started.stdout?.setEncoding("utf8")
    started.stdout?.on("data", (chunk: string) => {
      output += chunk
    })
    const settle = (code: number | null) => {
      if (done) return
      done = true
      clearTimeout(timer)
      if (drain !== undefined) clearTimeout(drain)
      signal?.removeEventListener("abort", end)
      resolve(error === undefined ? { output, code, timedOut } : { output, code, timedOut, error })
    }
    const end = () => {
      timedOut = true
      if (started.pid === undefined) return
      try {
        process.kill(-started.pid, "SIGTERM")
      } catch {
        return
      }
      setTimeout(() => {
        try {
          process.kill(-started.pid!, "SIGKILL")
        } catch {
          // The group is already gone.
        }
      }, GRACE).unref()
    }
    const timer = setTimeout(end, wait)
    if (signal?.aborted) end()
    else signal?.addEventListener("abort", end, { once: true })
    started.on("error", raised => {
      error = raised
      settle(null)
    })
    started.on("exit", code => {
      drain = setTimeout(() => settle(code), GRACE)
    })
    started.on("close", code => settle(code))
  })
}

/** A body's return as a result: a string is the text; `{ text, data? }` passes; anything else is a failure. */
function result(what: string, returned: unknown): Result {
  if (typeof returned === "string") return { text: returned }
  if (returned !== null && typeof returned === "object" && "text" in returned && typeof returned.text === "string") {
    return "data" in returned && returned.data !== undefined ? { text: returned.text, data: returned.data } : { text: returned.text }
  }
  throw failed(what, `the body returned ${describe(returned)}, not text or { text, data }`)
}

function describe(value: unknown): string {
  if (value === null) return "null"
  if (Array.isArray(value)) return "an array"
  return typeof value === "object" ? "an object without text" : typeof value
}

/** How a child ended: its code, or a signal. */
function ended(code: number | null): string {
  return code === null ? "was killed" : `exited ${code}`
}

function failed(what: string, why: string, fix: Fix = { type: "rerun" }): FailureError {
  return new FailureError(what, why, fix, command(fix, `run(d)`))
}
