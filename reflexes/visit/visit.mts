// Open a site in the browser: `open <url>`, the word's value being the URL. A private window goes through the
// default browser's own flag — the browser LaunchServices names for `https`, Safari when it names none; Safari
// opens no private window from a script, so the flag needs one of the others.
import { execFile } from "node:child_process"
import { existsSync } from "node:fs"
import { homedir } from "node:os"
import { join } from "node:path"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

/** The flag each browser takes for a private window, by bundle identifier. */
const PRIVATE: Record<string, string> = {
  "com.google.chrome": "--incognito",
  "com.brave.browser": "--incognito",
  "com.vivaldi.vivaldi": "--incognito",
  "com.microsoft.edgemac": "--inprivate",
  "org.mozilla.firefox": "--private-window",
}

const HANDLERS = join(homedir(), "Library/Preferences/com.apple.LaunchServices/com.apple.launchservices.secure.plist")

export default (async ({ site, incognito }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  if (!URL.canParse(site) || !/^https?:$/.test(new URL(site).protocol)) {
    throw new Error(`"${site}" is not a URL; give the word one: evoke vocab sites add <word> "<meaning>" --value <url>`)
  }
  const opened = async (args: string[]) => {
    try {
      await exec("open", args, { signal })
    } catch (error) {
      if (signal.aborted) throw signal.reason
      throw new Error(said(error) ?? `${site} did not open`)
    }
  }
  if (!incognito) {
    await opened([site])
    return `opened ${site}`
  }
  const browser = await defaultBrowser(signal)
  const flag = PRIVATE[browser]
  if (flag === undefined) {
    throw new Error(`${browser} opens no private window from a script; make Chrome, Brave, Vivaldi, Edge or Firefox the default browser`)
  }
  await opened(["-n", "-b", browser, "--args", flag, site])
  return `opened ${site} in a private window`
}) satisfies Reflex

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}

/** The bundle identifier LaunchServices hands `https` to; Safari when it names none. */
async function defaultBrowser(signal: AbortSignal): Promise<string> {
  if (!existsSync(HANDLERS)) return "com.apple.safari"
  const { stdout } = await exec("plutil", ["-convert", "json", "-o", "-", HANDLERS], { signal })
  const { LSHandlers = [] } = JSON.parse(stdout) as { LSHandlers?: { LSHandlerURLScheme?: string; LSHandlerRoleAll?: string }[] }
  return LSHandlers.find(handler => handler.LSHandlerURLScheme === "https")?.LSHandlerRoleAll ?? "com.apple.safari"
}
