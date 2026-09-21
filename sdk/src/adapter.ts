// Who answers: the contract every adapter meets — one object, the design's — and what the SDK does around one
// call: the deadline as a signal joined with the caller's, the answer raced against it so an adapter that ignores
// it still ends, the call timed, a throw that is not evoke's own turned into a transport fault. In: an adapter,
// a request, a deadline, a signal. Out: the raw answers with one trace entry, or a FaultError; the caller's abort
// passes through as its own reason.

import { faulted } from "./core.ts"
import { EvokeError, FaultError } from "./errors.ts"
import type { Gate, Limits, Question, Raw, Request, State } from "./types.ts"

/** An adapter: what it declares, and one stateless `answer` over any subset of a plan's questions. */
export interface Adapter {
  /** Opaque; changes whenever answers could. */
  readonly id: string
  readonly limits?: Limits | undefined
  /** The floors, each meaning P(correct); none means every decision confirms. */
  readonly gate?: Gate | undefined
  /** The plan digest a recording stands for; a project over another plan refuses it. */
  readonly plan?: string | undefined
  /** A distribution per question asked, keys among those offered; `signal` aborts at the deadline. */
  answer(state: State, questions: Record<string, Question>, signal: AbortSignal): Promise<Raw>
}

/** One call as the SDK saw it: which adapter, how many questions, how long. */
export interface Trace {
  adapter: string
  questions: number
  ms: number
}

/** The adapter asked within the deadline, its answers timed. */
export async function answered(
  adapter: Adapter,
  request: Request,
  deadline: number,
  signal: AbortSignal | undefined,
  invoked: string,
): Promise<{ raw: Raw; trace: Trace }> {
  signal?.throwIfAborted()
  // A referenced timer, so a hung adapter that holds no handle cannot let the process exit before the deadline.
  const timeout = new AbortController()
  const timer = setTimeout(() => timeout.abort(new DOMException(`no answer within ${deadline} ms`, "TimeoutError")), deadline)
  const own = signal === undefined ? timeout.signal : AbortSignal.any([signal, timeout.signal])
  const started = performance.now()
  try {
    const raw = await Promise.race([adapter.answer(request.state, request.questions, own), aborted(own)])
    const ms = Math.round(performance.now() - started)
    return { raw, trace: { adapter: adapter.id, questions: Object.keys(request.questions).length, ms } }
  } catch (error) {
    if (signal?.aborted) throw signal.reason
    // An adapter that could not name the call renders its fault again with it; one that knew better stands.
    if (error instanceof FaultError) throw error.command === "" ? faulted(error.fault, invoked) : error
    if (error instanceof EvokeError) throw error
    const message = timeout.signal.aborted
      ? `did not answer within ${deadline} ms`
      : error instanceof Error
        ? error.message
        : String(error)
    throw faulted({ type: "transport", message }, invoked)
  } finally {
    clearTimeout(timer)
  }
}

/** A promise that rejects with the signal's reason, at once when it is already aborted. */
function aborted(signal: AbortSignal): Promise<never> {
  return new Promise((_, reject) => {
    if (signal.aborted) reject(signal.reason)
    else signal.addEventListener("abort", () => reject(signal.reason), { once: true })
  })
}
