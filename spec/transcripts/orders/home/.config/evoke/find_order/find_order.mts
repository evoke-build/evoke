export default async ({ order }: { order?: number }) => {
  const found = order ?? 1024
  return { text: `order ${found}: placed, unpaid`, data: { order: found } }
}
