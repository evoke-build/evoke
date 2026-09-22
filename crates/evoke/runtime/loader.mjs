// The loader: one envelope line on stdin — { run, args, input, config, deadline } — the body `run` names imported
// and its default export called with (args, { input, config, signal }), then one result line on stdout:
// { text, data? } or { error }. It exits when stdin closes, before or during a body, so its life is bounded by its
// parent's. The body's console and stdout go to stderr, as does the frame of an error it throws, without the frames
// inside Node itself. SIGTERM and the deadline abort `signal`; a body that has not settled a second later is
// abandoned. The SDK ships this same file.
import { writeSync } from "node:fs";
import { pathToFileURL } from "node:url";

const GRACE = 1000;
const controller = new AbortController();
const out = (value) => writeSync(1, `${JSON.stringify(value)}\n`);
const fail = (message) => {
  out({ error: message });
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
    if (error instanceof Error && error.stack) process.stderr.write(`${where(error.stack)}\n`);
    fail(error instanceof Error ? error.message : String(error));
  }
}

function where(stack) {
  return stack.split("\n").filter((line) => !line.includes("node:internal") && !line.includes("[eval")).join("\n");
}

function describe(value) {
  if (value === null) return "null";
  if (Array.isArray(value)) return "an array";
  return typeof value === "object" ? "an object without text" : typeof value;
}
