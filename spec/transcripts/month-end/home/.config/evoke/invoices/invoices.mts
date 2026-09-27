// The invoices stand-in of the month-end flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ month }: { month: string }) => ({ text: `${month}: 38 invoices, 35 paid`, data: { month, issued: 38, paid: 35 } })
