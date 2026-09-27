// The bank stand-in of the plans flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ month }: { month: string }) => ({ text: `${month}: 214 transactions, 18 204.55 at the close`, data: { month, transactions: 214, balance: 18204.55 } })
