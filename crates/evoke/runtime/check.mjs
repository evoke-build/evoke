// The probe `evoke check` runs: the body named by the one argument is imported, never called, and its default
// export must be a function. Otherwise one line on stdout says why — { error } — and stderr says where: Node's
// own frame and message, without the frames inside Node itself.
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

function fail(message) {
  process.stdout.write(`${JSON.stringify({ error: message })}\n`);
  process.exit(1);
}

function where(stack) {
  return stack.split("\n").filter((line) => !line.includes("node:internal")).join("\n");
}
