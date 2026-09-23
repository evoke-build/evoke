export default async ({ to, cc, subject }: { to: string; cc?: string; subject?: string }) =>
  `drafted to ${to}${cc === undefined ? "" : `, copy ${cc}`}${subject === undefined ? "" : ` about ${subject}`}`
