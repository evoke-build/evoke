// The hotel stand-in of the cancelled-flight flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const ROOMS = [
  { hotel: "Hotel Avenida", price: 98 },
  { hotel: "Casa do Rio", price: 126 },
  { hotel: "Palacio Suites", price: 175 },
]

const CHOSEN = (rooms?: { rooms?: { hotel: string }[] }) => rooms?.rooms?.[0]?.hotel ?? ROOMS[0]!.hotel

export default async ({ rooms }: { rooms?: { rooms?: { hotel: string }[] } }) => ({
  text: `booked ${CHOSEN(rooms)} for 1 night`,
  data: { hotel: CHOSEN(rooms), nights: 1 },
})
