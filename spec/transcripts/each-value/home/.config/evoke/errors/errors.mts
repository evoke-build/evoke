// The errors stand-in: fixed data for the service and the region it was handed.

export default async ({ service, region }: { service: string; region?: string }) => ({
  text: `${service}${region === undefined ? "" : ` in ${region}`}: 8.4% errors since 14:02`,
  data: { service, region: region ?? "all", rate: 0.084, since: "14:02" },
})
