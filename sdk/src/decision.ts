// The decision as an app reads it: the CLI's --json line — the wire decision tagged by outcome — plus what the SDK
// knows around it: the input as decided, the plan it was decided under, one trace entry per adapter call, and the
// plain values a body receives beside the typed args. A union narrowed by `outcome`, then by `reflex`. R is what
// each reflex's arguments carry — `Reflexes` from evoke.d.ts, or inferred from reflexes handed as code.

import type { Trace } from "./adapter.ts"
import type { Result } from "./runtime.ts"
import type * as W from "./types.ts"

/** The four shapes evoke.d.ts declares, so a generated Reflexes and an inline one are one kind of thing. */
export type Option<K extends string = string> = { type: "option"; key: K }
export type Word = { type: "word"; word: string; value?: string }
export type Pick<T extends W.Recognizer = W.Recognizer, V = number | string> = {
  type: "pick"
  span: W.Span
  value: { type: T; value: V }
}
export type Flag = { type: "flag" }
/** An argument's value as the wire carries it. */
export type Value = W.Value

/** What a decision's args carry per reflex when nothing narrows it: any reflex, any arguments. */
export type AnyReflexes = Record<string, Record<string, Value>>

/** A value as a body receives it: the key, the word's value else the word, the number or text, `true`. */
export type Plain<V> = V extends Option<infer K>
  ? K
  : V extends Word
    ? string
    : V extends Pick<W.Recognizer, infer X>
      ? X
      : V extends Flag
        ? true
        : never

/** Plain values per argument; an optional argument stays optional. */
export type Values<A> = { [N in keyof A]: Plain<NonNullable<A[N]>> }

/** What every decision carries beside the core's fields. */
export interface Line {
  /** The input as decided. */
  input: string
  /** The digest of the plan it was decided under: `fill` and `run` refuse another. */
  plan: string
  /** One entry per adapter call. */
  trace: Trace[]
}

/** A complete call: the typed args as read, the plain values a body receives, the one call grammar, the effect. */
export type Chosen<R, K extends keyof R & string> = Line &
  W.Judged & {
    reflex: K
    args: R[K]
    values: Values<R[K]>
    /** `lights room="den" state="off"` */
    call: string
    effect: W.Effect
  }

/** A complete call over the floor: run it. */
export type Run<R = AnyReflexes> = { [K in keyof R & string]: { outcome: "run" } & Chosen<R, K> }[keyof R & string]
/** A complete call that needs a yes: the prompt, and every reason in a fixed order. */
export type Confirm<R = AnyReflexes> = {
  [K in keyof R & string]: { outcome: "confirm"; prompt: W.Prompt; because: W.NonEmpty<W.Cap> } & Chosen<R, K>
}[keyof R & string]
/** A winner with required arguments missing: what was read so far, and what to ask. */
export type Ask<R = AnyReflexes> = {
  [K in keyof R & string]: Line &
    W.Judged & {
      outcome: "ask"
      reflex: K
      args: Partial<R[K]>
      values: Partial<Values<R[K]>>
      unconsumed: W.Span[]
      missing: W.NonEmpty<W.Missing>
    }
}[keyof R & string]
/** `none` won or the route is under its floor: the ranking, and every judgment. */
export type Abstain = Line & { outcome: "abstain"; contenders: W.Contender[]; judgments: W.Judgment[] }

/** One input decided. */
export type Decision<R = AnyReflexes> = Abstain | Run<R> | Confirm<R> | Ask<R>

/** What an ask takes back for one value: an option's key, else the text a person typed. A flag is never asked. */
type Answer<P> = P extends true ? never : P extends string ? P : string

/** What an ask is answered with, by argument name. */
export type Given<D> = D extends { values: Partial<infer V> }
  ? { [N in keyof V as NonNullable<V[N]> extends true ? never : N]?: Answer<NonNullable<V[N]>> | undefined }
  : never

/** What `handle` returns; a failure of any kind throws. */
export type Handled<R = AnyReflexes> =
  | { outcome: "ran"; decision: Run<R> | Confirm<R>; result: Result }
  | { outcome: "abstained"; decision: Abstain }
  | { outcome: "declined"; decision: Confirm<R> | Ask<R> }
  | { outcome: "unanswered"; decision: Confirm<R> | Ask<R> }
