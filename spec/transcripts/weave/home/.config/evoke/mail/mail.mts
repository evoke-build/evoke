export default async ({ to, subject }: { to: string; subject?: string }) =>
  `drafted to ${to}${subject === undefined ? "" : ` about ${subject}`}`
