// @evoke-build/evoke/openjev: the same model through OpenJEV, an independent service that forwards requests to
// Jev, under OPENJEV_API_KEY. The wire and the transport are ./systemone's; this entry names the door. In:
// options. Out: an Adapter.

import type { Adapter } from "./adapter.ts"
import { type DoorOptions, through } from "./systemone.ts"

export type OpenJevOptions = DoorOptions

/** Jev through OpenJEV, ready to answer; throws at once when no key is set, an override is not a probability, or
 * the proxy named in the environment is no proxy address. */
export function openjev(options: OpenJevOptions = {}): Adapter {
  return through("openjev", options)
}
