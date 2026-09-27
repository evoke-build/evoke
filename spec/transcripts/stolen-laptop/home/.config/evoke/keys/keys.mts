// The keys stand-in of the stolen-laptop flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const KEYS = [
  { name: "deploy", created: "2026-03-04" },
  { name: "ci", created: "2025-11-19" },
]

export default async ({ person }: { person: string }) => ({
  text: `${person}: ${KEYS.length} keys, ${KEYS.map(k => k.name).join(", ")}`,
  data: { person, keys: KEYS },
})
