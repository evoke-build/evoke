// Turn Wi-Fi on or off through `networksetup`, on the device the hardware port named Wi-Fi has — `en0` on a
// laptop, often not on a desktop with Ethernet — never one assumed.
import { execFile } from "node:child_process"
import { promisify } from "node:util"
import type { Reflex } from "./reflex.d.ts"

const exec = promisify(execFile)

export default (async ({ state }, { signal }) => {
  if (process.platform !== "darwin") throw new Error("runs on macOS only")
  const device = await wifiDevice(signal)
  try {
    await exec("networksetup", ["-setairportpower", device, state], { signal })
  } catch (error) {
    if (signal.aborted) throw signal.reason
    throw new Error(said(error) ?? `networksetup could not turn Wi-Fi ${state}`)
  }
  return `wi-fi ${state}`
}) satisfies Reflex

/** The device of the hardware port named Wi-Fi, or AirPort on an older system, as `networksetup` lists them. */
async function wifiDevice(signal: AbortSignal): Promise<string> {
  const { stdout } = await exec("networksetup", ["-listallhardwareports"], { signal })
  const port = stdout.split(/\n\s*\n/).find(block => /^Hardware Port: (Wi-Fi|AirPort)$/m.test(block))
  const device = port?.match(/^Device: (\S+)$/m)?.[1]
  if (device === undefined) throw new Error("this Mac has no Wi-Fi device")
  return device
}

/** What a failed command said on stderr, when it said anything; else nothing, and the caller's own words stand. */
function said(error: unknown): string | undefined {
  const text = typeof error === "object" && error !== null && "stderr" in error ? error.stderr : undefined
  return typeof text === "string" && text.trim() !== "" ? text.trim() : undefined
}
