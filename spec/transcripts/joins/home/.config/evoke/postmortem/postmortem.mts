// The incident record reaches the body whole: its number names the file.
export default async ({ record }: { record: { number: number } }) => ({
  text: `drafted the postmortem of incident ${record.number}`,
  data: { path: `~/postmortems/${record.number}.md` },
})
