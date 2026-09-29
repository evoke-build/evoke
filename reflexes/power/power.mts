// Restart or shut down: the login window's own restart and shutdown events, sent through AppleScript, ask every
// app to quit first, as the Apple menu does, and without its dialog — the decision was confirmed before this runs.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async ({ action }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const send = async (event: string, what: string) => {
    try {
      await exec("osascript", ["-e", `tell application "loginwindow" to «event ${event}»`], { signal })
    } catch (error) {
      if (signal.aborted) throw signal.reason
      throw new Error(said(error) ?? `${what} was refused`)
    }
  }
  switch (action) {
    case "restart":
      await send("aevtrrst", "the restart")
      return "restarting"
    case "shutdown":
      await send("aevtrsdn", "the shutdown")
      return "shutting down"
  }
}) satisfies Reflex

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}
