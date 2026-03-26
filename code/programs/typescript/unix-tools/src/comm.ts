/**
 * comm -- compare two sorted files line by line.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `comm` utility in TypeScript.
 * It reads two sorted files and produces three columns of output:
 *
 *     Column 1: Lines unique to FILE1.
 *     Column 2: Lines unique to FILE2.
 *     Column 3: Lines common to both files.
 *
 * === How comm Works ===
 *
 * comm uses a merge-like algorithm on two sorted sequences. At each step,
 * it compares the current line from each file:
 *
 *     If line1 < line2  =>  line1 is unique to FILE1 (column 1).
 *     If line1 > line2  =>  line2 is unique to FILE2 (column 2).
 *     If line1 == line2 =>  the line is common (column 3).
 *
 * This is the same algorithm used in the "merge" step of merge sort.
 * It requires both inputs to be sorted, otherwise the output is undefined.
 *
 * === Column Suppression ===
 *
 * The `-1`, `-2`, `-3` flags suppress the corresponding columns:
 *
 *     comm -12 file1 file2   =>  Only column 3 (common lines).
 *     comm -3 file1 file2    =>  Only unique lines from either file.
 *     comm -23 file1 file2   =>  Only lines unique to FILE1.
 *
 * === Output Format ===
 *
 * Each line is indented with TABs to indicate its column:
 *
 *     Column 1: no indent    (unique to FILE1)
 *     Column 2: one TAB      (unique to FILE2)
 *     Column 3: two TABs     (common to both)
 *
 * When columns are suppressed, the remaining columns shift left.
 *
 * @module comm
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
const SPEC_FILE = path.resolve(__dirname, "..", "comm.json");

// ---------------------------------------------------------------------------
// Business Logic: Compare two sorted arrays.
// ---------------------------------------------------------------------------

/**
 * Compare two sorted arrays of lines and produce comm-style output.
 *
 * The suppress tuple controls which columns appear:
 *   suppress[0] => suppress column 1 (unique to lines1)
 *   suppress[1] => suppress column 2 (unique to lines2)
 *   suppress[2] => suppress column 3 (common lines)
 *
 * === Algorithm ===
 *
 * We maintain two pointers (i, j), one for each input. At each step:
 *
 *     lines1[i] < lines2[j]  =>  i is unique to file1, advance i
 *     lines1[i] > lines2[j]  =>  j is unique to file2, advance j
 *     lines1[i] == lines2[j] =>  common, advance both
 *
 * After one array is exhausted, the remaining lines from the other
 * array are all unique to that file.
 *
 * @param lines1    - Sorted lines from file 1.
 * @param lines2    - Sorted lines from file 2.
 * @param suppress  - Which columns to suppress [col1, col2, col3].
 * @param delimiter - Column separator (default: TAB).
 * @returns Array of formatted output lines.
 */
export function compareSorted(
  lines1: string[],
  lines2: string[],
  suppress: [boolean, boolean, boolean],
  delimiter: string = "\t"
): string[] {
  const output: string[] = [];
  let i = 0;
  let j = 0;

  // Compute the prefix for each column.
  // Column 1 has no prefix.
  // Column 2 is indented by one delimiter (unless col1 is suppressed).
  // Column 3 is indented by delimiters for each non-suppressed preceding column.
  const col1Prefix = "";
  const col2Prefix = suppress[0] ? "" : delimiter;
  const col3Prefix =
    (suppress[0] ? "" : delimiter) + (suppress[1] ? "" : delimiter);

  while (i < lines1.length && j < lines2.length) {
    if (lines1[i] < lines2[j]) {
      // Line is unique to file1 (column 1).
      if (!suppress[0]) {
        output.push(col1Prefix + lines1[i]);
      }
      i++;
    } else if (lines1[i] > lines2[j]) {
      // Line is unique to file2 (column 2).
      if (!suppress[1]) {
        output.push(col2Prefix + lines2[j]);
      }
      j++;
    } else {
      // Line is common (column 3).
      if (!suppress[2]) {
        output.push(col3Prefix + lines1[i]);
      }
      i++;
      j++;
    }
  }

  // Process remaining lines from file1.
  while (i < lines1.length) {
    if (!suppress[0]) {
      output.push(col1Prefix + lines1[i]);
    }
    i++;
  }

  // Process remaining lines from file2.
  while (j < lines2.length) {
    if (!suppress[1]) {
      output.push(col2Prefix + lines2[j]);
    }
    j++;
  }

  return output;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then compare.
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
        process.stderr.write(`comm: ${error.message}\n`);
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

  const suppress: [boolean, boolean, boolean] = [
    !!flags["suppress_col1"],
    !!flags["suppress_col2"],
    !!flags["suppress_col3"],
  ];

  const outputDelimiter =
    (flags["output_delimiter"] as string) ?? "\t";
  const zeroTerminated = !!flags["zero_terminated"];
  const lineDelim = zeroTerminated ? "\0" : "\n";

  const file1Path = args["file1"] as string;
  const file2Path = args["file2"] as string;

  // Read both files.
  const readFile = (filePath: string): string[] => {
    let content: string;
    if (filePath === "-") {
      content = fs.readFileSync(0, "utf-8");
    } else {
      content = fs.readFileSync(filePath, "utf-8");
    }
    const lines = content.split(lineDelim);
    if (lines[lines.length - 1] === "") {
      lines.pop();
    }
    return lines;
  };

  let lines1: string[];
  let lines2: string[];

  try {
    lines1 = readFile(file1Path);
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    process.stderr.write(`comm: ${file1Path}: ${message}\n`);
    process.exit(1);
    return;
  }

  try {
    lines2 = readFile(file2Path);
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    process.stderr.write(`comm: ${file2Path}: ${message}\n`);
    process.exit(1);
    return;
  }

  const output = compareSorted(lines1, lines2, suppress, outputDelimiter);

  for (const line of output) {
    process.stdout.write(line + lineDelim);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
