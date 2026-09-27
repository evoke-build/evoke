// The wipe stand-in of the stolen-laptop flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ serial }: { serial: string }) => ({ text: `${serial} wipes at its next check-in`, data: { serial, wiped: true } })
