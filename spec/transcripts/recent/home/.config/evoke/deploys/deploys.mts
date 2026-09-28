// The deploys stand-in of the recent flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const DEPLOYS = [
  { release: "4.12.0", at: "13:58", by: "sam" },
  { release: "4.11.3", at: "09:12", by: "ana" },
]

export default async ({ service }: { service: string }) => ({
  text: `${service}: ${DEPLOYS.length} deploys today, the last ${DEPLOYS[0]!.release} at ${DEPLOYS[0]!.at}`,
  data: { service, deploys: DEPLOYS },
})
