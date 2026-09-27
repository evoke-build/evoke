// The rotate stand-in of the stolen-laptop flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

const KEYS = [
  { name: "deploy", created: "2026-03-04" },
  { name: "ci", created: "2025-11-19" },
]

export default async ({ keys }: { keys?: { keys?: unknown[] } }) => {
  const rotated = keys?.keys?.length ?? KEYS.length
  return { text: `${rotated} keys rotated`, data: { rotated } }
}
