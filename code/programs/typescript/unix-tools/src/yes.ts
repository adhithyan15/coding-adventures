/**
 * yes -- output a string repeatedly until killed.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `yes` utility in TypeScript.
 * It repeatedly outputs a line of text. If no arguments are given, it
 * outputs "y" on each line. If arguments are given, they are joined
 * with spaces and that string is output on each line.
 *
 * === Why Does yes Exist? ===
 *
 * `yes` is a tiny tool with a big role in shell scripting. Many Unix
 * commands prompt for confirmation ("Are you sure? [y/n]"). By piping
 * `yes` into them, you can automate the "yes to everything" response:
 *
 *     yes | rm -i *.tmp       (answers "y" to every prompt)
 *     yes DELETE | some-tool  (answers "DELETE" to every prompt)
 *
 * === Variadic Arguments ===
 *
 * When multiple arguments are given, they are joined with spaces, just
 * like `echo`:
 *
 *     yes hello world    =>    "hello world\n" repeated forever
 *
 * === Testability ===
 *
 * The real `yes` runs forever (until killed by a signal or broken pipe).
 * For testing, we export a `yesOutput` function that generates a finite
 * number of lines, allowing unit tests to verify the output without
 * hanging.
 *
 * @module yes
 */

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
const SPEC_FILE = path.resolve(__dirname, "..", "yes.json");

// ---------------------------------------------------------------------------
// Business Logic: Generate repeated output lines.
// ---------------------------------------------------------------------------

/**
 * Generate an array of repeated lines for testing.
 *
 * This function is the testable core of `yes`. Instead of printing
 * forever, it returns exactly `maxLines` copies of the given line.
 *
 * === Why a separate function? ===
 *
 * The real `yes` is an infinite loop -- it writes until the receiving
 * process closes the pipe (SIGPIPE). That behavior is impossible to
 * unit-test directly. By extracting the line-generation logic into a
 * pure function that accepts a line count, we can test the output
 * format without hanging the test runner.
 *
 * @param line     The string to repeat on each line.
 * @param maxLines How many lines to generate.
 * @returns        An array of `maxLines` copies of `line`.
 *
 * @example
 *   yesOutput("y", 3)     => ["y", "y", "y"]
 *   yesOutput("hello", 2) => ["hello", "hello"]
 */
export function yesOutput(line: string, maxLines: number): string[] {
  // Array.from with a mapping function is a clean way to create an
  // array of N identical values. The underscore indicates we don't
  // use the element value (which would be undefined).
  return Array.from({ length: maxLines }, () => line);
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then output the line forever.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * The flow is:
 * 1. Parse arguments with CLI Builder.
 * 2. Handle --help and --version.
 * 3. Join positional arguments (or default to "y").
 * 4. Print the line repeatedly until the process is killed.
 *
 * In practice, `yes` terminates when the receiving process closes the
 * pipe, causing a SIGPIPE signal. Node.js handles this by emitting an
 * 'error' event on stdout with code 'EPIPE'.
 */
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
        process.stderr.write(`yes: ${error.message}\n`);
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

  // --- Step 3: Determine the line to output --------------------------------
  // The "string" argument is variadic and optional. If no arguments were
  // given, it defaults to "y". If multiple arguments were given, they are
  // joined with spaces.

  const args = (result as { arguments: Record<string, unknown> }).arguments;
  let strings = args["string"] as string[] | string | undefined;

  let line: string;
  if (!strings || (Array.isArray(strings) && strings.length === 0)) {
    line = "y";
  } else if (typeof strings === "string") {
    line = strings;
  } else {
    line = strings.join(" ");
  }

  // --- Step 4: Output the line forever -------------------------------------
  // We handle EPIPE gracefully: when the receiving process closes its
  // stdin (e.g., `yes | head -5`), Node.js will emit an error on stdout.
  // We catch it and exit cleanly.

  process.stdout.on("error", (err: NodeJS.ErrnoException) => {
    if (err.code === "EPIPE") {
      process.exit(0);
    }
    process.exit(1);
  });

  // Write in a loop. This will run until SIGPIPE or the process is killed.
  const lineWithNewline = line + "\n";
  const writeLoop = (): void => {
    while (process.stdout.write(lineWithNewline)) {
      // The write succeeded synchronously; keep going.
    }
    // The write returned false (backpressure). Wait for 'drain' to resume.
    process.stdout.once("drain", writeLoop);
  };

  writeLoop();
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------
// We guard the main() call so that this module can be imported for testing
// without triggering the infinite output loop. The `VITEST` environment
// variable is set automatically by vitest when running tests.

if (!process.env.VITEST) {
  main();
}
