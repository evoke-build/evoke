// The status stand-in of the outage flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ component, state }: { component: string; state?: string }) => ({ text: `${component}: ${state ?? "monitoring"} on the status page`, data: { component } })
