import { writeFileSync } from "node:fs"

// For the spec: the loader's process noted under the home, so a flow can check that the group is gone; then a
// minute's sleep, whatever the signal says — the grace ends the body, not the body itself.
export default async () => {
  writeFileSync(`${process.env.HOME}/wait.pid`, `${process.pid}\n`)
  console.log("waiting")
  await new Promise(resolve => setTimeout(resolve, 60_000))
  return "waited"
}
