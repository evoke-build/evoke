// Set the output volume through AppleScript: `set volume output volume <0–100>`.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async ({ level }, { signal }) => {
  const percent = Math.round(level)
  // The one value written into AppleScript source: an integer, so it can only ever be digits.
  if (!Number.isInteger(percent)) throw new Error(`level ${level} is not a number`)
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  try {
    await exec("osascript", ["-e", `set volume output volume ${percent}`], { signal })
  } catch (error) {
    if (signal.aborted) throw signal.reason
    throw new Error(said(error) ?? "the volume did not change")
  }
  return `volume ${percent}%`
}) satisfies Reflex

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}
