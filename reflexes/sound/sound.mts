// Mute, unmute, or move the output volume through AppleScript: `set volume output muted true`, and
// `set volume output volume <0–100>` from the level `get volume settings` reports.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

/** How far up or down goes when the sentence names no amount. */
const STEP = 10

export default (async ({ state, by }, { signal }) => {
  const points = by === undefined ? STEP : Math.round(by)
  // The one value written into AppleScript source: an integer, so it can only ever be digits.
  if (!Number.isInteger(points)) throw new Error(`by ${by} is not a number`)
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  try {
    if (state === "off" || state === "on") {
      await exec("osascript", ["-e", `set volume output muted ${state === "off"}`], { signal })
      return state === "off" ? "muted" : "sound on"
    }
    const { stdout } = await exec("osascript", ["-e", "output volume of (get volume settings)"], { signal })
    const level = Number.parseInt(stdout.trim(), 10)
    if (!Number.isInteger(level)) throw new Error("the volume could not be read")
    const moved = Math.min(100, Math.max(0, state === "up" ? level + points : level - points))
    await exec("osascript", ["-e", `set volume output volume ${moved}`], { signal })
    return `volume ${moved}%`
  } catch (error) {
    if (signal.aborted) throw signal.reason
    throw new Error(said(error) ?? "the sound did not change")
  }
}) satisfies Reflex

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}
