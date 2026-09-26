export default async ({ service }: { service: string }) => ({ text: `${service}: 412 timeouts calling payments`, data: { service, count: 412, top: "timeout calling payments" } })
