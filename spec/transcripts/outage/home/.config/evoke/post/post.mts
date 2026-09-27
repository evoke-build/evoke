// The post stand-in of the outage flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ channel }: { channel: string }) => ({ text: `posted to ${channel}`, data: { channel } })
