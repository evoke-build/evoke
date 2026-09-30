// A stand-in for the collection's body, which mutes a Mac or moves its output volume: it says what that one says,
// and changes nothing, so the flow runs anywhere.
export default async ({ state, by }: { state: string; by?: number }) =>
  state === "off" ? "muted" : state === "on" ? "sound on" : `volume ${state === "up" ? 40 + (by ?? 10) : 40 - (by ?? 10)}%`
