// The loader: core.wasm instantiated once, on first use, and one function over its three exports — the op and
// the input written as UTF-8 into buffers the module allocates, the call, the reply read back, every buffer freed.
// Synchronous: a rule costs microseconds and nothing here waits on the world. Nothing else in the SDK touches the
// module. In: an op name and one JSON argument object. Out: the raw reply, or the value with its error thrown.

import { readFileSync } from "node:fs"

import { DiagnosticError, FaultError, type Problem } from "./errors.ts"
import type { Op, Ops } from "./ops.ts"
import type { Diagnostic, Fault, Fix, Result } from "./types.ts"

/** The module's exports: its memory, and the three functions of the boundary. */
interface Exports {
  memory: WebAssembly.Memory
  alloc(len: number): number
  free(ptr: number, len: number): void
  call(op: number, opLen: number, input: number, inputLen: number): bigint
}

/** What the op table answers: the result, the function's own error, or a bug in the call the SDK made. */
export type Reply = { ok: unknown } | { err: unknown } | { bug: string }

const encoder = new TextEncoder()
const decoder = new TextDecoder()

let loaded: Exports | undefined

/** The module, instantiated once; after a trap it is instantiated again, since a trap leaves its memory as it was. */
function core(): Exports {
  if (loaded === undefined) {
    const module = new WebAssembly.Module(readFileSync(new URL("../core.wasm", import.meta.url)))
    loaded = new WebAssembly.Instance(module, {}).exports as unknown as Exports
  }
  return loaded
}

/** One op on one input: the raw reply. A trap in the core is a bug in evoke, thrown as one. */
export function reply(op: string, input: object): Reply {
  const wasm = core()
  const opBytes = encoder.encode(op)
  const inputBytes = encoder.encode(JSON.stringify(input))
  try {
    const opPtr = wasm.alloc(opBytes.length)
    const inputPtr = wasm.alloc(inputBytes.length)
    // Both views are made after both allocations: growing the memory detaches an earlier view.
    new Uint8Array(wasm.memory.buffer, opPtr, opBytes.length).set(opBytes)
    new Uint8Array(wasm.memory.buffer, inputPtr, inputBytes.length).set(inputBytes)
    const packed = wasm.call(opPtr, opBytes.length, inputPtr, inputBytes.length)
    const ptr = Number(packed >> 32n)
    const len = Number(packed & 0xffff_ffffn)
    const text = decoder.decode(new Uint8Array(wasm.memory.buffer, ptr, len))
    wasm.free(ptr, len)
    wasm.free(opPtr, opBytes.length)
    wasm.free(inputPtr, inputBytes.length)
    return JSON.parse(text) as Reply
  } catch (error) {
    if (error instanceof WebAssembly.RuntimeError) {
      loaded = undefined
      throw new Error(`evoke's core hit a bug in ${op}: ${error.message}`, { cause: error })
    }
    throw error
  }
}

/** An op's value: what it answers under `ok`, the Result unwrapped where the function returns one. */
export type Ok<O extends Op> = Ops[O]["output"] extends Result<infer T, infer _E> ? T : Ops[O]["output"]

/**
 * One op, typed: the value, or the op's own error thrown as the CLI would print it — a `DiagnosticError` for
 * diagnostics, a `FaultError` for a fault. `invoked` names the SDK call for a fix that says to try again.
 */
export function call<O extends Op>(op: O, input: Ops[O]["input"], invoked = ""): Ok<O> {
  const answer = reply(op, input)
  if ("ok" in answer) return answer.ok as Ok<O>
  if ("bug" in answer) throw bug(answer.bug)
  throw raised(answer.err, invoked)
}

/** The command that fixes a problem, rendered by the core. */
export function command(fix: Fix, invoked = ""): string {
  return call("fix", { fix, invoked })
}

/** A diagnostic with its command rendered. */
export function problem(diagnostic: Diagnostic, invoked = ""): Problem {
  return { ...diagnostic, command: command(diagnostic.fix, invoked) }
}

/** Diagnostics over something given in code — a manifest, words, a gate — where the core's `evoke check`, meant
 *  for a file, cannot apply: the fix is the call itself. */
export function fromCode(diagnostics: readonly Diagnostic[], invoked: string): DiagnosticError {
  return new DiagnosticError(
    diagnostics.map(diagnostic => ({ ...diagnostic, command: diagnostic.fix.type === "check" ? invoked : command(diagnostic.fix, invoked) })),
  )
}

/** A fault as the error it is, its words and command rendered by the core. */
export function faulted(fault: Fault, invoked = ""): FaultError {
  const { message, command } = call("fault", { fault, invoked })
  return new FaultError(fault, message, command)
}

/** A bug in evoke — never a user error. */
export function bug(message: string): Error {
  return new Error(`evoke's core hit a bug: ${message}`)
}

/** An op's `err` as the error it is: one diagnostic or several, or a fault; anything else is a bug in the SDK. */
function raised(err: unknown, invoked: string): Error {
  if (Array.isArray(err)) {
    return new DiagnosticError((err as Diagnostic[]).map(diagnostic => problem(diagnostic, invoked)))
  }
  if (err !== null && typeof err === "object") {
    if ("fix" in err) return new DiagnosticError([problem(err as Diagnostic, invoked)])
    if ("type" in err) return faulted(err as Fault, invoked)
  }
  return bug(`an error of an unknown shape: ${JSON.stringify(err)}`)
}
