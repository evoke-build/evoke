// A project: the owned files under a root, reflexes handed as code, and who answers — read, verified and compiled
// once; never fetched, never written. On it: `with`, the same project over a vocabulary of the call's own — a
// tenant's words, their own plan in milliseconds; `decide`, one input asked, answered, read and gated; `fill`, an
// ask answered and gated again; `run`, the chosen call's body; `handle`, the whole loop; `steps`, one request
// read into steps and sealed as a plan file — the sentence, the answers, the pins — and `weave`, the steps run
// stage by stage under the same handlers, from a sentence or from such a file, whose pins are checked and whose
// plan is made again from its answers. In: LoadOptions; inputs; decisions; plan files. Out: a Project, decisions,
// results, plans.

import { realpathSync, statSync } from "node:fs"
import { homedir } from "node:os"

import { type Adapter, type Trace, answered } from "./adapter.ts"
import { PLATFORM, facts, scratch, status } from "./contain.ts"
import { type Ok, bug, call, command, fromCode, misnamed, problem, reply } from "./core.ts"
import type { Abstain, AnyReflexes, Ask, Confirm, Decision, Given, Handled, Line, Run } from "./decision.ts"
import { DiagnosticError, FailureError, type Problem } from "./errors.ts"
import { entry, snapshot, text } from "./files.ts"
import type { Op, Ops } from "./ops.ts"
import type { Inline } from "./reflex.ts"
import { type Reflex, type Result, Refusal, child, inline, program, resolved, today } from "./runtime.ts"
import type * as W from "./types.ts"

export interface LoadOptions<R = AnyReflexes> {
  /** The project directory: evoke.toml, evoke.lock, overlays/, vocab/, local reflexes. Absent: no files at all. */
  root?: string | undefined
  /** Reflexes handed as code, by local name; a name evoke.toml also lists is an error. Overlays under root apply.
   *  R is inferred from this map when no root is given; with a root, write `load<Reflexes & ReflexesOf<typeof own>>`. */
  reflexes?: { [K in keyof R]?: Inline<R[K]> | undefined } | undefined
  /** Who answers. Absent: the adapter evoke.toml names, from its own subpath; required without a root. */
  adapter?: Adapter | undefined
  /** The platform a manifest's `platforms` is judged by: this machine's when absent. A reflex named for another
   *  is inactive; name one to decide or test as that machine would. A body still runs here. */
  platform?: W.Platform | undefined
}

export interface DecideOptions {
  /** Only reflexes carrying one of these tags are offered. */
  tags?: string[] | undefined
  /** This reflex alone is offered: how a weave decides a fragment, or a step with a value written into its words. */
  only?: string | undefined
  /** Aborts the adapter call; `decide` rejects with the signal's reason. A weave's run it cancels: every body
   *  ended, every step that did not finish `skipped · cancelled`, the rejection's reason carrying the record. */
  signal?: AbortSignal | undefined
  /** The process's results, newest first — a reflex and the `data` its body returned — for an ask to offer back
   *  the values of the field a `recent` pick names, when the words state none. A whole sentence's ask alone: a
   *  step's, or a part's, recalls nothing; `steps` seals a plan from the words alone and takes none. */
  recent?: W.Recent[] | undefined
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

/** Which step of a weave, and which of its rounds, a handler is asked for. */
export interface Turn {
  /** From 1, as the plan numbers them. */
  step: number
  /** From 0; a step bound to a list runs one round per record. */
  round: number
}

/** What `weave` takes beside the decision's options: `handle`'s handlers, each told which step asks, and one
 *  more for the plan itself. */
export interface WeaveOptions<R = AnyReflexes> extends DecideOptions {
  /** Asked at a step's confirm: `true` runs, `false` declines the step, which ends the weave after its stage.
   *  Absent and reached: the step is `unanswered`. */
  confirm?: ((decision: Confirm<R>, turn: Turn) => boolean | Promise<boolean>) | undefined
  /** Asked at a step's ask — before anything runs for a required argument no binding covers, else at the step's
   *  turn — as `handle` asks one. Absent and reached: `unanswered`. */
  ask?: ((decision: Ask<R>, turn: Turn) => Given<Ask<R>> | undefined | Promise<Given<Ask<R>> | undefined>) | undefined
  /** Asked when the plan holds at confirm — a step refers to another whose result it takes nothing from: `true`
   *  runs the plan as it stands. Absent and reached: nothing runs. */
  proceed?: ((plan: W.Weave) => boolean | Promise<boolean>) | undefined
}

/** What `weave` takes for a plan read from a file — a `Pinned`, as `steps` returned it or as `JSON.parse` read it
 *  back — beside a step's handlers: the yes over the whole plan, asked whatever its verdict, since a file's numbers
 *  are whoever wrote them. Absent and reached: nothing runs, `unanswered`. The tags are the file's. */
export interface PinnedOptions<R = AnyReflexes> {
  confirm?: WeaveOptions<R>["confirm"]
  ask?: WeaveOptions<R>["ask"]
  proceed?: ((pinned: W.Pinned) => boolean | Promise<boolean>) | undefined
  signal?: AbortSignal | undefined
}

/** One round of a step as it ran: the decision the loop settled, the input its body saw, what became of it. */
export interface WovenRound<R = AnyReflexes> {
  round: number
  input: string
  decision: Decision<R>
  status: W.Status
  why?: W.WeaveWhy | undefined
  result?: Result | undefined
}

/** What became of one step of a weave. */
export interface WovenStep<R = AnyReflexes> {
  step: number
  status: W.Status
  why?: W.WeaveWhy | undefined
  bound: W.Bound[]
  rounds: WovenRound<R>[]
}

/** A request planned and run: the plan; the whole's status — the worst step's, as the exit codes rank them, or
 *  why nothing ran: refused by the verdict, declined or unanswered at what the plan asked first; per step what
 *  became of it, none when nothing ran. A weave the signal cancelled rejects with the signal's reason and carries
 *  this record on it as `woven`: the steps that finished as they ended, every other one `skipped · cancelled`. */
export interface Woven<R = AnyReflexes> {
  plan: W.Weave
  status: W.Status
  steps: WovenStep<R>[]
}

/** Vocabularies in the file's form: per name, word → what it means, or `{ what, value? }`. */
export type Vocab = Record<string, Record<string, string | { what: string; value?: string | undefined }>>

/** A reflex the project holds: active, with its effect and how it runs — a playbook runs as a plan; or inactive,
 *  with each problem and its fix. */
export type ReflexStatus =
  | { active: true; effect: W.Effect; runs: "inline" | "file" | "argv" | "plan" }
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
  /** One request read into its steps, each decided as `decide` decides one, ordered by the words, a result of
   *  one threaded into a later one, sealed as a plan file: the sentence, the plan with its verdict before anything
   *  runs, every answer the plan took and the pins they were gathered under. `JSON.stringify` writes it; `weave`
   *  takes it, here or on another machine with the same project. With `ask`, what the plan asks first — a
   *  playbook's slot the sentence does not state — is answered before the plan is sealed, so the plan shows whole;
   *  such a plan is a sheet, not a file: `weave(pinned)` refuses it, since a file holds no one's answers, and the
   *  page runs it by its sentence. Runs nothing. */
  steps(input: string, options?: Omit<DecideOptions, "recent"> & Pick<WeaveOptions<R>, "ask">): Promise<W.Pinned>
  /** The steps, what the plan asks first answered, then every step run stage by stage under the same handlers. */
  weave(input: string, options?: WeaveOptions<R>): Promise<Woven<R>>
  /** A plan file run exactly: its pins checked against this project — a `DiagnosticError` names the first that
   *  moved and its fix — the plan made again from the file's answers under the file's gate and refused when it does
   *  not read the same, `proceed` asked, then every step run under the same handlers, a value bound at run time
   *  decided by this project's adapter. */
  weave(pinned: W.Pinned, options?: PinnedOptions<R>): Promise<Woven<R>>
}

/** The home project as the CLI writes it on first use: what a root without evoke.toml means. */
const DEFAULT: W.Project = { adapter: "jev", reflexes: {}, config: {}, adapters: {} }

/** What a project is made of, before its plan: the installed set, the plain settings' values beside it, the
 *  project and its lock as the pins read them, who answers, where each body lives, each shipped manifest and which
 *  reflexes are local — whose declaration is their own file's. */
interface Ground {
  installed: W.Installed
  values: W.Values
  project: W.Project
  lock: W.Lock | undefined
  adapter: Adapter
  /** The platform the plan judges a manifest's `platforms` by. */
  platform: W.Platform
  dirs: Record<string, string>
  bodies: Record<string, Reflex<Record<string, unknown>, Record<never, string>>>
  shipped: Record<string, W.Manifest>
  local: Set<string>
}

/** A project from its root: its installed reflexes are what `Reflexes` from evoke.d.ts names, so R is written. */
export function load<R extends object = AnyReflexes>(
  options: { root: string; reflexes?: NoInfer<{ [K in keyof R]?: Inline<R[K]> | undefined }> | undefined; adapter?: Adapter | undefined; platform?: W.Platform | undefined },
): Promise<Project<R>>
/** A project from reflexes handed as code alone: R is inferred from them. */
export function load<R extends object = AnyReflexes>(
  options: { root?: undefined; reflexes?: { [K in keyof R]?: Inline<R[K]> | undefined } | undefined; adapter: Adapter; platform?: W.Platform | undefined },
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
  const values: W.Values = {}
  const dirs: Record<string, string> = {}
  const bodies: Ground["bodies"] = {}
  const shipped: Record<string, W.Manifest> = {}
  const local = new Set<string>()
  const given = (options.reflexes ?? {}) as Record<string, Inline | undefined>
  for (const name of Object.keys(given)) {
    const why = misnamed(name, "local")
    if (why !== undefined) throw refused(undefined, why, { type: "rerun" }, "load({ reflexes })")
  }
  for (const [name, location] of Object.entries(project.reflexes)) {
    if (given[name] !== undefined) {
      throw refused(name, `${name} is both in evoke.toml and passed to load`, { type: "remove", reflex: name })
    }
    const configured = held(project.config[name] ?? {})
    for (const [key, setting] of Object.entries(project.config[name] ?? {})) {
      if (setting.type === "plain") (values[name] ??= {})[key] = setting.value
    }
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
      reflexes[name] = {
        wording: worded.wording,
        consented: worded.manifest?.effect ?? "destructive",
        ...(worded.manifest?.needs === undefined ? {} : { needs: worded.manifest.needs }),
        configured,
      }
      dirs[name] = dir
      if (worded.manifest !== undefined) shipped[name] = worded.manifest
      local.add(name)
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
    reflexes[name] = { wording: worded.wording, consented: locked.effect, ...(locked.needs === undefined ? {} : { needs: locked.needs }), configured }
    dirs[name] = dir
    if (worded.manifest !== undefined) shipped[name] = worded.manifest
  }
  for (const [name, handed] of Object.entries(given)) {
    if (handed === undefined) continue
    const worded = word(name, { file: { type: "manifest", name }, json: { reflex: 1, ...handed.manifest } }, files.overlays[name])
    // The manifest is the app's own code: its problems are the app's to fix now, not a reflex to leave inactive.
    if ("err" in worded.wording && worded.manifest === undefined) throw fromCode(worded.wording.err, handed.body === undefined ? `playbook(${name})` : `reflex(${name})`)
    reflexes[name] = { wording: worded.wording, consented: worded.manifest?.effect ?? "destructive", configured: {} }
    // A playbook has no body: it expands inside a plan.
    if (handed.body !== undefined) bodies[name] = handed.body
  }
  const vocab: Record<string, W.Vocabulary> = {}
  for (const [name, toml] of Object.entries(files.vocab)) {
    vocab[name] = call("vocabulary", { doc: { file: { type: "vocab", name }, toml } }, "load()")
  }
  const installed: W.Installed = { reflexes, vocab, adapter: adapter.id, evoke: call("version", {}) }
  const platform = options.platform ?? PLATFORM
  return make({ installed, values, project, lock, adapter, platform, dirs, bodies, shipped, local }, "load()") as unknown as Project<R>
}

/** The adapter evoke.toml names, from its own subpath; a recording is never resolved by name. */
async function named(project: W.Project, root: string | undefined): Promise<Adapter> {
  const name = project.adapter
  const line = (message: string, adapter: string) => {
    const form = root === undefined ? `load({ reflexes, adapter: ${adapter} })` : `load({ root, adapter: ${adapter} })`
    return Promise.reject(new DiagnosticError([{ message, fix: { type: "rerun" }, command: form }]))
  }
  if (root === undefined) return line("no adapter: none is named and none was passed", "jev()")
  if (name === "jev" || name === "openjev") {
    const { through } = await import("./systemone.ts")
    return through(name, { table: project.adapters[name] })
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

/** How each configured key is held: plain, its value handed beside the set, or a variable and whether it is set —
 *  never a value, never a secret. */
function held(settings: Record<string, W.Setting>): Record<string, W.Held> {
  const held: Record<string, W.Held> = {}
  for (const [key, setting] of Object.entries(settings)) {
    held[key] = setting.type === "plain" ? { type: "plain" } : { type: "env", var: setting.var, set: process.env[setting.var] !== undefined }
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
  return `decide(${JSON.stringify(shown(input))})`
}

/** An input with nothing in it never reaches the adapter: refused, with the call to make. */
function nonEmpty(input: string, what: string): void {
  if (input.trim() === "") throw refused(undefined, "the input is empty", { type: "rerun" }, `${what}("<input>")`)
}

/** An input as a fix line shows it: elided past sixty characters. */
function shown(input: string): string {
  return input.length > 60 ? `${input.slice(0, 59)}…` : input
}

/** What was asked, as one key: the text, the tags, the one reflex. */
function key(asked: W.Asked): string {
  return JSON.stringify([asked.text, asked.tags ?? [], asked.only ?? null, asked.whole ?? false])
}

/** What the planner asked to decide a step, as its repair and its shared words tell: a fragment narrowed to its
 *  neighbour's reflex, or spliced into its words, was decided under that reflex alone, and so were words a shared word
 *  was written into; any other step over the tags, whole when its words are the whole request. */
function askedFor(step: W.Step, tags: string[], input: string): W.Asked {
  const narrowed = step.repair === "narrowed" || step.repair === "spliced" || Object.values(step.shared ?? {}).some(shared => shared.via === "rewrite")
  const own = narrowed ? step.reflex : undefined
  if (own !== undefined) return { text: step.text, only: own }
  return step.text === input.trim() ? { text: step.text, tags, whole: true } : { text: step.text, tags }
}

/** One text read for the plan: its decision, and its answers as the plan file keeps them. */
interface Read {
  decision: W.Decision
  entry: W.Answer
}

/** The answers a plan gathers: what it decided is always there to push to. */
type Gathered = W.Answers & { decided: [W.Asked, W.Decision][] }

/** Everything a plan gathers on its way: the answers the planner reads, each text's trace by what was asked, and
 *  every engine answer as the plan file keeps them — the weave's own questions under the request's text. */
interface Gathering {
  answers: Gathered
  traces: Map<string, Trace[]>
  entries: W.Answer[]
}

function gathering(): Gathering {
  return { answers: { decided: [] }, traces: new Map(), entries: [] }
}

/** What became of one round: its status, why it stopped, what its body returned. */
interface Became {
  status: W.Status
  why?: W.WeaveWhy | undefined
  result?: Result | undefined
}

/** A decision taken to the point of running, or where it stopped. */
type Readied =
  | { ready: true; decision: Run<AnyReflexes> | Confirm<AnyReflexes> }
  | { ready: false; status: "refused"; decision: Abstain; why: W.WeaveWhy }
  | { ready: false; status: "declined" | "unanswered"; decision: Ask<AnyReflexes> | Confirm<AnyReflexes>; why: W.WeaveWhy }

/** The project over its ground: the set compiled, every reflex's status read off the plan. The implementation
 *  speaks the wire's shapes; the app's R lives on the interface alone. */
function make(ground: Ground, invoked: string): Project<AnyReflexes> {
  const { installed, values, project: owned, lock, adapter, platform, dirs, bodies, shipped, local } = ground
  const plan: W.Plan = call("compile", { set: installed, values, platform, ...(adapter.limits === undefined ? {} : { limits: adapter.limits }) }, invoked)
  if (adapter.plan !== undefined && adapter.plan !== plan.digest) {
    const message = `the recording was made against plan ${adapter.plan}, not ${plan.digest}`
    throw new DiagnosticError([{ message, fix: { type: "rerun" }, command: "replay(file, { record: jev() })" }])
  }
  const reflexes: Record<string, ReflexStatus> = {}
  for (const name of Object.keys(installed.reflexes)) {
    const active = plan.active[name]
    if (active !== undefined) {
      const runs = (active.steps?.length ?? 0) > 0 ? "plan" : bodies[name] !== undefined ? "inline" : Array.isArray(active.run) ? "argv" : "file"
      reflexes[name] = { active: true, effect: active.effect, runs }
    } else {
      reflexes[name] = { active: false, problems: (plan.inactive[name] ?? []).map(diagnostic => problem(diagnostic, invoked)) }
    }
  }
  const gate = adapter.gate === undefined ? {} : { gate: adapter.gate }
  const declared: W.Declared = { id: adapter.id, ...(adapter.limits === undefined ? {} : { limits: adapter.limits }), ...gate }
  const locked = lock === undefined ? {} : { lock }
  const project: Project<AnyReflexes> = {
    reflexes,
    plan: plan.digest,

    with({ vocab }) {
      const replaced = { ...installed.vocab }
      for (const [name, words] of Object.entries(vocab)) {
        const why = misnamed(name, "vocab")
        if (why !== undefined) throw new DiagnosticError([{ message: why, fix: { type: "rerun" }, command: "with({ vocab })" }])
        const read = reply("vocabulary", { doc: { file: { type: "vocab", name }, json: words } })
        if ("bug" in read) throw bug(read.bug)
        if ("err" in read) throw fromCode(read.err as W.Diagnostic[], `with({ vocab: { ${name} } })`)
        replaced[name] = read.ok as W.Vocabulary
      }
      return make({ ...ground, installed: { ...installed, vocab: replaced } }, "with({ vocab })")
    },

    async decide(input, options = {}) {
      nonEmpty(input, "decide")
      return (await decidedText(input, options, true)).decision
    },

    fill(decision, given) {
      const asking = own(decision, "fill")
      const invoked = `fill(d, ${JSON.stringify(given)})`
      const wanted = asking.missing.map(missing => missing.arg)
      const values: Record<string, W.Value> = {}
      for (const [name, answer] of Object.entries(given as Record<string, unknown>)) {
        if (answer === undefined) continue
        const missing = asking.missing.find(missing => missing.arg === name)
        if (missing === undefined) {
          throw refused(undefined, `${name} is not being asked; the ask wants ${wanted.join(", ")}`, { type: "rerun" }, invoked)
        }
        values[name] = typed(missing, String(answer), invoked)
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
      // A whole result is handed by the plan alone: a call that takes one runs only as a step of a weave.
      taker(chosen)
      return running(chosen, {}, options)
    },

    async handle(input, options = {}) {
      nonEmpty(input, "handle")
      const decision = await project.decide(input, options)
      if (decision.outcome !== "abstain") taker(decision)
      const readied = await ready(decision, options)
      if (readied.ready) return { outcome: "ran", decision: readied.decision, result: await running(readied.decision, {}, signalled(options.signal)) }
      if (readied.status === "refused") return { outcome: "abstained", decision: readied.decision }
      return { outcome: readied.status, decision: readied.decision }
    },

    async steps(input, options = {}) {
      nonEmpty(input, "steps")
      const gathered = gathering()
      let weave = await planned(input, options, gathered)
      // What the plan asks first, answered when a handler is given: the plan is made again over the answer, so a
      // playbook whose slot the sentence lacks expands and the plan shows whole.
      while (options.ask !== undefined && weave.verdict.outcome === "ask") {
        const filled = await askedUpFront(weave, gathered.answers, options, gathered.traces)
        if (filled !== "filled") break
        const again = await planned(input, options, gathered)
        if (JSON.stringify(again.verdict) === JSON.stringify(weave.verdict)) break
        weave = again
      }
      return call("weave.pin", { installed, plan, project: owned, ...locked, declared, input, tags: options.tags ?? [], weave, answers: gathered.entries })
    },

    async weave(input: string | W.Pinned, options: WeaveOptions<AnyReflexes> | PinnedOptions<AnyReflexes> = {}) {
      if (typeof input !== "string") return pinnedRun(input, options as PinnedOptions<AnyReflexes>)
      const handlers = options as WeaveOptions<AnyReflexes>
      nonEmpty(input, "weave")
      const gathered = gathering()
      let woven = await planned(input, handlers, gathered)
      // What the plan asks before anything runs: a step's own question, answered, and the plan made again.
      while (woven.verdict.outcome === "ask") {
        const filled = await askedUpFront(woven, gathered.answers, handlers, gathered.traces)
        if (filled !== "filled") return { plan: woven, status: filled, steps: [] }
        const again = await planned(input, handlers, gathered)
        // A plan that asks the same again could not take the answer: unanswered, never a loop.
        if (JSON.stringify(again.verdict) === JSON.stringify(woven.verdict)) return { plan: again, status: "unanswered", steps: [] }
        woven = again
      }
      if (woven.verdict.outcome === "refuse") return { plan: woven, status: "refused", steps: [] }
      if (woven.verdict.outcome === "confirm") {
        if (handlers.proceed === undefined) return { plan: woven, status: "unanswered", steps: [] }
        if (!(await handlers.proceed(woven))) return { plan: woven, status: "declined", steps: [] }
      }
      return executed(woven, handlers, gathered)
    },
  }

  /** One text asked, answered, read and gated, its raw answers kept beside the decision — for the plan file. The
   *  text's questions are asked, then each round of questions its answers open, until nothing is left to ask. A
   *  text decided `alone`, with no plan around it, is held as a plan holds a step no other step repeats. */
  async function decidedText(input: string, options: DecideOptions, alone = false): Promise<{ decision: Decision<AnyReflexes>; raw: W.Raw }> {
    const invoked = invocation(input)
    const recent = options.recent === undefined || options.recent.length === 0 ? {} : { recent: options.recent }
    const request = call("request", { plan, input, tags: options.tags ?? [], ...(options.only === undefined ? {} : { only: options.only }), scope: "full", ...recent }, invoked)
    const raw: W.Raw = {}
    const traces: Trace[] = []
    for (let round = request; ; ) {
      const answer = await answered(adapter, round, plan.deadline, options.signal, invoked)
      Object.assign(raw, answer.raw)
      traces.push(answer.trace)
      const read = call("read", { plan, ...gate, request, raw }, invoked)
      if (read.type === "done") {
        const { type: _, ...reading } = read
        const decision = call("gate", { plan, reading, ...gate, ...(alone ? { alone } : {}) })
        return { decision: lined(decision, { input, plan: plan.digest, trace: traces }), raw }
      }
      round = read.request
      Object.assign(request.questions, round.questions)
    }
  }

  /** The plan over the answers gathered so far: the adapter asked and texts decided until it stands, every answer
   *  kept as an entry of the plan file. A text the plan sends ahead of the cut is decided from the cut's round on
   *  and taken when the plan asks for it, or at the end, so that the plan file holds its answers whatever the plan
   *  made of it. */
  async function planned(input: string, options: DecideOptions, gathered: Gathering): Promise<W.Weave> {
    const invoked = `steps(${JSON.stringify(shown(input))})`
    const { answers, entries } = gathered
    const ahead = new Map<string, Promise<Read>>()
    // A text's decision, started ahead or now; its entry lands in the plan file in the order the plan took it.
    const deciding = async (asked: W.Asked): Promise<[W.Asked, W.Decision]> => {
      const started = ahead.get(key(asked))
      ahead.delete(key(asked))
      const read = await (started ?? decided(asked, options, gathered))
      entries.push(read.entry)
      return [asked, read.decision]
    }
    for (;;) {
      const planning = call("weave.plan", { plan, ...gate, input, tags: options.tags ?? [], answers }, invoked)
      if (planning.type === "done") {
        for (const read of await Promise.all(ahead.values())) entries.push(read.entry)
        return planning.weave
      }
      const { need } = planning
      if (need.type === "decide") {
        // Side by side: each text is its own adapter call.
        answers.decided.push(...(await Promise.all(need.asked.map(deciding))))
        continue
      }
      if (need.type === "judge") {
        for (const asked of need.ahead ?? []) {
          const started = decided(asked, options, gathered)
          // Its failure reaches whoever takes it, or the end; unheard, it would be an unhandled rejection.
          started.catch(() => undefined)
          ahead.set(key(asked), started)
        }
      }
      const { raw } = await answered(adapter, need.request, plan.deadline, options.signal, invoked)
      entries.push({ text: need.request.state.request, raw })
      if (need.type === "judge") answers.judged = raw
      else if (need.type === "refer") answers.referred = raw
      else answers.verified = { ...answers.verified, ...raw }
    }
  }

  /** One text decided as the plan asks — over the tags, or one reflex alone — its trace kept by what was asked and
   *  its raw answers as an entry for the caller to keep; the process's results reach the whole request's text
   *  alone. */
  async function decided(asked: W.Asked, options: DecideOptions, gathered: Gathering): Promise<Read> {
    const { decision, raw } = await decidedText(asked.text, {
      ...(asked.only === undefined ? { tags: asked.tags ?? [] } : { only: asked.only }),
      ...(options.signal === undefined ? {} : { signal: options.signal }),
      ...(asked.whole === true && options.recent !== undefined ? { recent: options.recent } : {}),
    })
    gathered.traces.set(key(asked), decision.trace)
    // The whole decision crosses: the core reads its own fields and ignores the SDK's.
    return { decision: decision as unknown as W.Decision, entry: { text: asked.text, raw } }
  }

  /** A plan file run: read through the core, `plan = 1` required; its pins against this project, the first that
   *  moved refused; the plan made again from its answers under its gate, refused when it does not read the same;
   *  a plan that asks or refuses runs nothing, since a file holds no one's answers; then the yes over the whole
   *  plan, and every step under the handlers, a value bound at run time decided by this project's adapter. */
  async function pinnedRun(json: W.Pinned, options: PinnedOptions<AnyReflexes>): Promise<Woven<AnyReflexes>> {
    const path = "plan"
    const sentence = typeof json === "object" && json !== null && typeof json.input === "string" ? json.input : "<input>"
    const pinned = planFile("weave.pinned", { path, json }, sentence)
    planFile("weave.stale", { path, pinned, installed, plan, ...locked, declared }, pinned.input)
    const replanned = planFile("weave.replan", { path, pinned, plan }, pinned.input)
    const woven = replanned.weave
    if (woven.verdict.outcome === "ask") return { plan: woven, status: "unanswered", steps: [] }
    if (woven.verdict.outcome === "refuse") return { plan: woven, status: "refused", steps: [] }
    if (options.proceed === undefined) return { plan: woven, status: "unanswered", steps: [] }
    if (!(await options.proceed(pinned))) return { plan: woven, status: "declined", steps: [] }
    const handlers: WeaveOptions<AnyReflexes> = {
      tags: pinned.tags ?? [],
      ...(options.confirm === undefined ? {} : { confirm: options.confirm }),
      ...(options.ask === undefined ? {} : { ask: options.ask }),
      ...(options.signal === undefined ? {} : { signal: options.signal }),
    }
    return executed(woven, handlers, gathering())
  }

  /** One op over a plan file: its refusal a `DiagnosticError` whose save line reads as the call that makes the
   *  plan again here. */
  function planFile<O extends Op>(op: O, input: Ops[O]["input"], sentence: string): Ok<O> {
    const answer = reply(op, input)
    if ("ok" in answer) return answer.ok as Ok<O>
    if ("bug" in answer) throw bug(answer.bug)
    const diagnostic = answer.err as W.Diagnostic
    const fixed = diagnostic.fix.type === "save" ? `steps(${JSON.stringify(shown(sentence))})` : command(diagnostic.fix)
    throw new DiagnosticError([{ ...diagnostic, command: fixed }])
  }

  /** The plan's own questions before anything runs. A step's required argument no binding covers is asked as
   *  `handle` asks one — the ask narrowed to what nothing binds — and the step's decision replaced for the plan
   *  to stand again. Several fields, or one record of several, no handler can answer: unanswered. */
  async function askedUpFront(woven: W.Weave, answers: Gathered, options: WeaveOptions<AnyReflexes>, traces: Map<string, Trace[]>): Promise<"filled" | "declined" | "unanswered"> {
    const because = woven.verdict.because ?? []
    if (options.ask === undefined || because.some(b => b.type === "several" || b.type === "one_of_many")) return "unanswered"
    for (const n of new Set(because.flatMap(b => (b.type === "needs" ? [b.step] : [])))) {
      const step = woven.steps[n - 1]
      if (step === undefined || step.decision.outcome !== "ask") continue
      const bound = new Set((woven.binds ?? []).filter(b => b.to === n).map(b => b.arg))
      const asked = askedFor(step, options.tags ?? [], woven.input)
      let decision = lined(step.decision, { input: step.text, plan: plan.digest, trace: traces.get(key(asked)) ?? [] })
      let refusals = 0
      while (decision.outcome === "ask") {
        const unbound = decision.missing.filter(m => !bound.has(m.arg))
        if (unbound.length === 0) break
        const narrowed = { ...decision, missing: unbound as W.NonEmpty<W.Missing> }
        const given = await options.ask(narrowed, { step: n, round: 0 })
        if (given === undefined) return "declined"
        let filled: Decision<AnyReflexes>
        try {
          filled = project.fill(narrowed, given)
        } catch (error) {
          if (!(error instanceof DiagnosticError)) throw error
          // An answer that does not read is asked again once; twice is a decline.
          if (refusals++ > 0) return "declined"
          continue
        }
        if (filled.outcome === "ask" && same(filled.missing, decision.missing)) return "declined"
        decision = filled
      }
      for (const entry of answers.decided) if (key(entry[0]) === key(asked)) entry[1] = decision as unknown as W.Decision
    }
    return "filled"
  }

  /** The plan run: stage by stage, a stage's rounds each taken to the point of running in the words' order, then
   *  their bodies together; a step decided again with its bound values in its words when the run asks for it. The
   *  signal aborted mid-run is a cancel: every body's group ended, the rounds under way and every round after
   *  reported `skipped · cancelled`, and the rejection — the signal's reason — carrying the record once it is
   *  whole. */
  async function executed(woven: W.Weave, options: WeaveOptions<AnyReflexes>, gathered: Gathering): Promise<Woven<AnyReflexes>> {
    const invoked = `weave(${JSON.stringify(shown(woven.input))})`
    const { signal } = options
    const { traces } = gathered
    const progress: Required<W.Progress> = { decided: [], handled: [] }
    const rounds = new Map<number, WovenRound<AnyReflexes>[]>()
    const record = (handling: W.Handling, decision: Decision<AnyReflexes>, became: Became) => {
      const { status, why, result } = became
      const said = { ...(why === undefined ? {} : { why }), ...(result === undefined ? {} : { result: result as W.Returned }) }
      progress.handled.push({ step: handling.step, round: handling.round, status, ...said })
      const list = rounds.get(handling.step) ?? []
      list.push({ round: handling.round, input: handling.input, decision, status, ...said })
      rounds.set(handling.step, list)
    }
    for (;;) {
      const running = call("weave.execute", { plan, ...gate, weave: woven, progress }, invoked)
      if (running.type === "done") {
        const { executed } = running
        const whole: Woven<AnyReflexes> = {
          plan: woven,
          status: executed.worst,
          steps: executed.steps.map(step => ({
            step: step.step,
            status: step.status,
            ...(step.why === undefined ? {} : { why: step.why }),
            bound: step.bound ?? [],
            rounds: rounds.get(step.step) ?? [],
          })),
        }
        if (signal?.aborted) throw carrying(signal.reason, whole)
        return whole
      }
      const { todo } = running
      if (todo.type === "decide") {
        try {
          const read = await decided(todo.asked, options, gathered)
          gathered.entries.push(read.entry)
          progress.decided.push([todo.asked, read.decision])
        } catch (error) {
          // The signal aborted the decision's call: the round it was for never starts.
          if (!signal?.aborted || error !== signal.reason) throw error
          progress.handled.push({ step: todo.step, round: todo.round, ...CANCELLED })
        }
        continue
      }
      const bodies: Promise<void>[] = []
      for (const handling of todo.handling) {
        const step = woven.steps[handling.step - 1]
        // A round decided again with its values in its words has its own trace; any other round has its step's.
        const own = step?.reflex === undefined || !handling.bound?.length ? undefined : traces.get(key({ text: handling.input, only: step.reflex }))
        const trace = own ?? (step === undefined ? undefined : traces.get(key(askedFor(step, options.tags ?? [], woven.input)))) ?? []
        const decision = lined(handling.decision, { input: handling.input, plan: plan.digest, trace })
        // Nothing starts after the signal: a round handed after it is cancelled without a question asked.
        if (signal?.aborted) {
          record(handling, decision, CANCELLED)
          continue
        }
        const readied = await ready(decision, told(options, { step: handling.step, round: handling.round }))
        if (!readied.ready) {
          record(handling, readied.decision, { status: readied.status, why: readied.why })
          continue
        }
        bodies.push(ran(readied.decision, handling.taken ?? {}, signal).then(became => record(handling, readied.decision, became)))
      }
      await Promise.all(bodies)
    }
  }

  /** The foundation's loop up to the run, as `handle` takes a decision: ask and fill until nothing is missing,
   *  then confirm. What stands ready to run, or where it stopped and why. */
  async function ready(start: Decision<AnyReflexes>, options: Handlers<AnyReflexes>): Promise<Readied> {
    let decision = start
    let refusals = 0
    for (;;) {
      switch (decision.outcome) {
        case "abstain":
          return { ready: false, status: "refused", decision, why: { type: "no_reflex" } }
        case "ask": {
          const why = { type: "said", message: decision.missing.map(m => m.arg).join(", ") } as const
          if (options.ask === undefined) return { ready: false, status: "unanswered", decision, why }
          const given = await options.ask(decision)
          if (given === undefined) return { ready: false, status: "declined", decision, why }
          let filled: Decision<AnyReflexes>
          try {
            filled = project.fill(decision, given)
          } catch (error) {
            if (!(error instanceof DiagnosticError)) throw error
            // An answer that does not read is asked again once; twice is a decline.
            if (refusals++ > 0) return { ready: false, status: "declined", decision, why }
            continue
          }
          // An answer that leaves the ask exactly as it was is a decline, so a handler that never answers ends.
          if (filled.outcome === "ask" && same(filled.missing, decision.missing)) return { ready: false, status: "declined", decision: filled, why }
          decision = filled
          continue
        }
        case "confirm": {
          const why = { type: "said", message: decision.prompt.own } as const
          if (options.confirm === undefined) return { ready: false, status: "unanswered", decision, why }
          if (!(await options.confirm(decision))) return { ready: false, status: "declined", decision, why }
          return { ready: true, decision }
        }
        case "run":
          return { ready: true, decision }
      }
    }
  }

  /** `weave`'s handlers as `ready` takes them: each told which step and round asks. */
  function told(options: WeaveOptions<AnyReflexes>, turn: Turn): Handlers<AnyReflexes> {
    const { confirm, ask } = options
    return {
      ...(confirm === undefined ? {} : { confirm: (decision: Confirm<AnyReflexes>) => confirm(decision, turn) }),
      ...(ask === undefined ? {} : { ask: (decision: Ask<AnyReflexes>) => ask(decision, turn) }),
    }
  }

  /** A reflex that takes a whole result, met outside a weave: refused before any confirm, since only a request of
   *  several steps hands one. A playbook too: it expands inside a plan, and has no body to run. */
  function taker(decision: { reflex: string }): void {
    const active = plan.active[decision.reflex]
    if ((active?.steps?.length ?? 0) > 0) {
      throw refused(decision.reflex, `${decision.reflex} is a plan of steps; say it in a sentence`, { type: "show", reflex: decision.reflex }, 'weave("<input>")')
    }
    const takes = Object.values(active?.takes ?? {})
    if (takes.length === 0) return
    throw refused(decision.reflex, `${decision.reflex} takes ${words(takes)}, which a step before it in the same request returns`, { type: "rerun" }, 'weave("<input>")')
  }

  /** The chosen call's body — in-process, in a child through the loader, or as an argv — with the whole results it
   *  takes beside its values, which only the weave hands; one the plan did not hand is refused by the envelope, the
   *  last guard. What `run` does for a call that takes none, and every step of a weave for its round. */
  async function running(decision: Run<AnyReflexes> | Confirm<AnyReflexes>, taken: Record<string, W.Json>, options: RunOptions): Promise<Result> {
    const chosen = own(decision, "run")
    const active = plan.active[chosen.reflex]
    if (active === undefined) throw refused(chosen.reflex, `${chosen.reflex} is not active`, { type: "rerun" }, "run(d)")
    const spent = chosen.trace.reduce((sum, entry) => sum + entry.ms, 0)
    const what = `running ${chosen.reflex}`
    // Today once per run: a relative day resolves against it at the door, and the declaration reads it the same.
    const day = today(what)
    // The whole decision crosses: the core reads the fields of a Chosen and ignores the SDK's own.
    const wire = chosen as unknown as W.Chosen
    const envelope = call("envelope", { chosen: wire, active, taken, input: chosen.input, deadline: Math.max(plan.deadline - spent, 0), home: homedir(), today: day }, "run(d)")
    const body = bodies[chosen.reflex]
    if (body !== undefined) return inline(what, body, envelope, options.signal)
    const dir = dirs[chosen.reflex]
    if (dir === undefined) throw refused(chosen.reflex, `${chosen.reflex} has no body to run`, { type: "sync" }, "run(d)")
    return contained(what, chosen, active, dir, envelope, day, options.signal)
  }

  /** One body run for a weave, the whole results its step takes beside it: what it returned, or its failure as the
   *  step's own outcome, never the weave's; a body the signal ended is the round cancelled. */
  async function ran(decision: Run<AnyReflexes> | Confirm<AnyReflexes>, taken: Record<string, W.Json>, signal: AbortSignal | undefined): Promise<Became> {
    try {
      return { status: "ran", result: await running(decision, taken, signalled(signal)) }
    } catch (error) {
      if (signal?.aborted && error === signal.reason) return CANCELLED
      if (error instanceof FailureError) {
        return { status: "failed", why: { type: "said", message: error.why === undefined ? error.what : `${error.what}: ${error.why}` } }
      }
      throw error
    }
  }

  /** A file or an argv body under its declaration: the policy resolved with the call's values, a relative day
   *  against today, the machine's facts gathered — a declared path or program it lacks is the failure, before
   *  anything runs — then the loader or the program in the body's directory with a private temporary folder, the
   *  layers around it; a refusal past the declaration names the path, the key and the fix. */
  async function contained(
    what: string,
    chosen: Run<AnyReflexes> | Confirm<AnyReflexes>,
    active: W.Active,
    dir: string,
    envelope: W.Envelope,
    day: string,
    signal: AbortSignal | undefined,
  ): Promise<Result> {
    const reflex = chosen.reflex
    const wire = chosen as unknown as W.Chosen
    const called: W.Call = { reflex, args: chosen.args, call: chosen.call }
    const config = resolved(what, envelope.config)
    const home = homedir()
    const policy: W.Policy = call("needs.resolve", { needs: active.needs ?? {}, call: called, active, config, home, today: day }, "run(d)")
    const argv = typeof active.run === "string" ? undefined : call("argv", { chosen: wire, active, home, today: day }, "run(d)")
    const body = realpathSync(dir)
    const origin = (): W.Origin => {
      if (!local.has(reflex)) return { type: "fetched" }
      const file = { type: "manifest", name: reflex } as const
      const toml = text(`${dir}/reflex.toml`) ?? ""
      return { type: "local", at: call("needs.declared_at", { doc: { file, toml } }) }
    }
    const named = (diagnostic: W.Diagnostic) => new FailureError(what, diagnostic.message, diagnostic.fix, command(diagnostic.fix, "run(d)"))
    const kind = argv === undefined ? "file" : "argv"
    const tmp = scratch()
    try {
      const gathered = facts(policy, kind, body, tmp.path)
      if ("place" in gathered || "program" in gathered) {
        const lacking: W.Lacking = "place" in gathered ? { type: "place", place: gathered.place, key: gathered.key } : { type: "program", program: gathered.program }
        throw named(call("needs.lacking", { lacking, reflex, active, origin: origin(), home }))
      }
      const layers = { policy, facts: gathered }
      let result: Result
      try {
        result =
          argv === undefined
            ? await child(what, body, { ...envelope, run: active.run as string }, layers, signal)
            : await program(what, argv, envelope, layers, signal)
      } catch (error) {
        if (!(error instanceof Refusal)) throw error
        const at = origin()
        let upstream: W.Policy | undefined
        const declared = shipped[reflex]?.needs
        if (at.type === "fetched" && declared !== undefined) {
          const answer = reply("needs.resolve", { needs: declared, call: called, active, config, home, today: day })
          if ("ok" in answer) upstream = answer.ok as W.Policy
        }
        const diagnostic = call("needs.refusal", { policy, ...(upstream === undefined ? {} : { upstream }), reflex, origin: at, refused: error.refused, home })
        if (diagnostic === null) throw new FailureError(what, error.message, { type: "rerun" }, command({ type: "rerun" }, "run(d)"))
        throw named(diagnostic)
      }
      return { ...result, contained: status(kind) }
    } finally {
      tmp.remove()
    }
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

/** A round the signal cancelled, as its step and the record read it. */
const CANCELLED = { status: "skipped", why: { type: "cancelled" } } as const satisfies Became

/** The signal's reason with the record on it as `woven`, when the reason is something that can carry one: what
 *  a cancelled `weave` rejects with, so the caller's own test for a cancel holds and the steps are there to read. */
function carrying(reason: unknown, woven: Woven<AnyReflexes>): unknown {
  if (reason !== null && (typeof reason === "object" || typeof reason === "function") && Object.isExtensible(reason)) {
    Object.assign(reason, { woven })
  }
  return reason
}

/** A signal as run options: none when there is none. */
function signalled(signal: AbortSignal | undefined): RunOptions {
  return signal === undefined ? {} : { signal }
}

/** `a`, `a and b`, `a, b and c`. */
function words(names: string[]): string {
  const last = names.at(-1) ?? ""
  return names.length < 2 ? last : `${names.slice(0, -1).join(", ")} and ${last}`
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
    case "date":
      return "a date"
    case "time":
      return "a clock time"
    case "amount":
      return "an amount"
    case "code":
      return "a code"
  }
}
