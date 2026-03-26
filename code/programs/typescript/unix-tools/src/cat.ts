/**
 * cat -- concatenate files and print on standard output.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `cat` utility in TypeScript. It
 * reads files sequentially and writes their contents to standard output.
 * When no files are given, or when a file is `-`, it reads from stdin.
 *
 * === How cat Works ===
 *
 * At its simplest, cat just copies bytes from input to output:
 *
 *     cat file1.txt file2.txt    =>   contents of file1 followed by file2
 *     cat                        =>   copies stdin to stdout (interactive)
 *     cat -                      =>   same as above
 *
 * The name "cat" comes from "concatenate" -- it joins files together
 * end-to-end.
 *
 * === Display Flags ===
 *
 * cat offers several flags that transform the output:
 *
 * - `-n` (--number):         Number ALL output lines starting from 1.
 * - `-b` (--number-nonblank): Number only non-blank lines (overrides -n).
 * - `-s` (--squeeze-blank):  Collapse consecutive blank lines into one.
 * - `-T` (--show-tabs):      Display TAB characters as `^I`.
 * - `-E` (--show-ends):      Display `$` at the end of each line.
 * - `-v` (--show-nonprinting): Use `^` and `M-` notation for control chars.
 * - `-A` (--show-all):       Equivalent to `-vET` (show everything).
 *
 * === Line Numbering Semantics ===
 *
 * When numbering lines (`-n` or `-b`), the line counter is global across
 * all files. This matches GNU cat behavior:
 *
 *     $ cat -n file1.txt file2.txt
 *          1  first file line 1
 *          2  first file line 2
 *          3  second file line 1
 *
 * The number is right-justified in a field of width 6, followed by a tab.
 *
 * @module cat
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
const SPEC_FILE = path.resolve(__dirname, "..", "cat.json");

// ---------------------------------------------------------------------------
// Types: Options that control cat's output transformation.
// ---------------------------------------------------------------------------

/**
 * Configuration extracted from parsed flags.
 *
 * We collect all the boolean flags into a single options object so that
 * the processing functions have a clean interface. The `-A` flag is
 * expanded into its component flags at parse time.
 */
interface CatOptions {
  numberLines: boolean;
  numberNonblank: boolean;
  squeezeBlank: boolean;
  showTabs: boolean;
  showEnds: boolean;
  showNonprinting: boolean;
}

// ---------------------------------------------------------------------------
// Business Logic: Process and output file contents.
// ---------------------------------------------------------------------------

/**
 * Process a single file's content through cat's transformation pipeline.
 *
 * This function implements all of cat's display transformations:
 * 1. Split input into lines.
 * 2. Squeeze consecutive blank lines (if -s).
 * 3. Number lines (if -n or -b).
 * 4. Show tabs as ^I (if -T).
 * 5. Show $ at end of lines (if -E).
 * 6. Show non-printing characters (if -v).
 *
 * The line number counter is passed in and returned so it persists
 * across multiple files.
 *
 * @param content    The file content as a string.
 * @param options    The display options.
 * @param lineNum    The current line number (for numbering across files).
 * @returns          The new line number after processing this file.
 */
function processContent(
  content: string,
  options: CatOptions,
  lineNum: number
): number {
  // Split into lines. We use a regex that preserves the line endings
  // so we can handle the last line correctly (it may or may not have
  // a trailing newline).
  const lines = content.split("\n");

  // Track consecutive blank lines for squeeze mode.
  let consecutiveBlankCount = 0;

  for (let i = 0; i < lines.length; i++) {
    let line = lines[i];

    // The split creates an empty string after the final \n.
    // Don't print an extra empty line for it.
    if (i === lines.length - 1 && line === "") {
      break;
    }

    const isBlank = line.trim() === "";

    // --- Squeeze blank lines (-s) ----------------------------------------
    // If we've seen more than one consecutive blank line, skip this one.
    // This collapses runs of blank lines down to a single blank line.
    if (options.squeezeBlank) {
      if (isBlank) {
        consecutiveBlankCount++;
        if (consecutiveBlankCount > 1) {
          continue;
        }
      } else {
        consecutiveBlankCount = 0;
      }
    }

    // --- Show non-printing characters (-v) --------------------------------
    // Replace control characters with ^ notation and high-bit characters
    // with M- notation. Tabs and newlines are NOT replaced (they have
    // their own flags).
    if (options.showNonprinting) {
      line = showNonprinting(line);
    }

    // --- Show tabs (-T) ---------------------------------------------------
    // Replace each tab character with the two-character sequence ^I.
    if (options.showTabs) {
      line = line.replace(/\t/g, "^I");
    }

    // --- Show ends (-E) ---------------------------------------------------
    // Append $ at the end of the line (before the newline).
    if (options.showEnds) {
      line = line + "$";
    }

    // --- Number lines (-n or -b) ------------------------------------------
    // -b numbers only non-blank lines. -n numbers all lines.
    // -b overrides -n (if both are specified, only non-blank are numbered).
    if (options.numberNonblank) {
      if (!isBlank) {
        line = `${String(lineNum).padStart(6)} \t${line}`;
        lineNum++;
      }
    } else if (options.numberLines) {
      line = `${String(lineNum).padStart(6)}\t${line}`;
      lineNum++;
    }

    process.stdout.write(line + "\n");
  }

  return lineNum;
}

/**
 * Convert non-printing characters to visible notation.
 *
 * This implements the same notation as GNU cat -v:
 * - Control characters (0x00-0x1F) become ^@ through ^_ (except TAB and LF)
 * - DEL (0x7F) becomes ^?
 * - High-bit characters (0x80-0xFF) become M- prefixed
 *
 * === The ^ Notation ===
 *
 * Control characters are encoded by adding 64 to their ASCII value and
 * prefixing with ^. For example:
 *   0x01 (SOH) => ^A  (1 + 64 = 65 = 'A')
 *   0x1A (SUB) => ^Z  (26 + 64 = 90 = 'Z')
 *   0x00 (NUL) => ^@  (0 + 64 = 64 = '@')
 */
function showNonprinting(line: string): string {
  const output: string[] = [];

  for (let i = 0; i < line.length; i++) {
    const code = line.charCodeAt(i);

    if (code === 9) {
      // Tab -- leave as-is (handled by -T flag separately).
      output.push("\t");
    } else if (code === 10) {
      // Newline -- leave as-is.
      output.push("\n");
    } else if (code < 32) {
      // Control character: ^@ through ^_
      output.push("^" + String.fromCharCode(code + 64));
    } else if (code === 127) {
      // DEL character
      output.push("^?");
    } else if (code >= 128 && code < 160) {
      // High-bit control characters: M-^@ through M-^_
      output.push("M-^" + String.fromCharCode(code - 128 + 64));
    } else if (code >= 160 && code < 255) {
      // High-bit printable characters: M-  through M-~
      output.push("M-" + String.fromCharCode(code - 128));
    } else if (code === 255) {
      // M-^?
      output.push("M-^?");
    } else {
      // Regular printable character.
      output.push(line[i]);
    }
  }

  return output.join("");
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then concatenate files.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * 1. Parse arguments with CLI Builder.
 * 2. Handle --help and --version.
 * 3. Extract flags into a CatOptions object.
 * 4. Process each file in order, maintaining a global line counter.
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
        process.stderr.write(`cat: ${error.message}\n`);
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

  // --- Step 3: Extract flags -----------------------------------------------
  // Expand -A (show-all) into its component flags: -v, -E, -T.

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const showAll = !!flags["show_all"];

  const options: CatOptions = {
    numberLines: !!flags["number"],
    numberNonblank: !!flags["number_nonblank"],
    squeezeBlank: !!flags["squeeze_blank"],
    showTabs: showAll || !!flags["show_tabs"],
    showEnds: showAll || !!flags["show_ends"],
    showNonprinting: showAll || !!flags["show_nonprinting"],
  };

  // --- Step 4: Process files -----------------------------------------------

  const args = (result as { arguments: Record<string, unknown> }).arguments;
  let files = args["files"] as string[] | string | undefined;

  // Normalize to an array. If no files given, default to stdin.
  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

  let lineNum = 1;

  for (const file of files) {
    if (file === "-") {
      // Read from stdin. In a synchronous context, we read all of stdin
      // at once using fs.readFileSync on fd 0.
      try {
        const content = fs.readFileSync(0, "utf-8");
        lineNum = processContent(content, options, lineNum);
      } catch {
        // stdin may not be available (e.g., in certain test environments).
        // Silently continue to the next file.
      }
    } else {
      // Read from a file.
      try {
        const content = fs.readFileSync(file, "utf-8");
        lineNum = processContent(content, options, lineNum);
      } catch (err: unknown) {
        const message =
          err instanceof Error ? err.message : String(err);
        process.stderr.write(`cat: ${file}: ${message}\n`);
        process.exitCode = 1;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
