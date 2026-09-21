// Lock the screen: the system's own lock-screen app, opened; everything keeps running behind it, and no permission
// is asked, as sending the keystroke of the Apple menu's item would.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

const LOCK_SCREEN = "/System/Library/CoreServices/RemoteManagement/AppleVNCServer.bundle/Contents/Support/LockScreen.app"

export default (async (_, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  await exec("open", [LOCK_SCREEN], { signal })
  return "locked"
}) satisfies Reflex
