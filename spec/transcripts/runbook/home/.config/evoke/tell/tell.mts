// The tell stand-in of the runbook flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ channel }: { channel: string }) => ({ text: `told ${channel}`, data: { channel } })
