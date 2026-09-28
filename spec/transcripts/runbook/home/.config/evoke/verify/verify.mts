// The verify stand-in of the runbook flow, as R5's fixture wrote it: prints what it was handed and returns fixed data.

export default async (_: Record<string, never>, context?: { config?: { landing?: string } }) => {
  const landing = context?.config?.landing ?? "yes"
  const text = landing === "yes" ? "writes landing on the new primary" : landing === "no" ? "writes not landing on the new primary" : `writes ${landing} landing on the new primary`
  return { text, data: { landing } }
}
