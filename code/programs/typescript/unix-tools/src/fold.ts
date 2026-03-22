/**
 * fold -- wrap each input line to fit in specified width.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `fold` utility in TypeScript. It
 * wraps input lines that are longer than a specified width (default: 80
 * columns).
 *
 * === How fold Works ===
 *
 * fold reads input and inserts newlines to ensure no output line exceeds
 * the specified width:
 *
 *     fold file.txt              =>   wrap at 80 columns (default)
 *     fold -w 40 file.txt        =>   wrap at 40 columns
 *     fold -s file.txt           =>   break at word boundaries (spaces)
 *     fold -b file.txt           =>   count bytes, not columns
 *
 * === Column Counting vs Byte Counting ===
 *
 * By default, fold counts display columns. A tab character advances to
 * the next tab stop (every 8 columns), and a backspace moves back one
 * column. With `-b`, fold simply counts bytes, treating every byte
 * (including tabs and control characters) as one unit.
 *
 * === Breaking at Spaces (-s) ===
 *
 * Without `-s`, fold breaks lines at exactly the width, even in the
 * middle of a word. With `-s`, fold tries to break at the last space
 * before the width limit. If there are no spaces in the line, it still
 * breaks at the width.
 *
 * @module fold
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
const SPEC_FILE = path.resolve(__dirname, "..", "fold.json");

// ---------------------------------------------------------------------------
// Business Logic: Line folding.
// ---------------------------------------------------------------------------

/**
 * Fold a single line to fit within the specified width.
 *
 * Returns the folded output (with inserted newlines).
 *
 * === Algorithm ===
 *
 * We walk through the line character by character, tracking the current
 * column width. When we exceed the width:
 *
 * - Without `-s`: Insert a newline at the exact width position.
 * - With `-s`: Look back for the last space within the width. If found,
 *   break there. If not, break at the width.
 *
 * === Tab Handling ===
 *
 * In column mode (not `-b`), tabs advance to the next multiple of 8.
 * Backspaces (`\b`) move back one column (but not below 0).
 */
function foldLine(
  line: string,
  width: number,
  breakAtSpaces: boolean,
  countBytes: boolean
): string {
  if (width <= 0) return line;

  const segments: string[] = [];
  let currentSegment = "";
  let currentWidth = 0;
  let lastSpaceIndex = -1;
  let lastSpaceWidth = 0;

  for (let i = 0; i < line.length; i++) {
    const ch = line[i];

    // Calculate the width contribution of this character.
    let charWidth: number;
    if (countBytes) {
      charWidth = Buffer.byteLength(ch, "utf-8");
    } else if (ch === "\t") {
      // Tab advances to the next tab stop (every 8 columns).
      charWidth = 8 - (currentWidth % 8);
    } else if (ch === "\b") {
      // Backspace moves back one column.
      charWidth = currentWidth > 0 ? -1 : 0;
    } else {
      charWidth = 1;
    }

    // Track the last space position for -s mode.
    if (breakAtSpaces && ch === " ") {
      lastSpaceIndex = currentSegment.length;
      lastSpaceWidth = currentWidth;
    }

    // Check if adding this character would exceed the width.
    if (currentWidth + charWidth > width) {
      if (breakAtSpaces && lastSpaceIndex >= 0) {
        // Break at the last space.
        segments.push(currentSegment.substring(0, lastSpaceIndex + 1));
        currentSegment = currentSegment.substring(lastSpaceIndex + 1) + ch;
        currentWidth = currentWidth - lastSpaceWidth - 1 + charWidth;
        lastSpaceIndex = -1;
        lastSpaceWidth = 0;
      } else {
        // Break at the current position.
        segments.push(currentSegment);
        currentSegment = ch;
        currentWidth = charWidth;
        lastSpaceIndex = -1;
        lastSpaceWidth = 0;
      }
    } else {
      currentSegment += ch;
      currentWidth += charWidth;
    }
  }

  // Don't forget the last segment.
  if (currentSegment.length > 0) {
    segments.push(currentSegment);
  }

  return segments.join("\n");
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then fold lines.
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
        process.stderr.write(`fold: ${error.message}\n`);
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

  const countBytes = !!flags["bytes"];
  const breakAtSpaces = !!flags["spaces"];
  const width = (flags["width"] as number) ?? 80;

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
        process.stderr.write(`fold: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    // Process each line independently. Fold operates on individual lines,
    // preserving existing line breaks.
    const lines = content.split("\n");

    for (let i = 0; i < lines.length; i++) {
      const folded = foldLine(lines[i], width, breakAtSpaces, countBytes);
      if (i < lines.length - 1) {
        process.stdout.write(folded + "\n");
      } else if (lines[i] !== "") {
        process.stdout.write(folded);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
