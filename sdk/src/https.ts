// One HTTPS POST, one attempt, over a kept-alive agent: the signal bounds the whole attempt, the timeout each
// step after connect; never retries. In: the url, a bearer, a JSON body, an agent, a signal, the step timeout.
// Out: the status and body, or a Transport saying in plain words what went wrong and whether a connection had
// been made — an attempt that never connected, or a pooled socket the server had already closed, is safe to
// repeat.

import type { Agent } from "node:https"
import { request } from "node:https"
import { getSystemErrorMessage } from "node:util"

/** The most a response may carry: a decision's answers are kilobytes, so anything beyond is not the classifier. */
const LIMIT = 10 * 1024 * 1024

/** What the server answered. */
export interface Response {
  status: number
  body: string
}

/** Why nothing was answered, and whether a connection was made first. */
export class Transport extends Error {
  readonly connected: boolean

  constructor(message: string, connected: boolean) {
    super(message)
    this.name = "Transport"
    this.connected = connected
  }
}

/** `POST url` with a bearer token and a JSON body. */
export function post(
  url: string,
  bearer: string,
  body: string,
  agent: Agent,
  signal: AbortSignal,
  timeout: number,
): Promise<Response> {
  const host = new URL(url).host
  return new Promise((resolve, reject) => {
    let connected = false
    const sent = request(
      url,
      {
        method: "POST",
        agent,
        signal,
        headers: {
          authorization: `Bearer ${bearer}`,
          "content-type": "application/json",
          "content-length": Buffer.byteLength(body),
        },
      },
      response => {
        connected = true
        let text = ""
        let received = 0
        response.setEncoding("utf8")
        response.on("data", (chunk: string) => {
          received += Buffer.byteLength(chunk)
          if (received > LIMIT) response.destroy(new Error(`more than ${LIMIT} bytes`))
          else text += chunk
        })
        response.on("end", () => resolve({ status: response.statusCode ?? 0, body: text }))
        response.on("error", error => reject(new Transport(`reading the response from ${host}: ${reason(error)}`, true)))
      },
    )
    // The step timeout is armed once the socket is connected; Node removes it when the socket returns to the pool.
    sent.setTimeout(timeout, () => sent.destroy(new Error(`no answer within ${timeout / 1000} s`)))
    sent.on("socket", socket => {
      if (socket.connecting) socket.once("connect", () => (connected = true))
      else connected = true
    })
    sent.on("error", (error: NodeJS.ErrnoException) => {
      // A pooled socket the server closed while idle: nothing was received, so the attempt is safe to repeat.
      const stale = sent.reusedSocket && error.code === "ECONNRESET"
      const made = connected && !stale
      reject(new Transport(describe(error, host, made), made))
    })
    sent.end(body)
  })
}

/** What went wrong, in plain words: the host named, the system's own cause, no code and no number. */
export function describe(error: NodeJS.ErrnoException, host: string, connected: boolean): string {
  if (error.syscall === "getaddrinfo") {
    const named = "hostname" in error && typeof error.hostname === "string" ? error.hostname : host
    return `could not resolve ${named}`
  }
  const cause = reason(error)
  return connected ? `${host}: ${cause}` : `could not connect to ${host}: ${cause}`
}

/** The system's words for the error's number, the first of an aggregate's; else its code; else its message. */
function reason(error: NodeJS.ErrnoException): string {
  const first = error instanceof AggregateError ? (error.errors[0] as NodeJS.ErrnoException | undefined) : undefined
  const errno = error.errno ?? first?.errno
  if (typeof errno === "number") {
    try {
      return getSystemErrorMessage(errno)
    } catch {
      // Not a number the system names: the code says it.
    }
  }
  return error.code ?? first?.code ?? error.message
}
