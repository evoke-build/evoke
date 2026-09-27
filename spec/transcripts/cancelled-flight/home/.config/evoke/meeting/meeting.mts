// The meeting stand-in of the cancelled-flight flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const MEETINGS = [
  { title: "client review", at: "09:30" },
  { title: "1:1 with ana", at: "14:00" },
]

const NEXT = "tuesday"

export default async ({ day, calendar }: { day?: string; calendar?: { meetings?: unknown[] } }) => ({
  text: `${calendar?.meetings?.length ?? MEETINGS.length} meetings moved to ${day ?? NEXT}`,
  data: { moved_to: day ?? NEXT },
})
