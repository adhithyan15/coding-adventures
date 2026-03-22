/**
 * uniq -- report or omit repeated lines.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `uniq` utility in TypeScript. It
 * filters adjacent matching lines from input, writing the results to
 * output.
 *
 * === How uniq Works ===
 *
 * uniq compares **adjacent** lines. It does NOT sort the input -- if you
 * want to find all duplicates regardless of position, pipe through `sort`
 * first:
 *
 *     sort file.txt | uniq         =>   unique lines (sorted)
 *     uniq file.txt                =>   collapse adjacent duplicates
 *
 * === Operation Modes ===
 *
 * By default, uniq outputs one copy of each group of adjacent identical
 * lines (keeping the first):
 *
 *     Input:     Output:
 *     apple      apple
 *     apple      banana
 *     banana     cherry
 *     cherry
 *     cherry
 *
 * Flags modify what gets output:
 *
 * - `-c` (count): Prefix each line with the number of occurrences.
 * - `-d` (repeated): Only show lines that appear more than once.
 * - `-u` (unique): Only show lines that appear exactly once.
 * - `-D` (all-repeated): Show ALL duplicate lines (not just one per group).
 *
 * === Comparison Options ===
 *
 * - `-i`: Ignore case when comparing.
 * - `-f N`: Skip the first N fields (whitespace-separated) before comparing.
 * - `-s N`: Skip the first N characters before comparing.
 * - `-w N`: Compare no more than N characters.
 *
 * @module uniq
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
const SPEC_FILE = path.resolve(__dirname, "..", "uniq.json");

// ---------------------------------------------------------------------------
// Business Logic: Line comparison with field/char skipping.
// ---------------------------------------------------------------------------

/**
 * Extract the comparison key from a line, applying skip and check options.
 *
 * This implements the comparison pipeline:
 * 1. Skip the first `skipFields` whitespace-delimited fields.
 * 2. Skip the first `skipChars` characters of what remains.
 * 3. Take only the first `checkChars` characters (if specified).
 *
 * === Why Fields First, Then Characters? ===
 *
 * This matches GNU uniq behavior. Fields are skipped first because fields
 * can have variable length. Then character skipping is applied to what
 * remains.
 */
function getComparisonKey(
  line: string,
  skipFields: number,
  skipChars: number,
  checkChars: number | undefined,
  ignoreCase: boolean
): string {
  let key = line;

  // Step 1: Skip fields.
  if (skipFields > 0) {
    // A "field" is a run of whitespace followed by non-whitespace.
    // We skip `skipFields` such groups.
    let pos = 0;
    let fieldsSkipped = 0;

    while (fieldsSkipped < skipFields && pos < key.length) {
      // Skip whitespace.
      while (pos < key.length && (key[pos] === " " || key[pos] === "\t")) {
        pos++;
      }
      // Skip non-whitespace (the field content).
      while (pos < key.length && key[pos] !== " " && key[pos] !== "\t") {
        pos++;
      }
      fieldsSkipped++;
    }

    key = key.substring(pos);
  }

  // Step 2: Skip characters.
  if (skipChars > 0) {
    key = key.substring(skipChars);
  }

  // Step 3: Limit to checkChars characters.
  if (checkChars !== undefined) {
    key = key.substring(0, checkChars);
  }

  // Step 4: Case-insensitive comparison.
  if (ignoreCase) {
    key = key.toLowerCase();
  }

  return key;
}

// ---------------------------------------------------------------------------
// Types: A group of identical adjacent lines.
// ---------------------------------------------------------------------------

/**
 * A group represents a run of adjacent identical lines.
 * We store the first line (for output) and the count of occurrences.
 */
interface LineGroup {
  line: string;
  count: number;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then filter repeated lines.
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
        process.stderr.write(`uniq: ${error.message}\n`);
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

  const showCount = !!flags["count"];
  const repeatedOnly = !!flags["repeated"];
  const uniqueOnly = !!flags["unique"];
  const skipFields = (flags["skip_fields"] as number) ?? 0;
  const skipChars = (flags["skip_chars"] as number) ?? 0;
  const checkChars = flags["check_chars"] as number | undefined;
  const ignoreCase = !!flags["ignore_case"];
  const zeroTerminated = !!flags["zero_terminated"];
  const delimiter = zeroTerminated ? "\0" : "\n";

  const inputFile = args["input"] as string | undefined;
  const outputFile = args["output"] as string | undefined;

  // --- Step 4: Read input --------------------------------------------------

  let content: string;

  if (!inputFile || inputFile === "-") {
    try {
      content = fs.readFileSync(0, "utf-8");
    } catch {
      return;
    }
  } else {
    try {
      content = fs.readFileSync(inputFile, "utf-8");
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`uniq: ${inputFile}: ${message}\n`);
      process.exit(1);
    }
  }

  // --- Step 5: Group adjacent identical lines ------------------------------

  const lines = content.split(delimiter);

  // Remove trailing empty element if content ends with delimiter.
  const hasTrailing = content.endsWith(delimiter);
  if (hasTrailing && lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }

  // Group adjacent lines with matching comparison keys.
  const groups: LineGroup[] = [];

  for (const line of lines) {
    const key = getComparisonKey(line, skipFields, skipChars, checkChars, ignoreCase);

    if (
      groups.length > 0 &&
      getComparisonKey(groups[groups.length - 1].line, skipFields, skipChars, checkChars, ignoreCase) === key
    ) {
      groups[groups.length - 1].count++;
    } else {
      groups.push({ line, count: 1 });
    }
  }

  // --- Step 6: Format and output -------------------------------------------

  const outputLines: string[] = [];

  for (const group of groups) {
    // Apply filters.
    if (repeatedOnly && group.count < 2) continue;
    if (uniqueOnly && group.count > 1) continue;

    // Format the line.
    if (showCount) {
      outputLines.push(`      ${group.count} ${group.line}`);
    } else {
      outputLines.push(group.line);
    }
  }

  const outputContent = outputLines.length > 0
    ? outputLines.join(delimiter) + delimiter
    : "";

  // Write output.
  if (outputFile) {
    try {
      fs.writeFileSync(outputFile, outputContent);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`uniq: ${outputFile}: ${message}\n`);
      process.exit(1);
    }
  } else {
    process.stdout.write(outputContent);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
