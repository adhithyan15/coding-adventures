/**
 * join -- join lines of two files on a common field.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `join` utility in TypeScript.
 * It reads two files that are sorted on a join field and produces output
 * lines by pairing records from the two files that share the same key.
 *
 * === How join Works ===
 *
 * Think of it like a SQL INNER JOIN, but for text files:
 *
 *     File 1:                File 2:
 *     1 Alice                1 Engineering
 *     2 Bob                  3 Marketing
 *     3 Charlie
 *
 *     $ join file1 file2
 *     1 Alice Engineering
 *     3 Charlie Marketing
 *
 * By default, join uses the first field (column 1) as the join key.
 * Fields are separated by whitespace.
 *
 * === The Merge-Join Algorithm ===
 *
 * Since both files must be sorted on the join field, we can use a
 * merge-join algorithm. This is the same algorithm databases use for
 * sorted inputs -- it scans both files simultaneously in O(n + m) time:
 *
 *     1. Read the next record from each file.
 *     2. Compare the join keys:
 *        - If key1 < key2: advance file 1 (key1 has no match).
 *        - If key1 > key2: advance file 2 (key2 has no match).
 *        - If key1 == key2: output the joined record, advance both.
 *     3. Repeat until one file is exhausted.
 *
 * For duplicate keys, join produces the cross product: if file1 has
 * 2 records with key "A" and file2 has 3 records with key "A", the
 * output contains 2 x 3 = 6 lines.
 *
 * === Unpaired Lines ===
 *
 *     -a 1   Also print unpairable lines from file 1 (LEFT JOIN)
 *     -a 2   Also print unpairable lines from file 2 (RIGHT JOIN)
 *     -a 1 -a 2   Print all unpairable lines (FULL OUTER JOIN)
 *     -v 1   Only print unpairable lines from file 1 (ANTI JOIN)
 *
 * === Field Selection ===
 *
 *     -1 N   Join on field N of file 1 (default: 1)
 *     -2 N   Join on field N of file 2 (default: 1)
 *     -j N   Short for -1 N -2 N
 *     -t C   Use character C as field separator
 *
 * @module join
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

const __filename_join = fileURLToPath(import.meta.url);
const __dirname_join = path.dirname(__filename_join);
const SPEC_FILE = path.resolve(__dirname_join, "..", "join.json");

// ---------------------------------------------------------------------------
// Types: Options that control join's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for join operations.
 */
export interface JoinOptions {
  /** Field number to join on for file 1 (1-based, default 1). */
  field1: number;
  /** Field number to join on for file 2 (1-based, default 1). */
  field2: number;
  /** Field separator character (default: whitespace). */
  separator: string | null;
  /** Also print unpairable lines from these files (1, 2, or both). */
  unpaired: number[];
  /** Only print unpairable lines from this file (1 or 2). */
  onlyUnpaired: number | null;
  /** Replacement for missing fields. */
  empty: string;
  /** Ignore case when comparing join fields (-i). */
  ignoreCase: boolean;
}

// ---------------------------------------------------------------------------
// Business Logic: Parse a line into fields.
// ---------------------------------------------------------------------------

/**
 * Split a line into fields using the specified separator.
 *
 * When no separator is specified (null), split on runs of whitespace
 * (matching POSIX join behavior). When a separator is specified,
 * split on exactly that character.
 *
 * @param line       The line to split.
 * @param separator  The field separator, or null for whitespace.
 * @returns          Array of field values.
 *
 * @example
 * ```ts
 * splitFields("Alice  100  NYC", null);      // => ["Alice", "100", "NYC"]
 * splitFields("Alice,100,NYC", ",");          // => ["Alice", "100", "NYC"]
 * ```
 */
export function splitFields(line: string, separator: string | null): string[] {
  if (separator === null) {
    // Split on whitespace runs, trimming leading/trailing whitespace.
    return line.trim().split(/\s+/);
  }
  return line.split(separator);
}

// ---------------------------------------------------------------------------
// Business Logic: Extract the join key from a set of fields.
// ---------------------------------------------------------------------------

/**
 * Get the join key from a record's fields.
 *
 * @param fields      Array of field values.
 * @param fieldIndex  The 1-based field number to use as the key.
 * @param ignoreCase  Whether to lowercase the key for comparison.
 * @returns           The key string.
 */
export function getKey(
  fields: string[],
  fieldIndex: number,
  ignoreCase: boolean
): string {
  // Convert 1-based to 0-based index.
  const idx = fieldIndex - 1;

  if (idx < 0 || idx >= fields.length) {
    return "";
  }

  const key = fields[idx];
  return ignoreCase ? key.toLowerCase() : key;
}

// ---------------------------------------------------------------------------
// Business Logic: Build an output line from joined fields.
// ---------------------------------------------------------------------------

/**
 * Build the output line from two records that share a join key.
 *
 * The output format is:
 *     join-key  remaining-fields-from-file1  remaining-fields-from-file2
 *
 * "Remaining fields" means all fields except the join field.
 *
 * @param key         The join key value.
 * @param fields1     Fields from file 1.
 * @param fields2     Fields from file 2.
 * @param fieldIdx1   1-based join field index for file 1.
 * @param fieldIdx2   1-based join field index for file 2.
 * @param separator   Output separator (space if null).
 * @returns           The formatted output line.
 */
export function buildOutputLine(
  key: string,
  fields1: string[],
  fields2: string[],
  fieldIdx1: number,
  fieldIdx2: number,
  separator: string | null
): string {
  const sep = separator ?? " ";

  // Collect non-key fields from each file.
  const rest1 = fields1.filter((_, i) => i !== fieldIdx1 - 1);
  const rest2 = fields2.filter((_, i) => i !== fieldIdx2 - 1);

  const parts = [key, ...rest1, ...rest2];
  return parts.join(sep);
}

// ---------------------------------------------------------------------------
// Business Logic: Build an unpaired output line.
// ---------------------------------------------------------------------------

/**
 * Build the output line for an unmatched record.
 *
 * @param fields     The record's fields.
 * @param separator  Output separator (space if null).
 * @returns          The formatted output line.
 */
export function buildUnpairedLine(
  fields: string[],
  separator: string | null
): string {
  const sep = separator ?? " ";
  return fields.join(sep);
}

// ---------------------------------------------------------------------------
// Business Logic: The core merge-join algorithm.
// ---------------------------------------------------------------------------

/**
 * Perform a merge-join on two arrays of lines.
 *
 * Both arrays must be sorted on their respective join fields. This
 * function scans both arrays simultaneously, producing output for
 * matching pairs (and optionally for unmatched lines).
 *
 * This is the heart of the join utility. The algorithm is O(n + m)
 * for inputs with unique keys, and O(n * m) in the worst case of
 * all-duplicate keys (cross product).
 *
 * @param lines1  Lines from file 1 (sorted on join field).
 * @param lines2  Lines from file 2 (sorted on join field).
 * @param opts    Join options.
 * @returns       Array of output lines.
 *
 * @example
 * ```ts
 * joinLines(
 *   ["1 Alice", "2 Bob", "3 Charlie"],
 *   ["1 Engineering", "3 Marketing"],
 *   { field1: 1, field2: 1, separator: null, unpaired: [], onlyUnpaired: null, empty: "", ignoreCase: false }
 * );
 * // => ["1 Alice Engineering", "3 Charlie Marketing"]
 * ```
 */
export function joinLines(
  lines1: string[],
  lines2: string[],
  opts: JoinOptions
): string[] {
  const output: string[] = [];

  // Parse all lines into field arrays upfront.
  const records1 = lines1.map((line) => splitFields(line, opts.separator));
  const records2 = lines2.map((line) => splitFields(line, opts.separator));

  let i = 0;
  let j = 0;

  while (i < records1.length && j < records2.length) {
    const key1 = getKey(records1[i], opts.field1, opts.ignoreCase);
    const key2 = getKey(records2[j], opts.field2, opts.ignoreCase);

    if (key1 < key2) {
      // File 1 record has no match in file 2.
      if (opts.unpaired.includes(1) || opts.onlyUnpaired === 1) {
        output.push(buildUnpairedLine(records1[i], opts.separator));
      }
      i++;
    } else if (key1 > key2) {
      // File 2 record has no match in file 1.
      if (opts.unpaired.includes(2) || opts.onlyUnpaired === 2) {
        output.push(buildUnpairedLine(records2[j], opts.separator));
      }
      j++;
    } else {
      // Keys match -- produce the cross product for all records
      // with the same key in both files.

      // Find the range of records in file 1 with this key.
      let endI = i;
      while (endI < records1.length && getKey(records1[endI], opts.field1, opts.ignoreCase) === key1) {
        endI++;
      }

      // Find the range of records in file 2 with this key.
      let endJ = j;
      while (endJ < records2.length && getKey(records2[endJ], opts.field2, opts.ignoreCase) === key2) {
        endJ++;
      }

      // Only produce joined output if we're not in onlyUnpaired mode.
      if (opts.onlyUnpaired === null) {
        for (let ii = i; ii < endI; ii++) {
          for (let jj = j; jj < endJ; jj++) {
            output.push(
              buildOutputLine(
                key1,
                records1[ii],
                records2[jj],
                opts.field1,
                opts.field2,
                opts.separator
              )
            );
          }
        }
      }

      i = endI;
      j = endJ;
    }
  }

  // Handle remaining unmatched records.
  while (i < records1.length) {
    if (opts.unpaired.includes(1) || opts.onlyUnpaired === 1) {
      output.push(buildUnpairedLine(records1[i], opts.separator));
    }
    i++;
  }

  while (j < records2.length) {
    if (opts.unpaired.includes(2) || opts.onlyUnpaired === 2) {
      output.push(buildUnpairedLine(records2[j], opts.separator));
    }
    j++;
  }

  return output;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then join files.
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
        process.stderr.write(`join: ${error.message}\n`);
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

  const flags = result.flags || {};
  const file1Path: string = result.args?.file1 || "";
  const file2Path: string = result.args?.file2 || "";

  // Determine join fields. -j sets both; -1 and -2 override individually.
  let f1 = flags.join_field ?? flags.field1 ?? 1;
  let f2 = flags.join_field ?? flags.field2 ?? 1;

  const unpaired: number[] = [];
  if (flags.unpaired) {
    const vals = Array.isArray(flags.unpaired) ? flags.unpaired : [flags.unpaired];
    for (const v of vals) {
      unpaired.push(Number(v));
    }
  }

  const opts: JoinOptions = {
    field1: f1,
    field2: f2,
    separator: flags.separator ?? null,
    unpaired,
    onlyUnpaired: flags.only_unpaired ? Number(flags.only_unpaired) : null,
    empty: flags.empty ?? "",
    ignoreCase: flags.ignore_case || false,
  };

  try {
    const content1 = fs.readFileSync(file1Path, "utf-8");
    const content2 = fs.readFileSync(file2Path, "utf-8");

    const lines1 = content1.split("\n").filter((l) => l !== "");
    const lines2 = content2.split("\n").filter((l) => l !== "");

    const output = joinLines(lines1, lines2, opts);

    for (const line of output) {
      process.stdout.write(line + "\n");
    }
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(`join: ${err.message}\n`);
    }
    process.exit(1);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
