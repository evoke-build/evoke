// The buy stand-in of the late-shipment flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ supplier }: { supplier: string }) => ({ text: `ordered from ${supplier}`, data: { supplier, ordered: true } })
