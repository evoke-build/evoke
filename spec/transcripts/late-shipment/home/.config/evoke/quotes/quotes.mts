// The quotes stand-in of the late-shipment flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const QUOTES = [
  { supplier: "nordic", sku: "hs-0409", price: 18.4 },
  { supplier: "baltic", sku: "hs-0409", price: 21.9 },
]

export default async ({ shipment }: { shipment: string }) => ({
  text: `${shipment}: ${QUOTES[0]!.sku} at ${QUOTES.map(quote => `${quote.price.toFixed(2)} from ${quote.supplier}`).join(", ")}`,
  data: { shipment, quotes: QUOTES },
})
