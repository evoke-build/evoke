export default async ({ product, quantity }: { product: string; quantity?: number }) => `in your cart: ${quantity ?? 1} of ${product}`
