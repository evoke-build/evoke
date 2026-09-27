// The cards stand-in of the plans flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ month }: { month: string }) => ({ text: `${month}: 57 card charges, 2 without a receipt`, data: { month, charges: 57, missing: 2 } })
