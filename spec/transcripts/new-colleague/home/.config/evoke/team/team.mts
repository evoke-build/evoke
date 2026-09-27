// The team stand-in of the new-colleague flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const TEAMS = [
  { team: "platform", members: ["jo", "leila", "tomas"] },
  { team: "sales", members: ["omar", "priya", "nils"] },
]

const MEMBERS = (team: string) => TEAMS.find(t => t.team === team)?.members ?? TEAMS[0]!.members

export default async ({ team }: { team: string }) => ({
  text: `${team}: ${MEMBERS(team).join(", ")}`,
  data: { team, members: MEMBERS(team) },
})
