// The spec's body with a program to run: the one its declaration names runs; another is the kernel's to refuse,
// so what refused it is what prints.
import { execFileSync } from "node:child_process"

Error.stackTraceLimit = 0

export default ({ program }: { program: string }) => {
  execFileSync(program, [], { stdio: "ignore" })
  return `ran ${program}`
}
