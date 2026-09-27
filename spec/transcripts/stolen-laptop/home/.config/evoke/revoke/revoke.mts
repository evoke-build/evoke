// The revoke stand-in of the stolen-laptop flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const SESSIONS = [
  { app: "mail", since: "06:12" },
  { app: "chat", since: "06:15" },
  { app: "vpn", since: "07:02" },
]

export default async ({ sessions }: { sessions?: { sessions?: unknown[] } }) => {
  const revoked = sessions?.sessions?.length ?? SESSIONS.length
  return { text: `${revoked} sessions revoked`, data: { revoked } }
}
