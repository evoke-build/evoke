// The reconcile stand-in of the month-end flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async () => ({ text: "3 transactions to review", data: { unmatched: 3 } })
