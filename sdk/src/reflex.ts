// A reflex as code: the manifest as an object — the file's shape, less `run` and `config` — and its body, whose
// argument types come from the manifest literal: option keys as a union, a word or a quoted, email or url pick as
// a string, a number or duration as a number, a flag as `true`, an optional argument optional. In: a manifest
// literal and a body. Out: an Inline, which `load` takes and runs in-process.

import type { Flag, Option, Pick, Value, Values, Word } from "./decision.ts"
import type { Reflex } from "./runtime.ts"
import type { Effect } from "./types.ts"

/** `reflex.toml` as an object: no `run`, no `config` — a body closes over what it needs — no `reflex` key. */
export interface InlineManifest {
  description: string
  not_for?: string[]
  tags?: string[]
  /** Absent means destructive: every decision confirms. */
  effect?: Effect
  /** The one-line template a person confirms, naming required arguments only. */
  confirm: string
  args?: Record<string, InlineArg>
  examples?: InlineRecords
  tests?: InlineRecords
}

/** An argument: its question and exactly one source; a flag is optional by nature. */
export type InlineArg = { ask: string } & (
  | { options: Record<string, string>; optional?: boolean }
  | { vocab: string; optional?: boolean }
  | { pick: "number" | "duration"; range?: [number, number]; optional?: boolean }
  | { pick: "email" | "url" | "quoted"; optional?: boolean }
  | { flag: true }
)

/** Utterances with what they assert: a record per argument — the text, or `false` for unstated, `true` for a flag —
 *  or `false` for never this reflex. */
export type InlineRecords = Record<string, Record<string, string | boolean> | false>

type Typed<A> = A extends { options: infer O }
  ? Option<keyof O & string>
  : A extends { vocab: string }
    ? Word
    : A extends { pick: infer P extends "number" | "duration" }
      ? Pick<P, number>
      : A extends { pick: infer P extends "email" | "url" | "quoted" }
        ? Pick<P, string>
        : A extends { flag: true }
          ? Flag
          : never
type Absent<A> = A extends { optional: true } | { flag: true } ? true : false
type Flat<T> = { [K in keyof T]: T[K] } & {}
type ArgsOf<M extends InlineManifest> = NonNullable<M["args"]> extends Record<string, InlineArg> ? NonNullable<M["args"]> : Record<never, InlineArg>

/** What a decision carries for a reflex handed as code: its arguments as the wire types them, optional ones optional. */
export type Carried<M extends InlineManifest> = Flat<
  { [N in keyof ArgsOf<M> as Absent<ArgsOf<M>[N]> extends true ? never : N]: Typed<ArgsOf<M>[N]> } & {
    [N in keyof ArgsOf<M> as Absent<ArgsOf<M>[N]> extends true ? N : never]?: Typed<ArgsOf<M>[N]>
  }
>

/** What decisions carry for a map of reflexes handed as code: `load<Reflexes & ReflexesOf<typeof own>>` when a
 *  project has installed reflexes beside them. */
export type ReflexesOf<I> = { [K in keyof I]: I[K] extends Inline<infer S> ? S : never }

/** The body's arguments, plain: what `reflex`'s body receives. */
export type Args<M extends InlineManifest> = Values<Carried<M>>

/** What `reflex` returns and `load` takes; `shape` never holds a value — it is what a decision carries, for inference. */
export interface Inline<S = Record<string, Value>> {
  readonly manifest: InlineManifest
  readonly body: Reflex<Record<string, unknown>, Record<never, string>>
  readonly shape?: S | undefined
}

/** A reflex as code: its body runs in-process, its arguments typed from the manifest. */
export function reflex<const M extends InlineManifest>(manifest: M, body: Reflex<Args<M>, Record<never, string>>): Inline<Carried<M>> {
  return { manifest, body: body as Reflex<Record<string, unknown>, Record<never, string>> }
}
