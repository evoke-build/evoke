// The transport's words, and the proxy read from the environment.
import { equal, throws } from "node:assert/strict"
import { constants } from "node:os"
import { test } from "node:test"

import type { DiagnosticError } from "../src/errors.ts"
import { describe } from "../src/https.ts"
import { proxied } from "../src/systemone.ts"

const HOST = "api.typesafe.ai"

test("a failure names the host, or the proxy that carried the connection, in plain words", () => {
  const refused = Object.assign(new Error("connect ECONNREFUSED 127.0.0.1:443"), { code: "ECONNREFUSED", errno: -constants.errno.ECONNREFUSED, syscall: "connect" })
  equal(describe(refused, HOST, false), "could not connect to api.typesafe.ai: connection refused")
  equal(describe(refused, HOST, false, "127.0.0.1:3128"), "could not connect to the proxy 127.0.0.1:3128: connection refused")
  const tunnel = Object.assign(new Error("Failed to establish tunnel"), { code: "ERR_PROXY_TUNNEL", statusCode: 407 })
  equal(describe(tunnel, HOST, false, "127.0.0.1:3128"), "the proxy 127.0.0.1:3128 refused the connection: 407")
  const hungUp = Object.assign(new Error("socket hang up"), { code: "ECONNRESET" })
  equal(describe(hungUp, HOST, true), "api.typesafe.ai: connection reset")
  const unresolved = Object.assign(new Error("getaddrinfo ENOTFOUND api.typesafe.ai"), { code: "ENOTFOUND", syscall: "getaddrinfo", hostname: HOST })
  equal(describe(unresolved, HOST, false), "could not resolve api.typesafe.ai")
  equal(describe(new Error("no answer within 1.5 s"), HOST, true), "api.typesafe.ai: no answer within 1.5 s")
})

test("the proxy comes from https_proxy, then HTTPS_PROXY; a value that is no proxy address is refused", () => {
  const kept = { ...process.env }
  const clear = () => { for (const name of ["https_proxy", "HTTPS_PROXY", "no_proxy", "NO_PROXY"]) delete process.env[name] }
  try {
    clear()
    equal(proxied("jev"), undefined)
    process.env.HTTPS_PROXY = "http://ana:s3cret@proxy.example.com:3128"
    process.env.NO_PROXY = "localhost,.internal.example.com"
    const named = proxied("jev")
    equal(named?.via, "proxy.example.com:3128")
    equal(named?.env.NO_PROXY, "localhost,.internal.example.com")
    process.env.https_proxy = "http://first:8080"
    equal(proxied("jev")?.via, "first:8080")
    clear()
    process.env.HTTPS_PROXY = "socks5://127.0.0.1:1080"
    throws(
      () => proxied("jev"),
      (error: DiagnosticError) => error.message === "HTTPS_PROXY is not an http or https proxy address  →  export HTTPS_PROXY=<value>",
    )
    process.env.HTTPS_PROXY = "not a url"
    throws(() => proxied("jev"), (error: DiagnosticError) => error.message.startsWith("HTTPS_PROXY is not an http or https proxy address"))
  } finally {
    clear()
    Object.assign(process.env, kept)
  }
})
