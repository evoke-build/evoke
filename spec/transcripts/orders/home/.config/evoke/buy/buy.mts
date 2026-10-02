// The buy stand-in of the orders flow: one order number per product, so the session's results stay the same.
const ORDERS: Record<string, number> = { "OUT-503": 1025, "BOK-603": 1026, "HOM-403": 1027, "KIT-301": 1028, "BOK-602": 1029 }

export default async ({ product, quantity }: { product: string; quantity?: number }) => {
  const order = ORDERS[product] ?? 1030
  return { text: `order ${order} placed: ${quantity ?? 1} of ${product}`, data: { order } }
}
