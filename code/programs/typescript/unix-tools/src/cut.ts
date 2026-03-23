/**
 * cut -- remove sections from each line of files.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `cut` utility in TypeScript. It
 * selects parts of each line based on byte positions, character positions,
 * or delimited fields, and prints only the selected parts.
 *
 * === How cut Works ===
 *
 * cut operates in one of three modes (mutually exclusive):
 *
 * - **Bytes** (`-b LIST`): Select specific byte positions.
 * - **Characters** (`-c LIST`): Select specific character positions.
 * - **Fields** (`-f LIST`): Select specific fields (delimited by TAB
 *   or a custom delimiter).
 *
 * === Range List Syntax ===
 *
 * The LIST argument specifies which bytes/chars/fields to select:
 *
 *     N       - The Nth element (1-indexed).
 *     N-M     - From Nth to Mth element (inclusive).
 *     N-      - From Nth element to end of line.
 *     -M      - From first element to Mth (same as 1-M).
 *     N,M,... - Multiple ranges, comma-separated.
 *
 * Examples:
 *
 *     cut -c 1-5 file.txt         =>  first 5 characters of each line
 *     cut -f 2,4 -d ',' file.csv  =>  fields 2 and 4 (comma-delimited)
 *     cut -b 1-3,7- file.txt      =>  bytes 1-3 and 7 to end
 *
 * === Field Mode Specifics ===
 *
 * In field mode (`-f`):
 * - The default delimiter is TAB.
 * - Lines without the delimiter are printed unchanged (unless `-s` is set).
 * - `-s` (only-delimited): Suppress lines that don't contain the delimiter.
 * - `--output-delimiter`: Use a different delimiter in the output.
 *
 * @module cut
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
const SPEC_FILE = path.resolve(__dirname, "..", "cut.json");

// ---------------------------------------------------------------------------
// Types: Options for cut operations.
// ---------------------------------------------------------------------------

/**
 * Options controlling how cut processes each line.
 *
 * Only one of `bytes`, `characters`, or `fields` will be set, matching
 * the mutually exclusive flag group in the spec.
 */
export interface CutOptions {
  /** Byte positions to select (e.g., "1-3,5,7-"). */
  bytes?: string;
  /** Character positions to select. */
  characters?: string;
  /** Field numbers to select. */
  fields?: string;
  /** Field delimiter (default: TAB). */
  delimiter: string;
  /** Only print lines containing the delimiter (field mode). */
  onlyDelimited: boolean;
  /** Output delimiter string. */
  outputDelimiter?: string;
  /** Complement the selection. */
  complement: boolean;
}

// ---------------------------------------------------------------------------
// Types: Parsed range.
// ---------------------------------------------------------------------------

/**
 * A parsed range from a range list.
 *
 * Ranges are 1-indexed (matching cut convention). An undefined `end`
 * means "to the end of the line" (e.g., "5-" means start=5, end=Infinity).
 */
export interface Range {
  start: number;
  end: number;
}

// ---------------------------------------------------------------------------
// Business Logic: Parse a range list like "1-3,5,7-".
// ---------------------------------------------------------------------------

/**
 * Parse a range list string into an array of Range objects.
 *
 * The range list syntax is:
 *
 *     "1"      => [{start: 1, end: 1}]
 *     "1-3"    => [{start: 1, end: 3}]
 *     "5-"     => [{start: 5, end: Infinity}]
 *     "-3"     => [{start: 1, end: 3}]
 *     "1,3,5"  => [{start:1,end:1}, {start:3,end:3}, {start:5,end:5}]
 *
 * @param rangeStr - The range list string.
 * @returns An array of parsed ranges, sorted by start position.
 */
export function parseRangeList(rangeStr: string): Range[] {
  const ranges: Range[] = [];

  for (const part of rangeStr.split(",")) {
    const trimmed = part.trim();

    if (trimmed.includes("-")) {
      const [startStr, endStr] = trimmed.split("-", 2);
      const start = startStr === "" ? 1 : parseInt(startStr, 10);
      const end = endStr === "" ? Infinity : parseInt(endStr, 10);
      ranges.push({ start, end });
    } else {
      const n = parseInt(trimmed, 10);
      ranges.push({ start: n, end: n });
    }
  }

  // Sort by start position for efficient processing.
  ranges.sort((a, b) => a.start - b.start);
  return ranges;
}

// ---------------------------------------------------------------------------
// Business Logic: Check if an index is selected by the ranges.
// ---------------------------------------------------------------------------

/**
 * Check if a 1-indexed position is selected by any of the ranges.
 *
 * @param pos    - The 1-indexed position to check.
 * @param ranges - The parsed ranges.
 * @returns True if the position is within any range.
 */
export function isSelected(pos: number, ranges: Range[]): boolean {
  for (const range of ranges) {
    if (pos >= range.start && pos <= range.end) {
      return true;
    }
  }
  return false;
}

// ---------------------------------------------------------------------------
// Business Logic: Cut a single line by bytes or characters.
// ---------------------------------------------------------------------------

/**
 * Cut a line by selecting specific character or byte positions.
 *
 * For simplicity, we treat bytes and characters the same way (since
 * Node.js strings are UTF-16 and we're working with string indexing).
 * This matches the behavior for ASCII text. For true byte-level
 * cutting, we'd need Buffer operations.
 *
 * @param line       - The input line.
 * @param rangeStr   - The range list string.
 * @param complement - If true, select everything NOT in the ranges.
 * @param outputDelimiter - Delimiter between selected ranges in output.
 * @returns The cut line.
 */
export function cutByChars(
  line: string,
  rangeStr: string,
  complement: boolean,
  outputDelimiter?: string
): string {
  const ranges = parseRangeList(rangeStr);
  const chars = [...line]; // Handle multi-byte chars correctly.
  const selected: string[] = [];

  for (let i = 0; i < chars.length; i++) {
    const pos = i + 1; // 1-indexed.
    const inRange = isSelected(pos, ranges);
    if (complement ? !inRange : inRange) {
      selected.push(chars[i]);
    }
  }

  // When an output delimiter is specified, we group consecutive selected
  // chars and join groups with the delimiter. Without it, just concatenate.
  if (outputDelimiter !== undefined) {
    // Group consecutive selected positions.
    const groups: string[][] = [];
    let currentGroup: string[] = [];
    let lastPos = -1;

    for (let i = 0; i < chars.length; i++) {
      const pos = i + 1;
      const inRange = isSelected(pos, ranges);
      if (complement ? !inRange : inRange) {
        if (lastPos !== -1 && pos !== lastPos + 1) {
          groups.push(currentGroup);
          currentGroup = [];
        }
        currentGroup.push(chars[i]);
        lastPos = pos;
      }
    }
    if (currentGroup.length > 0) groups.push(currentGroup);
    return groups.map((g) => g.join("")).join(outputDelimiter);
  }

  return selected.join("");
}

// ---------------------------------------------------------------------------
// Business Logic: Cut a single line by fields.
// ---------------------------------------------------------------------------

/**
 * Cut a line by selecting specific delimited fields.
 *
 * Fields are 1-indexed and separated by the delimiter character. If the
 * line doesn't contain the delimiter and `onlyDelimited` is false, the
 * entire line is returned unchanged.
 *
 * @param line    - The input line.
 * @param opts    - Cut options (delimiter, ranges, etc.).
 * @returns The cut line, or null if the line should be suppressed.
 */
export function cutByFields(
  line: string,
  opts: CutOptions
): string | null {
  const delimiter = opts.delimiter;
  const rangeStr = opts.fields!;

  // If line doesn't contain the delimiter, handle specially.
  if (!line.includes(delimiter)) {
    if (opts.onlyDelimited) return null;
    return line;
  }

  const fields = line.split(delimiter);
  const ranges = parseRangeList(rangeStr);
  const selected: string[] = [];

  for (let i = 0; i < fields.length; i++) {
    const pos = i + 1; // 1-indexed.
    const inRange = isSelected(pos, ranges);
    if (opts.complement ? !inRange : inRange) {
      selected.push(fields[i]);
    }
  }

  const outDelim = opts.outputDelimiter ?? delimiter;
  return selected.join(outDelim);
}

// ---------------------------------------------------------------------------
// Business Logic: Cut a single line (dispatcher).
// ---------------------------------------------------------------------------

/**
 * Process a single line through cut.
 *
 * This dispatches to the appropriate cut function based on which mode
 * is active (bytes, characters, or fields).
 *
 * @param line - The input line.
 * @param opts - Cut options.
 * @returns The processed line, or null to suppress it.
 */
export function cutLine(line: string, opts: CutOptions): string | null {
  if (opts.fields) {
    return cutByFields(line, opts);
  }

  const rangeStr = opts.bytes ?? opts.characters ?? "";
  return cutByChars(line, rangeStr, opts.complement, opts.outputDelimiter);
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then cut.
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
        process.stderr.write(`cut: ${error.message}\n`);
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

  // --- Step 3: Build options -----------------------------------------------

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const args = (result as { arguments: Record<string, unknown> }).arguments;

  const opts: CutOptions = {
    bytes: flags["bytes"] as string | undefined,
    characters: flags["characters"] as string | undefined,
    fields: flags["fields"] as string | undefined,
    delimiter: (flags["delimiter"] as string) ?? "\t",
    onlyDelimited: !!flags["only_delimited"],
    outputDelimiter: flags["output_delimiter"] as string | undefined,
    complement: !!flags["complement"],
  };

  const zeroTerminated = !!flags["zero_terminated"];
  const delimiter = zeroTerminated ? "\0" : "\n";

  // --- Step 4: Read and process input --------------------------------------

  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

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
        process.stderr.write(`cut: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    const lines = content.split(delimiter);
    // Remove trailing empty line from final delimiter.
    if (lines[lines.length - 1] === "") {
      lines.pop();
    }

    for (const line of lines) {
      const output = cutLine(line, opts);
      if (output !== null) {
        process.stdout.write(output + delimiter);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
