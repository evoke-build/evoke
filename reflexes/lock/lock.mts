// Lock the screen: the system's own lock-screen app, opened; everything keeps running behind it, and no permission
// is asked, as sending the keystroke of the Apple menu's item would.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

const LOCK_SCREEN = "/System/Library/CoreServices/RemoteManagement/AppleVNCServer.bundle/Contents/Support/LockScreen.app"

export default (async (_, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  try {
    await exec("open", [LOCK_SCREEN], { signal })
  } catch (error) {
    if (signal.aborted) throw signal.reason
    throw new Error(said(error) ?? "the lock screen did not open")
  }
  return "locked"
}) satisfies Reflex

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}
