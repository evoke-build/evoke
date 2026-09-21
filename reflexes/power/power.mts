// Sleep, restart or shut down: `pmset sleepnow` sleeps at once; the login window's own restart and shutdown events,
// sent through AppleScript, ask every app to quit first, as the Apple menu does, and without its dialog — the
// decision was confirmed before this runs.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async ({ action }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  switch (action) {
    case "sleep":
      await exec("pmset", ["sleepnow"], { signal })
      return "sleeping"
    case "restart":
      await exec("osascript", ["-e", 'tell application "loginwindow" to «event aevtrrst»'], { signal })
      return "restarting"
    case "shutdown":
      await exec("osascript", ["-e", 'tell application "loginwindow" to «event aevtrsdn»'], { signal })
      return "shutting down"
  }
}) satisfies Reflex
