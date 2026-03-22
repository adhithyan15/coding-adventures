/**
 * expand -- convert tabs to spaces.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `expand` utility in TypeScript.
 * It converts tab characters in files to the appropriate number of spaces,
 * maintaining proper column alignment.
 *
 * === How expand Works ===
 *
 * Tabs in text files are not a fixed number of spaces. Instead, they
 * advance the cursor to the next **tab stop**. By default, tab stops are
 * every 8 columns:
 *
 *     Column:  0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
 *     Stops:   *                       *                       *
 *
 * A tab at column 3 advances to column 8 (5 spaces).
 * A tab at column 7 advances to column 8 (1 space).
 * A tab at column 8 advances to column 16 (8 spaces).
 *
 * === The -t Flag (Tab Stops) ===
 *
 * The `-t` flag changes the tab stop interval:
 *
 *     expand -t 4 file.txt      =>   tabs every 4 columns
 *     expand -t 2,6,10 file.txt =>   variable tab stops at columns 2, 6, 10
 *
 * With a single number, tabs are evenly spaced. With a comma-separated
 * list, tabs stop at exactly those column positions. After the last
 * specified stop, tabs advance by one space.
 *
 * === The -i Flag (Initial Only) ===
 *
 * With `-i`, only leading tabs (before any non-blank character) are
 * expanded. Tabs after non-blank characters are left as-is. This is
 * useful for preserving intentional tabs within text while normalizing
 * indentation.
 *
 * @module expand
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
const SPEC_FILE = path.resolve(__dirname, "..", "expand.json");

// ---------------------------------------------------------------------------
// Business Logic: Tab expansion.
// ---------------------------------------------------------------------------

/**
 * Parse the tab stops specification.
 *
 * Returns either a single number (uniform tab stops) or an array of
 * numbers (variable tab stops).
 *
 * Examples:
 *   "4"      => 4 (tabs every 4 columns)
 *   "2,6,10" => [2, 6, 10] (specific tab stop positions)
 */
function parseTabStops(tabsStr: string): number | number[] {
  if (tabsStr.includes(",")) {
    return tabsStr.split(",").map((s) => parseInt(s.trim(), 10));
  }
  return parseInt(tabsStr, 10);
}

/**
 * Calculate the number of spaces needed to reach the next tab stop.
 *
 * For uniform tab stops (a single number), this is:
 *   tabSize - (column % tabSize)
 *
 * For variable tab stops (an array), we find the smallest stop position
 * that is greater than the current column. If we're past all defined
 * stops, we use 1 space (matching GNU behavior).
 */
function spacesToNextTab(column: number, tabStops: number | number[]): number {
  if (typeof tabStops === "number") {
    // Uniform tab stops: advance to the next multiple of tabStops.
    return tabStops - (column % tabStops);
  }

  // Variable tab stops: find the next stop position after current column.
  for (const stop of tabStops) {
    if (stop > column) {
      return stop - column;
    }
  }

  // Past all defined stops: use 1 space.
  return 1;
}

/**
 * Expand tabs in a single line.
 *
 * We process the line character by character, tracking the current column
 * position. When we encounter a tab:
 * - If `initialOnly` is true and we've seen a non-blank character, keep
 *   the tab as-is.
 * - Otherwise, replace the tab with the appropriate number of spaces.
 *
 * Non-tab characters simply advance the column by 1.
 */
function expandLine(
  line: string,
  tabStops: number | number[],
  initialOnly: boolean
): string {
  let result = "";
  let column = 0;
  let seenNonBlank = false;

  for (const ch of line) {
    if (ch === "\t") {
      if (initialOnly && seenNonBlank) {
        // Past the initial blanks -- keep the tab.
        result += "\t";
        // Tab still advances the column.
        column += spacesToNextTab(column, tabStops);
      } else {
        // Replace tab with spaces.
        const spaces = spacesToNextTab(column, tabStops);
        result += " ".repeat(spaces);
        column += spaces;
      }
    } else {
      if (ch !== " ") {
        seenNonBlank = true;
      }
      result += ch;
      column++;
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then expand tabs.
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
        process.stderr.write(`expand: ${error.message}\n`);
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

  const initialOnly = !!flags["initial"];
  const tabsStr = flags["tabs"] as string | undefined;
  const tabStops = tabsStr ? parseTabStops(tabsStr) : 8;

  // Normalize file list.
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
        process.stderr.write(`expand: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    // Process line by line, preserving the original line endings.
    const lines = content.split("\n");

    for (let i = 0; i < lines.length; i++) {
      const expanded = expandLine(lines[i], tabStops, initialOnly);
      if (i < lines.length - 1) {
        process.stdout.write(expanded + "\n");
      } else if (lines[i] !== "") {
        // Last line without trailing newline.
        process.stdout.write(expanded);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
