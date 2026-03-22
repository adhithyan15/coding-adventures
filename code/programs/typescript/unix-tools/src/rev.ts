/**
 * rev -- reverse lines characterwise.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the `rev` utility in TypeScript. It
 * copies the specified files to standard output, reversing the order
 * of characters in every line.
 *
 * === How rev Works ===
 *
 * rev reverses each line independently:
 *
 *     echo "hello" | rev    =>   "olleh"
 *     echo "abc\nxyz" | rev =>   "cba\nzyx"
 *
 * === Why rev Exists ===
 *
 * rev is useful in shell pipelines for manipulating text. A common
 * idiom is using `rev | cut | rev` to extract fields from the end
 * of a line:
 *
 *     echo "/usr/local/bin/node" | rev | cut -d/ -f1 | rev
 *     => "node"
 *
 * === Unicode Considerations ===
 *
 * Reversing a string is trickier than it sounds when Unicode is involved.
 * JavaScript strings are UTF-16, and some characters (like emoji) are
 * represented as surrogate pairs -- two 16-bit code units for one visible
 * character.
 *
 * We use the spread operator `[...str]` to split into Unicode code points
 * (not UTF-16 code units), then reverse and rejoin. This correctly handles
 * characters outside the Basic Multilingual Plane (BMP).
 *
 * @module rev
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
const SPEC_FILE = path.resolve(__dirname, "..", "rev.json");

// ---------------------------------------------------------------------------
// Business Logic: Reverse each line.
// ---------------------------------------------------------------------------

/**
 * Reverse the characters in a single line.
 *
 * We use the spread operator to properly handle multi-byte Unicode
 * characters (emoji, CJK, etc.). Without the spread, a naive
 * `split('').reverse().join('')` would break surrogate pairs.
 *
 * === Example ===
 *
 *     reverseLine("hello")  => "olleh"
 *     reverseLine("ab cd")  => "dc ba"
 *     reverseLine("")       => ""
 */
function reverseLine(line: string): string {
  return [...line].reverse().join("");
}

/**
 * Process file content: reverse each line and write to stdout.
 *
 * We split on newlines, reverse each line independently, and rejoin.
 * The trailing newline (if present) is preserved.
 */
function processContent(content: string): void {
  const lines = content.split("\n");

  // Handle trailing newline: split produces an empty final element.
  const hasTrailing = content.endsWith("\n");
  const effectiveLines = hasTrailing ? lines.slice(0, -1) : lines;

  for (const line of effectiveLines) {
    process.stdout.write(reverseLine(line) + "\n");
  }

  // If the original content didn't end with a newline, we've added one.
  // That's actually what GNU rev does too -- it always outputs a newline
  // after each line.
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then reverse lines.
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
        process.stderr.write(`rev: ${error.message}\n`);
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

  // --- Step 3: Extract arguments -------------------------------------------

  const args = (result as { arguments: Record<string, unknown> }).arguments;

  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

  // --- Step 4: Process each file -------------------------------------------

  for (const file of files) {
    let content: string;

    if (file === "-") {
      try {
        content = fs.readFileSync(0, "utf-8");
      } catch {
        continue;
      }
    } else {
      try {
        content = fs.readFileSync(file, "utf-8");
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        process.stderr.write(`rev: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    processContent(content);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
