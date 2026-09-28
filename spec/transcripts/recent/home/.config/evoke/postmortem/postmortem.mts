// The postmortem stand-in of the recent flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ incident }: { incident?: { number?: number } }) => ({ text: `drafted ~/postmortems/${incident?.number ?? "unknown"}.md`, data: { path: `~/postmortems/${incident?.number ?? "unknown"}.md` } })
