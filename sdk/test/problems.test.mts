// Closing the month through the SDK, over its flow's home and recording: the long sentence planned, the month
// stated once reaching every lookup and the ledger, and the ledger post — under the write floor — waiting in a
// queue for a second person, who reads the decision as plain data and answers it; the weave picks up where it
// waited, and nothing ran twice. The pattern of the maker-checker example, inside one plan.

import { deepStrictEqual, equal, ok } from "node:assert/strict"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

import { type Confirm, load } from "../src/index.ts"
import { replay } from "../src/testing.ts"

const home = fileURLToPath(new URL("../../spec/transcripts/month-end/home/.config/evoke", import.meta.url))
const answers = new URL("../../spec/transcripts/month-end/answers.toml", import.meta.url)
const sentence = "pull september's bank transactions, invoices, card expenses and payroll, reconcile them, post the closing entries to the ledger, then send the report to cfo@example.com"

/** A decision waiting for a person, as plain data, and the answer they give. */
interface Waiting {
  decision: string
  step: number
  answer: (yes: boolean) => void
}

test("the month stated once reaches every lookup and the ledger, and a joint the engine kept whole is two steps", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const plan = await project.steps(sentence)
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
