// The calendar stand-in of the cancelled-flight flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const MEETINGS = [
  { title: "client review", at: "09:30" },
  { title: "1:1 with ana", at: "14:00" },
]

export default async ({ day }: { day: string }) => ({
  text: `${day}: ${MEETINGS.length} meetings, ${MEETINGS.map(m => `${m.title} at ${m.at}`).join(" and ")}`,
  data: { day, meetings: MEETINGS },
})
