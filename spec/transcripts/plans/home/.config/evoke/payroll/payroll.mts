// The payroll stand-in of the plans flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ month }: { month: string }) => ({ text: `${month}: payroll 48 300.00`, data: { month, total: 48300 } })
