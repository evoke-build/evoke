// The stock stand-in of the late-shipment flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const LEVELS = [
  { sku: "vx-2210", units: 40 },
  { sku: "vx-2214", units: 12 },
  { sku: "hs-0409", units: 0 },
]

export default async ({ shipment }: { shipment: string }) => ({
  text: `${shipment}: ${LEVELS.map(level => `${level.units} of ${level.sku}`).join(", ")} in the warehouse`,
  data: { shipment, levels: LEVELS },
})
