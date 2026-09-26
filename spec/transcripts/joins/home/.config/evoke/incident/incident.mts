export default async ({ number }: { number: number }) => ({ text: `incident ${number}: checkout down since 14:02`, data: { number, service: "checkout", opened: "14:02" } })
