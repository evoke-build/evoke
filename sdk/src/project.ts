// A project: the owned files under a root, reflexes handed as code, and who answers — read, verified and compiled
// once; never fetched, never written. On it: `with`, the same project over a vocabulary of the call's own — a
// tenant's words, their own plan in milliseconds; `decide`, one input asked, answered, read and gated; `fill`, an
// ask answered and gated again; `run`, the chosen call's body; `handle`, the whole loop. In: LoadOptions; inputs;
// decisions. Out: a Project, decisions, results.

import { statSync } from "node:fs"

import { type Adapter, answered } from "./adapter.ts"
import { bug, call, command, fromCode, problem, reply } from "./core.ts"
import type { Abstain, AnyReflexes, Ask, Confirm, Decision, Given, Handled, Line, Run } from "./decision.ts"
import { DiagnosticError, type Problem } from "./errors.ts"
import { entry, snapshot, text } from "./files.ts"
import type { Inline } from "./reflex.ts"
import { type Reflex, type Result, child, inline, program } from "./runtime.ts"
import type * as W from "./types.ts"

export interface LoadOptions<R = AnyReflexes> {
  /** The project directory: evoke.toml, evoke.lock, overlays/, vocab/, local reflexes. Absent: no files at all. */
  root?: string | undefined
  /** Reflexes handed as code, by local name; a name evoke.toml also lists is an error. Overlays under root apply.
   *  R is inferred from this map when no root is given; with a root, write `load<Reflexes & ReflexesOf<typeof own>>`. */
  reflexes?: { [K in keyof R]?: Inline<R[K]> | undefined } | undefined
  /** Who answers. Absent: the adapter evoke.toml names, from its own subpath; required without a root. */
  adapter?: Adapter | undefined
}

export interface DecideOptions {
  /** Only reflexes carrying one of these tags are offered. */
  tags?: string[] | undefined
  /** Aborts the adapter call; `decide` rejects with the signal's reason. */
  signal?: AbortSignal | undefined
}

export interface RunOptions {
  /** Aborts the body: its `signal` in-process, its group otherwise; `run` rejects with the signal's reason. */
  signal?: AbortSignal | undefined
}

export interface Handlers<R = AnyReflexes> {
  /** Asked at a confirm: `true` runs, `false` declines. Absent and reached: `unanswered`. */
  confirm?: ((decision: Confirm<R>) => boolean | Promise<boolean>) | undefined
  /** Asked at an ask: answers by argument name, or `undefined` to decline; asked again when an answer does not
   *  read or leaves something missing, and declined when it changes nothing. Absent and reached: `unanswered`. */
  ask?: ((decision: Ask<R>) => Given<Ask<R>> | undefined | Promise<Given<Ask<R>> | undefined>) | undefined
}

/** Vocabularies in the file's form: per name, word → what it means, or `{ what, value? }`. */
export type Vocab = Record<string, Record<string, string | { what: string; value?: string | undefined }>>

/** A reflex the project holds: active, with its effect and how it runs; or inactive, with each problem and its fix. */
export type ReflexStatus =
  | { active: true; effect: W.Effect; runs: "inline" | "file" | "argv" }
  | { active: false; problems: Problem[] }

export interface Project<R = AnyReflexes> {
  /** Every reflex the project holds. */
  readonly reflexes: Readonly<Record<string, ReflexStatus>>
  /** The plan's digest — `h1:<hex>` — which every decision carries and a recording may pin. */
  readonly plan: string
  /** This project over vocabularies of the call's own, compiled to their own plan; nothing else changes. */
  with(options: { vocab: Vocab }): Project<R>
  /** One input decided: asked, answered, read, gated. Runs nothing. */
  decide(input: string, options?: DecideOptions): Promise<Decision<R>>
  /** An ask answered — an option's key, else the text a person typed — and gated again. */
  fill<D extends Ask<R>>(decision: D, given: Given<D>): Decision<R>
  /** The chosen call's body: in-process, in a child through the loader, or as an argv. */
  run(decision: Run<R>, options?: RunOptions): Promise<Result>
  /** A confirm decision runs only once confirmed. */
  run(decision: Confirm<R>, options: RunOptions & { confirmed: true }): Promise<Result>
  /** The whole loop: decide; ask and fill until nothing is missing; confirm; run. */
  handle(input: string, options?: DecideOptions & Handlers<R>): Promise<Handled<R>>
}

/** The home project as the CLI writes it on first use: what a root without evoke.toml means. */
const DEFAULT: W.Project = { adapter: "jev", reflexes: {}, config: {}, adapters: {} }

/** What a project is made of, before its plan: the installed set, who answers, where each body lives. */
interface Ground {
  installed: W.Installed
  adapter: Adapter
  dirs: Record<string, string>
  bodies: Record<string, Reflex<Record<string, unknown>, Record<never, string>>>
}

/** A project from its root: its installed reflexes are what `Reflexes` from evoke.d.ts names, so R is written. */
export function load<R extends object = AnyReflexes>(
  options: { root: string; reflexes?: NoInfer<{ [K in keyof R]?: Inline<R[K]> | undefined }> | undefined; adapter?: Adapter | undefined },
): Promise<Project<R>>
/** A project from reflexes handed as code alone: R is inferred from them. */
export function load<R extends object = AnyReflexes>(
  options: { root?: undefined; reflexes?: { [K in keyof R]?: Inline<R[K]> | undefined } | undefined; adapter?: Adapter | undefined },
): Promise<Project<R>>
export async function load<R extends object = AnyReflexes>(options: LoadOptions<R>): Promise<Project<R>> {
  const { root } = options
  if (root !== undefined && !directory(root)) {
    throw refused(undefined, `${JSON.stringify(root)} is not a directory`, { type: "rerun" }, "load({ root })")
  }
  const files = root === undefined ? { overlays: {}, vocab: {} } : snapshot(root)
  const project: W.Project =
    files.project === undefined ? DEFAULT : call("project", { doc: { file: { type: "project" }, toml: files.project } }, "load()")
  const lock: W.Lock | undefined =
    files.lock === undefined ? undefined : call("lock", { doc: { file: { type: "lock" }, toml: files.lock } }, "load()")
  const adapter = options.adapter ?? (await named(project, root))
  const reflexes: Record<string, W.Item> = {}
  const dirs: Record<string, string> = {}
  const bodies: Ground["bodies"] = {}
  const given = (options.reflexes ?? {}) as Record<string, Inline | undefined>
  for (const [name, location] of Object.entries(project.reflexes)) {
    if (given[name] !== undefined) {
      throw refused(name, `${name} is both in evoke.toml and passed to load`, { type: "remove", reflex: name })
    }
    const configured = held(project.config[name] ?? {})
    if (location.type === "local") {
      if (root === undefined) throw refused(name, `${name} is a local reflex, but load has no root`, { type: "remove", reflex: name })
      const dir = `${root}/${location.path}`
      const manifest = text(`${dir}/reflex.toml`)
      if (manifest === undefined) {
        const fix = { type: "remove", reflex: name } as const
        reflexes[name] = { wording: { err: [{ reflex: name, message: `${location.path}/reflex.toml is missing`, fix }] }, consented: "destructive", configured }
        continue
      }
      const worded = word(name, { file: { type: "manifest", name }, toml: manifest }, files.overlays[name])
      reflexes[name] = { wording: worded.wording, consented: worded.manifest?.effect ?? "destructive", configured }
      dirs[name] = dir
      continue
    }
    const locked = lock?.reflexes[name]
    if (locked === undefined) {
      throw refused(name, `${name} is not locked`, { type: "add_ref", reference: call("location", { location }), name })
    }
    const dir = entry(locked.h1)
    if (dir === undefined) throw refused(name, `${name} is not in the store`, { type: "sync" })
    const manifest = text(`${dir}/reflex.toml`)
    if (manifest === undefined) throw refused(name, `${name} has no reflex.toml in the store`, { type: "sync" })
    const worded = word(name, { file: { type: "manifest", name }, toml: manifest }, files.overlays[name])
    reflexes[name] = { wording: worded.wording, consented: locked.effect, configured }
    dirs[name] = dir
  }
  for (const [name, handed] of Object.entries(given)) {
    if (handed === undefined) continue
    const worded = word(name, { file: { type: "manifest", name }, json: { reflex: 1, ...handed.manifest } }, files.overlays[name])
    // The manifest is the app's own code: its problems are the app's to fix now, not a reflex to leave inactive.
    if ("err" in worded.wording && worded.manifest === undefined) throw fromCode(worded.wording.err, `reflex(${name})`)
    reflexes[name] = { wording: worded.wording, consented: worded.manifest?.effect ?? "destructive", configured: {} }
    bodies[name] = handed.body
  }
  const vocab: Record<string, W.Vocabulary> = {}
  for (const [name, toml] of Object.entries(files.vocab)) {
    vocab[name] = call("vocabulary", { doc: { file: { type: "vocab", name }, toml } }, "load()")
  }
  const installed: W.Installed = { reflexes, vocab, adapter: adapter.id, evoke: call("version", {}) }
  return make({ installed, adapter, dirs, bodies }, "load()") as unknown as Project<R>
}

/** The adapter evoke.toml names, from its own subpath; a recording is never resolved by name. */
async function named(project: W.Project, root: string | undefined): Promise<Adapter> {
  const name = project.adapter
  const line = (message: string, adapter: string) => {
    const form = root === undefined ? `load({ reflexes, adapter: ${adapter} })` : `load({ root, adapter: ${adapter} })`
    return Promise.reject(new DiagnosticError([{ message, fix: { type: "rerun" }, command: form }]))
  }
  if (root === undefined) return line("no adapter: none is named and none was passed", "jev()")
  if (name === "jev") {
    const { jev } = await import("./jev.ts")
    return jev({ table: project.adapters[name] })
  }
  if (name === "replay") return line(`adapter "replay" names a recording; pass one`, "replay(file)")
  return line(`adapter "${name}" is unknown; pass one`, "adapter")
}

/** A manifest read with the overlay named for it: the effective wording, or the problems, which make the reflex
 *  inactive rather than refusing the load. */
function word(
  name: string,
  doc: W.Document,
  overlay: string | undefined,
): { wording: W.Result<W.Effective, W.Diagnostic[]>; manifest?: W.Manifest } {
  const parsed = reply("manifest", { doc })
  if ("bug" in parsed) throw bug(parsed.bug)
  if ("err" in parsed) return { wording: { err: parsed.err as W.Diagnostic[] } }
  const manifest = parsed.ok as W.Manifest
  let yours: W.Overlay | undefined
  if (overlay !== undefined) {
    const read = reply("overlay", { doc: { file: { type: "overlay", name }, toml: overlay }, of: manifest })
    if ("bug" in read) throw bug(read.bug)
    if ("err" in read) return { wording: { err: read.err as W.Diagnostic[] }, manifest }
    yours = read.ok as W.Overlay
  }
  const effective = call("effective", { shipped: manifest, ...(yours === undefined ? {} : { yours }) })
  return { wording: { ok: effective }, manifest }
}

/** How each configured key is held: the value, or a variable and whether it is set — never a secret's value. */
function held(settings: Record<string, W.Setting>): Record<string, W.Held> {
  const held: Record<string, W.Held> = {}
  for (const [key, setting] of Object.entries(settings)) {
    held[key] = setting.type === "plain" ? setting : { type: "env", var: setting.var, set: process.env[setting.var] !== undefined }
  }
  return held
}

function directory(path: string): boolean {
  try {
    return statSync(path).isDirectory()
  } catch {
    return false
  }
}

function refused(reflex: string | undefined, message: string, fix: W.Fix, invoked = "load()"): DiagnosticError {
  return new DiagnosticError([{ ...(reflex === undefined ? {} : { reflex }), message, fix, command: command(fix, invoked) }])
}

/** The SDK call as a fix line names it, the input elided past sixty characters. */
function invocation(input: string): string {
  const shown = input.length > 60 ? `${input.slice(0, 59)}…` : input
  return `decide(${JSON.stringify(shown)})`
}

/** The project over its ground: the set compiled, every reflex's status read off the plan. The implementation
 *  speaks the wire's shapes; the app's R lives on the interface alone. */
function make(ground: Ground, invoked: string): Project<AnyReflexes> {
  const { installed, adapter, dirs, bodies } = ground
  const plan: W.Plan = call("compile", { set: installed, ...(adapter.limits === undefined ? {} : { limits: adapter.limits }) }, invoked)
  if (adapter.plan !== undefined && adapter.plan !== plan.digest) {
    const message = `the recording was made against plan ${adapter.plan}, not ${plan.digest}`
    throw new DiagnosticError([{ message, fix: { type: "rerun" }, command: "replay(file, { record: jev() })" }])
  }
  const reflexes: Record<string, ReflexStatus> = {}
  for (const name of Object.keys(installed.reflexes)) {
    const active = plan.active[name]
    if (active !== undefined) {
      reflexes[name] = { active: true, effect: active.effect, runs: bodies[name] !== undefined ? "inline" : Array.isArray(active.run) ? "argv" : "file" }
    } else {
      reflexes[name] = { active: false, problems: (plan.inactive[name] ?? []).map(diagnostic => problem(diagnostic, invoked)) }
    }
  }
  const gate = adapter.gate === undefined ? {} : { gate: adapter.gate }
  const project: Project<AnyReflexes> = {
    reflexes,
    plan: plan.digest,

    with({ vocab }) {
      const replaced = { ...installed.vocab }
      for (const [name, words] of Object.entries(vocab)) {
        const read = reply("vocabulary", { doc: { file: { type: "vocab", name }, json: words } })
        if ("bug" in read) throw bug(read.bug)
        if ("err" in read) throw fromCode(read.err as W.Diagnostic[], `with({ vocab: { ${name} } })`)
        replaced[name] = read.ok as W.Vocabulary
      }
      return make({ ...ground, installed: { ...installed, vocab: replaced } }, "with({ vocab })")
    },

    async decide(input, options = {}) {
      const invoked = invocation(input)
      const request = call("request", { plan, input, tags: options.tags ?? [], scope: "full" }, invoked)
      const { raw, trace } = await answered(adapter, request, plan.deadline, options.signal, invoked)
      const reading = call("read", { plan, request, raw }, invoked)
      const decision = call("gate", { plan, reading, ...gate })
      return lined(decision, { input, plan: plan.digest, trace: [trace] })
    },

    fill(decision, given) {
      const asking = own(decision, "fill")
      const invoked = `fill(d, ${JSON.stringify(given)})`
      const wanted = asking.missing.map(missing => missing.arg)
      const values: Record<string, W.Value> = {}
      for (const [name, answer] of Object.entries(given as Record<string, string | undefined>)) {
        if (answer === undefined) continue
        const missing = asking.missing.find(missing => missing.arg === name)
        if (missing === undefined) {
          throw refused(undefined, `${name} is not being asked; the ask wants ${wanted.join(", ")}`, { type: "rerun" }, invoked)
        }
        values[name] = typed(missing, answer, invoked)
      }
      // The whole decision crosses: the core reads the fields of an Asking and ignores the SDK's own.
      const filled = call("fill", { plan, asking: asking as unknown as W.Asking, given: values, ...gate })
      return lined(filled, { input: asking.input, plan: plan.digest, trace: asking.trace })
    },

    async run(decision: Run<AnyReflexes> | Confirm<AnyReflexes>, options: RunOptions & { confirmed?: true } = {}) {
      const chosen = own(decision, "run")
      const outcome: string = chosen.outcome
      if (outcome === "confirm" && options.confirmed !== true) {
        throw new TypeError("a confirm decision runs only with { confirmed: true }")
      }
      if (outcome !== "run" && outcome !== "confirm") throw new TypeError(`a ${outcome} decision cannot run`)
      const active = plan.active[chosen.reflex]
      if (active === undefined) throw refused(chosen.reflex, `${chosen.reflex} is not active`, { type: "rerun" }, "run(d)")
      const spent = chosen.trace.reduce((sum, entry) => sum + entry.ms, 0)
      // The whole decision crosses: the core reads the fields of a Chosen and ignores the SDK's own.
      const wire = chosen as unknown as W.Chosen
      const envelope = call("envelope", { chosen: wire, active, input: chosen.input, deadline: Math.max(plan.deadline - spent, 0) })
      const what = `running ${chosen.reflex}`
      const body = bodies[chosen.reflex]
      if (body !== undefined) return inline(what, body, envelope, options.signal)
      const dir = dirs[chosen.reflex]
      if (dir === undefined) throw refused(chosen.reflex, `${chosen.reflex} has no body to run`, { type: "sync" }, "run(d)")
      if (typeof active.run === "string") return child(what, dir, { ...envelope, run: active.run }, options.signal)
      const argv = call("argv", { chosen: wire, active }, "run(d)")
      return program(what, argv, envelope, options.signal)
    },

    async handle(input, options = {}) {
      let decision = await project.decide(input, options)
      let refusals = 0
      for (;;) {
        switch (decision.outcome) {
          case "abstain":
            return { outcome: "abstained", decision }
          case "ask": {
            if (options.ask === undefined) return { outcome: "unanswered", decision }
            const given = await options.ask(decision)
            if (given === undefined) return { outcome: "declined", decision }
            let filled: Decision<AnyReflexes>
            try {
              filled = project.fill(decision, given)
            } catch (error) {
              // An answer that does not read is asked again once, as at the terminal; twice is a decline.
              if (!(error instanceof DiagnosticError) || refusals++ > 0) throw error
              continue
            }
            // An answer that leaves the ask exactly as it was is a decline, so a handler that never answers ends.
            if (filled.outcome === "ask" && same(filled.missing, decision.missing)) return { outcome: "declined", decision: filled }
            decision = filled
            continue
          }
          case "confirm": {
            if (options.confirm === undefined) return { outcome: "unanswered", decision }
            if (!(await options.confirm(decision))) return { outcome: "declined", decision }
            const result = await project.run(decision, { confirmed: true, ...(options.signal === undefined ? {} : { signal: options.signal }) })
            return { outcome: "ran", decision, result }
          }
          case "run": {
            const result = await project.run(decision, options.signal === undefined ? {} : { signal: options.signal })
            return { outcome: "ran", decision, result }
          }
        }
      }
    },
  }

  /** A decision made under this plan; another plan's is misuse — the wrong tenant's project, or words that changed. */
  function own<D extends { plan: string }>(decision: D, what: string): D {
    if (decision.plan !== plan.digest) {
      throw new TypeError(`the decision was made under another plan; ${what} it on the project that decided it`)
    }
    return decision
  }

  return project
}

/** Whether two asks want the same things for the same reasons. */
function same(after: readonly W.Missing[], before: readonly W.Missing[]): boolean {
  return JSON.stringify(after) === JSON.stringify(before)
}

/** The wire decision with the SDK's fields: input, plan, trace, and the plain values of a chosen or asking call. */
function lined(decision: W.Decision, line: Line): Decision<AnyReflexes> {
  if (decision.outcome === "abstain") return { ...decision, ...line } as Abstain
  return { ...decision, values: call("values", { args: decision.args }), ...line } as unknown as Decision<AnyReflexes>
}

/** What a person answered, as the value the ask offered: a key among the options, a word among the words, a pick
 *  through its recognizer; anything else is refused naming the argument and what it may be. */
function typed(missing: W.Missing, answer: string, invoked: string): W.Value {
  const { choices } = missing
  const text = answer.trim()
  const fix = { type: "rerun" } as const
  switch (choices.type) {
    case "options": {
      if (Object.hasOwn(choices.options, text)) return { type: "option", key: text }
      throw refused(undefined, `${missing.arg}: ${JSON.stringify(answer)} is not one of ${Object.keys(choices.options).join(", ")}`, fix, invoked)
    }
    case "vocab": {
      if (Object.hasOwn(choices.words, text)) return { type: "word", word: text }
      throw refused(undefined, `${missing.arg}: ${JSON.stringify(answer)} is not one of ${Object.keys(choices.words).join(", ")}`, fix, invoked)
    }
    case "pick": {
      const picked = call("picked", { text, recognizer: choices.pick })
      if (picked !== null) return picked
      throw refused(undefined, `${missing.arg}: ${JSON.stringify(answer)} is not ${wants(choices.pick)}`, fix, invoked)
    }
  }
}

/** What a recognizer reads, in the core's words. */
function wants(recognizer: W.Recognizer): string {
  switch (recognizer) {
    case "number":
      return "a number"
    case "duration":
      return "a duration"
    case "email":
      return "an email address"
    case "url":
      return "a URL"
    case "quoted":
      return "text"
  }
}
