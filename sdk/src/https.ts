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

/** What the server answered; with a 429, the pause it asked for before the next attempt, in milliseconds. */
export interface Response {
  status: number
  body: string
  retryAfter?: number | undefined
}

/** The pause a 429 asks for: `Retry-After` in seconds, at least one, so a service that says "now" is still paced;
 * an HTTP date, which no engine sends, names no pause. */
export function pause(header: string | string[] | undefined): number | undefined {
  const text = Array.isArray(header) ? header[0] : header
  if (text === undefined || !/^\s*\d+\s*$/.test(text)) return undefined
  return Math.max(1, Number.parseInt(text, 10)) * 1000
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

/** `POST url` with a bearer token and a JSON body; `via` is the proxy's host when one carries the connection. */
export function post(
  url: string,
  bearer: string,
  body: string,
  agent: Agent,
  signal: AbortSignal,
  timeout: number,
  via?: string,
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
        response.on("end", () => {
          const status = response.statusCode ?? 0
          const retryAfter = status === 429 ? pause(response.headers["retry-after"]) : undefined
          resolve({ status, body: text, ...(retryAfter === undefined ? {} : { retryAfter }) })
        })
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
      reject(new Transport(describe(error, host, made, via), made))
    })
    sent.end(body)
  })
}

/** What went wrong, in plain words: the host named, or the proxy when one carries the connection; the system's
 * own cause, no code and no number. */
export function describe(error: NodeJS.ErrnoException, host: string, connected: boolean, via?: string): string {
  if (error.code === "ERR_PROXY_TUNNEL") {
    const status = "statusCode" in error && typeof error.statusCode === "number" ? `: ${error.statusCode}` : ""
    return `the proxy ${via ?? "the environment names"} refused the connection${status}`
  }
  if (error.syscall === "getaddrinfo") {
    const named = "hostname" in error && typeof error.hostname === "string" ? error.hostname : (via ?? host)
    return `could not resolve ${named}`
  }
  const cause = reason(error)
  if (connected) return `${host}: ${cause}`
  return via === undefined ? `could not connect to ${host}: ${cause}` : `could not connect to the proxy ${via}: ${cause}`
}

/** The system's words for the error's number, the first of an aggregate's; a reset by its name; else the message,
 * else the code. */
function reason(error: NodeJS.ErrnoException): string {
  const first = error instanceof AggregateError ? (error.errors[0] as NodeJS.ErrnoException | undefined) : undefined
  const errno = error.errno ?? first?.errno
  if (typeof errno === "number") {
    try {
      return getSystemErrorMessage(errno)
    } catch {
      // Not a number the system names: the words below say it.
    }
  }
  if ((error.code ?? first?.code) === "ECONNRESET") return "connection reset"
  return error.message || first?.message || error.code || first?.code || "no reason given"
}
