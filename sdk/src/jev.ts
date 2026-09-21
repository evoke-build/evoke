// @evoke-build/evoke/jev: the first adapter. The mapping is the core's — jev.settings, jev.request, jev.answers
// through the module — and the SDK adds the transport: one kept-alive agent for the process, the bearer key, and
// the policy loop the settings declare: once more after a connect error or a retried status, never after a
// client error. In: options. Out: an Adapter.

import { Agent } from "node:https"

import type { Adapter } from "./adapter.ts"
import { call, command, fromCode, reply } from "./core.ts"
import { DiagnosticError } from "./errors.ts"
import { type Response, Transport, post } from "./https.ts"
import type { Diagnostic, Gate, Question, Raw, Settings, State } from "./types.ts"

export interface JevOptions {
  /** The API key; absent, `TYPESAFE_API_KEY` from the environment. */
  key?: string | undefined
  /** Floors over the defaults, as `[adapters.jev] gate = { … }` in evoke.toml. */
  gate?: Partial<Gate> | undefined
  /** @internal The project's `[adapters.jev]` table, when `load` resolves the adapter by name. */
  table?: unknown
}

/** One POST: what the loop is written over, so a test can stand in for the network. */
export type Post = (url: string, bearer: string, body: string, signal: AbortSignal, timeout: number) => Promise<Response>

let shared: Agent | undefined

/** Jev, ready to answer; throws at once when no key is set or an override is not a probability. */
export function jev(options: JevOptions = {}): Adapter {
  shared ??= new Agent({ keepAlive: true })
  const agent = shared
  return over(options, (url, bearer, body, signal, timeout) => post(url, bearer, body, agent, signal, timeout))
}

/** @internal The adapter over any transport; `jev()` gives it the network. */
export function over(options: JevOptions, send: Post): Adapter {
  const table = options.table ?? (options.gate === undefined ? undefined : { gate: options.gate })
  const settings = validated(table, options.table === undefined)
  const key = options.key ?? process.env[settings.credential]
  if (key === undefined) {
    const fix = { type: "export_key", var: settings.credential } as const
    throw new DiagnosticError([{ message: `jev needs ${settings.credential}`, fix, command: command(fix) }])
  }
  const { policy, url } = settings
  const [lowest, highest] = policy.retry_statuses
  const declared = settings.declared
  return {
    id: declared.id,
    ...(declared.limits === undefined ? {} : { limits: declared.limits }),
    ...(declared.gate === undefined ? {} : { gate: declared.gate }),
    async answer(state: State, questions: Record<string, Question>, signal: AbortSignal): Promise<Raw> {
      const body = JSON.stringify(call("jev.request", { request: { state, questions, proposed: [] } }))
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
        return call("jev.answers", { status: response.status, body: response.body })
      }
    },
  }
}

/** The settings from the table: a problem in a table from code ends in `jev({ gate })`, in a file's in `evoke check`. */
function validated(table: unknown, fromOptions: boolean): Settings {
  const answer = reply("jev.settings", table === undefined ? {} : { table })
  if ("ok" in answer) return answer.ok as Settings
  if ("bug" in answer) throw new Error(`evoke's core hit a bug: ${answer.bug}`)
  const problems = answer.err as Diagnostic[]
  if (fromOptions) throw fromCode(problems, "jev({ gate })")
  throw new DiagnosticError(problems.map(problem => ({ ...problem, command: command(problem.fix, "jev()") })))
}
