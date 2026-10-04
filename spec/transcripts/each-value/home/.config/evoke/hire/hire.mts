// The hire stand-in: fixed data for the person it was handed.

const HIRES = [
  { person: "maria", role: "backend engineer", starts: "monday 5 october" },
  { person: "sam", role: "account executive", starts: "monday 12 october" },
  { person: "ana", role: "product designer", starts: "monday 19 october" },
]

const HIRE = (person: string) => HIRES.find(h => h.person === person) ?? HIRES[0]!

export default async ({ person }: { person: string }) => {
  const { role, starts } = HIRE(person)
  return { text: `${person}: ${role}, starts ${starts}`, data: { person, role, starts } }
}
