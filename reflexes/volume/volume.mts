// Set the output volume through AppleScript: `set volume output volume <0–100>`.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async ({ level }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const percent = Math.round(level)
  await exec("osascript", ["-e", `set volume output volume ${percent}`], { signal })
  return `volume ${percent}%`
}) satisfies Reflex
