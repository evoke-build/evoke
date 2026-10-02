// The errors stand-in of the whole-sentence flow: prints what it was handed and returns fixed data.

export default async ({ service, region }: { service: string; region?: string }) => ({
  text: `${service}: 8.4% errors since 14:02`,
  data: { service, region: region ?? "all", rate: 0.084, since: "14:02" },
})
