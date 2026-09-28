// The incidents stand-in of the recent flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const INCIDENTS = [
  { number: 311,  },
  { number: 312,  },
  { number: 313,  },
]

export default async () => ({ text: `${INCIDENTS.length} open: ${INCIDENTS.map(i => i.number).join(", ")}`, data: { incidents: INCIDENTS } })
