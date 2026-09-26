export default async ({ service }: { service: string }) => ({ text: `${service}: 8.4% errors since 14:02`, data: { service, rate: 0.084, since: "14:02" } })
