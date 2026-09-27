// The book stand-in of the cancelled-flight flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async ({ flight }: { flight: string }) => ({ text: `booked ${flight}, seat 14C`, data: { flight, booked: true } })
