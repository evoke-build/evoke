export default async ({ duration }: { duration: number }) => {
  if (duration <= 0) throw "nothing to count down"
  return `${duration / 60} minute timer started`
}
