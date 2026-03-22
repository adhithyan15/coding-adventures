/**
 * tail -- output the last part of files.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `tail` utility in TypeScript. It
 * prints the last N lines (or bytes) of each file to standard output.
 * By default, N is 10.
 *
 * === How tail Works ===
 *
 * tail is the complement of head:
 *
 *     tail file.txt          =>   last 10 lines
 *     tail -n 5 file.txt     =>   last 5 lines
 *     tail -n +3 file.txt    =>   everything starting from line 3
 *     tail -c 100 file.txt   =>   last 100 bytes
 *
 * === The +NUM Syntax ===
 *
 * GNU tail supports a special prefix syntax:
 *
 * - `-n 5` or `-n -5`: Output the last 5 lines.
 * - `-n +5`: Output starting from line 5 (1-indexed).
 *
 * This means the -n and -c flags accept strings, not plain integers,
 * because the `+` prefix changes the semantics entirely. A leading `-`
 * is ignored (it's the default behavior).
 *
 * === Follow Mode (-f) ===
 *
 * With `-f`, tail does not exit after printing the last lines. Instead,
 * it watches the file for new content and prints it as it appears. This
 * is commonly used to monitor log files in real time.
 *
 * Our implementation supports a basic version of follow mode using
 * `fs.watchFile`.
 *
 * @module tail
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
const SPEC_FILE = path.resolve(__dirname, "..", "tail.json");

// ---------------------------------------------------------------------------
// Business Logic: Parse the +/- count syntax.
// ---------------------------------------------------------------------------

/**
 * Parse a count string that may have a + or - prefix.
 *
 * Returns an object with:
 * - `value`: the absolute numeric value
 * - `fromStart`: true if the count starts from the beginning (+ prefix)
 *
 * === Truth Table ===
 *
 *     Input    value   fromStart
 *     -----    -----   ---------
 *     "10"     10      false       (last 10)
 *     "-10"    10      false       (last 10, explicit)
 *     "+10"    10      true        (from line/byte 10)
 */
function parseCount(input: string): { value: number; fromStart: boolean } {
  if (input.startsWith("+")) {
    return { value: parseInt(input.slice(1), 10), fromStart: true };
  } else if (input.startsWith("-")) {
    return { value: parseInt(input.slice(1), 10), fromStart: false };
  } else {
    return { value: parseInt(input, 10), fromStart: false };
  }
}

// ---------------------------------------------------------------------------
// Business Logic: Extract the last N lines or bytes.
// ---------------------------------------------------------------------------

/**
 * Extract lines from content according to the parsed count.
 *
 * If `fromStart` is true, we output from line N onward (1-indexed).
 * Otherwise, we output the last N lines.
 */
function tailLines(
  content: string,
  count: number,
  fromStart: boolean,
  delimiter: string
): string {
  const lines = content.split(delimiter);

  // Remove trailing empty element from final delimiter.
  const hasTrailing = content.endsWith(delimiter);
  const effectiveLines = hasTrailing ? lines.slice(0, -1) : lines;

  let selected: string[];

  if (fromStart) {
    // +N means "start from line N" (1-indexed).
    // +1 means all lines, +2 means skip the first line, etc.
    selected = effectiveLines.slice(count - 1);
  } else {
    // Last N lines.
    selected = count >= effectiveLines.length
      ? effectiveLines
      : effectiveLines.slice(-count);
  }

  if (selected.length === 0) return "";
  return selected.join(delimiter) + delimiter;
}

/**
 * Extract bytes from content according to the parsed count.
 */
function tailBytes(
  content: string,
  count: number,
  fromStart: boolean
): string {
  const buf = Buffer.from(content, "utf-8");

  if (fromStart) {
    // +N means start from byte N (1-indexed).
    return buf.subarray(count - 1).toString("utf-8");
  } else {
    // Last N bytes.
    return buf.subarray(Math.max(0, buf.length - count)).toString("utf-8");
  }
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then output last part of files.
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
        process.stderr.write(`tail: ${error.message}\n`);
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

  const linesStr = (flags["lines"] as string) ?? "10";
  const bytesStr = flags["bytes"] as string | undefined;
  const byteMode = bytesStr !== undefined && bytesStr !== null;
  const follow = !!flags["follow"];
  const quiet = !!flags["quiet"];
  const verbose = !!flags["verbose"];
  const zeroTerminated = !!flags["zero_terminated"];
  const delimiter = zeroTerminated ? "\0" : "\n";

  // Parse the count (handles +/- prefix).
  const linesParsed = parseCount(linesStr);
  const bytesParsed = byteMode ? parseCount(bytesStr!) : null;

  // Normalize file list.
  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

  const showHeaders = quiet ? false : verbose ? true : files.length > 1;

  // --- Step 4: Process each file -------------------------------------------

  for (let i = 0; i < files.length; i++) {
    const file = files[i];

    if (showHeaders) {
      if (i > 0) process.stdout.write("\n");
      const label = file === "-" ? "standard input" : file;
      process.stdout.write(`==> ${label} <==\n`);
    }

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
        process.stderr.write(`tail: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    if (byteMode && bytesParsed) {
      process.stdout.write(tailBytes(content, bytesParsed.value, bytesParsed.fromStart));
    } else {
      process.stdout.write(tailLines(content, linesParsed.value, linesParsed.fromStart, delimiter));
    }
  }

  // --- Step 5: Follow mode (basic implementation) --------------------------
  // If -f is set and we're reading from real files (not stdin), watch for
  // changes. This is a simplified version; GNU tail is much more sophisticated.

  if (follow) {
    const realFiles = files.filter((f) => f !== "-");
    if (realFiles.length === 0) {
      // Following stdin is not supported in this implementation.
      return;
    }

    // Track file sizes so we only print new content.
    const sizes = new Map<string, number>();
    for (const file of realFiles) {
      try {
        sizes.set(file, fs.statSync(file).size);
      } catch {
        sizes.set(file, 0);
      }
    }

    // Poll for changes every second.
    setInterval(() => {
      for (const file of realFiles) {
        try {
          const stat = fs.statSync(file);
          const prevSize = sizes.get(file) || 0;
          if (stat.size > prevSize) {
            const fd = fs.openSync(file, "r");
            const buf = Buffer.alloc(stat.size - prevSize);
            fs.readSync(fd, buf, 0, buf.length, prevSize);
            fs.closeSync(fd);
            process.stdout.write(buf.toString("utf-8"));
            sizes.set(file, stat.size);
          }
        } catch {
          // File may have been removed; ignore.
        }
      }
    }, 1000);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
