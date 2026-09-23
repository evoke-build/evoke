// The probe `evoke check` runs: the body named by the one argument is imported, never called, and its default
// export must be a function. Otherwise one line on stdout says why — { error } — and stderr says where: the
// frames, without those inside Node itself.
import { writeSync } from "node:fs";
import { pathToFileURL } from "node:url";

const run = process.argv[1];
let body;
try {
  body = await import(pathToFileURL(run).href);
} catch (error) {
  if (error instanceof Error && error.stack) process.stderr.write(`${where(error.stack)}\n`);
  fail(`${run} does not load: ${error instanceof Error ? error.message : String(error)}`);
}
if (body.default === undefined) fail(`${run} has no default export`);
if (typeof body.default !== "function") fail(`the default export of ${run} is not a function`);

// Written whole and synchronously, so the line is there before the exit.
function fail(message) {
  const bytes = Buffer.from(`${JSON.stringify({ error: message })}\n`);
  for (let written = 0; written < bytes.length; ) {
    try {
      written += writeSync(1, bytes, written);
    } catch (error) {
      if (error.code !== "EAGAIN") throw error;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1);
    }
  }
  process.exit(1);
}

function where(stack) {
  const lines = stack.split("\n");
  const first = lines.findIndex((line) => line.startsWith("    at "));
  return (first < 0 ? [] : lines.slice(first)).filter((line) => !line.includes("node:internal")).join("\n");
}
