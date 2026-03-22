/**
 * unexpand -- convert spaces to tabs.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `unexpand` utility in TypeScript.
 * It converts leading sequences of spaces to tabs, the inverse of `expand`.
 *
 * === How unexpand Works ===
 *
 * By default, unexpand only converts leading blanks (spaces and tabs at
 * the beginning of a line) to tabs. With `-a`, it converts all sequences
 * of spaces that align with tab stops.
 *
 *     unexpand file.txt          =>   convert leading spaces to tabs
 *     unexpand -a file.txt       =>   convert ALL aligned spaces to tabs
 *     unexpand -t 4 file.txt     =>   use tab stops every 4 columns
 *
 * === The Algorithm ===
 *
 * For each line, we track the current column position. When we encounter
 * a sequence of spaces that spans a tab stop boundary, we replace those
 * spaces with a tab character. The key insight is that we only replace
 * spaces that exactly reach a tab stop -- partial spans remain as spaces.
 *
 * === Default vs -a Behavior ===
 *
 * Without `-a`, unexpand only processes the leading whitespace. Once a
 * non-blank character is encountered, the rest of the line is copied
 * verbatim. This is the safe default because tabs in the middle of text
 * can change the visual alignment in unpredictable ways.
 *
 * With `-a`, all whitespace sequences are candidates for conversion,
 * even those in the middle of a line.
 *
 * @module unexpand
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
const SPEC_FILE = path.resolve(__dirname, "..", "unexpand.json");

// ---------------------------------------------------------------------------
// Business Logic: Space-to-tab conversion.
// ---------------------------------------------------------------------------

/**
 * Parse the tab stops specification (same as expand).
 */
function parseTabStops(tabsStr: string): number {
  return parseInt(tabsStr, 10);
}

/**
 * Convert spaces to tabs in a single line.
 *
 * We track the column position and accumulate spaces. When we reach a tab
 * stop boundary with accumulated spaces, we replace them with a tab.
 *
 * @param line         The input line (without newline).
 * @param tabSize      The tab stop interval.
 * @param convertAll   If true, convert all spaces; if false, only leading.
 */
function unexpandLine(
  line: string,
  tabSize: number,
  convertAll: boolean
): string {
  let result = "";
  let column = 0;
  let pendingSpaces = 0;
  let seenNonBlank = false;

  for (const ch of line) {
    if (ch === " " && (convertAll || !seenNonBlank)) {
      // Accumulate spaces.
      pendingSpaces++;
      column++;

      // Check if we've reached a tab stop.
      if (column % tabSize === 0) {
        // Replace accumulated spaces with a single tab.
        result += "\t";
        pendingSpaces = 0;
      }
    } else if (ch === "\t" && (convertAll || !seenNonBlank)) {
      // A tab already -- flush any pending spaces and keep the tab.
      pendingSpaces = 0;
      result += "\t";
      // Advance to the next tab stop.
      column = column + (tabSize - (column % tabSize));
    } else {
      // Non-blank character (or blank after non-blank when not in -a mode).
      if (!seenNonBlank && ch !== " " && ch !== "\t") {
        seenNonBlank = true;
      }

      // Flush any pending spaces as literal spaces.
      if (pendingSpaces > 0) {
        result += " ".repeat(pendingSpaces);
        pendingSpaces = 0;
      }

      result += ch;
      column++;
    }
  }

  // Flush remaining pending spaces.
  if (pendingSpaces > 0) {
    result += " ".repeat(pendingSpaces);
  }

  return result;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then convert spaces to tabs.
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
        process.stderr.write(`unexpand: ${error.message}\n`);
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

  const convertAll = !!flags["all"] && !flags["first_only"];
  const tabsStr = flags["tabs"] as string | undefined;
  const tabSize = tabsStr ? parseTabStops(tabsStr) : 8;

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
        process.stderr.write(`unexpand: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    // Process line by line.
    const lines = content.split("\n");

    for (let i = 0; i < lines.length; i++) {
      const converted = unexpandLine(lines[i], tabSize, convertAll);
      if (i < lines.length - 1) {
        process.stdout.write(converted + "\n");
      } else if (lines[i] !== "") {
        process.stdout.write(converted);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
