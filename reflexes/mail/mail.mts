// Start an email in the default mail app: a `mailto:` URL opened, with the subject when one was given; nothing is
// sent.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async ({ to, subject }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const url = subject === undefined ? `mailto:${to}` : `mailto:${to}?subject=${encodeURIComponent(subject)}`
  await exec("open", [url], { signal })
  return subject === undefined ? `new mail to ${to}` : `new mail to ${to} about "${subject}"`
}) satisfies Reflex
