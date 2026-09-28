// The shipment stand-in of the late-shipment flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const SKUS = ["VX-2210", "VX-2214", "HS-0409"]

const CARRIER = { eta: "thursday 06:00", late_days: 4 }

export default async ({ shipment }: { shipment: string }) => ({
  text: `${shipment}: lands ${CARRIER.eta}, ${CARRIER.late_days} days late, carrying ${SKUS.join(", ")}`,
  data: { shipment, eta: CARRIER.eta, late_days: CARRIER.late_days, skus: SKUS },
})
