// The wire types, by hand, from the Rust: what crosses the boundary as JSON — every value the op table takes and
// answers — in the core's module order. Kept honest by the vectors: build/vectors.ts makes every case of
// spec/vectors satisfy its op's types, so a field that drifts here fails the check. The rules of the wire
// (spec/README.md#wire):
// snake_case keys; an Option absent when none; a newtype as its value; an enum tagged by `type` (a Decision by
// `outcome`); a Result as { ok } | { err }.

// ---- text, name, diagnostic, document ----

/** A `Result` on the wire: `{ ok }` or `{ err }`. */
export type Result<T, E> = { ok: T } | { err: E }

// text.rs

/** Text that cannot repaint a terminal: no C0 or C1 control but the line feed, no bidi control. */
export type Clean = string

/** What a person typed: NFC, at most 2 000 characters, spelling kept; untrusted, never cleaned — only its spans are. */
export type Input = string

/** What keys a record: NFC, lower-cased, whitespace collapsed and trimmed, terminal punctuation dropped; the core normalizes any string it is handed as one. */
export type Identity = string

/** An utterance as written, with its identity: the text is what the classifier reads, the id what keys it. */
export interface Utterance {
  /** One non-empty line. */
  text: Clean
  /** The identity of `text`; anything else is refused. */
  id: Identity
}

/** A verbatim piece of one input: character offsets, `start < end`, and the text between them. */
export interface Span {
  start: number
  end: number
  text: Clean
}

/** A list that refuses to be empty; on the wire, a plain array — never `[]`. */
export type NonEmpty<T> = [T, ...T[]]

// name.rs

/** The `[reflexes]` key, the overlay's file name, a route option: `[a-z][a-z0-9_]*`, never `none`, `unstated` or `fits`. */
export type LocalName = string

/** An argument: `[a-z][a-z0-9_]*`, never a JavaScript reserved word, so a body can destructure it. */
export type ArgName = string

/** An option's key, which the body receives: one clean line, never `none` or `unstated`. */
export type OptionKey = string

/** A vocabulary word: one clean line, trimmed, never `none` or `unstated`; spaces allowed. */
export type Word = string

/** A vocabulary, `vocab/<name>.toml`: `[a-z][a-z0-9_]*`. */
export type VocabName = string

/** A key under `[config]`: `[a-z][a-z0-9_]*`. */
export type ConfigKey = string

/** A scope for `--tag`: `[a-z][a-z0-9_]*`. */
export type Tag = string

/** An adapter, by the name it is resolved under: `[a-z][a-z0-9_]*`. */
export type AdapterName = string

/** What an adapter declares itself as: non-empty; compared, never parsed. */
export type AdapterId = string

/** An environment variable: `[A-Za-z_][A-Za-z0-9_]*`. */
export type VarName = string

/** A path inside one reflex directory: relative, plain segments, no `..`. */
export type RelPath = string

/** A GitHub owner, user or organization: `[A-Za-z0-9][A-Za-z0-9-]*`. */
export type Owner = string

/** One segment of a ref: `[A-Za-z0-9._][A-Za-z0-9._-]*`, never option-shaped. */
export type Segment = string

// diagnostic.rs

/** A problem a person has to fix, ending in the command that fixes it. */
export interface Diagnostic {
  /** The reflex the problem belongs to, when it belongs to one. */
  reflex?: LocalName
  /** The line, when the file has lines; a JSON document has none and fixes with `check`. */
  at?: At
  message: string
  fix: Fix
}

/** A line of an owned file. */
export interface At {
  file: File
  line: number
  column: number
}

/** One of the project's owned files, by role; it displays as its project-relative path. */
export type File =
  | { type: "manifest"; name: LocalName } // <name>/reflex.toml
  | { type: "overlay"; name: LocalName } // overlays/<name>.toml
  | { type: "vocab"; name: VocabName } // vocab/<name>.toml
  | { type: "project" } // evoke.toml
  | { type: "lock" } // evoke.lock

/** The closed set of fixing commands, each rendered as the literal last line of a diagnostic. */
export type Fix =
  | { type: "vocab_add"; vocab: VocabName } // evoke vocab <vocab> add <word> "<meaning>"
  | { type: "config_set"; reflex: LocalName; key: ConfigKey } // evoke config <reflex> <key> <value>
  | { type: "config_env"; reflex: LocalName; key: ConfigKey } // evoke config <reflex> <key> --env <VAR>
  | { type: "update"; reflex?: LocalName } // evoke update [reflex]
  | { type: "accept"; reflex: LocalName } // evoke update --accept <reflex>
  | { type: "trust" } // evoke trust
  | { type: "remove"; reflex: LocalName } // evoke remove <reflex>
  | { type: "sync" } // evoke sync
  | { type: "check" } // evoke check
  | { type: "edit_line"; at: At } // <file>:<line>:<column>
  | { type: "export_key"; var: VarName } // export <VAR>=<value>
  | { type: "rerun" } // the command that was run, again
  | { type: "add" } // evoke add evoke-build/reflexes
  | { type: "show"; reflex?: LocalName } // evoke show [reflex]
  | { type: "add_ref"; reference: string; name?: LocalName } // evoke add <reference> --as <name>
  | { type: "teach_not"; utterance: string; reflex: LocalName } // evoke teach "<utterance>" not <reflex>
  | { type: "new" } // evoke new <name>

// document.rs

/** A JSON value that keeps its key order: the wire form of every boundary type. */
export type Json = unknown

/** A file to parse: its role, and its text as written — TOML, with lines — or as JSON, without. */
export type Document = { file: File; toml: string } | { file: File; json: Json }

/** Where in a file, by key: `["args", "state", "ask"]` is `args.state.ask`; a segment is verbatim, unescaped. */
export type KeyPath = string[]

// ---- manifest ----

/** A reflex's manifest, normalized: `effect` explicit, every table present, records typed. */
export interface Manifest {
  description: Description
  not_for: Clean[]
  tags: Tag[]
  effect: Effect
  confirm: Template
  /** Absent: inline, a function the SDK holds. */
  run?: Run
  config: Record<ConfigKey, ConfigSpec>
  args: Record<ArgName, Argument>
  examples: Records
  tests: Records
  /** Keys the format does not know: reported, never fatal. */
  unknown: KeyPath[]
}

/** What the reflex does: a summary line, never empty; the rest, when there is one, after a line feed. */
export type Description = string

/** What running the reflex does to the world; greater is tighter: read < write < destructive. */
export type Effect = "read" | "write" | "destructive"

/** The confirm prompt: text with `{placeholder}`s, each naming a required, non-flag argument. */
export type Template = string

/** The body: an entrypoint run in a child, or an argv that never touches a shell. */
export type Run = Entrypoint | Argv

/** The program, a literal, then each element. */
export type Argv = [program: Clean, ...rest: Element[]]

/** A path inside the reflex directory ending in `.mts` or `.mjs`. */
export type Entrypoint = RelPath

/** One argv element after the program: a literal as written, or an argument's `{name}` — never a flag's. */
export type Element = string

/** A setting the user provides; a secret is set only from an environment variable. */
export interface ConfigSpec {
  about: Clean
  secret: boolean
}

/** An argument: its question, where its values come from, and its former names. */
export type Argument = {
  ask: Clean
  /** Flat and cumulative; a retired name never returns. */
  was: ArgName[]
} & Kind

/** A flag, or a value with exactly one source; a flag is optional by nature. */
export type Kind = { flag: true } | (Source & { optional: boolean })

/** Where an argument's values come from: the author, the user or the input. */
export type Source = { options: Options } | { vocab: VocabName } | Pick

/** The author's closed set, never empty: key = what the body receives, value = what it means. */
export type Options = Record<OptionKey, Clean>

/** A built-in recognizer over the input; a range only where a number exists, whole seconds for a duration. */
export type Pick =
  | { pick: "number" | "duration"; range?: Range }
  | { pick: "email" | "url" | "quoted" }

/** One of the five recognizers, by the name a manifest's `pick` writes. */
export type Recognizer = "number" | "duration" | "email" | "url" | "quoted"

/** `[min, max]` on a value, min ≤ max. */
export type Range = [min: number, max: number]

/** Utterances, each with what it asserts; keyed by spelling, no two sharing an identity. */
export type Records = Record<Clean, Asserted>

/** `false`, or the asserted arguments; `{}` asserts the route alone. */
export type Asserted = false | Record<ArgName, Assertion>

/** One argument's assertion: `false` unstated, `true` a flag, else an option key, a word or a span of the utterance. */
export type Assertion = boolean | OptionKey | Word | Clean

// ---- overlay, vocabulary, project, digest ----

// overlay

/** Your wording for one reflex, its names already resolved through `was`. Read, never built. */
export interface Overlay {
  description?: Description
  not_for?: Clean[]
  tags?: Tag[]
  effect?: Effect
  confirm?: Template
  args: Record<ArgName, Wording>
  examples: Records
  tests: Records
  /** Former names the file used, with the name each resolved to. */
  renamed: [ArgName, ArgName][]
  /** Keys that address nothing: skipped, the rest applies. */
  orphaned: KeyPath[]
}

/** New wording for an argument: its question, and existing option keys. */
export interface Wording {
  ask?: Clean
  options: Record<OptionKey, Clean>
}

/** `shipped ⊕ yours`, with the key paths your file decided: what `show` marks. */
export interface Effective {
  manifest: Manifest
  yours: KeyPath[]
}

/** What an update means for your file. */
export interface Report {
  renamed: [ArgName, ArgName][]
  /** You override what upstream changed: yours wins, both are shown once. */
  stale: KeyPath[]
  orphaned: KeyPath[]
}

// vocabulary

/** Your closed set for the arguments that name it; may be empty, which `compile` reports. */
export type Vocabulary = Record<Word, Meaning>

/** What a word means, and what the body receives instead of it when set. */
export interface Meaning {
  what: Clean
  /** What the body receives instead of the word; never shown to the classifier. */
  value?: string
}

// project

/** What you wrote: the adapter that decides, the reflexes you installed, their settings, the adapters' own tables. */
export interface Project {
  /** Always stated; a name, never a path. */
  adapter: AdapterName
  reflexes: Record<LocalName, Location>
  config: Record<LocalName, Record<ConfigKey, Setting>>
  /** Inert unless selected; then its adapter validates it. */
  adapters: Record<AdapterName, Json>
}

/** Where a reflex comes from: a directory as written, resolved by the host, or a repository. */
export type Location =
  | { type: "local"; path: string }
  | { type: "remote"; reference: Reference; pin?: Version }

/** A repository and a directory in it. */
export interface Reference {
  repo: Repo
  dir?: RelPath
}

/** A repository on GitHub, or by git URL. */
export type Repo =
  | { type: "github"; owner: Owner; name: Segment }
  | { type: "url"; url: GitUrl }

/** A git URL over an allow-listed scheme: `https` or `ssh`, no `#`, `@` or whitespace. */
export type GitUrl = string

/** A tag `[v]X.Y.Z`, printed without the `v`: `"1.2.0"`. */
export type Version = string

/** A config value as you stored it: plain, or a reference to an environment variable. */
export type Setting =
  | { type: "plain"; value: string }
  | { type: "env"; var: VarName }

/** What `add`, `update` and `remove` write whole and `sync` realises; a local reflex is never in it. */
export interface Lock {
  evoke: Version
  adapter: LockedAdapter
  reflexes: Record<LocalName, Locked>
}

/** The adapter the lock was written under, by name and id. */
export interface LockedAdapter {
  name: AdapterName
  id: AdapterId
}

/** One remote reflex as pinned. */
export interface Locked {
  reference: Reference
  tag: Version
  commit: Commit
  h1: Digest
  effect: Effect
}

/** A commit as git printed it: 40 or 64 lowercase hex; compared, never parsed. */
export type Commit = string

// digest

/** A SHA-256, displayed and serialized as `h1:<hex>`. */
export type Digest = string

// ---- adapter, plan, propose ----

// adapter.rs

/** A choice's key exactly as offered: an option key, a word, a candidate `<start>-<end>`, a flag's `yes` or `no`, a local name, `none` or `unstated`. The plan's slot gives it meaning at read. */
export type Key = string

/** Which question: `route`, `fits.<reflex>` or `<reflex>.<argument>`. */
export type QuestionId = string

/** One question for the adapter: a choice over keys, or a yes/no with both sides described. */
export type Question =
  | ({ type: "choice" } & Choice)
  | {
      type: "yesno"
      ask: Clean
      /** The reflex's description. */
      yes: Text
      /** What the reflex is not for. */
      no: Text
    }

/** A choice: the question, its options in order, and which key means "none of these". */
export interface Choice {
  ask: Clean
  options: Record<Key, Text>
  /** One of `options`; absent when the choice has no sentinel. */
  otherwise?: Key
}

/** What an option means: a line, or a description with what it is not for and examples that assert it. */
export type Text =
  | Clean
  | {
      what: Clean
      /** Absent when empty. */
      not_for?: Clean[]
      /** Absent when empty. */
      examples?: Clean[]
    }

/** The only state an adapter ever sees. */
export interface State {
  request: Input
}

/** One call of `answer`: the state, the questions, and the candidate spans the pick questions were built from. */
export interface Request {
  state: State
  questions: Record<QuestionId, Question>
  proposed: Proposed[]
}

/** A finite number in `[0, 1]`. */
export type Prob = number

/** Per-call ceilings an adapter declares. */
export interface Limits {
  /** The most options one choice may offer; `compile` refuses a plan over it. */
  options?: number
  tokens?: number
}

/** The floors an adapter ships, each meaning P(correct); `read` never above `write`, and no destructive number exists. */
export interface Gate {
  route: Prob
  /** The runner-up's floor; a plan without `fits` questions skips it. */
  fits?: Prob
  read: Prob
  write: Prob
}

/** What an adapter declares about itself. */
export interface Declared {
  id: AdapterId
  limits?: Limits
  gate?: Gate
}

/** Answers as the adapter gave them: per question, a number per key. `read` validates them against their `Request`. */
export type Raw = Record<QuestionId, Record<Key, number>>

/** How an adapter fails, or how its answers failed validation; each ends in a fixing command. */
export type Fault =
  | { type: "transport"; message: string }
  | { type: "status"; status: number }
  | { type: "retired"; id: AdapterId }
  | { type: "unanswered"; question: QuestionId }
  | { type: "malformed"; question: QuestionId; message: string }
  | { type: "unrecorded"; identity: Identity }

// plan.rs

/** Everything the host found; `compile` alone judges activity. */
export interface Installed {
  reflexes: Record<LocalName, Item>
  vocab: Record<VocabName, Vocabulary>
  adapter: AdapterId
  evoke: Version
}

/** One installed reflex as found: its wording or why it has none, the effect consented to, how its config is held. */
export interface Item {
  wording: Result<Effective, Diagnostic[]>
  consented: Effect
  configured: Record<ConfigKey, Held>
}

/** How a set config key is held: the value itself, or a variable that is set or not. Never a secret's value. */
export type Held =
  | { type: "plain"; value: string }
  | { type: "env"; var: VarName; set: boolean }

/** A duration in milliseconds; the host makes it an instant. */
export type Millis = number

/** The compiled set: the active reflexes, the inactive ones with a fix per problem, every input-independent question, and the digest that keys what is derived from it. */
export interface Plan {
  /** SHA-256 of the compact JSON of `Installed`: keys the decision cache and the baselines. */
  digest: Digest
  active: Record<LocalName, Active>
  /** Every problem of every inactive reflex, each with its fix. */
  inactive: Record<LocalName, NonEmpty<Diagnostic>>
  /** `route`, then per active reflex `fits.<name>` and every argument, in that order. */
  slots: Record<QuestionId, Slot>
  /** Per vocabulary, the words that carry a value. */
  values: Record<VocabName, Record<Word, string>>
  /** The core's one deadline: 30 000 ms. */
  deadline: Millis
}

/** An active reflex as the decision needs it; `effect` is the tighter of the manifest's and the consented one. */
export interface Active {
  effect: Effect
  /** Absent: inline, a function the SDK holds. */
  run?: Run
  confirm: Template
  args: Record<ArgName, Argument>
  config: Record<ConfigKey, Setting>
  tags: Tag[]
}

/** A question that does not depend on the input, or a pick whose options exist only per input. */
export type Slot =
  | Question
  | { type: "pick"; ask: Clean; pick: Recognizer; optional: boolean }

// propose.rs

/** A candidate for a pick: where it is in the input, and what its recognizer read. */
export interface Proposed {
  span: Span
  value: PickValue
}

/** What a pick hands the body: the number, the seconds, or the text. */
export type PickValue =
  | { type: "number"; value: number }
  | { type: "duration"; value: number } // whole seconds
  | { type: "email"; value: Clean }
  | { type: "url"; value: Clean }
  | { type: "quoted"; value: Clean }

// ---- decide, call, run ----

// decide.rs

/** What a request asks: everything, or the route alone — the conflict test at `add`. */
export type Scope = "full" | "route"

/** What the answers said: the reflexes ranked, every choice read, and the winner with its values. */
export interface Reading {
  /** Sorted by route probability; `try` prints it. */
  ranking: Contender[]
  judgments: Judgment[]
  /** Absent when `none` won. */
  winner?: Winner
}

/** A reflex in the ranking: its route probability, and its `fits` when that was asked. */
export interface Contender {
  reflex: LocalName
  route: Prob
  fits?: Prob
}

/** The reflex that won the route, with what its arguments read. */
export interface Winner {
  reflex: LocalName
  args: Record<ArgName, Value>
  missing: Missing[]
  /** Typed spans no argument consumed. */
  unconsumed: Span[]
  runner_up?: Contender
}

/** One choice read: which key came out on top, and how probable it was. */
export interface Judgment {
  question: QuestionId
  top: Key
  p: Prob
}

/** An argument without a usable value, and what a person may choose from. */
export interface Missing {
  arg: ArgName
  /** The argument's question, as the manifest asks it. */
  ask: Clean
  because: Why
  choices: Choices
}

/** Why a value is missing: the input never stated it, or a pick fell outside its range. */
export type Why =
  | { type: "unstated" }
  | { type: "out_of_range"; span: Span; range: Range }

/** What a person may answer with; a vocabulary also prompts to add a word. */
export type Choices =
  | { type: "options"; options: Record<OptionKey, Clean> }
  | { type: "vocab"; words: Record<Word, Clean> }
  | { type: "pick"; pick: Recognizer }

/** The outcome, tagged by `outcome` on the wire, the chosen call's fields flattened beside it. */
export type Decision =
  | { outcome: "abstain"; contenders: Contender[]; judgments: Judgment[] }
  | ({ outcome: "run" } & Chosen)
  | ({ outcome: "confirm" } & Chosen & { prompt: Prompt; because: NonEmpty<Cap> })
  | ({ outcome: "ask" } & Asking & { missing: NonEmpty<Missing> })

/** A complete call with its effect and, unless called by name, what the classifier judged. */
export type Chosen = Call & { effect: Effect } & (Judged | Unjudged)

/** Called by name, so nothing was judged: no field of `Judged` is present. */
export type Unjudged = { [K in keyof Judged]?: never }

/** The judgments a decision rests on: confidence is the weakest one's probability, over the route and every argument question of the winner. */
export interface Judged {
  /** `weakest.p`: the minimum over `judgments`. */
  confidence: Prob
  weakest: Judgment
  judgments: NonEmpty<Judgment>
  runner_up?: Contender
  /** The ranking. */
  contenders: Contender[]
}

/** An ask in flight: what the gate reads again once the missing values are given. */
export interface Asking extends Judged {
  reflex: LocalName
  args: Record<ArgName, Value>
  /** Typed spans no argument consumed. */
  unconsumed: Span[]
}

/** Why a decision stops at confirm; `because` lists them in this order. */
export type Cap =
  | { type: "destructive" }
  | { type: "no_gate" }
  | { type: "under_floor"; judgment: Judgment; floor: Prob }
  | { type: "unconsumed_span"; span: Span }
  | { type: "two_things"; contender: Contender }

/** The confirm prompt: `evoke`'s own line, then the manifest's template filled in. */
export interface Prompt {
  /** The call, its effect, the weakest judgment and each cap that names itself, joined by ` · `. */
  own: string
  template: Clean
}

// call.rs

/** A call resolved and complete; on the wire `{ reflex, args, call }`, the last being its rendering. */
export interface Call {
  reflex: LocalName
  args: Record<ArgName, Value>
  /** The call on one line: the name, then each argument as `name="value"` or a bare flag. */
  call: string
}

/** An argument's value: a key, a word, a verbatim span, or a flag that is present. Never minted by the model. */
export type Value =
  | { type: "option"; key: OptionKey }
  | {
      type: "word"
      word: Word
      /** What the body receives instead of the word, when the vocabulary sets one. */
      value?: string
    }
  | { type: "pick"; span: Span; value: PickValue }
  | { type: "flag" }

/** A call as typed: `lights room=den state=off`; a bare argument is a flag. Resolved by `by_name` or typed by a lesson, never run as it is. */
export interface Written {
  reflex: LocalName
  /** As typed; `null` is written bare, so a flag. */
  args: Record<ArgName, string | null>
}

// run.rs

/** One JSON line on the loader's stdin. */
export interface Envelope {
  reflex: LocalName
  /** Absent for an inline body. */
  run?: Run
  /** An option key, a word's `value` if set else the word, a pick's value, `true` for a flag. */
  args: Record<ArgName, string | number | true>
  input: Input
  /** An `env` setting stays a reference; the host resolves it. */
  config: Record<ConfigKey, Setting>
  deadline: Millis
}

// ---- contract, edit, test; the adapters' jev and replay ----

// contract

/** What changed in the contract from one version to the next, and how much it matters. */
export interface ContractDiff {
  level: Level
  changes: Change[]
  violations: WasViolation[]
}

/** `same`: nothing but wording. `minor`: additions, and a config key gone. `major`: something a person's files or calls may not survive. */
export type Level = "same" | "minor" | "major"

/** One change to the contract, in the order the diff walks: the previous arguments, the added ones, the body, config. */
export type Change =
  | { type: "arg_removed"; arg: ArgName }
  | { type: "option_removed"; arg: ArgName; key: OptionKey }
  | { type: "arg_renamed"; from: ArgName; to: ArgName }
  | { type: "source_changed"; arg: ArgName }
  | { type: "range_changed"; arg: ArgName }
  | { type: "run_changed" }
  | { type: "arg_added"; arg: ArgName }
  | { type: "option_added"; arg: ArgName; key: OptionKey }
  | { type: "config_added"; key: ConfigKey }
  | { type: "config_removed"; key: ConfigKey }

/** `was` is flat and cumulative: a retired name never returns as a live argument, and never leaves the lists. */
export type WasViolation =
  | { type: "returned"; arg: ArgName }
  | { type: "dropped"; arg: ArgName }

/** What an update does to the effect a person consented to: upstream may keep or tighten it, and loosens it only through `evoke update --accept`. */
export type Consent =
  | { type: "kept"; effect: Effect }
  | { type: "tightened"; effect: Effect }
  | { type: "needs_accept"; locked: Effect; upstream: Effect }

/** What `lint` finds: a size cap passed, or text that addresses the model instead of describing an action. */
export type LintRule = "size_cap" | "addresses_model"

/** One thing `lint` found, at the key path it concerns; reported at `add` and by `check`, never a refusal. */
export interface Finding {
  rule: LintRule
  path: KeyPath
  message: string
}

// edit

/** One change to an owned file, which the host applies keeping the file's own shape: a key path set, or removed. */
export type Edit =
  | {
      type: "set"
      file: Owned
      path: KeyPath
      /** The line's value as JSON: a record, a string, `{ env }` or `{ what, value }`. */
      value: Json
    }
  | { type: "remove"; file: Owned; path: KeyPath }

/** A file `evoke` edits in place; the lock and `evoke.d.ts` are rendered whole, never edited. */
export type Owned =
  | { type: "project" }
  | { type: "overlay"; name: LocalName }
  | { type: "vocab"; name: VocabName }

/** The overlay line itself: which reflex an utterance belongs to and what it asserts; `not <name>` is `false`. */
export interface Lesson {
  reflex: LocalName
  /** Names follow `was`; each value is typed against the plan at the door. */
  record: Asserted
}

/** What `evoke vocab <name> add | remove` does to the file. */
export type VocabChange =
  | { type: "add"; word: Word; meaning: Meaning }
  | { type: "remove"; word: Word }

// test

/** One record as a test: whose it is, what was said, what should come of it, and the table it came from. */
export interface Case {
  reflex: LocalName
  utterance: Utterance
  expect: Expected
  from: Table
}

/** Where a case came from: examples are sent to the classifier, tests are held out. */
export type Table = "examples" | "tests"

/** What a record expects, as a decision compares to it: `false` for never this reflex, or a claim per argument — `{}` asserts the route alone. */
export type Expected = false | Record<ArgName, Claim>

/** One argument's claim: `false` unstated, `true` a flag raised, or the text a decision's value must show — an option key, a word or a span, which all compare as text. What a decision read takes the same shape. */
export type Claim = boolean | Clean

/** What a decision made of a case: it passed, or where it first missed. */
export type Verdict =
  | { type: "pass" }
  | { type: "fail"; mismatch: Mismatch }

/** Where a decision missed a case: the route — read as another reflex, or as none — or the first asserted argument read as something else. */
export type Mismatch =
  | { type: "route"; read: LocalName | null }
  | { type: "arg"; arg: ArgName; read: Claim }

/** The last run's verdict per case, by reflex and utterance identity; the host keeps one per plan digest. */
export type Baseline = Record<LocalName, Record<Identity, Verdict>>

/** A case that passed at the last run and fails now, two of three uncached repeats. */
export interface Regression {
  case: Case
  now: Mismatch
}

/** A phrase an installed reflex claims that a newcomer wins at `add`. */
export interface Theft {
  phrase: Utterance
  owner: LocalName
  thief: LocalName
}

// jev

/** What both hosts read before the first call. */
export interface Settings {
  declared: Declared
  /** The variable the API key is read from. */
  credential: VarName
  /** The endpoint both hosts post to. */
  url: string
  policy: Policy
}

/** The transport policy as a value, executed by each host: the wait after connect, how many times a connect error or a retried status is tried again, and which statuses those are. */
export interface Policy {
  timeout: Millis
  retries: number
  /** `[min, max]`, both retried. */
  retry_statuses: Range
}

// replay

/** Hand-writable answers keyed by utterance identity, with what the adapter declares — `id`, `limits` and `gate` are its `Declared`, flattened. */
export interface Recording {
  id: AdapterId
  limits?: Limits
  gate?: Gate
  /** The plan the answers were recorded against; the host compares it with `Plan.digest` before calling `answer`. */
  plan?: Digest
  answers: Record<Identity, Raw>
}
