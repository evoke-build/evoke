// One sentence, several reflexes, through the SDK: the weave flow's own home — lights, timer, volume, a contact
// that yields an address, a mail that takes one — over its recording. `steps` reads the plan; `weave` runs it
// under the same handlers `handle` takes: a result threaded into a later step, a question asked before anything
// runs, a reference that takes nothing confirmed, a refusal, a decline that ends the weave after its stage, and a
// signal mid-run over the cancel flow's recording — the body ended, the rest cancelled, the record on the reason.

import { deepStrictEqual, equal, ok, rejects } from "node:assert/strict"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

import { status } from "../src/contain.ts"
import { type Woven, load, reflex } from "../src/index.ts"
import { replay } from "../src/testing.ts"

const home = fileURLToPath(new URL("../../spec/transcripts/weave/home/.config/evoke", import.meta.url))
const answers = new URL("../../spec/transcripts/weave/answers.toml", import.meta.url)

test("steps reads a sentence into its steps, each decided as decide decides one", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const plan = await project.steps("kill the lights in the den and start a 10 minute timer")
  deepStrictEqual(plan.steps.map(step => [step.n, step.text, step.reflex, step.decision.outcome]), [
    [1, "kill the lights in the den", "lights", "run"],
    [2, "start a 10 minute timer", "timer", "run"],
  ])
  deepStrictEqual(plan.splits?.map(split => [split.word, split.p]), [["and", 0.95]])
  deepStrictEqual(plan.stages, [[1], [2]])
  equal(plan.exclusive, true)
  deepStrictEqual(plan.verdict, { outcome: "run" })
})

test("weave runs the steps in the words' order, each round its own decision and result", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const woven = await project.weave("kill the lights in the den and start a 10 minute timer")
  equal(woven.status, "ran")
  deepStrictEqual(
    woven.steps.map(step => [step.step, step.status, step.rounds.map(round => [round.input, round.decision.outcome, round.result?.text])]),
    [
      [1, "ran", [["kill the lights in the den", "run", "den lights off"]]],
      [2, "ran", [["start a 10 minute timer", "run", "10 minute timer started"]]],
    ],
  )
  equal(woven.steps[0]?.rounds[0]?.decision.plan, project.plan)
  equal(woven.steps[0]?.rounds[0]?.decision.trace.length, 1)
})

test("a pronoun takes the address the step before yielded, into the ask it fills", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const woven = await project.weave("look up dana's address and email them")
  deepStrictEqual(woven.plan.binds, [{ from: 1, to: 2, arg: "to", field: "email", kind: "email", via: "fill" }])
  equal(woven.plan.steps[1]?.decision.outcome, "ask")
  equal(woven.status, "ran")
  deepStrictEqual(woven.steps[1]?.bound, [{ arg: "to", from: 1, field: "email", value: "dana@example.com" }])
  const round = woven.steps[1]?.rounds[0]
  equal(round?.decision.outcome, "run")
  if (round?.decision.outcome !== "run") return
  equal(round.decision.call, 'mail to="dana@example.com"')
  deepStrictEqual(round.result, { text: "drafted to dana@example.com", contained: status("file") })
  deepStrictEqual(woven.steps[0]?.rounds[0]?.result, { text: "dana <dana@example.com>", data: { email: "dana@example.com" }, contained: status("file") })
})

test("a required argument no binding covers is asked before anything runs", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const asked: string[][] = []
  const woven = await project.weave("kill the lights and start a 10 minute timer", {
    ask: d => {
      asked.push(d.missing.map(m => m.arg))
      return { room: "den" }
    },
    confirm: () => true,
  })
  deepStrictEqual(asked, [["room"]])
  deepStrictEqual(woven.plan.verdict, { outcome: "run" })
  // Filled, the step confirms at its turn: its state was read under the floor.
  equal(woven.plan.steps[0]?.decision.outcome, "confirm")
  equal(woven.status, "ran")
  equal(woven.steps[0]?.rounds[0]?.result?.text, "den lights off")
  const declined = await project.weave("kill the lights and start a 10 minute timer", { ask: () => undefined })
  equal(declined.status, "declined")
  deepStrictEqual(declined.steps, [])
  const unanswered = await project.weave("kill the lights and start a 10 minute timer")
  equal(unanswered.status, "unanswered")
  deepStrictEqual(unanswered.plan.verdict, { outcome: "ask", because: [{ type: "needs", step: 1, arg: "room" }] })
})

test("a reference that takes nothing runs only once proceed says so", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const input = "look up dana's address and start a 10 minute timer for them"
  const held = await project.weave(input)
  equal(held.status, "unanswered")
  deepStrictEqual(held.plan.verdict, { outcome: "confirm", because: [{ type: "takes_nothing", step: 2, sources: [1] }] })
  equal((await project.weave(input, { proceed: () => false })).status, "declined")
  const woven = await project.weave(input, { proceed: () => true })
  equal(woven.status, "ran")
  deepStrictEqual(woven.steps.map(step => step.rounds[0]?.result?.text), ["dana <dana@example.com>", "10 minute timer started"])
})

test("a part that matches nothing refuses the whole, and nothing runs", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const woven = await project.weave("kill the lights in the den and feed the cat")
  equal(woven.status, "refused")
  deepStrictEqual(woven.plan.verdict, { outcome: "refuse", because: [{ type: "no_reflex", step: 2 }] })
  deepStrictEqual(woven.steps, [])
})

test("a decline at a step's confirm ends the weave after its stage", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const woven = await project.weave("start a 25 minute timer and kill the lights in the den", { confirm: () => false })
  equal(woven.status, "declined")
  deepStrictEqual(
    woven.steps.map(step => [step.step, step.status, step.why]),
    [
      [1, "declined", { type: "said", message: 'timer duration="25 minute" · write · weakest: duration 0.70' }],
      [2, "skipped", { type: "earlier_step" }],
    ],
  )
  const ran = await project.weave("start a 25 minute timer and kill the lights in the den", { confirm: () => true })
  equal(ran.status, "ran")
  ok(ran.steps.every(step => step.status === "ran"))
})

const cancelAnswers = new URL("../../spec/transcripts/cancel/answers.toml", import.meta.url)

/** The cancel flow's reflexes as code: a body that waits until its signal says stop, a program that would, a timer
 *  that counts how often it ran. */
function waiting() {
  let started!: () => void
  const running = new Promise<void>(resolve => (started = resolve))
  const heard: string[] = []
  let timers = 0
  const wait = reflex({ description: "Wait, doing nothing, until told to stop.", effect: "write", confirm: "Wait?", examples: { "wait a while": {} } }, (_args, { signal }) => {
    started()
    return new Promise<string>(resolve => signal.addEventListener("abort", () => resolve(heard.push("stop") ? "stopped" : "")))
  })
  const pause = reflex({ description: "Pause: a program that sleeps.", effect: "write", confirm: "Pause?", examples: { "pause a moment": {} } }, () => "paused")
  const timer = reflex(
    { description: "Count down a duration.", effect: "write", confirm: "Start a {duration} timer?", args: { duration: { ask: "How long?", pick: "duration" } }, examples: { "set a timer for 10 minutes": { duration: "10 minutes" } } },
    () => `${++timers} timer started`,
  )
  return { reflexes: { wait, pause, timer }, running, heard, timers: () => timers }
}

test("a signal mid-run ends the body, starts nothing after, and rejects with the reason carrying the record", async () => {
  const { reflexes, running, heard, timers } = waiting()
  const project = await load({ reflexes, adapter: replay(cancelAnswers) })
  const controller = new AbortController()
  const woven = project.weave("wait a while and start a 10 minute timer", { signal: controller.signal })
  await running
  controller.abort()
  await rejects(woven, (error: Error & { woven?: Woven }) => {
    equal(error, controller.signal.reason)
    deepStrictEqual(
      error.woven?.steps.map(step => [step.step, step.status, step.why, step.rounds.map(round => round.status)]),
      [
        [1, "skipped", { type: "cancelled" }, ["skipped"]],
        [2, "skipped", { type: "cancelled" }, []],
      ],
    )
    equal(error.woven?.plan.steps.length, 2)
    return true
  })
  deepStrictEqual(heard, ["stop"])
  equal(timers(), 0)
})

test("a signal already aborted rejects with its reason before anything is planned or run", async () => {
  const { reflexes, timers } = waiting()
  const project = await load({ reflexes, adapter: replay(cancelAnswers) })
  const reason = new Error("not now")
  await rejects(project.weave("wait a while and start a 10 minute timer", { signal: AbortSignal.abort(reason) }), (error: Error & { woven?: Woven }) => error === reason && error.woven === undefined)
  equal(timers(), 0)
})
