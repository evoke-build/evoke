// The alert stand-in of the stolen-laptop flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ channel }: { channel: string }) => ({ text: `alerted ${channel}`, data: { channel } })
