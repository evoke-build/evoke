// Composed values through the SDK: a day read as a date and resolved against `EVOKE_TODAY` at the body's door,
// over the cancelled flight's home and recording; memory as the ask's choices over the recent flow's home — the
// results an app hands `decide` offered back at the ask of a whole sentence, a value chosen at the prompt filled
// as a pick, and a sentence of two steps taking none.

import { deepStrictEqual, equal, ok } from "node:assert/strict"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

import { load } from "../src/index.ts"
import { replay } from "../src/testing.ts"

const flight = fileURLToPath(new URL("../../spec/transcripts/cancelled-flight/home/.config/evoke", import.meta.url))
const flightAnswers = new URL("../../spec/transcripts/cancelled-flight/answers.toml", import.meta.url)
const recent = fileURLToPath(new URL("../../spec/transcripts/recent/home/.config/evoke", import.meta.url))
const recentAnswers = new URL("../../spec/transcripts/recent/answers.toml", import.meta.url)

/** The two releases the deploys stand-in returns, as an app would hand them back. */
const DEPLOYED = [{ reflex: "deploys", data: { service: "checkout", deploys: [{ release: "4.12.0", at: "13:58" }, { release: "4.11.3", at: "09:12" }] } }]

test("a day reads as a date and reaches the body resolved against EVOKE_TODAY", async () => {
  const project = await load({ root: flight, adapter: replay(flightAnswers) })
  const decision = await project.decide("tomorrow's calendar")
  equal(decision.outcome, "run")
  if (decision.outcome !== "run") return
  deepStrictEqual(decision.args.day, { type: "pick", span: { start: 0, end: 8, text: "tomorrow" }, value: { type: "date", value: { type: "offset", days: 1 } } })
  deepStrictEqual(decision.values, { day: { type: "offset", days: 1 } })
  const before = process.env.EVOKE_TODAY
  process.env.EVOKE_TODAY = "2026-09-28"
  try {
    const result = await project.run(decision)
    ok(result.text.startsWith("2026-09-29: 2 meetings"), result.text)
  } finally {
    if (before === undefined) delete process.env.EVOKE_TODAY
    else process.env.EVOKE_TODAY = before
  }
})

test("a day that is no day in EVOKE_TODAY is the run's failure, naming the variable", async () => {
  const project = await load({ root: flight, adapter: replay(flightAnswers) })
  const decision = await project.decide("tomorrow's calendar")
  if (decision.outcome !== "run") throw new Error(decision.outcome)
  const before = process.env.EVOKE_TODAY
  process.env.EVOKE_TODAY = "tomorrow"
  try {
    await project.run(decision)
    throw new Error("ran")
  } catch (error) {
    ok(error instanceof Error && error.message.includes('EVOKE_TODAY: "tomorrow" is not a date: YYYY-MM-DD'), String(error))
  } finally {
    if (before === undefined) delete process.env.EVOKE_TODAY
    else process.env.EVOKE_TODAY = before
  }
})

test("the results an app hands back are the ask's choices for a whole sentence, and a chosen one fills as a pick", async () => {
  const project = await load({ root: recent, adapter: replay(recentAnswers) })
  const asked = await project.decide("roll back the last deploy", { recent: DEPLOYED })
  equal(asked.outcome, "ask")
  if (asked.outcome !== "ask") return
  deepStrictEqual(asked.missing.map(m => [m.arg, m.choices]), [["release", { type: "pick", pick: "code", recent: ["4.12.0", "4.11.3"] }]])
  const filled = project.fill(asked, { release: "4.12.0" })
  equal(filled.outcome, "confirm")
  if (filled.outcome !== "confirm") return
  equal(filled.call, 'rollback release="4.12.0"')
  equal(filled.prompt.template, "Roll back 4.12.0?")
  deepStrictEqual(filled.values, { release: "4.12.0" })
  // Nothing handed back: the same sentence asks with no list.
  const alone = await project.decide("roll back the last deploy")
  if (alone.outcome !== "ask") throw new Error(alone.outcome)
  deepStrictEqual(alone.missing[0]?.choices, { type: "pick", pick: "code" })
})

test("a sentence of two steps takes no memory: its second step asks with no list", async () => {
  const project = await load({ root: recent, adapter: replay(recentAnswers) })
  const listed: unknown[] = []
  const woven = await project.weave("list the checkout deploys, then roll back the last one", {
    recent: DEPLOYED,
    ask: decision => {
      listed.push(decision.missing.map(m => m.choices))
      return undefined
    },
  })
  equal(woven.status, "declined")
  deepStrictEqual(listed, [[{ type: "pick", pick: "code" }]])
})
