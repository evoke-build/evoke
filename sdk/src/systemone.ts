// The System One wire, behind four doors: jev, TypeSafe AI's own address under TYPESAFE_API_KEY; openjev,
// OpenJEV, an independent service that forwards requests to Jev, under OPENJEV_API_KEY; clef and clef_flash,
// Cloudflare's Clef and Clef-flash on Workers AI, under CLOUDFLARE_API_TOKEN at the account CLOUDFLARE_ACCOUNT_ID
// names. The mapping is the core's — systemone.settings, systemone.address, systemone.request and
// systemone.answers through the module — and the SDK adds the transport: one kept-alive agent for the process,
// through the proxy the environment names, the bearer key, and the policy loop the settings declare: once more
// after a connect error or a retried status, never after a client error, and a 429 that names a pause waited out
// and sent again while the deadline allows. In: a door and its options. Out: an Adapter.

import { Agent } from "node:https"

import type { Adapter } from "./adapter.ts"
import { call, command, fromCode, reply } from "./core.ts"
import { DiagnosticError, FailureError, type Problem } from "./errors.ts"
import { type Response, Unanswered, post } from "./https.ts"
import type { Diagnostic, Door, Gate, Question, Raw, Settings, State } from "./types.ts"

export interface DoorOptions {
  /** The API key; absent, the door's own variable from the environment: `TYPESAFE_API_KEY` for jev, `OPENJEV_API_KEY` for openjev, `CLOUDFLARE_API_TOKEN` for clef and clef_flash. */
  key?: string | undefined
  /** The account's id, at a door whose address names an account; absent, `CLOUDFLARE_ACCOUNT_ID` from the environment. */
  account?: string | undefined
  /** Floors over the defaults, as `[adapters.<door>] gate = { … }` in evoke.toml. */
  gate?: Partial<Gate> | undefined
  /** @internal The project's `[adapters.<door>]` table, when `load` resolves the adapter by name. */
  table?: unknown
}

/** One POST: what the loop is written over, so a test can stand in for the network. */
export type Post = (url: string, bearer: string, body: string, signal: AbortSignal, timeout: number) => Promise<Response>

let shared: Agent | undefined

/** The door, ready to answer; throws at once when no key is set, an override is not a probability, or the proxy
 * named in the environment is no proxy address. */
export function through(door: Door, options: DoorOptions = {}): Adapter {
  const proxy = proxied(door)
  // One agent for the process. Through a proxy, the socket timeout is the one bound on the tunnel Node opens for
  // it — no signal reaches that — so it is the decision's deadline; a direct connection is bounded by the request.
  shared ??= new Agent({ keepAlive: true, ...(proxy === undefined ? {} : { proxyEnv: proxy.env, timeout: 30_000 }) })
  const agent = shared
  return over(door, options, (url, bearer, body, signal, timeout) => post(url, bearer, body, agent, signal, timeout, proxy?.via))
}

/** The proxy the environment names, as the CLI reads it: `https_proxy` before `HTTPS_PROXY`, `no_proxy` before
 * `NO_PROXY`; an `http` or `https` address, else refused rather than bypassed in silence; and it needs the Node
 * that carries `proxyEnv`. */
export function proxied(door: Door): { env: Record<string, string>; via: string } | undefined {
  const [name, value] = process.env.https_proxy ? ["https_proxy", process.env.https_proxy] : ["HTTPS_PROXY", process.env.HTTPS_PROXY]
  if (!value) return undefined
  const url = URL.canParse(value) ? new URL(value) : undefined
  if (url === undefined || !/^https?:$/.test(url.protocol)) {
    const fix = { type: "export_key", var: name } as const
    throw new DiagnosticError([{ message: `${name} is not an http or https proxy address`, fix, command: command(fix) }])
  }
  const [major = 0, minor = 0] = process.versions.node.split(".").map(Number)
  if (major < 24 || (major === 24 && minor < 5)) {
    throw new FailureError(`connecting through the proxy ${url.host}`, `needs Node 24.5 or newer, and this is ${process.versions.node}`, { type: "rerun" }, `${door}()`)
  }
  const bypass = process.env.no_proxy || process.env.NO_PROXY
  return { env: { HTTPS_PROXY: value, ...(bypass ? { NO_PROXY: bypass } : {}) }, via: url.host }
}

/** @internal The door over any transport; `through` gives it the network. */
export function over(door: Door, options: DoorOptions, send: Post): Adapter {
  const table = options.table ?? (options.gate === undefined ? undefined : { gate: options.gate })
  const settings = validated(door, table, options.table === undefined)
  // An empty key is no key. Every variable the door lacks is named at once: the key, and the account its
  // address names.
  const key = options.key || process.env[settings.credential] || undefined
  const url = address(door, settings, options.account)
  if (key === undefined || typeof url !== "string") {
    const fix = { type: "export_key", var: settings.credential } as const
    const keyless: Problem = { message: `${door} needs ${settings.credential}, a key from ${settings.issuer}`, fix, command: command(fix) }
    throw new DiagnosticError([...(key === undefined ? [keyless] : []), ...(typeof url === "string" ? [] : [url])])
  }
  const { policy } = settings
  const [lowest, highest] = policy.retry_statuses
  const declared = settings.declared
  /** The policy loop over one body. */
  const posted = async (body: string, signal: AbortSignal): Promise<Raw> => {
    let retried = 0
    for (;;) {
      const again = retried < policy.retries
      let response: Response
      try {
        response = await send(url, key, body, signal, policy.timeout)
      } catch (error) {
        if (error instanceof Unanswered && again && !error.connected && !signal.aborted) {
          retried += 1
          continue
        }
        throw error
      }
      if (again && response.status >= lowest && response.status <= highest) {
        retried += 1
        continue
      }
      // The service asked for a pause before the next attempt: honoured until the deadline ends the wait.
      if (response.status === 429 && response.retryAfter !== undefined && !signal.aborted) {
        await pausing(response.retryAfter, signal)
        if (!signal.aborted) continue
      }
      return call("systemone.answers", { door, status: response.status, body: response.body })
    }
  }
  return {
    id: declared.id,
    ...(declared.limits === undefined ? {} : { limits: declared.limits }),
    ...(declared.gate === undefined ? {} : { gate: declared.gate }),
    /** The request's bodies, one after the other under the one signal; their answers together are the request's. */
    async answer(state: State, questions: Record<string, Question>, signal: AbortSignal): Promise<Raw> {
      const bodies = call("systemone.request", { door, request: { state, questions, proposed: [] } })
      const raw: Raw = {}
      for (const body of bodies) Object.assign(raw, await posted(JSON.stringify(body), signal))
      return raw
    },
  }
}

/** The address the door posts to: its own, or with the account its variable holds; a missing or malformed id is
 * the problem, with the export line that fixes it. */
function address(door: Door, settings: Settings, given: string | undefined): string | Problem {
  // An empty id is no id.
  const account = given || (settings.account === undefined ? undefined : process.env[settings.account]) || undefined
  const answer = reply("systemone.address", account === undefined ? { door } : { door, account })
  if ("ok" in answer) return answer.ok as string
  if ("bug" in answer) throw new Error(`evoke's core hit a bug: ${answer.bug}`)
  const diagnostic = answer.err as Diagnostic
  return { ...diagnostic, command: command(diagnostic.fix) }
}

/** A pause the service asked for, ended early by the signal. */
function pausing(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise(resolve => {
    const done = () => {
      clearTimeout(timer)
      signal.removeEventListener("abort", done)
      resolve()
    }
    const timer = setTimeout(done, ms)
    signal.addEventListener("abort", done, { once: true })
  })
}

/** The door's settings from the table: a problem in a table from code ends in `<door>({ gate })`, in a file's in
 * `evoke check`. */
function validated(door: Door, table: unknown, fromOptions: boolean): Settings {
  const answer = reply("systemone.settings", table === undefined ? { door } : { door, table })
  if ("ok" in answer) return answer.ok as Settings
  if ("bug" in answer) throw new Error(`evoke's core hit a bug: ${answer.bug}`)
  const problems = answer.err as Diagnostic[]
  if (fromOptions) throw fromCode(problems, `${door}({ gate })`)
  throw new DiagnosticError(problems.map(problem => ({ ...problem, command: command(problem.fix, `${door}()`) })))
}
