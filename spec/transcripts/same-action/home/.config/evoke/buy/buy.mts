export default async ({ product, quantity }: { product: string; quantity?: number }) => `bought ${quantity ?? 1} of ${product}`
