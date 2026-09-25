// The loader: one envelope line on stdin — { run, args, input, config, deadline } — the body `run` names imported
// and its default export called with (args, { input, config, signal }), then one result line on stdout:
// { text, data? } or { error, refused? } — `refused` when the error is a refusal by Node's permission model or the
// kernel: what was refused, a permission or a syscall, and the path, so the host names the declaration's key. It
// exits when stdin closes, before or during a body, so its life is bounded by its parent's. The body's console
// and stdout go to stderr, as do the frames of an error it throws — the frames alone, without the message the
// host reports, and without the frames inside Node itself; an error thrown from a callback ends the body as a
// rejection does. SIGTERM and the deadline abort `signal`; a body that has not settled a second later is
// abandoned. The SDK ships this same file.
import { writeSync } from "node:fs";
import module from "node:module";
import { pathToFileURL } from "node:url";

const GRACE = 1000;
// What a refusal's code is: Node's permission model's, the kernel's, or the resolver's when its socket is refused.
const REFUSALS = new Set(["ERR_ACCESS_DENIED", "EACCES", "EPERM", "EAI_AGAIN", "ENOTFOUND"]);
// Node's type stripper loads on the first `.mts` import, some twenty milliseconds a body would pay after its
// envelope arrived: loaded now instead, while the envelope is on its way. The host turns the API's experimental
// warning off; a runtime without the API skips this, and a body's own import says if types cannot be stripped.
try {
  module.stripTypeScriptTypes?.("let warm: true = true");
} catch {
  // Nothing to warm.
}
const controller = new AbortController();
// The whole line, however long: `process.stdout` below puts the pipe in non-blocking mode, so one write may take
// part of it, and the next may be told to wait a moment.
const out = (value) => {
  const bytes = Buffer.from(`${JSON.stringify(value)}\n`);
  for (let written = 0; written < bytes.length; ) {
    try {
      written += writeSync(1, bytes, written);
    } catch (error) {
      if (error.code !== "EAGAIN") throw error;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1);
    }
  }
};
const fail = (message, refused) => {
  out(refused === undefined ? { error: message } : { error: message, refused });
  process.exit(1);
};

process.stdout.write = (chunk, encoding, callback) => process.stderr.write(chunk, encoding, callback);
globalThis.console = new console.Console({ stdout: process.stderr, stderr: process.stderr });

let buffered = "";
let started = false;
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  if (started) return;
  buffered += chunk;
  const end = buffered.indexOf("\n");
  if (end >= 0) {
    started = true;
    run(buffered.slice(0, end));
  }
});
process.stdin.on("end", () => process.exit(started ? 1 : 0));

async function run(line) {
  let envelope;
  try {
    envelope = JSON.parse(line);
  } catch (error) {
    return fail(`the envelope is not JSON: ${error.message}`);
  }
  const { run, args, input, config, deadline } = envelope;
  const stop = (why) => {
    if (!controller.signal.aborted) controller.abort(new Error(why));
    setTimeout(() => fail(why), GRACE);
  };
  process.on("SIGTERM", () => stop("terminated"));
  process.on("uncaughtException", thrown);
  const timer = setTimeout(() => stop(`timed out after ${deadline} ms`), deadline);
  try {
    const module = await import(pathToFileURL(run).href);
    if (module.default === undefined) throw new Error(`${run} has no default export`);
    if (typeof module.default !== "function") throw new Error(`the default export of ${run} is not a function`);
    const returned = await module.default(args, { input, config, signal: controller.signal });
    const result = typeof returned === "string" ? { text: returned } : returned;
    if (result === null || typeof result !== "object" || typeof result.text !== "string") {
      throw new Error(`the body returned ${describe(returned)}, not text or { text, data }`);
    }
    clearTimeout(timer);
    out(result.data === undefined ? { text: result.text } : { text: result.text, data: result.data });
    process.exit(0);
  } catch (error) {
    thrown(error);
  }
}

// A body's error, whether its promise rejected with it or a callback threw it: the frames on stderr, then the
// message and what refused the body, when something did, as the one line.
function thrown(error) {
  const frames = error instanceof Error && error.stack ? where(error.stack) : "";
  if (frames) process.stderr.write(`${frames}\n`);
  fail(error instanceof Error ? error.message : String(error), refusal(error));
}

// What refused the body, when something did. A lookup names the host, whether Node's permission model refused it
// or the kernel refused the resolver its socket; any other refused call names the syscall and the path, or the
// address and a port when there is one, the kernel's and, on the network, Node's alike; anything else Node's
// permission model refuses names the permission and the resource. A fetch wraps what refused it in its cause.
function refusal(error) {
  const cause = error?.cause;
  const e = cause !== null && typeof cause === "object" && "code" in cause ? cause : error;
  if (e === null || typeof e !== "object" || !REFUSALS.has(e.code)) return undefined;
  const text = (value) => (value === undefined || value === null ? "" : String(value));
  if (e.hostname !== undefined) return { what: "resolve", path: text(e.hostname) };
  if (e.syscall === undefined) return { what: text(e.permission), path: text(e.resource) };
  const address = [e.address, e.port].filter((part) => part !== undefined).join(":");
  return { what: text(e.syscall), path: text(e.path) || address };
}

function where(stack) {
  const lines = stack.split("\n");
  const first = lines.findIndex((line) => line.startsWith("    at "));
  return (first < 0 ? [] : lines.slice(first))
    .filter((line) => !/[( ]node:/.test(line) && !line.includes("[eval"))
    .join("\n");
}

function describe(value) {
  if (value === null) return "null";
  if (Array.isArray(value)) return "an array";
  return typeof value === "object" ? "an object without text" : typeof value;
}
