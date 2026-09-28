// Closing the month through the SDK, over its flow's home and recording: the long sentence planned, the month
// stated once reaching every lookup and the ledger, and the ledger post — under the write floor — waiting in a
// queue for a second person, who reads the decision as plain data and answers it; the weave picks up where it
// waited, and nothing ran twice. The pattern of the maker-checker example, inside one plan. Then the whole plan
// handed over: the maker seals it with `steps`, the checker reads it back with `weave` over the same project, says
// yes to the plan and to the ledger, and a checker whose engine is another is refused.

import { deepStrictEqual, equal, ok, rejects } from "node:assert/strict"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

import { type Confirm, DiagnosticError, type Pinned, load, playbook } from "../src/index.ts"
import { replay } from "../src/testing.ts"

const home = fileURLToPath(new URL("../../spec/transcripts/month-end/home/.config/evoke", import.meta.url))
const answers = new URL("../../spec/transcripts/month-end/answers.toml", import.meta.url)
const otherEngine = new URL("../../spec/transcripts/plans/home/runner/other.toml", import.meta.url)
const sentence = "pull september's bank transactions, invoices, card expenses and payroll, reconcile them, post the closing entries to the ledger, then send the report to cfo@example.com"

/** A decision waiting for a person, as plain data, and the answer they give. */
interface Waiting {
  decision: string
  step: number
  answer: (yes: boolean) => void
}

test("the month stated once reaches every lookup and the ledger, and a joint the engine kept whole is two steps", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const { weave: plan } = await project.steps(sentence)
  deepStrictEqual(
    plan.steps.map(step => [step.reflex, step.decision.outcome, step.repair, Object.fromEntries(Object.entries(step.shared ?? {}).map(([arg, shared]) => [arg, `${shared.word} by ${shared.via}`]))]),
    [
      ["bank", "run", undefined, {}],
      ["invoices", "run", "split", { month: "september by fill" }],
      ["cards", "run", "split", { month: "september by fill" }],
      ["payroll", "run", undefined, { month: "september by fill" }],
      ["reconcile", "run", undefined, {}],
      ["ledger", "confirm", undefined, { month: "september by fill" }],
      ["send", "run", undefined, {}],
    ],
  )
  deepStrictEqual(plan.verdict, { outcome: "run" })
  deepStrictEqual(plan.binds?.map(b => `${b.from}→${b.to} ${b.field} ${b.via}`), ["1→5 transactions takes", "2→5 invoices takes", "3→5 expenses takes", "4→5 payroll takes"])
})

test("the ledger post waits in a queue for a second person, who answers the decision as plain data", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const queue: Waiting[] = []
  // The maker: the sentence, its plan run, every confirm handed to whoever reads the queue.
  const woven = project.weave(sentence, {
    confirm: (decision: Confirm, turn) => new Promise<boolean>(answer => queue.push({ decision: JSON.stringify(decision), step: turn.step, answer })),
  })
  // The checker: reads the queue once the maker's plan reaches its confirm.
  while (queue.length === 0) await new Promise(resolve => setTimeout(resolve, 5))
  const [waiting] = queue
  ok(waiting !== undefined)
  equal(waiting.step, 6)
  const decision = JSON.parse(waiting.decision) as Confirm
  equal(decision.reflex, "ledger")
  equal(decision.call, 'ledger month="september"')
  equal(decision.effect, "write")
  deepStrictEqual(decision.because.map(cap => cap.type), ["under_floor"])
  waiting.answer(true)
  const done = await woven
  equal(done.status, "ran")
  deepStrictEqual(
    done.steps.map(step => [step.step, step.status, step.rounds[0]?.result?.text]),
    [
      [1, "ran", "september: 214 transactions, 18 204.55 at the close"],
      [2, "ran", "september: 38 invoices, 35 paid"],
      [3, "ran", "september: 57 card charges, 2 without a receipt"],
      [4, "ran", "september: payroll 48 300.00"],
      [5, "ran", "3 transactions to review"],
      [6, "ran", "september posted to the ledger"],
      [7, "ran", "sent to cfo@example.com"],
    ],
  )
  equal(queue.length, 1)
})

test("a second person's no ends the weave at the ledger, and nothing after it runs", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const woven = await project.weave(sentence, { confirm: (_decision, turn) => turn.step !== 6 })
  equal(woven.status, "declined")
  // A write among the steps: every step at its turn in the words' order, the reconcile before the post.
  deepStrictEqual(woven.plan.stages, [[1], [2], [3], [4], [5], [6], [7]])
  deepStrictEqual(
    woven.steps.map(step => [step.step, step.status, step.why?.type]),
    [
      [1, "ran", undefined],
      [2, "ran", undefined],
      [3, "ran", undefined],
      [4, "ran", undefined],
      [5, "ran", undefined],
      [6, "declined", "said"],
      [7, "skipped", "earlier_step"],
    ],
  )
})

test("the maker seals the plan as a file, and the checker runs it whole from the file, answering the ledger", async () => {
  const maker = await load({ root: home, adapter: replay(answers) })
  const pinned = await maker.steps(sentence)
  equal(pinned.plan, 1)
  equal(pinned.input, sentence)
  equal(pinned.set, maker.plan)
  deepStrictEqual(pinned.adapter, { name: "replay", id: "replay" })
  deepStrictEqual(pinned.gate, { route: 0.5, fits: 0.3, read: 0.6, write: 0.8 })
  deepStrictEqual(Object.keys(pinned.reflexes), ["bank", "invoices", "cards", "payroll", "reconcile", "ledger", "send"])
  deepStrictEqual(Object.keys(pinned.vocab), ["months"])
  // The weave's own questions under the sentence, then each text decided: nine entries, none the engine's numbers.
  equal(pinned.answers.length, 9)
  equal(pinned.answers[0]?.text, sentence)
  // The file travels however files do: here as the text JSON.stringify writes.
  const file = JSON.stringify(pinned)
  // The checker: the same project on another load, an engine that would have to be asked for nothing.
  const checker = await load({ root: home, adapter: replay(answers) })
  const confirmed: number[] = []
  const woven = await checker.weave(JSON.parse(file) as Pinned, {
    proceed: (plan: Pinned) => plan.input === sentence,
    confirm: (decision: Confirm, turn) => {
      confirmed.push(turn.step)
      return decision.reflex === "ledger"
    },
  })
  equal(woven.status, "ran")
  deepStrictEqual(confirmed, [6])
  deepStrictEqual(
    woven.steps.map(step => [step.step, step.status, step.rounds[0]?.result?.text]),
    [
      [1, "ran", "september: 214 transactions, 18 204.55 at the close"],
      [2, "ran", "september: 38 invoices, 35 paid"],
      [3, "ran", "september: 57 card charges, 2 without a receipt"],
      [4, "ran", "september: payroll 48 300.00"],
      [5, "ran", "3 transactions to review"],
      [6, "ran", "september posted to the ledger"],
      [7, "ran", "sent to cfo@example.com"],
    ],
  )
  // No yes over the plan, nothing runs; a no over the plan, nothing runs.
  equal((await checker.weave(pinned, {})).status, "unanswered")
  equal((await checker.weave(pinned, { proceed: () => false })).status, "declined")
})

test("a checker whose engine is another is refused with the pin that moved", async () => {
  const maker = await load({ root: home, adapter: replay(answers) })
  const pinned = await maker.steps(sentence)
  const checker = await load({ root: home, adapter: replay(otherEngine) })
  await rejects(
    checker.weave(pinned, { proceed: () => true }),
    (error: DiagnosticError) => error.message === `the plan was decided by replay; this project's adapter is replay-2  →  steps(${JSON.stringify(`${sentence.slice(0, 59)}…`)})`,
  )
  // A file of another revision, or one whose answers were edited, is refused before anything runs.
  await rejects(checker.weave({ ...pinned, plan: 2 } as unknown as Pinned, { proceed: () => true }), (error: DiagnosticError) => error.message.startsWith("plan must be 1, not 2"))
})

// ---- the outage from its short sentence: a playbook installed beside the stand-ins, its plan made by the
// product, run after one yes; a slot the sentence lacks asked through `steps({ ask })`; a playbook never a body.

const outageHome = fileURLToPath(new URL("../../spec/transcripts/outage/home/.config/evoke", import.meta.url))
const outageAnswers = new URL("../../spec/transcripts/outage/answers.toml", import.meta.url)

test("a short sentence picks its playbook, whose steps stand in the plan with where each came from", async () => {
  const project = await load({ root: outageHome, adapter: replay(outageAnswers) })
  equal(project.reflexes.outage?.active && project.reflexes.outage.runs, "plan")
  const pinned = await project.steps("checkout is failing in eu-west")
  const plan = pinned.weave
  deepStrictEqual(
    plan.steps.map(step => [step.reflex, step.from?.map(from => `${from.playbook} ${from.step} ${JSON.stringify(from.slots)}`)]),
    [
      ["errors", ['outage 1 {"service":"checkout","region":"eu-west"}']],
      ["deploys", ['outage 2 {"service":"checkout","region":"eu-west"}']],
      ["logs", ['outage 3 {"service":"checkout","region":"eu-west"}']],
      ["suspect", ["outage 4 {}"]],
      ["rollback", ['outage 5 {"service":"checkout","region":"eu-west"}']],
      ["post", ["outage 6 {}"]],
      ["status", ['outage 7 {"service":"checkout"}']],
    ],
  )
  equal(plan.verdict.outcome, "confirm")
  const reviewed = plan.verdict.because?.[0]
  ok(reviewed?.type === "reviewed")
  equal(reviewed.playbook, "outage")
  equal(reviewed.step, 1)
  equal(reviewed.prompt.template, "Run the outage plan for checkout?")
  // The playbook is pinned among the reflexes, and the sentence's route among the answers.
  ok("outage" in pinned.reflexes)
  ok(pinned.answers.some(answer => answer.text === "checkout is failing in eu-west" && "route" in answer.raw))
  // A part of the sentence that repeats a step folds into it.
  const folded = await project.steps("checkout is failing in eu-west, show me the error rate")
  deepStrictEqual(folded.weave.folded, [{ text: "show me the error rate", into: 1 }])
  equal(folded.weave.steps.length, 7)
})

test("the plan runs after one yes over the whole, the rollback confirming at its turn", async () => {
  const project = await load({ root: outageHome, adapter: replay(outageAnswers) })
  const confirmed: number[] = []
  const woven = await project.weave("checkout is failing in eu-west", {
    proceed: plan => plan.verdict.because?.some(b => b.type === "reviewed") === true,
    confirm: (decision: Confirm, turn) => {
      confirmed.push(turn.step)
      return decision.reflex === "rollback"
    },
  })
  equal(woven.status, "ran")
  deepStrictEqual(confirmed, [5])
  deepStrictEqual(woven.steps.map(step => step.rounds[0]?.result?.text), [
    "checkout: 8.4% errors since 14:02",
    "checkout: 2 deploys today, the last 4.12.0 at 13:58",
    "checkout: 412 timeouts calling payments",
    "4.12.0, out at 13:58, four minutes before the errors rose",
    "checkout rolled back from 4.12.0 to 4.11.3",
    "posted to #incident",
    "checkout: monitoring on the status page",
  ])
  // No yes over the plan, nothing runs.
  equal((await project.weave("checkout is failing in eu-west", {})).status, "unanswered")
})

test("a slot the sentence lacks is asked through steps({ ask }), and such a plan is a sheet, not a file", async () => {
  const project = await load({ root: outageHome, adapter: replay(outageAnswers) })
  const asking = await project.steps("we have an outage")
  equal(asking.weave.verdict.outcome, "ask")
  equal(asking.weave.steps.length, 1)
  const sheet = await project.steps("we have an outage", { ask: () => ({ service: "payments" }) })
  equal(sheet.weave.verdict.outcome, "confirm")
  deepStrictEqual(
    sheet.weave.steps.map(step => step.reflex),
    ["errors", "deploys", "logs", "suspect", "rollback", "post", "status"],
  )
  deepStrictEqual(
    sheet.weave.steps.map(step => step.from?.map(from => `${from.playbook} ${from.step} ${from.slots.service}`)),
    [1, 2, 3, 4, 5, 6, 7].map(n => [`outage ${n} ${n === 4 || n === 6 ? "undefined" : "payments"}`]), // a step without the slot carries none
  )
  // The file holds no one's answers: run by its sentence, never as a file.
  await rejects(project.weave(sheet, { proceed: () => true }), (error: DiagnosticError) => error.message.includes("does not read the same"))
  // A condition is no step's to judge.
  const conditional = await project.steps("if checkout is failing in eu-west, roll it back")
  deepStrictEqual(conditional.weave.verdict, { outcome: "refuse", because: [{ type: "conditional", text: "if checkout is failing in eu-west" }] })
})

test("a playbook is never run as a body: handle and run refuse it, and playbook() hands one to load", async () => {
  const project = await load({ root: outageHome, adapter: replay(outageAnswers) })
  const handled = project.handle("checkout is failing in eu-west", { confirm: () => true })
  await rejects(handled, (error: DiagnosticError) => error.message.includes("outage is a plan of steps; say it in a sentence"))
  const decision = await project.decide("checkout is failing in eu-west")
  ok(decision.outcome === "confirm")
  await rejects(project.run(decision, { confirmed: true }), (error: DiagnosticError) => error.message.includes("outage is a plan of steps"))
  const own = await load({
    reflexes: {
      evening: playbook({
        description: "Wind the house down.\nKills the lights and starts a timer.",
        confirm: "Wind down?",
        steps: ["kill the lights in the den", "start a 10 minute timer"],
        examples: { "wind down": {} },
      }),
    },
    adapter: { id: "replay", answer: () => Promise.reject(new Error("never asked")) },
  })
  equal(own.reflexes.evening?.active && own.reflexes.evening.runs, "plan")
})
