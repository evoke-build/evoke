// The rollback stand-in of the outage flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ service, release }: { service?: string; release: string }) => ({ text: `${service ?? "checkout"} rolled back from ${release} to 4.11.3`, data: { release: "4.11.3" } })
