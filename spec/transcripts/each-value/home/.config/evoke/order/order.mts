// The order stand-in: fixed data for the person it was handed, an order number each.

const HIRES = [
  { person: "maria", role: "backend engineer", starts: "monday 5 october", order: "PO-20461" },
  { person: "sam", role: "account executive", starts: "monday 12 october", order: "PO-20462" },
  { person: "ana", role: "product designer", starts: "monday 19 october", order: "PO-20463" },
]

const HIRE = (person: string) => HIRES.find(h => h.person === person) ?? HIRES[0]!

export default async ({ person, model }: { person: string; model?: string }) => {
  const { starts, order } = HIRE(person)
  return {
    text: `${model ?? "air"} ordered for ${person}, ${order}, at the desk by ${starts}`,
    data: { person, model: model ?? "air", ordered: true, order },
  }
}
