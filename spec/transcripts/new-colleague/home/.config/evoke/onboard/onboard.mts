// The onboard stand-in of the new-colleague flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const SESSIONS = [
  { what: "welcome with the team", at: "monday 10:00" },
  { what: "laptop, accounts and access", at: "monday 14:00" },
  { what: "first check-in with the lead", at: "friday 15:00" },
]

export default async ({ hire, team }: { hire?: { person?: string }; team?: { team?: string } }) => ({
  text: `booked ${SESSIONS.length} sessions for ${hire?.person ?? "the hire"} with ${team?.team ?? "the team"}`,
  data: { booked: SESSIONS },
})
