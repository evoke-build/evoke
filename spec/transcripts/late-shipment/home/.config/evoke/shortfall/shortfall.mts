// The shortfall stand-in of the late-shipment flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const SKUS = ["vx-2210", "vx-2214", "hs-0409"]

const SHORTFALL = Array.from({ length: 38 }, (_, i) => ({
  sku: `${SKUS[i % 3]!}-${String(Math.floor(i / 3) + 1).padStart(2, "0")}`,
  units: 6 + ((i * 7) % 30),
}))

export default async ({ shipment }: { shipment?: { shipment?: string } }) => ({
  text: `${shipment?.shipment ?? "rotterdam"}: ${SHORTFALL.length} product codes short, ${SHORTFALL.reduce((units, item) => units + item.units, 0)} units in all`,
  data: { items: SHORTFALL },
})
