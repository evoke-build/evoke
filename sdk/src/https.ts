// One HTTPS POST, one attempt, over a kept-alive agent: the signal bounds the whole attempt, the timeout each
// step after connect; never retries. In: the url, a bearer, a JSON body, an agent, a signal, the step timeout.
// Out: the status and body, or a Transport saying whether a connection had been made — an attempt that never
// connected, or a pooled socket the server had already closed, is safe to repeat.

import type { Agent } from "node:https"
import { request } from "node:https"

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
        response.setEncoding("utf8")
        response.on("data", (chunk: string) => {
          text += chunk
        })
        response.on("end", () => resolve({ status: response.statusCode ?? 0, body: text }))
        response.on("error", error => reject(new Transport(`reading the response: ${error.message}`, true)))
      },
    )
    // The step timeout is armed once the socket is connected; Node removes it when the socket returns to the pool.
    sent.setTimeout(timeout, () => sent.destroy(new Error(`no answer within ${timeout} ms after connect`)))
    sent.on("socket", socket => {
      if (socket.connecting) socket.once("connect", () => (connected = true))
      else connected = true
    })
    sent.on("error", (error: NodeJS.ErrnoException) => {
      // A pooled socket the server closed while idle: nothing was received, so the attempt is safe to repeat.
      const stale = sent.reusedSocket && error.code === "ECONNRESET"
      reject(new Transport(error.message, connected && !stale))
    })
    sent.end(body)
  })
}
