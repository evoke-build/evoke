export default async ({ order }: { order: number }) => ({ text: `order ${order} paid`, data: { order } })
