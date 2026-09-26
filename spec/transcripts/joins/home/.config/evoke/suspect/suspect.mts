// The three whole results reach the body as their sources returned them: the release comes from the deploys, the
// rate and the count from the errors and the logs.
type Errors = { rate: number; since: string }
type Deploys = { deploys: { release: string; at: string }[] }
type Logs = { count: number }
export default async ({ errors, deploys, logs }: { errors: Errors; deploys: Deploys; logs: Logs }) => {
  const [last] = deploys.deploys
  return {
    text: `${last.release} at ${last.at}, four minutes before ${errors.rate * 100}% errors and ${logs.count} timeouts`,
    data: { release: last.release },
  }
}
