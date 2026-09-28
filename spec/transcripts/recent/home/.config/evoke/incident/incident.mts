// The incident stand-in of the recent flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const INCIDENTS = [
  { number: 311,  },
  { number: 312,  },
  { number: 313,  },
]

export default async ({ number }: { number: number }) => ({
  text: `incident ${number}: ${INCIDENTS.find(i => i.number === number)?.service ?? "checkout"} down since 14:02`,
  data: { number, service: INCIDENTS.find(i => i.number === number)?.service ?? "checkout", opened: "14:02" },
})
