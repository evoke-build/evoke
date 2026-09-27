// The send stand-in of the month-end flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ to }: { to: string }) => ({ text: `sent to ${to}`, data: { to } })
