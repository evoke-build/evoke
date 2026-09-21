// Count down, then ring: a detached `osascript` waits out the duration and shows a notification with a sound, so
// this run returns at once and the alarm outlives it. The words reach the script through its environment, never
// through AppleScript source.
import { spawn } from "node:child_process"
import type { ChildProcess } from "node:child_process"
import type { Reflex } from "./reflex.d.ts"

const ALARM = [
  'delay ((system attribute "TIMER_SECONDS") as number)',
  'display notification (system attribute "TIMER_MESSAGE") with title (system attribute "TIMER_TITLE") sound name "Glass"',
]

export default (async ({ duration, label }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const rings = new Date(Date.now() + duration * 1000)
  const env = {
    ...process.env,
    TIMER_SECONDS: String(duration),
    TIMER_TITLE: label ?? "Timer",
    TIMER_MESSAGE: `${describe(duration)} passed`,
  }
  const alarm = await started(spawn("osascript", ALARM.flatMap(line => ["-e", line]), { detached: true, stdio: "ignore", env }))
  alarm.unref()
  const when = rings.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
  const text = `${label === undefined ? "" : `${label}: `}${describe(duration)}, rings at ${when}`
  return { text, data: { rings: rings.toISOString(), pid: alarm.pid } }
}) satisfies Reflex

/** The child once the system has started it, so a program that is not there fails this run instead of crashing it. */
function started<T extends ChildProcess>(child: T): Promise<T> {
  return new Promise((resolve, reject) => {
    child.once("spawn", () => resolve(child))
    child.once("error", reject)
  })
}

/** Seconds as a person says them: `90 seconds`, `10 minutes`, `1 hour 30 minutes`. */
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
