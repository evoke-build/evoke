// Sleep, restart or shut down: `pmset sleepnow` sleeps at once; the login window's own restart and shutdown events,
// sent through AppleScript, ask every app to quit first, as the Apple menu does, and without its dialog — the
// decision was confirmed before this runs.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async ({ action }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const run = async (program: string, args: string[], what: string) => {
    try {
      await exec(program, args, { signal })
    } catch (error) {
      if (signal.aborted) throw signal.reason
      throw new Error(said(error) ?? `${what} was refused`)
    }
  }
  switch (action) {
    case "sleep":
      await run("pmset", ["sleepnow"], "sleep")
      return "sleeping"
    case "restart":
      await run("osascript", ["-e", 'tell application "loginwindow" to «event aevtrrst»'], "the restart")
      return "restarting"
    case "shutdown":
      await run("osascript", ["-e", 'tell application "loginwindow" to «event aevtrsdn»'], "the shutdown")
      return "shutting down"
  }
}) satisfies Reflex

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}
