// Capture with `screencapture`, silently (`-x`): the whole screen, or a window (`-w`) or a selection (`-s`) picked
// interactively (`-i`); to a PNG on the desktop named as macOS names its own, or to the clipboard (`-c`).
import { execFile } from "node:child_process"
import { homedir } from "node:os"
import { join } from "node:path"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

const MODE = { screen: [], window: ["-i", "-w"], selection: ["-i", "-s"] }

export default (async ({ area = "screen", clipboard }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const file = join(homedir(), "Desktop", `Screenshot ${stamp(new Date())}.png`)
  try {
    await exec("screencapture", ["-x", ...MODE[area], ...(clipboard ? ["-c"] : [file])], { signal })
  } catch (error) {
    if (signal.aborted) throw signal.reason
    throw new Error(stderr(error) ?? "screenshot cancelled")
  }
  return clipboard ? "copied to the clipboard" : `saved ${shown(file)}`
}) satisfies Reflex

/** `2026-09-20 at 10.31.05`: the moment, as macOS names a screenshot. */
function stamp(date: Date): string {
  const two = (n: number) => String(n).padStart(2, "0")
  const day = `${date.getFullYear()}-${two(date.getMonth() + 1)}-${two(date.getDate())}`
  return `${day} at ${two(date.getHours())}.${two(date.getMinutes())}.${two(date.getSeconds())}`
}

/** What a failed command said on stderr, when it said anything. */
function stderr(error: unknown): string | undefined {
  const said = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof said === "string" && said.trim() !== "" ? said.trim() : undefined
}

/** A path under your home as `~/…`. */
function shown(path: string): string {
  const home = homedir()
  return path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path
}
