// Start an email in the default mail app: a `mailto:` URL opened, with the subject when one was given; nothing is
// sent.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async ({ to, subject }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const url = subject === undefined ? `mailto:${to}` : `mailto:${to}?subject=${encodeURIComponent(subject)}`
  try {
    await exec("open", [url], { signal })
  } catch (error) {
    if (signal.aborted) throw signal.reason
    throw new Error(said(error) ?? "the mail app did not open")
  }
  return subject === undefined ? `new mail to ${to}` : `new mail to ${to} about "${subject}"`
}) satisfies Reflex

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}
