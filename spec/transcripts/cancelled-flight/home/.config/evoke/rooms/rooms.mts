// The rooms stand-in of the cancelled-flight flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const ROOMS = [
  { hotel: "Hotel Avenida", price: 98 },
  { hotel: "Casa do Rio", price: 126 },
  { hotel: "Palacio Suites", price: 175 },
]

export default async ({ city }: { city: string }) => ({
  text: `${city}: ${ROOMS.length} rooms tonight, from ${ROOMS[0]!.price} a night at ${ROOMS[0]!.hotel}`,
  data: { city, rooms: ROOMS },
})
