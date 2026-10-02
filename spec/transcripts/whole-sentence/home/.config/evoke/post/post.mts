// The post stand-in of the whole-sentence flow: prints what it was handed and returns fixed data.

export default async ({ channel }: { channel: string }) => ({ text: `posted to ${channel}`, data: { channel } })
