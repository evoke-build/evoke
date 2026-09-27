// The order stand-in of the new-colleague flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const HIRES = [
  { person: "maria", role: "backend engineer", starts: "monday 5 october" },
  { person: "sam", role: "account executive", starts: "monday 12 october" },
  { person: "ana", role: "product designer", starts: "monday 19 october" },
]

const HIRE = (person: string) => HIRES.find(h => h.person === person) ?? HIRES[0]!

export default async ({ person, model }: { person: string; model?: string }) => ({
  text: `${model ?? "air"} ordered for ${person}, PO-20461, at the desk by ${HIRE(person).starts}`,
  data: { person, model: model ?? "air", ordered: true },
})
