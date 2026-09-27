// The sessions stand-in of the stolen-laptop flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const SESSIONS = [
  { app: "mail", since: "06:12" },
  { app: "chat", since: "06:15" },
  { app: "vpn", since: "07:02" },
]

export default async ({ person }: { person: string }) => ({
  text: `${person}: ${SESSIONS.length} sessions, ${SESSIONS.map(s => s.app).join(", ")}`,
  data: { person, sessions: SESSIONS },
})
