// Sleep: `pmset sleepnow` sleeps at once; the lid or a key wakes the laptop, and everything open stays as it was.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async (_, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  try {
    await exec("pmset", ["sleepnow"], { signal })
  } catch (error) {
    if (signal.aborted) throw signal.reason
    throw new Error(said(error) ?? "sleep was refused")
  }
  return "sleeping"
}) satisfies Reflex

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}
