/**
 * paste -- merge lines of files.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `paste` utility in TypeScript. It
 * merges corresponding lines from multiple files, joining them with a
 * delimiter (TAB by default).
 *
 * === How paste Works ===
 *
 * In parallel mode (default), paste reads one line from each file and
 * joins them:
 *
 *     File A:     File B:     Output:
 *     alpha       one         alpha\tone
 *     beta        two         beta\ttwo
 *     gamma       three       gamma\tthree
 *
 * If files have different lengths, missing values are empty strings.
 *
 * === Serial Mode (-s) ===
 *
 * In serial mode, paste processes one file at a time, joining all lines
 * from that file into a single output line:
 *
 *     File A:     Output:
 *     alpha       alpha\tbeta\tgamma
 *     beta
 *     gamma
 *
 * === Delimiter Cycling ===
 *
 * The delimiter list (`-d`) is cycled through. With `-d ',:'` and three
 * files, the output for each row uses comma between files 1-2, colon
 * between files 2-3, and then wraps back to comma for the next row.
 *
 * Special escape sequences in delimiter list:
 *   \n => newline, \t => tab, \\ => backslash, \0 => empty string
 *
 * @module paste
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
const SPEC_FILE = path.resolve(__dirname, "..", "paste.json");

// ---------------------------------------------------------------------------
// Business Logic: Parse delimiter list with escape sequences.
// ---------------------------------------------------------------------------

/**
 * Parse a delimiter list string, handling escape sequences.
 *
 * The delimiter list can contain special escape sequences:
 *   \\n => newline character
 *   \\t => tab character
 *   \\\\ => backslash
 *   \\0 => empty string (no delimiter)
 *
 * Each character (after escape processing) becomes one delimiter in the
 * rotation.
 *
 * @param delimStr - The raw delimiter string from the command line.
 * @returns An array of delimiter strings (each typically one char).
 */
export function parseDelimiters(delimStr: string): string[] {
  const delimiters: string[] = [];
  let i = 0;

  while (i < delimStr.length) {
    if (delimStr[i] === "\\" && i + 1 < delimStr.length) {
      const next = delimStr[i + 1];
      switch (next) {
        case "n":
          delimiters.push("\n");
          break;
        case "t":
          delimiters.push("\t");
          break;
        case "\\":
          delimiters.push("\\");
          break;
        case "0":
          delimiters.push("");
          break;
        default:
          delimiters.push(next);
          break;
      }
      i += 2;
    } else {
      delimiters.push(delimStr[i]);
      i++;
    }
  }

  return delimiters;
}

// ---------------------------------------------------------------------------
// Business Logic: Merge lines in parallel mode.
// ---------------------------------------------------------------------------

/**
 * Merge lines from multiple files in parallel mode.
 *
 * For each "row" (line number), we take one line from each input array
 * and join them with delimiters. The delimiters cycle through the
 * delimiter list.
 *
 * When an input array is shorter than the longest, missing values are
 * treated as empty strings.
 *
 * @param inputs     - Array of line arrays (one per file).
 * @param delimiters - Delimiter characters to cycle through.
 * @returns Array of merged output lines.
 */
export function pasteParallel(
  inputs: string[][],
  delimiters: string[]
): string[] {
  if (inputs.length === 0) return [];

  const maxLen = Math.max(...inputs.map((arr) => arr.length));
  const result: string[] = [];

  for (let row = 0; row < maxLen; row++) {
    const parts: string[] = [];
    for (let col = 0; col < inputs.length; col++) {
      if (col > 0) {
        // Use the delimiter at index (col - 1) % delimiters.length.
        const delimIdx = (col - 1) % delimiters.length;
        parts.push(delimiters[delimIdx]);
      }
      parts.push(inputs[col][row] ?? "");
    }
    result.push(parts.join(""));
  }

  return result;
}

// ---------------------------------------------------------------------------
// Business Logic: Merge lines in serial mode.
// ---------------------------------------------------------------------------

/**
 * Merge lines from multiple files in serial mode.
 *
 * Each input array becomes a single output line, with all its lines
 * joined by the cycling delimiters.
 *
 * @param inputs     - Array of line arrays (one per file).
 * @param delimiters - Delimiter characters to cycle through.
 * @returns Array of merged output lines (one per input file).
 */
export function pasteSerial(
  inputs: string[][],
  delimiters: string[]
): string[] {
  const result: string[] = [];

  for (const lines of inputs) {
    const parts: string[] = [];
    for (let i = 0; i < lines.length; i++) {
      if (i > 0) {
        const delimIdx = (i - 1) % delimiters.length;
        parts.push(delimiters[delimIdx]);
      }
      parts.push(lines[i]);
    }
    result.push(parts.join(""));
  }

  return result;
}

// ---------------------------------------------------------------------------
// Business Logic: Main paste function.
// ---------------------------------------------------------------------------

/**
 * Merge lines from multiple inputs.
 *
 * This is the top-level function that dispatches to parallel or serial
 * mode based on the `serial` flag.
 *
 * @param inputs     - Array of line arrays (one per file).
 * @param delimStr   - Delimiter string (default: TAB).
 * @param serial     - Whether to use serial mode.
 * @returns Array of merged output lines.
 */
export function pasteLines(
  inputs: string[][],
  delimStr: string,
  serial: boolean
): string[] {
  const delimiters = parseDelimiters(delimStr);

  if (serial) {
    return pasteSerial(inputs, delimiters);
  }
  return pasteParallel(inputs, delimiters);
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then paste.
// ---------------------------------------------------------------------------

function main(): void {
  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`paste: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw err;
  }

  if ("text" in result) {
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const args = (result as { arguments: Record<string, unknown> }).arguments;

  const delimStr = (flags["delimiters"] as string) ?? "\t";
  const serial = !!flags["serial"];
  const zeroTerminated = !!flags["zero_terminated"];
  const lineDelim = zeroTerminated ? "\0" : "\n";

  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

  // Read all files into arrays of lines.
  const inputs: string[][] = [];

  for (const file of files) {
    let content: string;
    if (file === "-") {
      try {
        content = fs.readFileSync(0, "utf-8");
      } catch {
        inputs.push([]);
        continue;
      }
    } else {
      try {
        content = fs.readFileSync(file, "utf-8");
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        process.stderr.write(`paste: ${file}: ${message}\n`);
        process.exitCode = 1;
        inputs.push([]);
        continue;
      }
    }

    const lines = content.split(lineDelim);
    if (lines[lines.length - 1] === "") {
      lines.pop();
    }
    inputs.push(lines);
  }

  const merged = pasteLines(inputs, delimStr, serial);

  for (const line of merged) {
    process.stdout.write(line + lineDelim);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
