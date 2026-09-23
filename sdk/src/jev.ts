// @evoke-build/evoke/jev: the first adapter, Jev at TypeSafe AI's own address, under TYPESAFE_API_KEY. The wire
// and the transport are ./systemone's; this entry names the door. In: options. Out: an Adapter.

import type { Adapter } from "./adapter.ts"
import { type DoorOptions, through } from "./systemone.ts"

export type JevOptions = DoorOptions

/** Jev, ready to answer; throws at once when no key is set, an override is not a probability, or the proxy named
 * in the environment is no proxy address. */
export function jev(options: JevOptions = {}): Adapter {
  return through("jev", options)
}
