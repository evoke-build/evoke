// The reserve stand-in of the late-shipment flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ sku }: { sku: string }) => ({ text: `${sku} reserved`, data: { sku, reserved: true } })
