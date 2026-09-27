// The flights stand-in of the cancelled-flight flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const FLIGHTS = [
  { flight: "TP1043", departs: "18:40" },
  { flight: "IB3107", departs: "20:15" },
  { flight: "TP1051", departs: "22:05" },
]

export default async ({ to }: { to: string }) => ({
  text: `${to}: ${FLIGHTS.length} flights tonight, the best ${FLIGHTS[0]!.flight} at ${FLIGHTS[0]!.departs}`,
  data: { to, options: FLIGHTS, flight: FLIGHTS[0]!.flight },
})
