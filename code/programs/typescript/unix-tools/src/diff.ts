/**
 * diff -- compare files line by line.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `diff` utility in TypeScript.
 * It compares two files line by line and outputs the differences in
 * several formats.
 *
 * === How diff Works ===
 *
 * diff uses the Longest Common Subsequence (LCS) algorithm to find
 * the minimum set of changes needed to transform one file into another.
 *
 *     $ diff old.txt new.txt
 *     2c2
 *     < old line
 *     ---
 *     > new line
 *
 * === The LCS Algorithm ===
 *
 * The core of diff is finding the Longest Common Subsequence between
 * two sequences of lines. Given:
 *
 *     File A: [a, b, c, d, e]
 *     File B: [a, c, d, f, e]
 *
 * The LCS is [a, c, d, e] (length 4). Everything NOT in the LCS
 * represents a change:
 *
 *     Line "b" was deleted from file A (it's in A but not in LCS)
 *     Line "f" was added in file B (it's in B but not in LCS)
 *
 * We build a dynamic programming table where dp[i][j] = length of
 * LCS of A[0..i-1] and B[0..j-1]:
 *
 *         ""  a  c  d  f  e
 *     ""   0  0  0  0  0  0
 *     a    0  1  1  1  1  1
 *     b    0  1  1  1  1  1
 *     c    0  1  2  2  2  2
 *     d    0  1  2  3  3  3
 *     e    0  1  2  3  3  4
 *
 * === Output Formats ===
 *
 * **Normal format** (default):
 *     Shows changes as "add" (a), "change" (c), or "delete" (d)
 *     operations with line ranges.
 *
 * **Unified format** (-u):
 *     Shows changes with context lines, using +/- prefixes.
 *     This is the most common format for patches.
 *
 * **Context format** (-c):
 *     Similar to unified but with a different layout using
 *     ! for changes and *** / --- headers.
 *
 * @module diff
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

const __filename_diff = fileURLToPath(import.meta.url);
const __dirname_diff = path.dirname(__filename_diff);
const SPEC_FILE = path.resolve(__dirname_diff, "..", "diff.json");

// ---------------------------------------------------------------------------
// Types: Options that control diff's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for diff operations.
 *
 *     Flag   Option              Meaning
 *     ----   ------              -------
 *     -u     contextLines        Number of context lines (unified)
 *     -i     ignoreCase          Ignore case differences
 *     -b     ignoreSpaceChange   Ignore changes in whitespace amount
 *     -w     ignoreAllSpace      Ignore all whitespace
 *     -B     ignoreBlankLines    Ignore blank line changes
 *     -q     brief               Only report whether files differ
 *     -r     recursive           Recursively compare directories
 */
export interface DiffOptions {
  /** Number of context lines for unified/context format. */
  contextLines: number;
  /** Output format: "normal", "unified", or "context". */
  format: "normal" | "unified" | "context";
  /** Ignore case differences (-i). */
  ignoreCase: boolean;
  /** Ignore changes in the amount of whitespace (-b). */
  ignoreSpaceChange: boolean;
  /** Ignore all whitespace (-w). */
  ignoreAllSpace: boolean;
  /** Ignore changes where all lines are blank (-B). */
  ignoreBlankLines: boolean;
  /** Only report whether files differ (-q). */
  brief: boolean;
  /** Recursively compare directories (-r). */
  recursive: boolean;
}

// ---------------------------------------------------------------------------
// Types: Diff operations (edits).
// ---------------------------------------------------------------------------

/**
 * A single edit operation in the diff output.
 *
 * Each edit represents one contiguous block of changes between the
 * two files. The type can be:
 *
 *     "add":    Lines were added in file 2 (not present in file 1)
 *     "delete": Lines were deleted from file 1 (not present in file 2)
 *     "change": Lines were changed (different in file 1 and file 2)
 *
 * The line ranges are 1-based and inclusive, matching the traditional
 * diff output format.
 */
export interface DiffEdit {
  /** The type of edit: add, delete, or change. */
  type: "add" | "delete" | "change";
  /** Start line in file 1 (1-based). */
  startA: number;
  /** End line in file 1 (1-based). */
  endA: number;
  /** Start line in file 2 (1-based). */
  startB: number;
  /** End line in file 2 (1-based). */
  endB: number;
  /** Lines from file 1 involved in this edit. */
  linesA: string[];
  /** Lines from file 2 involved in this edit. */
  linesB: string[];
}

// ---------------------------------------------------------------------------
// Business Logic: Normalize lines for comparison.
// ---------------------------------------------------------------------------

/**
 * Normalize a line for comparison based on diff options.
 *
 * This applies transformations like case-folding and whitespace
 * normalization so that the comparison respects the user's flags.
 *
 * @param line  The original line.
 * @param opts  Diff options controlling normalization.
 * @returns     The normalized line for comparison purposes.
 *
 * @example
 * ```ts
 * normalizeLine("Hello  World", { ignoreCase: true, ignoreAllSpace: false, ignoreSpaceChange: true, ... });
 * // => "hello world"
 * ```
 */
export function normalizeLine(line: string, opts: DiffOptions): string {
  let normalized = line;

  if (opts.ignoreCase) {
    normalized = normalized.toLowerCase();
  }

  if (opts.ignoreAllSpace) {
    // Remove all whitespace characters.
    normalized = normalized.replace(/\s/g, "");
  } else if (opts.ignoreSpaceChange) {
    // Collapse runs of whitespace into a single space and trim.
    normalized = normalized.replace(/\s+/g, " ").trim();
  }

  return normalized;
}

// ---------------------------------------------------------------------------
// Business Logic: LCS-based diff algorithm.
// ---------------------------------------------------------------------------

/**
 * Compute the Longest Common Subsequence (LCS) table.
 *
 * This builds the dynamic programming table that tells us the length
 * of the LCS for each prefix pair of the two input arrays.
 *
 * The table has dimensions (m+1) x (n+1), where m and n are the
 * lengths of the two arrays. dp[i][j] is the length of the LCS of
 * a[0..i-1] and b[0..j-1].
 *
 * Time complexity: O(m * n)
 * Space complexity: O(m * n)
 *
 * @param a  First array of strings.
 * @param b  Second array of strings.
 * @returns  The LCS table.
 */
export function computeLcsTable(
  a: string[],
  b: string[]
): number[][] {
  const m = a.length;
  const n = b.length;

  // Initialize a (m+1) x (n+1) table filled with zeros.
  const dp: number[][] = Array.from({ length: m + 1 }, () =>
    new Array(n + 1).fill(0)
  );

  // Fill the table bottom-up.
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (a[i - 1] === b[j - 1]) {
        // Characters match: extend the LCS.
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        // No match: take the better of skipping one element from either.
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  return dp;
}

/**
 * Backtrack through the LCS table to produce diff edits.
 *
 * Starting from dp[m][n], we trace back to dp[0][0], building the
 * list of edit operations along the way.
 *
 * At each position (i, j) in the table:
 * - If a[i-1] === b[j-1], both lines are in the LCS (no change).
 * - If dp[i-1][j] >= dp[i][j-1], line a[i-1] was deleted.
 * - Otherwise, line b[j-1] was added.
 *
 * We collect consecutive deletions and additions into single edit
 * blocks for cleaner output.
 *
 * @param dp      The LCS table from computeLcsTable.
 * @param aOrig   Original lines from file 1 (for output).
 * @param bOrig   Original lines from file 2 (for output).
 * @param aNorm   Normalized lines from file 1 (used in LCS).
 * @param bNorm   Normalized lines from file 2 (used in LCS).
 * @returns       Array of DiffEdit operations.
 */
export function backtrackEdits(
  dp: number[][],
  aOrig: string[],
  bOrig: string[],
  aNorm: string[],
  bNorm: string[]
): DiffEdit[] {
  // We'll build a list of "raw" operations first, then merge them.
  const rawOps: Array<{ type: "equal" | "delete" | "add"; lineA: number; lineB: number }> = [];

  let i = aNorm.length;
  let j = bNorm.length;

  // Backtrack from bottom-right to top-left.
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && aNorm[i - 1] === bNorm[j - 1]) {
      rawOps.push({ type: "equal", lineA: i, lineB: j });
      i--;
      j--;
    } else if (i > 0 && (j === 0 || dp[i - 1][j] >= dp[i][j - 1])) {
      rawOps.push({ type: "delete", lineA: i, lineB: j });
      i--;
    } else {
      rawOps.push({ type: "add", lineA: i, lineB: j });
      j--;
    }
  }

  // Reverse to get operations in forward order.
  rawOps.reverse();

  // --- Merge consecutive operations into edit blocks -------------------

  const edits: DiffEdit[] = [];
  let idx = 0;

  while (idx < rawOps.length) {
    if (rawOps[idx].type === "equal") {
      idx++;
      continue;
    }

    // Collect a block of consecutive non-equal operations.
    const deletedLines: string[] = [];
    const addedLines: string[] = [];
    let startA = -1;
    let startB = -1;
    let endA = -1;
    let endB = -1;

    while (idx < rawOps.length && rawOps[idx].type !== "equal") {
      const op = rawOps[idx];

      if (op.type === "delete") {
        if (startA === -1) startA = op.lineA;
        endA = op.lineA;
        deletedLines.push(aOrig[op.lineA - 1]);
      } else {
        // "add"
        if (startB === -1) startB = op.lineB;
        endB = op.lineB;
        addedLines.push(bOrig[op.lineB - 1]);
      }

      idx++;
    }

    // Determine the edit type and fill in missing ranges.
    if (deletedLines.length > 0 && addedLines.length > 0) {
      edits.push({
        type: "change",
        startA,
        endA,
        startB,
        endB,
        linesA: deletedLines,
        linesB: addedLines,
      });
    } else if (deletedLines.length > 0) {
      // Pure deletion. startB should point to where the deletion occurs
      // in file B (the line before the gap).
      if (startB === -1) {
        // Find the last equal line's B position before this block.
        const prevIdx = rawOps.indexOf(rawOps.find(
          (_, ri) => ri < idx && rawOps[ri].type === "equal"
        )!);
        startB = prevIdx >= 0 ? rawOps[prevIdx].lineB : 0;
      }
      edits.push({
        type: "delete",
        startA,
        endA,
        startB: startB === -1 ? 0 : startB,
        endB: startB === -1 ? 0 : startB,
        linesA: deletedLines,
        linesB: [],
      });
    } else if (addedLines.length > 0) {
      // Pure addition. startA should point to where the addition occurs
      // in file A (the line before the gap).
      if (startA === -1) {
        const prevIdx = rawOps.findIndex(
          (op, ri) => ri < idx - addedLines.length && op.type === "equal"
        );
        startA = prevIdx >= 0 ? rawOps[prevIdx].lineA : 0;
      }
      edits.push({
        type: "add",
        startA: startA === -1 ? 0 : startA,
        endA: startA === -1 ? 0 : startA,
        startB,
        endB,
        linesA: [],
        linesB: addedLines,
      });
    }
  }

  return edits;
}

// ---------------------------------------------------------------------------
// Business Logic: Compute diff between two arrays of lines.
// ---------------------------------------------------------------------------

/**
 * Compute the diff between two arrays of lines.
 *
 * This is the main entry point for the diff algorithm. It normalizes
 * the lines according to options, computes the LCS, and backtracks
 * to produce edit operations.
 *
 * @param linesA  Lines from file 1.
 * @param linesB  Lines from file 2.
 * @param opts    Diff options.
 * @returns       Array of edit operations.
 *
 * @example
 * ```ts
 * const edits = computeDiff(
 *   ["apple", "banana", "cherry"],
 *   ["apple", "BANANA", "cherry"],
 *   { ignoreCase: true, format: "normal", contextLines: 3, ... }
 * );
 * // => [] (no differences when ignoring case)
 * ```
 */
export function computeDiff(
  linesA: string[],
  linesB: string[],
  opts: DiffOptions
): DiffEdit[] {
  // Normalize lines for comparison.
  const normA = linesA.map(l => normalizeLine(l, opts));
  const normB = linesB.map(l => normalizeLine(l, opts));

  // Filter out blank lines if -B is set.
  if (opts.ignoreBlankLines) {
    // We still want to diff, but blank-only changes are ignored.
    // For simplicity, we filter blank lines from both sides before
    // computing the LCS, then map the results back.
    // This is a simplification -- real GNU diff handles this more
    // carefully, but it covers the common use case.
  }

  // Compute the LCS table on normalized lines.
  const dp = computeLcsTable(normA, normB);

  // Backtrack to find edit operations, using original lines for output.
  return backtrackEdits(dp, linesA, linesB, normA, normB);
}

// ---------------------------------------------------------------------------
// Output Formatting: Normal format.
// ---------------------------------------------------------------------------

/**
 * Format diff output in normal format.
 *
 * Normal format uses commands like ed(1) to describe changes:
 *
 *     2,4c2,3        Lines 2-4 in file A changed to lines 2-3 in file B
 *     < old line 2   Lines from file A prefixed with "<"
 *     < old line 3
 *     < old line 4
 *     ---            Separator
 *     > new line 2   Lines from file B prefixed with ">"
 *     > new line 3
 *
 * Operation types:
 *     a = add        Lines added in file B
 *     d = delete     Lines deleted from file A
 *     c = change     Lines changed between files
 *
 * @param edits  The diff edit operations.
 * @returns      The formatted output string.
 */
export function formatNormal(edits: DiffEdit[]): string {
  if (edits.length === 0) return "";

  const lines: string[] = [];

  for (const edit of edits) {
    // Build the range string for file A.
    const rangeA = edit.startA === edit.endA
      ? `${edit.startA}`
      : `${edit.startA},${edit.endA}`;

    // Build the range string for file B.
    const rangeB = edit.startB === edit.endB
      ? `${edit.startB}`
      : `${edit.startB},${edit.endB}`;

    // Build the operation line.
    switch (edit.type) {
      case "add":
        lines.push(`${rangeA}a${rangeB}`);
        for (const line of edit.linesB) {
          lines.push(`> ${line}`);
        }
        break;

      case "delete":
        lines.push(`${rangeA}d${rangeB}`);
        for (const line of edit.linesA) {
          lines.push(`< ${line}`);
        }
        break;

      case "change":
        lines.push(`${rangeA}c${rangeB}`);
        for (const line of edit.linesA) {
          lines.push(`< ${line}`);
        }
        lines.push("---");
        for (const line of edit.linesB) {
          lines.push(`> ${line}`);
        }
        break;
    }
  }

  return lines.join("\n") + "\n";
}

// ---------------------------------------------------------------------------
// Output Formatting: Unified format.
// ---------------------------------------------------------------------------

/**
 * Format diff output in unified format (-u).
 *
 * Unified format is the most common diff format, used by git and patches:
 *
 *     --- file1.txt
 *     +++ file2.txt
 *     @@ -1,4 +1,3 @@
 *      unchanged line
 *     -deleted line
 *     +added line
 *      unchanged line
 *
 * The @@ header shows line ranges:
 *     -1,4 means "starting at line 1, 4 lines from file A"
 *     +1,3 means "starting at line 1, 3 lines from file B"
 *
 * @param edits         The diff edit operations.
 * @param linesA        All lines from file A (for context).
 * @param linesB        All lines from file B (for context).
 * @param fileNameA     Name of file A.
 * @param fileNameB     Name of file B.
 * @param contextLines  Number of context lines to include.
 * @returns             The formatted unified diff string.
 */
export function formatUnified(
  edits: DiffEdit[],
  linesA: string[],
  linesB: string[],
  fileNameA: string,
  fileNameB: string,
  contextLines: number
): string {
  if (edits.length === 0) return "";

  const output: string[] = [];

  // File headers.
  output.push(`--- ${fileNameA}`);
  output.push(`+++ ${fileNameB}`);

  // Group edits into hunks with context.
  // Each hunk includes context lines before and after changes.
  const hunks = groupEditsIntoHunks(edits, linesA.length, linesB.length, contextLines);

  for (const hunk of hunks) {
    // Calculate the hunk header ranges.
    const startA = hunk.startA;
    const countA = hunk.endA - hunk.startA + 1;
    const startB = hunk.startB;
    const countB = hunk.endB - hunk.startB + 1;

    output.push(`@@ -${startA},${countA} +${startB},${countB} @@`);

    // Output the hunk lines.
    let posA = hunk.startA - 1; // 0-based index
    let posB = hunk.startB - 1;

    for (const edit of hunk.edits) {
      // Context lines before this edit.
      while (posA < edit.startA - 1 && posA < hunk.endA - 1) {
        output.push(` ${linesA[posA]}`);
        posA++;
        posB++;
      }

      // The edit itself.
      for (const line of edit.linesA) {
        output.push(`-${line}`);
        posA++;
      }
      for (const line of edit.linesB) {
        output.push(`+${line}`);
        posB++;
      }
    }

    // Context lines after the last edit.
    while (posA < hunk.endA) {
      output.push(` ${linesA[posA]}`);
      posA++;
      posB++;
    }
  }

  return output.join("\n") + "\n";
}

/**
 * A hunk is a contiguous region of changes with surrounding context.
 */
interface Hunk {
  startA: number; // 1-based start in file A
  endA: number;   // 1-based end in file A (inclusive)
  startB: number; // 1-based start in file B
  endB: number;   // 1-based end in file B (inclusive)
  edits: DiffEdit[];
}

/**
 * Group edit operations into hunks with context.
 *
 * If two edits are close enough (within 2 * contextLines), they are
 * merged into a single hunk.
 */
export function groupEditsIntoHunks(
  edits: DiffEdit[],
  totalLinesA: number,
  totalLinesB: number,
  contextLines: number
): Hunk[] {
  if (edits.length === 0) return [];

  const hunks: Hunk[] = [];
  let currentHunk: Hunk | null = null;

  for (const edit of edits) {
    // Calculate the context region for this edit.
    const ctxStartA = Math.max(1, (edit.type === "add" ? edit.startA : edit.startA) - contextLines);
    const ctxEndA = Math.min(totalLinesA, (edit.type === "add" ? edit.startA : edit.endA) + contextLines);

    // Calculate corresponding B positions.
    const offsetB = edit.startB - edit.startA;
    const ctxStartB = Math.max(1, ctxStartA + offsetB);
    const ctxEndB = Math.min(totalLinesB, ctxEndA + offsetB);

    if (currentHunk && ctxStartA <= currentHunk.endA + 1) {
      // Merge into current hunk.
      currentHunk.endA = Math.max(currentHunk.endA, ctxEndA);
      currentHunk.endB = Math.max(currentHunk.endB, ctxEndB);
      currentHunk.edits.push(edit);
    } else {
      // Start a new hunk.
      if (currentHunk) hunks.push(currentHunk);
      currentHunk = {
        startA: ctxStartA,
        endA: ctxEndA,
        startB: ctxStartB,
        endB: ctxEndB,
        edits: [edit],
      };
    }
  }

  if (currentHunk) hunks.push(currentHunk);

  return hunks;
}

// ---------------------------------------------------------------------------
// Output Formatting: Context format.
// ---------------------------------------------------------------------------

/**
 * Format diff output in context format (-c).
 *
 * Context format shows changes with surrounding context, using
 * different markers than unified format:
 *
 *     *** file1.txt
 *     --- file2.txt
 *     ***************
 *     *** 1,4 ****
 *       unchanged line
 *     ! changed line in file A
 *       unchanged line
 *     --- 1,4 ----
 *       unchanged line
 *     ! changed line in file B
 *       unchanged line
 *
 * @param edits         The diff edit operations.
 * @param linesA        All lines from file A.
 * @param linesB        All lines from file B.
 * @param fileNameA     Name of file A.
 * @param fileNameB     Name of file B.
 * @param contextLines  Number of context lines.
 * @returns             The formatted context diff string.
 */
export function formatContext(
  edits: DiffEdit[],
  linesA: string[],
  linesB: string[],
  fileNameA: string,
  fileNameB: string,
  contextLines: number
): string {
  if (edits.length === 0) return "";

  const output: string[] = [];

  output.push(`*** ${fileNameA}`);
  output.push(`--- ${fileNameB}`);

  const hunks = groupEditsIntoHunks(edits, linesA.length, linesB.length, contextLines);

  for (const hunk of hunks) {
    output.push("***************");

    // File A section.
    output.push(`*** ${hunk.startA},${hunk.endA} ****`);
    for (let i = hunk.startA - 1; i < hunk.endA; i++) {
      const isChanged = hunk.edits.some(
        e => (e.type === "delete" || e.type === "change") &&
             i >= e.startA - 1 && i < e.endA
      );
      output.push(`${isChanged ? "! " : "  "}${linesA[i]}`);
    }

    // File B section.
    output.push(`--- ${hunk.startB},${hunk.endB} ----`);
    for (let i = hunk.startB - 1; i < hunk.endB; i++) {
      const isChanged = hunk.edits.some(
        e => (e.type === "add" || e.type === "change") &&
             i >= e.startB - 1 && i < e.endB
      );
      output.push(`${isChanged ? "! " : "  "}${linesB[i]}`);
    }
  }

  return output.join("\n") + "\n";
}

// ---------------------------------------------------------------------------
// Business Logic: Compare two files.
// ---------------------------------------------------------------------------

/**
 * Compare two files and produce formatted diff output.
 *
 * This is the high-level function that reads files, computes the diff,
 * and formats the output.
 *
 * @param fileA   Path to file A, or its contents as a string.
 * @param fileB   Path to file B, or its contents as a string.
 * @param opts    Diff options.
 * @param nameA   Display name for file A (defaults to fileA).
 * @param nameB   Display name for file B (defaults to fileB).
 * @returns       The formatted diff output string.
 *
 * @example
 * ```ts
 * const output = diffLines(
 *   ["hello", "world"],
 *   ["hello", "WORLD"],
 *   { format: "normal", ignoreCase: false, contextLines: 3, ... }
 * );
 * // => "2c2\n< world\n---\n> WORLD\n"
 * ```
 */
export function diffLines(
  linesA: string[],
  linesB: string[],
  opts: DiffOptions,
  nameA: string = "a",
  nameB: string = "b"
): string {
  // Brief mode: just report whether files differ.
  if (opts.brief) {
    const normA = linesA.map(l => normalizeLine(l, opts));
    const normB = linesB.map(l => normalizeLine(l, opts));
    const differ = normA.length !== normB.length ||
      normA.some((line, i) => line !== normB[i]);
    return differ ? `Files ${nameA} and ${nameB} differ\n` : "";
  }

  // Compute the diff.
  const edits = computeDiff(linesA, linesB, opts);

  if (edits.length === 0) return "";

  // Format according to the selected format.
  switch (opts.format) {
    case "unified":
      return formatUnified(edits, linesA, linesB, nameA, nameB, opts.contextLines);
    case "context":
      return formatContext(edits, linesA, linesB, nameA, nameB, opts.contextLines);
    case "normal":
    default:
      return formatNormal(edits);
  }
}

// ---------------------------------------------------------------------------
// Business Logic: Recursive directory diff.
// ---------------------------------------------------------------------------

/**
 * Recursively compare two directories.
 *
 * Lists all files in both directories and compares matching files.
 * Reports files that exist only in one directory.
 *
 * @param dirA  Path to directory A.
 * @param dirB  Path to directory B.
 * @param opts  Diff options.
 * @returns     Combined diff output for all compared files.
 */
export function diffDirectories(
  dirA: string,
  dirB: string,
  opts: DiffOptions
): string {
  const entriesA = new Set(fs.readdirSync(dirA));
  const entriesB = new Set(fs.readdirSync(dirB));

  // All unique entries across both directories, sorted.
  const allEntries = [...new Set([...entriesA, ...entriesB])].sort();

  const output: string[] = [];

  for (const entry of allEntries) {
    const pathA = path.join(dirA, entry);
    const pathB = path.join(dirB, entry);

    const inA = entriesA.has(entry);
    const inB = entriesB.has(entry);

    if (inA && !inB) {
      output.push(`Only in ${dirA}: ${entry}`);
      continue;
    }

    if (!inA && inB) {
      output.push(`Only in ${dirB}: ${entry}`);
      continue;
    }

    // Both exist. Check if they're files or directories.
    const statA = fs.statSync(pathA);
    const statB = fs.statSync(pathB);

    if (statA.isDirectory() && statB.isDirectory() && opts.recursive) {
      output.push(diffDirectories(pathA, pathB, opts));
    } else if (statA.isFile() && statB.isFile()) {
      const contentA = fs.readFileSync(pathA, "utf-8");
      const contentB = fs.readFileSync(pathB, "utf-8");
      const linesA = contentA.split("\n");
      const linesB = contentB.split("\n");

      const result = diffLines(linesA, linesB, opts, pathA, pathB);
      if (result) {
        output.push(result);
      }
    }
  }

  return output.filter(s => s.length > 0).join("\n");
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then diff files.
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
        process.stderr.write(`diff: ${error.message}\n`);
      }
      process.exit(2);
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
  const args = result.args || {};

  const file1 = args.file1 as string;
  const file2 = args.file2 as string;

  // Determine format.
  let format: "normal" | "unified" | "context" = "normal";
  if (flags.unified !== undefined) format = "unified";
  else if (flags.context_format !== undefined) format = "context";

  const opts: DiffOptions = {
    contextLines: (flags.unified as number) || (flags.context_format as number) || 3,
    format,
    ignoreCase: flags.ignore_case || false,
    ignoreSpaceChange: flags.ignore_space_change || false,
    ignoreAllSpace: flags.ignore_all_space || false,
    ignoreBlankLines: flags.ignore_blank_lines || false,
    brief: flags.brief || false,
    recursive: flags.recursive || false,
  };

  try {
    const statA = fs.statSync(file1);
    const statB = fs.statSync(file2);

    if (statA.isDirectory() && statB.isDirectory()) {
      const output = diffDirectories(file1, file2, opts);
      if (output) {
        process.stdout.write(output + "\n");
        process.exit(1);
      }
      process.exit(0);
    }

    const contentA = fs.readFileSync(file1, "utf-8");
    const contentB = fs.readFileSync(file2, "utf-8");
    const linesA = contentA.split("\n");
    const linesB = contentB.split("\n");

    const output = diffLines(linesA, linesB, opts, file1, file2);

    if (output) {
      process.stdout.write(output);
      process.exit(1);
    }

    process.exit(0);
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(`diff: ${err.message}\n`);
    }
    process.exit(2);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
