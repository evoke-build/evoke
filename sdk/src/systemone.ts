// The System One wire, behind two doors: jev, TypeSafe AI's own address under TYPESAFE_API_KEY, and openjev,
// OpenJEV's public door to the same model under OPENJEV_API_KEY. The mapping is the core's — systemone.settings,
// systemone.request and systemone.answers through the module — and the SDK adds the transport: one kept-alive
// agent for the process, through the proxy the environment names, the bearer key, and the policy loop the
// settings declare: once more after a connect error or a retried status, never after a client error. In: a door
// and its options. Out: an Adapter.

import { Agent } from "node:https"

import type { Adapter } from "./adapter.ts"
import { call, command, fromCode, reply } from "./core.ts"
import { DiagnosticError, FailureError } from "./errors.ts"
import { type Response, Transport, post } from "./https.ts"
import type { Diagnostic, Door, Gate, Question, Raw, Settings, State } from "./types.ts"

export interface DoorOptions {
  /** The API key; absent, the door's own variable from the environment: `TYPESAFE_API_KEY` for jev, `OPENJEV_API_KEY` for openjev. */
  key?: string | undefined
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
  // An empty key is no key.
  const key = options.key || process.env[settings.credential] || undefined
  if (key === undefined) {
    const fix = { type: "export_key", var: settings.credential } as const
    throw new DiagnosticError([{ message: `${door} needs ${settings.credential}, a key from ${settings.issuer}`, fix, command: command(fix) }])
  }
  const { policy, url } = settings
  const [lowest, highest] = policy.retry_statuses
  const declared = settings.declared
  return {
    id: declared.id,
    ...(declared.limits === undefined ? {} : { limits: declared.limits }),
    ...(declared.gate === undefined ? {} : { gate: declared.gate }),
    async answer(state: State, questions: Record<string, Question>, signal: AbortSignal): Promise<Raw> {
      const body = JSON.stringify(call("systemone.request", { door, request: { state, questions, proposed: [] } }))
      for (let attempt = 1; ; attempt++) {
        const again = attempt <= policy.retries
        let response: Response
        try {
          response = await send(url, key, body, signal, policy.timeout)
        } catch (error) {
          if (error instanceof Transport && again && !error.connected && !signal.aborted) continue
          throw error
        }
        if (again && response.status >= lowest && response.status <= highest) continue
        return call("systemone.answers", { status: response.status, body: response.body, credential: settings.credential })
      }
    },
  }
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
