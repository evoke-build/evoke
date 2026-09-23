// The op table as types: every op's argument object and what it answers, mirroring crates/evoke-wasm/src/ops.rs
// name for name. A Result op answers { ok } | { err }; any other answers its value under ok. The vectors pin this
// table: build/vectors.ts makes every case satisfy Vector<op>.

import type * as T from "./types.ts"

/** Every op: its argument object, and its reply's ok value — a Result where the function returns one. */
export interface Ops {
  // parse
  identity: { input: { text: string }; output: T.Identity }
  manifest: { input: { doc: T.Document }; output: T.Result<T.Manifest, T.Diagnostic[]> }
  overlay: { input: { doc: T.Document; of: T.Manifest }; output: T.Result<T.Overlay, T.Diagnostic[]> }
  vocabulary: { input: { doc: T.Document }; output: T.Result<T.Vocabulary, T.Diagnostic[]> }
  project: { input: { doc: T.Document }; output: T.Result<T.Project, T.Diagnostic[]> }
  lock: { input: { doc: T.Document }; output: T.Result<T.Lock, T.Diagnostic[]> }
  render_lock: { input: { lock: T.Lock }; output: string }
  location: { input: { location: T.Location }; output: string }
  name: { input: { text: string; kind: "local" | "vocab" }; output: T.Result<string, string> }
  reference: { input: { text: string }; output: T.Result<[T.Reference, T.Version | null], T.Diagnostic> }
  digest: { input: { hashed: [T.RelPath, T.Digest][] }; output: T.Digest }
  fix: { input: { fix: T.Fix; invoked?: string }; output: string }
  fault: { input: { fault: T.Fault; invoked?: string }; output: { message: string; command: string } }
  version: { input: Record<never, never>; output: T.Version }
  // customize
  effective: { input: { shipped: T.Manifest; yours?: T.Overlay }; output: T.Effective }
  report: { input: { previous: T.Manifest; next: T.Manifest; yours?: T.Overlay }; output: T.Report }
  // decide
  compile: { input: { set: T.Installed; limits?: T.Limits }; output: T.Result<T.Plan, T.Diagnostic> }
  propose: { input: { input: T.Input }; output: T.Proposed[] }
  request: { input: { plan: T.Plan; input: string; tags: T.Tag[]; only?: T.LocalName; scope: T.Scope }; output: T.Result<T.Request, T.Diagnostic> }
  read: { input: { plan: T.Plan; request: T.Request; raw: T.Raw }; output: T.Result<T.Reading, T.Fault> }
  gate: { input: { plan: T.Plan; reading: T.Reading; gate?: T.Gate }; output: T.Decision }
  fill: { input: { plan: T.Plan; asking: T.Asking; given: Record<T.ArgName, T.Value>; gate?: T.Gate }; output: T.Decision }
  by_name: { input: { plan: T.Plan; written: T.Written }; output: T.Result<T.Decision, T.Diagnostic> }
  picked: { input: { text: string; recognizer: T.Recognizer }; output: T.Value | null }
  values: { input: { args: Record<T.ArgName, T.Value> }; output: Record<T.ArgName, string | number | true> }
  // run
  call: { input: { text: string }; output: T.Result<T.Written, T.Diagnostic> }
  envelope: { input: { chosen: T.Chosen; active: T.Active; input: T.Input; deadline: T.Millis }; output: T.Envelope }
  argv: { input: { chosen: T.Chosen; active: T.Active }; output: T.Result<string[], T.Diagnostic> }
  // tune, install, author, test
  teach: { input: { plan: T.Plan; utterance: T.Utterance; lesson: T.Lesson }; output: T.Result<T.Edit, T.Diagnostic> }
  set_config: {
    input: { name: T.LocalName; key: T.ConfigKey; setting: T.Setting; spec: T.ConfigSpec }
    output: T.Result<T.Edit, T.Diagnostic>
  }
  vocab_edit: { input: { name: T.VocabName; change: T.VocabChange }; output: T.Edit }
  add_entry: { input: { name: T.LocalName; location: T.Location }; output: T.Edit }
  remove_entry: { input: { name: T.LocalName }; output: T.Edit }
  diff: { input: { previous: T.Manifest; next: T.Manifest }; output: T.ContractDiff }
  consent: { input: { locked: T.Effect; upstream: T.Effect }; output: T.Consent }
  lint: { input: { manifest: T.Manifest }; output: T.Finding[] }
  reflex_dts: { input: { manifest: T.Manifest }; output: string }
  project_dts: { input: { set: T.Installed }; output: string }
  cases: { input: { set: T.Installed }; output: T.Case[] }
  judge: { input: { case: T.Case; decision: T.Decision }; output: T.CaseVerdict }
  regressions: { input: { before: T.Baseline; judged: [T.Case, T.NonEmpty<T.CaseVerdict>][] }; output: T.Regression[] }
  baseline: { input: { before: T.Baseline; judged: [T.Case, T.NonEmpty<T.CaseVerdict>][] }; output: T.Baseline }
  thieves: { input: { newcomers: T.LocalName[]; routed: [T.Case, T.LocalName | null][] }; output: T.Theft[] }
  // weave
  "weave.plan": { input: { plan: T.Plan; input: string; tags: T.Tag[]; answers: T.Answers }; output: T.Result<T.Planning, T.Fault> }
  "weave.execute": { input: { plan: T.Plan; gate?: T.Gate; weave: T.Weave; progress: T.Progress }; output: T.Running }
  // the adapters
  "systemone.settings": { input: { door: T.Door; table?: T.Json }; output: T.Result<T.Settings, T.Diagnostic[]> }
  "systemone.request": { input: { request: T.Request }; output: T.Json }
  "systemone.answers": { input: { status: number; body: string; credential: T.VarName }; output: T.Result<T.Raw, T.Fault> }
  "replay.recording": { input: { toml: string }; output: T.Result<T.Recording, string> }
  "replay.render": { input: { recording: T.Recording }; output: string }
  "replay.answer": { input: { recording: T.Recording; request: T.Request }; output: T.Result<T.Raw, T.Fault> }
}

export type Op = keyof Ops

/** One vector of spec/vectors/<op>/: the op's arguments, and what it must answer. */
export type Vector<O extends Op> = { input: Ops[O]["input"]; expect: Ops[O]["output"] }
