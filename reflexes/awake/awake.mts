// Keep the laptop awake with `caffeinate`, detached so it outlives this run: `-d` holds the display, `-i` the
// system, `-t` a duration in seconds; without one it runs until it is killed.
import { spawn } from "node:child_process"
import type { ChildProcess } from "node:child_process"
import type { Reflex } from "./reflex.d.ts"

export default (async ({ duration }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const args = duration === undefined ? ["-di"] : ["-di", "-t", String(duration)]
  const keeper = await started(spawn("caffeinate", args, { detached: true, stdio: "ignore" }))
  keeper.unref()
  const text = duration === undefined ? "awake until you stop it: pkill caffeinate" : `awake for ${describe(duration)}`
  return { text, data: { pid: keeper.pid } }
}) satisfies Reflex

/** The child once the system has started it, so a program that is not there fails this run instead of crashing it. */
function started<T extends ChildProcess>(child: T): Promise<T> {
  return new Promise((resolve, reject) => {
    child.once("spawn", () => resolve(child))
    child.once("error", reject)
  })
}

/** Seconds as a person says them: `45 minutes`, `2 hours`, `1 hour 30 minutes`. */
function describe(seconds: number): string {
  const parts: string[] = []
  let left = seconds
  for (const [unit, size] of [["hour", 3600], ["minute", 60], ["second", 1]] as const) {
    const count = Math.floor(left / size)
    left -= count * size
    if (count > 0) parts.push(`${count} ${unit}${count === 1 ? "" : "s"}`)
  }
  return parts.join(" ") || "0 seconds"
}
