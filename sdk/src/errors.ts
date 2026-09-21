// The three errors, as the CLI's exits 3, 4 and 1: what went wrong as data, and the line that fixes it, rendered
// by the core. In: the core's diagnostics or fault, or a host's failure, each with its fixing command already
// rendered. Out: an Error whose message is the CLI's own line — `<reflex>: <what>  →  <fix>`, one per problem.
// `fix` is always the structured value, `command` always the rendered line.

import type { Diagnostic, Fault, Fix } from "./types.ts"

/** Every error evoke raises: its kind, and the command or line that fixes it. */
export abstract class EvokeError extends Error {
  abstract readonly kind: "diagnostic" | "fault" | "failure"
  /** The literal command or line that fixes it; empty when nothing but trying again applies. */
  readonly command: string

  protected constructor(message: string, command: string) {
    super(message)
    this.name = new.target.name
    this.command = command
  }
}

/** A diagnostic with its command rendered. */
export type Problem = Diagnostic & { command: string }

/** Something a person fixes in the files or the environment: every problem the core found, each with its fix. */
export class DiagnosticError extends EvokeError {
  override readonly kind = "diagnostic"
  readonly problems: readonly Problem[]

  constructor(problems: readonly Problem[]) {
    super(problems.map(line).join("\n"), problems[0]?.command ?? "")
    this.problems = problems
  }
}

/** The adapter failed, or its answers did not validate. */
export class FaultError extends EvokeError {
  override readonly kind = "fault"
  readonly fault: Fault

  constructor(fault: Fault, message: string, command: string) {
    super(command === "" ? message : `${message}  →  ${command}`, command)
    this.fault = fault
  }
}

/** A host or a body failed: what was attempted, why, and what to do. */
export class FailureError extends EvokeError {
  override readonly kind = "failure"
  readonly what: string
  readonly why: string | undefined
  readonly fix: Fix

  constructor(what: string, why: string | undefined, fix: Fix, command: string) {
    super(`${what}${why === undefined ? "" : `: ${why}`}  →  ${command}`, command)
    this.what = what
    this.why = why
    this.fix = fix
  }
}

/** The CLI's line for one problem. */
function line(problem: Problem): string {
  const where = problem.reflex === undefined ? "" : `${problem.reflex}: `
  return `${where}${problem.message}  →  ${problem.command}`
}
