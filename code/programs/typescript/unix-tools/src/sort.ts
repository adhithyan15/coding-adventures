/**
 * sort -- sort lines of text files.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `sort` utility in TypeScript. It
 * reads lines from files (or stdin), sorts them according to various
 * criteria, and writes the result to stdout (or a file).
 *
 * === How sort Works ===
 *
 * By default, sort performs a lexicographic (dictionary) comparison of
 * entire lines:
 *
 *     $ echo -e "banana\napple\ncherry" | sort
 *     apple
 *     banana
 *     cherry
 *
 * === Sort Modes ===
 *
 * sort supports several comparison modes:
 *
 * - **Lexicographic** (default): Compare lines as strings using locale.
 * - **Numeric** (`-n`): Compare according to string numerical value.
 *   Leading whitespace is ignored. Lines that don't start with a number
 *   are treated as 0.
 * - **General numeric** (`-g`): Like numeric but handles scientific
 *   notation (1.5e3).
 * - **Human numeric** (`-h`): Compare human-readable numbers with SI
 *   suffixes (1K, 2M, 3G).
 * - **Month** (`-M`): Compare three-letter month abbreviations
 *   (JAN < FEB < ... < DEC).
 * - **Version** (`-V`): Natural sort of version numbers within text
 *   (e.g., file1 < file2 < file10).
 *
 * === Modifiers ===
 *
 * - `-r` (reverse): Reverse the comparison result.
 * - `-f` (ignore-case): Fold lowercase to uppercase before comparing.
 * - `-d` (dictionary-order): Only consider blanks and alphanumeric chars.
 * - `-i` (ignore-nonprinting): Only consider printable characters.
 * - `-b` (ignore-leading-blanks): Ignore leading whitespace.
 * - `-u` (unique): Output only the first line of an equal run.
 * - `-s` (stable): Preserve original order of equal elements.
 *
 * @module sort
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
const SPEC_FILE = path.resolve(__dirname, "..", "sort.json");

// ---------------------------------------------------------------------------
// Types: Options for sorting.
// ---------------------------------------------------------------------------

/**
 * Configuration options for the sort operation.
 *
 * Each boolean corresponds to a command-line flag. The `key` array
 * allows sorting by specific fields within each line (the `-k` flag).
 */
export interface SortOptions {
  reverse: boolean;
  numeric: boolean;
  humanNumeric: boolean;
  monthSort: boolean;
  generalNumeric: boolean;
  versionSort: boolean;
  unique: boolean;
  ignoreCase: boolean;
  dictionaryOrder: boolean;
  ignoreNonprinting: boolean;
  ignoreLeadingBlanks: boolean;
  stable: boolean;
  fieldSeparator?: string;
  key?: string[];
}

// ---------------------------------------------------------------------------
// Month name mapping for -M (month sort).
// ---------------------------------------------------------------------------

/**
 * Month abbreviation to sort order.
 *
 * Unknown strings sort before JAN (get value 0). This matches GNU
 * sort behavior where unrecognized month names are treated as less
 * than any valid month.
 */
const MONTHS: Record<string, number> = {
  JAN: 1,
  FEB: 2,
  MAR: 3,
  APR: 4,
  MAY: 5,
  JUN: 6,
  JUL: 7,
  AUG: 8,
  SEP: 9,
  OCT: 10,
  NOV: 11,
  DEC: 12,
};

// ---------------------------------------------------------------------------
// Human-readable suffix multipliers for -h (human numeric sort).
// ---------------------------------------------------------------------------

/**
 * SI suffix multipliers for human-readable sort.
 *
 * When sorting "2K" vs "1M", we multiply by these factors to get
 * a comparable numeric value. The suffixes are case-insensitive.
 */
const HUMAN_SUFFIXES: Record<string, number> = {
  K: 1e3,
  M: 1e6,
  G: 1e9,
  T: 1e12,
  P: 1e15,
  E: 1e18,
};

// ---------------------------------------------------------------------------
// Business Logic: Parse human-readable number.
// ---------------------------------------------------------------------------

/**
 * Parse a human-readable number like "2.5K" or "1G" into a plain number.
 *
 * The format is: optional whitespace, optional sign, digits (with optional
 * decimal point), optional SI suffix (K, M, G, T, P, E).
 *
 * @param s - The string to parse.
 * @returns The numeric value, or 0 if unparseable.
 */
export function parseHumanNumber(s: string): number {
  const trimmed = s.trim();
  const match = trimmed.match(/^([+-]?\d*\.?\d+)\s*([KMGTPE])?$/i);
  if (!match) return 0;
  const num = parseFloat(match[1]);
  const suffix = match[2]?.toUpperCase();
  if (suffix && suffix in HUMAN_SUFFIXES) {
    return num * HUMAN_SUFFIXES[suffix];
  }
  return num;
}

// ---------------------------------------------------------------------------
// Business Logic: Get month sort value.
// ---------------------------------------------------------------------------

/**
 * Get the sort order for a month abbreviation.
 *
 * We extract the first three non-whitespace characters and look them up
 * in the MONTHS table. Unknown values return 0 (sort before JAN).
 *
 * @param s - The string to extract a month from.
 * @returns A number 0-12 for sorting.
 */
export function getMonthValue(s: string): number {
  const trimmed = s.trim().slice(0, 3).toUpperCase();
  return MONTHS[trimmed] ?? 0;
}

// ---------------------------------------------------------------------------
// Business Logic: Version sort comparator.
// ---------------------------------------------------------------------------

/**
 * Compare two strings using "version sort" (natural sort).
 *
 * This splits each string into alternating text and numeric chunks,
 * then compares chunk by chunk. Numeric chunks are compared as numbers
 * (so "file2" < "file10"), while text chunks are compared lexicographically.
 *
 * === Algorithm ===
 *
 *     "file10.txt" => ["file", 10, ".txt"]
 *     "file2.txt"  => ["file", 2, ".txt"]
 *
 *     Compare "file" vs "file" => equal
 *     Compare 10 vs 2          => 10 > 2
 *     Result: "file10.txt" > "file2.txt"
 *
 * @param a - First string.
 * @param b - Second string.
 * @returns Negative if a < b, positive if a > b, 0 if equal.
 */
export function versionCompare(a: string, b: string): number {
  const splitVersion = (s: string): (string | number)[] => {
    const parts: (string | number)[] = [];
    let i = 0;
    while (i < s.length) {
      if (s[i] >= "0" && s[i] <= "9") {
        let num = "";
        while (i < s.length && s[i] >= "0" && s[i] <= "9") {
          num += s[i++];
        }
        parts.push(parseInt(num, 10));
      } else {
        let text = "";
        while (i < s.length && !(s[i] >= "0" && s[i] <= "9")) {
          text += s[i++];
        }
        parts.push(text);
      }
    }
    return parts;
  };

  const partsA = splitVersion(a);
  const partsB = splitVersion(b);
  const len = Math.max(partsA.length, partsB.length);

  for (let i = 0; i < len; i++) {
    const pa = partsA[i];
    const pb = partsB[i];

    // Missing parts sort before existing parts.
    if (pa === undefined) return -1;
    if (pb === undefined) return 1;

    // Both numbers: compare numerically.
    if (typeof pa === "number" && typeof pb === "number") {
      if (pa !== pb) return pa - pb;
      continue;
    }

    // Both strings: compare lexicographically.
    if (typeof pa === "string" && typeof pb === "string") {
      if (pa < pb) return -1;
      if (pa > pb) return 1;
      continue;
    }

    // Mixed: numbers sort before strings.
    if (typeof pa === "number") return -1;
    return 1;
  }

  return 0;
}

// ---------------------------------------------------------------------------
// Business Logic: Apply text transformations for comparison.
// ---------------------------------------------------------------------------

/**
 * Transform a line for comparison based on sort options.
 *
 * This applies modifiers like ignore-case, dictionary-order, etc.
 * The transformed string is used only for comparison -- the original
 * line is preserved in the output.
 *
 * @param line - The line to transform.
 * @param opts - Sort options controlling transformations.
 * @returns The transformed line for comparison.
 */
export function transformForComparison(
  line: string,
  opts: SortOptions
): string {
  let result = line;

  if (opts.ignoreLeadingBlanks) {
    result = result.replace(/^\s+/, "");
  }

  if (opts.ignoreCase) {
    result = result.toUpperCase();
  }

  if (opts.dictionaryOrder) {
    // Keep only blanks and alphanumeric characters.
    result = result.replace(/[^a-zA-Z0-9\s]/g, "");
  }

  if (opts.ignoreNonprinting) {
    // Keep only printable ASCII characters (0x20-0x7E).
    result = result.replace(/[^\x20-\x7e]/g, "");
  }

  return result;
}

// ---------------------------------------------------------------------------
// Business Logic: Build comparator from options.
// ---------------------------------------------------------------------------

/**
 * Build a comparison function from the given sort options.
 *
 * This is a higher-order function that returns a comparator suitable
 * for Array.sort(). The comparator applies the appropriate sort mode
 * (numeric, month, version, etc.) and modifiers (reverse, ignore-case).
 *
 * @param opts - Sort options.
 * @returns A comparison function (a, b) => number.
 */
export function buildComparator(
  opts: SortOptions
): (a: string, b: string) => number {
  return (a: string, b: string): number => {
    const ta = transformForComparison(a, opts);
    const tb = transformForComparison(b, opts);

    let result: number;

    if (opts.numeric) {
      // Numeric sort: parse as float, non-numeric lines become 0.
      const na = parseFloat(ta) || 0;
      const nb = parseFloat(tb) || 0;
      result = na - nb;
    } else if (opts.generalNumeric) {
      // General numeric: like numeric but handles scientific notation.
      const na = parseFloat(ta) || 0;
      const nb = parseFloat(tb) || 0;
      result = na - nb;
    } else if (opts.humanNumeric) {
      // Human numeric: parse SI suffixes.
      result = parseHumanNumber(ta) - parseHumanNumber(tb);
    } else if (opts.monthSort) {
      // Month sort: compare month abbreviations.
      result = getMonthValue(ta) - getMonthValue(tb);
    } else if (opts.versionSort) {
      // Version sort: natural sort of numbers within text.
      result = versionCompare(ta, tb);
    } else {
      // Default: lexicographic comparison.
      result = ta < tb ? -1 : ta > tb ? 1 : 0;
    }

    // Apply reverse if requested.
    if (opts.reverse) {
      result = -result;
    }

    return result;
  };
}

// ---------------------------------------------------------------------------
// Business Logic: Sort lines.
// ---------------------------------------------------------------------------

/**
 * Sort an array of lines according to the given options.
 *
 * This is the main sorting function. It:
 * 1. Builds a comparator from the options.
 * 2. Sorts the lines (stable sort is the default in modern JS engines).
 * 3. Removes duplicates if `-u` is set.
 *
 * === Stability ===
 *
 * JavaScript's Array.sort() is guaranteed stable as of ES2019. The
 * `-s` (stable) flag in GNU sort disables a "last resort" comparison
 * that breaks ties using the original string. Since JS sort is already
 * stable, we don't need to do anything special for `-s`.
 *
 * @param lines - The lines to sort.
 * @param opts  - Sort options.
 * @returns The sorted (and possibly deduplicated) lines.
 */
export function sortLines(lines: string[], opts: SortOptions): string[] {
  const comparator = buildComparator(opts);
  const sorted = [...lines].sort(comparator);

  if (opts.unique) {
    // Remove adjacent duplicates (lines that compare as equal).
    return sorted.filter(
      (line, i) => i === 0 || comparator(sorted[i - 1], line) !== 0
    );
  }

  return sorted;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then sort.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * 1. Parse arguments with CLI Builder.
 * 2. Handle --help and --version.
 * 3. Read input from files or stdin.
 * 4. Sort the lines.
 * 5. Write the output.
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
        process.stderr.write(`sort: ${error.message}\n`);
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

  // --- Step 3: Build options from flags ------------------------------------

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const args = (result as { arguments: Record<string, unknown> }).arguments;

  const opts: SortOptions = {
    reverse: !!flags["reverse"],
    numeric: !!flags["numeric_sort"],
    humanNumeric: !!flags["human_numeric_sort"],
    monthSort: !!flags["month_sort"],
    generalNumeric: !!flags["general_numeric_sort"],
    versionSort: !!flags["version_sort"],
    unique: !!flags["unique"],
    ignoreCase: !!flags["ignore_case"],
    dictionaryOrder: !!flags["dictionary_order"],
    ignoreNonprinting: !!flags["ignore_nonprinting"],
    ignoreLeadingBlanks: !!flags["ignore_leading_blanks"],
    stable: !!flags["stable"],
    fieldSeparator: flags["field_separator"] as string | undefined,
    key: flags["key"] as string[] | undefined,
  };

  const outputFile = flags["output"] as string | undefined;
  const zeroTerminated = !!flags["zero_terminated"];
  const delimiter = zeroTerminated ? "\0" : "\n";

  // --- Step 4: Read input --------------------------------------------------

  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

  let allLines: string[] = [];

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
        process.stderr.write(`sort: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    // Split into lines and remove trailing empty line from final newline.
    const lines = content.split(delimiter);
    if (lines[lines.length - 1] === "") {
      lines.pop();
    }
    allLines = allLines.concat(lines);
  }

  // --- Step 5: Sort --------------------------------------------------------

  const sorted = sortLines(allLines, opts);

  // --- Step 6: Output ------------------------------------------------------

  const output = sorted.join(delimiter) + (sorted.length > 0 ? delimiter : "");

  if (outputFile) {
    fs.writeFileSync(outputFile, output, "utf-8");
  } else {
    process.stdout.write(output);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
