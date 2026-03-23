/**
 * wc -- word, line, and byte count.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `wc` utility in TypeScript. It
 * counts newlines, words, and bytes in files, printing the results in
 * right-aligned columns.
 *
 * === How wc Works ===
 *
 * By default (no flags), wc prints three counts for each file:
 *
 *     $ wc file.txt
 *        10   42  256 file.txt
 *
 * That's: 10 lines, 42 words, 256 bytes. When multiple files are given,
 * wc also prints a "total" line summing all counts.
 *
 * === Counting Rules ===
 *
 * - **Lines** (`-l`): Count of newline characters (`\n`). A file that
 *   ends without a trailing newline has its last line *not* counted.
 *   This matches the Unix convention that a "line" is terminated by `\n`.
 *
 * - **Words** (`-w`): Count of whitespace-delimited sequences. A "word"
 *   is any contiguous run of non-whitespace characters. Whitespace means
 *   space, tab, newline, carriage return, form feed, or vertical tab.
 *
 * - **Bytes** (`-c`): Total bytes in the file. For ASCII text, this
 *   equals the character count. For UTF-8, multi-byte characters count
 *   as multiple bytes.
 *
 * - **Characters** (`-m`): Total characters (Unicode code points). This
 *   differs from bytes for multi-byte encodings.
 *
 * - **Max line length** (`-L`): The display width of the longest line
 *   (not counting the newline). Tabs count as one character.
 *
 * === Flag Selection ===
 *
 * Individual flags select which counts to display:
 *
 *     wc -l file.txt         => only line count
 *     wc -lw file.txt        => lines and words
 *     wc file.txt            => lines, words, and bytes (default)
 *
 * `-c` (bytes) and `-m` (chars) are mutually exclusive because they
 * occupy the same column position in the output.
 *
 * @module wc
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
const SPEC_FILE = path.resolve(__dirname, "..", "wc.json");

// ---------------------------------------------------------------------------
// Types: Count results for a single file.
// ---------------------------------------------------------------------------

/**
 * The counts computed for a single file.
 *
 * We always compute all counts regardless of which flags are set. The
 * flags only control which columns are *displayed*. This simplifies the
 * counting logic and lets us compute totals easily.
 */
interface FileCounts {
  lines: number;
  words: number;
  bytes: number;
  chars: number;
  maxLineLength: number;
  filename: string;
}

// ---------------------------------------------------------------------------
// Business Logic: Count lines, words, and bytes.
// ---------------------------------------------------------------------------

/**
 * Count everything in a string.
 *
 * This function computes all five metrics in a single pass through the
 * content. We track:
 * - `lines`: number of `\n` characters
 * - `words`: transitions from whitespace to non-whitespace
 * - `bytes`: length of the UTF-8 encoded content
 * - `chars`: number of Unicode code points
 * - `maxLineLength`: length of the longest line (excluding `\n`)
 *
 * === Word Counting Algorithm ===
 *
 * We use a state machine with one boolean: `inWord`. We start outside
 * a word (`inWord = false`). Each time we transition from whitespace to
 * non-whitespace, we increment the word count. This handles leading
 * whitespace, trailing whitespace, and multiple consecutive spaces
 * correctly.
 *
 *     "  hello   world  "
 *      ^^     ^^^     ^^
 *      |       |       |
 *      outside inside  outside
 *
 *     Two transitions into a word => 2 words.
 */
function countContent(content: string, filename: string): FileCounts {
  let lines = 0;
  let words = 0;
  let maxLineLength = 0;
  let currentLineLength = 0;
  let inWord = false;

  for (let i = 0; i < content.length; i++) {
    const ch = content[i];

    if (ch === "\n") {
      lines++;
      if (currentLineLength > maxLineLength) {
        maxLineLength = currentLineLength;
      }
      currentLineLength = 0;
    } else {
      currentLineLength++;
    }

    // A "word character" is anything that isn't whitespace.
    // Whitespace characters: space, tab, newline, carriage return,
    // form feed (\f), vertical tab (\v).
    const isWhitespace =
      ch === " " ||
      ch === "\t" ||
      ch === "\n" ||
      ch === "\r" ||
      ch === "\f" ||
      ch === "\v";

    if (isWhitespace) {
      inWord = false;
    } else if (!inWord) {
      // Transition from whitespace to non-whitespace: new word.
      inWord = true;
      words++;
    }
  }

  // Handle the last line if it doesn't end with \n.
  if (currentLineLength > maxLineLength) {
    maxLineLength = currentLineLength;
  }

  return {
    lines,
    words,
    bytes: Buffer.byteLength(content, "utf-8"),
    chars: [...content].length,
    maxLineLength,
    filename,
  };
}

// ---------------------------------------------------------------------------
// Output: Format and print counts.
// ---------------------------------------------------------------------------

/**
 * Which columns to display, determined by the flags.
 */
interface DisplayFlags {
  showLines: boolean;
  showWords: boolean;
  showBytes: boolean;
  showChars: boolean;
  showMaxLineLength: boolean;
}

/**
 * Format a single line of output.
 *
 * Each count is right-aligned in a column. The column width is determined
 * by the largest count across all files (including the total). GNU wc
 * uses a minimum width of 1, but dynamically expands based on the widest
 * number.
 *
 * @param counts       The counts for this file.
 * @param display      Which columns to show.
 * @param columnWidth  The width for numeric columns.
 */
function formatLine(
  counts: FileCounts,
  display: DisplayFlags,
  columnWidth: number
): string {
  const parts: string[] = [];

  if (display.showLines) {
    parts.push(String(counts.lines).padStart(columnWidth));
  }
  if (display.showWords) {
    parts.push(String(counts.words).padStart(columnWidth));
  }
  if (display.showBytes) {
    parts.push(String(counts.bytes).padStart(columnWidth));
  }
  if (display.showChars) {
    parts.push(String(counts.chars).padStart(columnWidth));
  }
  if (display.showMaxLineLength) {
    parts.push(String(counts.maxLineLength).padStart(columnWidth));
  }

  // Add the filename (unless it's stdin, which has no name to show
  // when only one file is being processed).
  if (counts.filename) {
    parts.push(counts.filename);
  }

  return parts.join(" ");
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then count.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * 1. Parse arguments with CLI Builder.
 * 2. Handle --help and --version.
 * 3. Determine which columns to display from flags.
 * 4. Read each file and compute counts.
 * 5. If multiple files, add a "total" line.
 * 6. Format and print all results.
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
        process.stderr.write(`wc: ${error.message}\n`);
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

  // --- Step 3: Determine which columns to display --------------------------
  // If no specific flags are given, show lines + words + bytes (the default).
  // If any flag is given, show only the selected columns.

  const flags = (result as { flags: Record<string, unknown> }).flags;

  const anyFlagSet =
    !!flags["lines"] ||
    !!flags["words"] ||
    !!flags["bytes"] ||
    !!flags["chars"] ||
    !!flags["max_line_length"];

  const display: DisplayFlags = {
    showLines: anyFlagSet ? !!flags["lines"] : true,
    showWords: anyFlagSet ? !!flags["words"] : true,
    showBytes: anyFlagSet ? !!flags["bytes"] : true,
    showChars: !!flags["chars"],
    showMaxLineLength: !!flags["max_line_length"],
  };

  // If no specific flags are set, default shows lines+words+bytes,
  // but NOT chars (since bytes is shown instead).
  if (!anyFlagSet) {
    display.showChars = false;
  }

  // --- Step 4: Read files and compute counts -------------------------------

  const args = (result as { arguments: Record<string, unknown> }).arguments;
  let files = args["files"] as string[] | string | undefined;

  if (!files) {
    files = ["-"];
  } else if (typeof files === "string") {
    files = [files];
  }

  const allCounts: FileCounts[] = [];

  for (const file of files) {
    let content: string;

    if (file === "-") {
      // Read from stdin.
      try {
        content = fs.readFileSync(0, "utf-8");
      } catch {
        continue;
      }
      allCounts.push(countContent(content, ""));
    } else {
      try {
        content = fs.readFileSync(file, "utf-8");
        allCounts.push(countContent(content, file));
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        process.stderr.write(`wc: ${file}: ${message}\n`);
        process.exitCode = 1;
      }
    }
  }

  // --- Step 5: Compute totals (if multiple files) --------------------------

  if (allCounts.length > 1) {
    const total: FileCounts = {
      lines: 0,
      words: 0,
      bytes: 0,
      chars: 0,
      maxLineLength: 0,
      filename: "total",
    };

    for (const c of allCounts) {
      total.lines += c.lines;
      total.words += c.words;
      total.bytes += c.bytes;
      total.chars += c.chars;
      if (c.maxLineLength > total.maxLineLength) {
        total.maxLineLength = c.maxLineLength;
      }
    }

    allCounts.push(total);
  }

  // --- Step 6: Determine column width and print ----------------------------
  // Find the widest number across all counts to set column width.

  let maxVal = 0;
  for (const c of allCounts) {
    if (c.lines > maxVal) maxVal = c.lines;
    if (c.words > maxVal) maxVal = c.words;
    if (c.bytes > maxVal) maxVal = c.bytes;
    if (c.chars > maxVal) maxVal = c.chars;
    if (c.maxLineLength > maxVal) maxVal = c.maxLineLength;
  }

  // Column width is at least 1, and wide enough for the largest number.
  const columnWidth = Math.max(1, String(maxVal).length);

  for (const counts of allCounts) {
    process.stdout.write(formatLine(counts, display, columnWidth) + "\n");
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
