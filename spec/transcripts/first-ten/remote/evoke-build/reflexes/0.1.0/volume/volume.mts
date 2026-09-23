// A stand-in for the collection's body, which sets a Mac's output volume: it says what that one says, and changes
// nothing, so the flow runs anywhere.
export default async ({ level }: { level: number }) => `volume ${Math.round(level)}%`
