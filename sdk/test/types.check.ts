// Compile-time checks of the typed layer, run by tsc under npm run check: what a reflex handed as code infers,
// what an ask takes back, what run refuses. A line marked @ts-expect-error must fail to compile.

import type { Ask, Confirm, Decision, Given, Run } from "../src/decision.ts"
import type { Args, InlineManifest, ReflexesOf } from "../src/reflex.ts"
import { load, reflex } from "../src/index.ts"

const lights = reflex(
  {
    description: "Lights.",
    effect: "write",
    confirm: "Set the {room} lights {state}?",
    args: {
      room: { ask: "Which room?", vocab: "rooms" },
      state: { ask: "What?", options: { on: "On.", off: "Off.", dim: "Dim." } },
      brightness: { ask: "How bright?", pick: "number", range: [1, 100], optional: true },
      loud: { ask: "Loud?", flag: true },
    },
  },
  async ({ room, state, brightness, loud }) => {
    const r: string = room
    const s: "on" | "off" | "dim" = state
    const b: number | undefined = brightness
    const l: true | undefined = loud
    return `${r} ${s} ${b ?? ""} ${l ?? ""}`
  },
)

type Shape = ReflexesOf<{ lights: typeof lights }>["lights"]
const shape: Shape = { room: { type: "word", word: "den" }, state: { type: "option", key: "off" } }
// @ts-expect-error a key the options lack
const wrongKey: Shape = { room: { type: "word", word: "den" }, state: { type: "option", key: "up" } }
void shape
void wrongKey

/** A manifest typed by the interface, not a literal, still gives its body readable arguments. */
const wide: InlineManifest = lights.manifest
reflex(wide, async args => String(args.room))
type Wide = Args<typeof wide>
const wideArgs: Wide = { room: "den", n: 3, f: true }
void wideArgs

async function typed(): Promise<void> {
  const project = await load({ reflexes: { lights }, adapter: { id: "x", answer: async () => ({}) } })
  const d = await project.decide("x")
  if (d.outcome === "run") {
    const name: "lights" = d.reflex
    const state: "on" | "off" | "dim" = d.values.state
    const brightness: number | undefined = d.values.brightness
    void name
    void state
    void brightness
    await project.run(d)
  }
  if (d.outcome === "ask") {
    project.fill(d, { room: "den", state: "off", brightness: "50" })
    // @ts-expect-error a flag is never asked
    project.fill(d, { loud: true })
    // @ts-expect-error a key the options lack
    project.fill(d, { state: "up" })
    // @ts-expect-error an ask does not run
    await project.run(d)
  }
  if (d.outcome === "confirm") {
    // @ts-expect-error a confirm runs only confirmed
    await project.run(d)
    await project.run(d, { confirmed: true })
  }
  if (d.outcome === "abstain") {
    // @ts-expect-error an abstain does not run
    await project.run(d)
  }
  const both = await load<{ timer: { duration: { type: "pick"; span: never; value: { type: "duration"; value: number } } } } & ReflexesOf<{ lights: typeof lights }>>({
    root: ".",
    reflexes: { lights },
  })
  void both
  // A root stops inference: the installed reflexes decide too, so R is what was written.
  const loose = await load({ root: ".", reflexes: { lights } })
  const anyDecision: Decision = await loose.decide("x")
  void anyDecision
}
void typed

type G = Given<Ask<ReflexesOf<{ lights: typeof lights }>>>
const given: G = { room: "den", state: "on", brightness: "3" }
void given
type R = Run<ReflexesOf<{ lights: typeof lights }>>
type C = Confirm<ReflexesOf<{ lights: typeof lights }>>
const runOrConfirm = (d: R | C): string => d.values.room
void runOrConfirm
