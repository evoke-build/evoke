export default async ({ name }: { name: string }) => {
  const email = `${name}@example.com`
  return { text: `${name} <${email}>`, data: { email } }
}
