// @evoke-build/evoke/clef: Clef and Clef-flash, Cloudflare's models on Workers AI, under CLOUDFLARE_API_TOKEN at
// the account CLOUDFLARE_ACCOUNT_ID names. The wire and the transport are ./systemone's; this entry names the two
// doors. Neither ships floors, so a call through one waits for a yes unless `gate` gives `route`, `read` and
// `write`. In: options. Out: an Adapter.

import type { Adapter } from "./adapter.ts"
import { type DoorOptions, through } from "./systemone.ts"

export type ClefOptions = DoorOptions

/** Clef, ready to answer; throws at once when no key or no account is set, the account is no account id, a
 * floor is not a probability, or the proxy named in the environment is no proxy address. */
export function clef(options: ClefOptions = {}): Adapter {
  return through("clef", options)
}

/** Clef-flash, Clef's smaller model, ready to answer; throws as `clef` does. */
export function clefFlash(options: ClefOptions = {}): Adapter {
  return through("clef_flash", options)
}
