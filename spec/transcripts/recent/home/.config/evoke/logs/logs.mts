// The logs stand-in of the recent flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ service, region }: { service: string; region?: string }) => ({
  text: `${service}: 412 timeouts calling payments`,
  data: { service, region: region ?? "all", count: 412, top: "timeout calling payments" },
})
