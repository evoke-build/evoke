// The project, end to end: the spec's own home through the recorded adapter — decide, run in a child, ask then
// confirm, handle; a reflex handed as code through an adapter that answers from the questions; a tenant's project
// through `with`; the guards; the store's miss.

import { deepStrictEqual, equal, ok, rejects, throws } from "node:assert/strict"
import { cpSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

import type { Adapter } from "../src/adapter.ts"
import { status } from "../src/contain.ts"
import { DiagnosticError, FaultError } from "../src/errors.ts"
import { load, reflex } from "../src/index.ts"
import { replay } from "../src/testing.ts"

const home = fileURLToPath(new URL("../../spec/transcripts/home/.config/evoke", import.meta.url))
const answers = new URL("../../spec/transcripts/use/answers.toml", import.meta.url)

test("the spec's home decides and runs as the CLI does", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  deepStrictEqual(project.reflexes, {
    lights: { active: true, effect: "write", runs: "file" },
    timer: { active: true, effect: "write", runs: "file" },
    volume: { active: true, effect: "write", runs: "file" },
    open: {
      active: false,
      problems: [
        {
          reflex: "open",
          message: 'vocabulary "places" is empty',
          fix: { type: "vocab_add", vocab: "places" },
          command: 'evoke vocab places add <word> "<meaning>"',
        },
      ],
    },
  })
  ok(project.plan.startsWith("h1:"))
  const d = await project.decide("kill the lights in the den")
  equal(d.outcome, "run")
  if (d.outcome !== "run") return
  equal(d.reflex, "lights")
  deepStrictEqual(d.values, { room: "den", state: "off" })
  deepStrictEqual(d.args, { room: { type: "word", word: "den" }, state: { type: "option", key: "off" } })
  equal(d.call, 'lights room="den" state="off"')
  equal(d.confidence, 0.85)
  equal(d.input, "kill the lights in the den")
  equal(d.plan, project.plan)
  equal(d.trace.length, 1)
  equal(d.trace[0]?.adapter, "replay")
  deepStrictEqual(await project.run(d), { text: "den lights off", contained: status() })
})

test("an ask is filled with what a person typed, then confirmed", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const asked = await project.decide("kill the lights")
  equal(asked.outcome, "ask")
  if (asked.outcome !== "ask") return
  deepStrictEqual(asked.missing.map(m => [m.arg, m.choices.type]), [["room", "vocab"]])
  deepStrictEqual(asked.values, { state: "off" })
  throws(
    () => project.fill(asked, { room: "attic" }),
    (error: DiagnosticError) => error.message === 'room: "attic" is not one of den, office  →  fill(d, {"room":"attic"})',
  )
  const filled = project.fill(asked, { room: "den" })
  equal(filled.outcome, "confirm")
  if (filled.outcome !== "confirm") return
  equal(filled.prompt.template, "Set the den lights off?")
  equal(filled.prompt.own, 'lights room="den" state="off" · write · weakest: state 0.58')
  await rejects(project.run(filled as never), (error: TypeError) => error.message === "a confirm decision runs only with { confirmed: true }")
  deepStrictEqual(await project.run(filled, { confirmed: true }), { text: "den lights off", contained: status() })
})

test("handle does the whole loop and returns every expected outcome", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const declined = await project.handle("dim the office", { confirm: () => false })
  equal(declined.outcome, "declined")
  const ran = await project.handle("dim the office", { confirm: d => d.prompt.template === "Set the office lights dim?" })
  equal(ran.outcome, "ran")
  if (ran.outcome === "ran") deepStrictEqual(ran.result, { text: "group-7 lights dim", contained: status() }) // the word's value reaches the body
  const unanswered = await project.handle("kill the lights")
  equal(unanswered.outcome, "unanswered")
  if (unanswered.outcome === "unanswered") equal(unanswered.decision.outcome, "ask")
  const nothing = await project.handle("kill the lights", { ask: () => undefined })
  equal(nothing.outcome, "declined")
  const stuck = await project.handle("kill the lights", { ask: () => ({}) })
  equal(stuck.outcome, "declined")
  const whole = await project.handle("kill the lights", { ask: () => ({ room: "den" }), confirm: () => true })
  equal(whole.outcome, "ran")
  if (whole.outcome === "ran") deepStrictEqual(whole.result, { text: "den lights off", contained: status() })
})

test("a tenant's project has its own plan, and refuses another project's decision", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const tenant = project.with({ vocab: { rooms: { kitchen: "The kitchen.", attic: { what: "The attic.", value: "group-9" } } } })
  ok(tenant.plan !== project.plan)
  deepStrictEqual(tenant.reflexes, project.reflexes)
  const d = await project.decide("kill the lights in the den")
  if (d.outcome !== "run") return
  await rejects(tenant.run(d), (error: TypeError) => error.message === "the decision was made under another plan; run it on the project that decided it")
  throws(
    () => project.with({ vocab: { rooms: { none: "Nothing." } } }),
    (error: DiagnosticError) => error.message === 'word "none" is reserved  →  with({ vocab: { rooms } })',
  )
})

test("a reflex handed as code runs in-process with its values typed from the manifest", async () => {
  const timer = reflex(
    {
      description: "Start a countdown timer.",
      effect: "write",
      confirm: "Start a {duration} timer?",
      args: {
        duration: { ask: "How long?", pick: "duration" },
        loud: { ask: "Ring until dismissed?", flag: true },
      },
      examples: { "timer for 10 minutes": { duration: "10 minutes" } },
    },
    async ({ duration, loud }, { input, signal }) => {
      const seconds: number = duration
      const ringing: true | undefined = loud
      return { text: `${seconds} s${ringing ? " loud" : ""}`, data: { input, aborted: signal.aborted } }
    },
  )
  const project = await load({ reflexes: { timer }, adapter: answering() })
  deepStrictEqual(project.reflexes, { timer: { active: true, effect: "write", runs: "inline" } })
  const d = await project.decide("set a timer for 10 minutes, loud")
  equal(d.outcome, "run")
  if (d.outcome !== "run") return
  const seconds: number = d.values.duration
  equal(seconds, 600)
  equal(d.values.loud, true)
  deepStrictEqual(await project.run(d), { text: "600 s loud", data: { input: "set a timer for 10 minutes, loud", aborted: false } })
  const quiet = await project.handle("set a timer for 3 minutes", { confirm: () => true })
  equal(quiet.outcome, "ran")
  if (quiet.outcome === "ran") equal(quiet.result.text, "180 s")
})

test("a manifest handed as code is refused at load, ending in reflex(<name>)", async () => {
  const broken = reflex({ description: "Say hello.", confirm: "Hello {who}?", args: { who: { ask: "Who?", pick: "quoted", optional: true } } }, async () => "hi")
  await rejects(
    load({ reflexes: { broken }, adapter: answering() }),
    (error: DiagnosticError) => error.message === "broken: confirm names {who}, an optional argument  →  reflex(broken)",
  )
  const keys = { TYPESAFE_API_KEY: process.env.TYPESAFE_API_KEY, OPENJEV_API_KEY: process.env.OPENJEV_API_KEY }
  delete process.env.TYPESAFE_API_KEY
  delete process.env.OPENJEV_API_KEY
  try {
    // A root without evoke.toml is the default project, which names jev.
    await rejects(
      load({ root: mkdtempSync(join(tmpdir(), "evoke-bare-")) }),
      (error: DiagnosticError) => error.message === "jev needs TYPESAFE_API_KEY, a key from typesafe.ai  →  export TYPESAFE_API_KEY=<value>",
    )
    // A project naming the other door asks for that door's key.
    const through = mkdtempSync(join(tmpdir(), "evoke-openjev-"))
    writeFileSync(join(through, "evoke.toml"), 'adapter = "openjev"\n')
    await rejects(
      load({ root: through }),
      (error: DiagnosticError) => error.message === "openjev needs OPENJEV_API_KEY, a key from openjev.sh  →  export OPENJEV_API_KEY=<value>",
    )
  } finally {
    for (const [name, value] of Object.entries(keys)) if (value !== undefined) process.env[name] = value
  }
})

test("a remote reflex the store lacks is evoke sync; a project naming replay wants a recording", async () => {
  process.env.XDG_CACHE_HOME = mkdtempSync(join(tmpdir(), "evoke-empty-"))
  const root = fileURLToPath(new URL("../../spec/transcripts/update/home/.config/evoke", import.meta.url))
  await rejects(
    load({ root, adapter: replay(answers) }),
    (error: DiagnosticError) => error.message === "lights: lights is not in the store  →  evoke sync",
  )
  await rejects(
    load({ root: home }),
    (error: DiagnosticError) => error.message === 'adapter "replay" names a recording; pass one  →  load({ root, adapter: replay(file) })',
  )
})

test("a recording made against another plan is refused, and an unrecorded input is a fault", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  await rejects(
    project.decide("what time is it"),
    (error: FaultError) => error.message.startsWith('"what time is it" is not recorded  →  replay("'),
  )
  const pinned: Adapter = { ...replay(answers), plan: "h1:" + "0".repeat(64) }
  await rejects(
    load({ root: home, adapter: pinned }),
    (error: DiagnosticError) => error.message.startsWith("the recording was made against plan h1:000") && error.command === "replay(file, { record: jev() })",
  )
})

/** An adapter that answers from the questions themselves: the route to `timer`, a pick to its first candidate, a
 *  flag by the input, every yes/no at 0.9. */
function answering(): Adapter {
  return {
    id: "answering",
    gate: { route: 0.5, fits: 0.3, read: 0.6, write: 0.8 },
    async answer(state, questions) {
      const raw: Record<string, Record<string, number>> = {}
      for (const [id, question] of Object.entries(questions)) {
        if (question.type === "yesno") {
          raw[id] = { yes: 0.9 }
          continue
        }
        const keys = Object.keys(question.options)
        const top =
          id === "route"
            ? "timer"
            : id.endsWith(".loud")
              ? state.request.includes("loud")
                ? "yes"
                : "no"
              : (keys.find(key => key !== "unstated") ?? "unstated")
        raw[id] = Object.fromEntries(keys.map(key => [key, key === top ? 1 : 0]))
      }
      return raw
    },
  }
}

test("a reflex that runs a program runs it with the call's values", async () => {
  const root = mkdtempSync(join(tmpdir(), "evoke-argv-"))
  cpSync(home, root, { recursive: true })
  const manifest = readFileSync(join(root, "lights", "reflex.toml"), "utf8")
  writeFileSync(join(root, "lights", "reflex.toml"), manifest.replace('run = "lights.mts"', 'run = ["/bin/echo", "{room}", "{state}"]'))
  const project = await load({ root, adapter: replay(answers) })
  equal(project.reflexes.lights?.active && project.reflexes.lights.runs, "argv")
  const d = await project.decide("kill the lights in the den")
  equal(d.outcome, "run")
  if (d.outcome !== "run") return
  deepStrictEqual(await project.run(d), { text: "den off", contained: status() })
})

test("a root that is not a directory, a project with no root and no adapter, and an unasked argument", async () => {
  await rejects(
    load({ root: join(tmpdir(), "nowhere-at-all") }),
    (error: DiagnosticError) => error.message.endsWith('is not a directory  →  load({ root })'),
  )
  await rejects(
    // The type requires the adapter without a root; JavaScript, which has no types, still gets the line.
    // @ts-expect-error
    load({ reflexes: {} }),
    (error: DiagnosticError) => error.message === "no adapter: none is named and none was passed  →  load({ reflexes, adapter: jev() })",
  )
  const project = await load({ root: home, adapter: replay(answers) })
  const asked = await project.decide("kill the lights")
  if (asked.outcome !== "ask") return
  throws(
    () => project.fill(asked, { state: "on" }),
    (error: DiagnosticError) => error.message === 'state is not being asked; the ask wants room  →  fill(d, {"state":"on"})',
  )
  const trimmed = project.fill(asked, { room: " den " })
  equal(trimmed.outcome, "confirm")
})

test("handle asks again once for an answer that does not read, then declines", async () => {
  const project = await load({ root: home, adapter: replay(answers) })
  const answers_ = ["attic", "den"]
  const recovered = await project.handle("kill the lights", { ask: () => ({ room: answers_.shift() }), confirm: () => true })
  equal(recovered.outcome, "ran")
  const declined = await project.handle("kill the lights", { ask: () => ({ room: "attic" }) })
  equal(declined.outcome, "declined")
  if (declined.outcome === "declined") equal(declined.decision.outcome, "ask")
})

test("a name that is none, an empty input and an answer that is no text are refused, never bugs", async () => {
  const timer = reflex({ description: "Start a timer.", confirm: "Start?", effect: "write" }, async () => "ok")
  await rejects(
    load({ reflexes: { "Bad Name": timer }, adapter: replay(answers) }),
    (error: DiagnosticError) => error.message === '"Bad Name" is not a name: [a-z][a-z0-9_]*  →  load({ reflexes })',
  )
  await rejects(
    load({ reflexes: { weave: timer }, adapter: replay(answers) }),
    (error: DiagnosticError) => error.message === '"weave" is reserved  →  load({ reflexes })',
  )
  const project = await load({ root: home, adapter: replay(answers) })
  throws(
    () => project.with({ vocab: { "Bad Name": { den: "The den." } } }),
    (error: DiagnosticError) => error.message === '"Bad Name" is not a name: [a-z][a-z0-9_]*  →  with({ vocab })',
  )
  await rejects(project.decide("   "), (error: DiagnosticError) => error.message === 'the input is empty  →  decide("<input>")')
  await rejects(project.steps(""), (error: DiagnosticError) => error.message === 'the input is empty  →  steps("<input>")')
  await rejects(project.weave(""), (error: DiagnosticError) => error.message === 'the input is empty  →  weave("<input>")')
  const asked = await project.decide("kill the lights")
  if (asked.outcome !== "ask") return
  throws(
    () => project.fill(asked, { room: 5 as never }),
    (error: DiagnosticError) => error.message === 'room: "5" is not one of den, office  →  fill(d, {"room":5})',
  )
})
