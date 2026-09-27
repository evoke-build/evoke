// The ledger stand-in of the plans flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ month }: { month: string }) => ({ text: `${month} posted to the ledger`, data: { month } })
