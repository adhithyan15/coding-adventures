/**
 * nl -- number lines of files.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `nl` utility in TypeScript. It
 * reads files and writes them to standard output with line numbers added.
 *
 * === How nl Works ===
 *
 * nl adds line numbers to the left of each line:
 *
 *     nl file.txt                =>   number non-empty lines
 *     nl -ba file.txt            =>   number ALL lines (including blank)
 *     nl -w 3 file.txt           =>   use 3-digit line numbers
 *     nl -s '. ' file.txt        =>   use ". " as the separator
 *
 * === Numbering Styles ===
 *
 * The numbering style controls which lines get numbers:
 *
 * - `a` (all): Number every line, including blank lines.
 * - `t` (text): Number only non-empty lines (the default for body).
 * - `n` (none): Don't number any lines (the default for header/footer).
 * - `pBRE`: Number only lines matching the basic regular expression BRE.
 *
 * === Number Formats ===
 *
 * The `-n` flag controls how line numbers are formatted:
 *
 * - `ln`: Left-justified, no leading zeros.
 * - `rn`: Right-justified, no leading zeros (the default).
 * - `rz`: Right-justified, leading zeros.
 *
 * === Logical Pages ===
 *
 * nl supports logical page sections (header, body, footer) delimited by
 * special lines. The default delimiters use `\:` (backslash-colon):
 *
 * - `\:\:\:` starts a header section.
 * - `\:\:` starts a body section.
 * - `\:` starts a footer section.
 *
 * Each section can have its own numbering style. By default, header and
 * footer lines are not numbered, and body lines are numbered if non-empty.
 *
 * @module nl
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
const SPEC_FILE = path.resolve(__dirname, "..", "nl.json");

// ---------------------------------------------------------------------------
// Business Logic: Line numbering.
// ---------------------------------------------------------------------------

/**
 * Determine if a line should be numbered based on the numbering style.
 *
 * @param line    The line content (without the line ending).
 * @param style   The numbering style: "a", "t", "n", or a pattern like "pBRE".
 * @returns       true if the line should be numbered.
 */
function shouldNumber(line: string, style: string): boolean {
  switch (style) {
    case "a":
      // Number all lines.
      return true;
    case "t":
      // Number only non-empty lines.
      return line.length > 0;
    case "n":
      // Number no lines.
      return false;
    default:
      // Pattern: "pBRE" -- number lines matching the regex.
      if (style.startsWith("p")) {
        const pattern = style.substring(1);
        try {
          const regex = new RegExp(pattern);
          return regex.test(line);
        } catch {
          return false;
        }
      }
      return false;
  }
}

/**
 * Format a line number according to the specified format and width.
 *
 * - `ln`: Left-justified, padded with spaces on the right.
 * - `rn`: Right-justified, padded with spaces on the left.
 * - `rz`: Right-justified, padded with zeros on the left.
 */
function formatNumber(
  num: number,
  format: string,
  width: number
): string {
  const numStr = String(num);

  switch (format) {
    case "ln":
      // Left-justified.
      return numStr.padEnd(width);
    case "rz":
      // Right-justified, zero-padded.
      return numStr.padStart(width, "0");
    case "rn":
    default:
      // Right-justified, space-padded.
      return numStr.padStart(width);
  }
}

/**
 * Format a line that should NOT be numbered.
 *
 * Non-numbered lines still get the space where the number would go,
 * but it's filled with blanks (no separator either -- just empty space).
 */
function formatUnnumberedLine(line: string, width: number): string {
  // GNU nl uses spaces of the same width as the number field.
  return " ".repeat(width) + "  " + line;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then number lines.
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
        process.stderr.write(`nl: ${error.message}\n`);
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

  const bodyNumbering = (flags["body_numbering"] as string) ?? "t";
  const headerNumbering = (flags["header_numbering"] as string) ?? "n";
  const footerNumbering = (flags["footer_numbering"] as string) ?? "n";
  const lineIncrement = (flags["line_increment"] as number) ?? 1;
  const numberFormat = (flags["number_format"] as string) ?? "rn";
  const numberWidth = (flags["number_width"] as number) ?? 6;
  const numberSeparator = (flags["number_separator"] as string) ?? "\t";
  const startingLineNumber = (flags["starting_line_number"] as number) ?? 1;
  const sectionDelimiter = (flags["section_delimiter"] as string) ?? "\\:";
  const noRenumber = !!flags["no_renumber"];

  // Normalize file list.
  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

  // --- Step 4: Build section delimiters ------------------------------------
  // Section delimiters are composed by repeating the delimiter string:
  //   - Header: delimiter repeated 3 times (e.g., "\:\:\:")
  //   - Body:   delimiter repeated 2 times (e.g., "\:\:")
  //   - Footer: delimiter repeated 1 time  (e.g., "\:")

  const headerDelim = sectionDelimiter.repeat(3);
  const bodyDelim = sectionDelimiter.repeat(2);
  const footerDelim = sectionDelimiter;

  // --- Step 5: Process each file -------------------------------------------

  let lineNumber = startingLineNumber;
  let currentStyle = bodyNumbering; // Start in body section.

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
        process.stderr.write(`nl: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    const lines = content.split("\n");
    const hasTrailing = content.endsWith("\n");

    // Process up to the last meaningful line.
    const limit = hasTrailing ? lines.length - 1 : lines.length;

    for (let i = 0; i < limit; i++) {
      const line = lines[i];

      // Check for section delimiters (longest match first).
      if (line === headerDelim) {
        currentStyle = headerNumbering;
        if (!noRenumber) {
          lineNumber = startingLineNumber;
        }
        process.stdout.write("\n");
        continue;
      }
      if (line === bodyDelim) {
        currentStyle = bodyNumbering;
        if (!noRenumber) {
          lineNumber = startingLineNumber;
        }
        process.stdout.write("\n");
        continue;
      }
      if (line === footerDelim) {
        currentStyle = footerNumbering;
        process.stdout.write("\n");
        continue;
      }

      // Number this line if the current style says so.
      if (shouldNumber(line, currentStyle)) {
        const numStr = formatNumber(lineNumber, numberFormat, numberWidth);
        process.stdout.write(numStr + numberSeparator + line + "\n");
        lineNumber += lineIncrement;
      } else {
        process.stdout.write(formatUnnumberedLine(line, numberWidth) + "\n");
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
