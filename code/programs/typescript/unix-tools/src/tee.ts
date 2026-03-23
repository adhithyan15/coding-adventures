/**
 * tee -- read from standard input and write to standard output and files.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `tee` utility in TypeScript. It
 * copies standard input to each specified FILE, and also to standard
 * output. It is named after the T-splitter used in plumbing.
 *
 * === How tee Works ===
 *
 * tee is like a pipe fitting that splits the data stream:
 *
 *     command | tee file.txt        =>   output goes to both stdout AND file.txt
 *     command | tee a.txt b.txt     =>   output goes to stdout, a.txt, AND b.txt
 *     command | tee -a file.txt     =>   appends to file.txt instead of overwriting
 *
 * === Why tee Exists ===
 *
 * In a Unix pipeline, data flows in one direction. Without tee, you
 * cannot both see the output AND save it to a file:
 *
 *     ls | grep ".ts"              =>   see output but don't save it
 *     ls | grep ".ts" > out.txt    =>   save it but don't see it
 *     ls | grep ".ts" | tee out.txt   =>   see it AND save it
 *
 * === Signal Handling (-i) ===
 *
 * With `-i`, tee ignores the SIGINT signal (Ctrl+C). This is useful in
 * long-running pipelines where you don't want tee to die when you
 * interrupt the upstream command.
 *
 * @module tee
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SPEC_FILE = path.resolve(__dirname, "..", "tee.json");

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then tee stdin to files and stdout.
// ---------------------------------------------------------------------------

function main(): void {
  // --- Step 1: Parse arguments ---------------------------------------------

  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`tee: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw err;
  }

  // --- Step 2: Dispatch on result type -------------------------------------

  if ("text" in result) {
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  // --- Step 3: Extract flags and arguments ---------------------------------

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const args = (result as { arguments: Record<string, unknown> }).arguments;

  const appendMode = !!flags["append"];
  const ignoreInterrupts = !!flags["ignore_interrupts"];

  // Normalize file list.
  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = [];
  } else if (typeof files === "string") {
    files = [files];
  }

  // --- Step 4: Set up signal handling --------------------------------------
  // If -i is specified, ignore SIGINT so tee survives Ctrl+C.

  if (ignoreInterrupts) {
    process.on("SIGINT", () => {
      // Intentionally do nothing -- ignore the interrupt.
    });
  }

  // --- Step 5: Read stdin and tee to stdout and files ----------------------
  // We read all of stdin at once (synchronous), then write to all targets.

  let input: string;
  try {
    input = fs.readFileSync(0, "utf-8");
  } catch {
    // If stdin is not available, exit gracefully.
    return;
  }

  // Write to stdout.
  process.stdout.write(input);

  // Write to each file.
  const writeFlag = appendMode ? "a" : "w";

  for (const file of files) {
    try {
      fs.writeFileSync(file, input, { flag: writeFlag });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`tee: ${file}: ${message}\n`);
      process.exitCode = 1;
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
