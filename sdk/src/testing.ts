// @evoke-build/evoke/testing: the recorded adapter. A recording is the CLI's own answers.toml — the declaration,
// then answers keyed by utterance identity — read and written through the core. With `record`, an utterance the
// file lacks, or one recorded under fewer questions than are asked now, is asked of that adapter and the file
// written back whole, so one run records a test suite and every run after is offline and deterministic. In: a
// file, an adapter to record from. Out: an Adapter.

import { readFileSync, renameSync, unlinkSync, writeFileSync } from "node:fs"
import { fileURLToPath } from "node:url"

import type { Adapter } from "./adapter.ts"
import { bug, call, faulted, reply } from "./core.ts"
import { DiagnosticError, FailureError } from "./errors.ts"
import type { Question, Raw, Recording, State } from "./types.ts"

export interface ReplayOptions {
  /** Answers the file lacks are asked of this adapter and written back. */
  record?: Adapter | undefined
}

/** Answers from a recording; a miss is a fault naming the utterance, or, with `record`, the answer written back. */
export function replay(file: string | URL, options: ReplayOptions = {}): Adapter {
  const path = typeof file === "string" ? file : fileURLToPath(file)
  const again = `replay(${JSON.stringify(path)}, { record: jev() })`
  const recording = read(path, options.record, again)
  return {
    id: recording.id,
    ...(recording.limits === undefined ? {} : { limits: recording.limits }),
    ...(recording.gate === undefined ? {} : { gate: recording.gate }),
    ...(recording.plan === undefined ? {} : { plan: recording.plan }),
    async answer(state: State, questions: Record<string, Question>, signal: AbortSignal): Promise<Raw> {
      const request = { state, questions, proposed: [] }
      const identity = call("identity", { text: state.request })
      const recorded = recording.answers[identity]
      if (recorded !== undefined && Object.keys(questions).every(question => Object.hasOwn(recorded, question))) {
        return call("replay.answer", { recording, request })
      }
      if (options.record === undefined) {
        if (recorded !== undefined) return call("replay.answer", { recording, request })
        throw faulted({ type: "unrecorded", identity }, again)
      }
      const raw = await options.record.answer(state, questions, signal)
      recording.answers[identity] = { ...recorded, ...raw }
      write(path, call("replay.render", { recording }), again)
      return call("replay.answer", { recording, request })
    },
  }
}

/** The recording as the file holds it; with `record` and no file yet, an empty one under that adapter's declaration. */
function read(path: string, record: Adapter | undefined, again: string): Recording {
  let text: string
  try {
    text = readFileSync(path, "utf8")
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error
    if (record === undefined) throw refused(path, "there is no such file", again)
    return {
      id: record.id,
      ...(record.limits === undefined ? {} : { limits: record.limits }),
      ...(record.gate === undefined ? {} : { gate: record.gate }),
      answers: {},
    }
  }
  const answer = reply("replay.recording", { toml: text })
  if ("ok" in answer) return answer.ok as Recording
  if ("bug" in answer) throw bug(answer.bug)
  throw refused(path, String(answer.err), again)
}

/** Written whole beside the file, then moved into place: a reader never sees half a recording. */
function write(path: string, text: string, again: string): void {
  const staged = `${path}.${process.pid}.tmp`
  try {
    writeFileSync(staged, text)
    renameSync(staged, path)
  } catch (error) {
    try {
      unlinkSync(staged)
    } catch {
      // Nothing was staged.
    }
    throw new FailureError(`recording ${path}`, error instanceof Error ? error.message : String(error), { type: "rerun" }, again)
  }
}

/** A file that is not a recording: the problem names it, and the fix is the call that records one. */
function refused(path: string, why: string, again: string): DiagnosticError {
  return new DiagnosticError([{ message: `${path}: ${why}`, fix: { type: "rerun" }, command: again }])
}
