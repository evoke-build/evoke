// The logs stand-in: fixed data for the service it was handed.

export default async ({ service, region }: { service: string; region?: string }) => ({
  text: `${service}: 412 timeouts calling payments`,
  data: { service, region: region ?? "all", count: 412, top: "timeout calling payments" },
})
