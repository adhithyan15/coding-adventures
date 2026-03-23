/**
 * split -- split a file into pieces.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `split` utility in TypeScript.
 * It reads an input file and writes it out in fixed-size chunks. Each
 * chunk gets a unique filename based on a prefix and a suffix.
 *
 * === How split Works ===
 *
 *     split bigfile.txt          =>  xaa, xab, xac, ... (1000 lines each)
 *     split -l 100 bigfile.txt   =>  xaa, xab, ... (100 lines each)
 *     split -b 1024 data.bin     =>  xaa, xab, ... (1024 bytes each)
 *     split -l 50 data out_      =>  out_aa, out_ab, ... (prefix "out_")
 *
 * === Suffix Generation ===
 *
 * By default, split generates two-character alphabetic suffixes:
 *
 *     aa, ab, ac, ..., az, ba, bb, ..., zz
 *
 * This gives 26^2 = 676 possible output files with the default
 * suffix length of 2.
 *
 * With -d (--numeric-suffixes), numeric suffixes are used instead:
 *
 *     00, 01, 02, ..., 99
 *
 * With -x (--hex-suffixes), hexadecimal suffixes are used:
 *
 *     00, 01, ..., 09, 0a, 0b, ..., ff
 *
 * The suffix length can be changed with -a N (default: 2).
 *
 * === Split Modes ===
 *
 *     -l N   Split by line count (default: 1000 lines per file)
 *     -b N   Split by byte count (e.g., -b 1024, -b 1M)
 *     -n N   Split into exactly N chunks
 *
 * These modes are mutually exclusive.
 *
 * @module split
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

const __filename_split = fileURLToPath(import.meta.url);
const __dirname_split = path.dirname(__filename_split);
const SPEC_FILE = path.resolve(__dirname_split, "..", "split.json");

// ---------------------------------------------------------------------------
// Types: Options that control split's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration for suffix generation.
 */
export interface SuffixOptions {
  /** Length of the suffix (default 2). */
  suffixLength: number;
  /** Use numeric suffixes instead of alphabetic (-d). */
  numeric: boolean;
  /** Use hexadecimal suffixes (-x). */
  hex: boolean;
  /** Additional suffix appended to each filename. */
  additionalSuffix: string;
}

// ---------------------------------------------------------------------------
// Business Logic: Generate output filenames.
// ---------------------------------------------------------------------------

/**
 * Generate a suffix string for the given chunk index.
 *
 * The suffix encodes the chunk number in the chosen base:
 * - Alphabetic (default): aa, ab, ..., az, ba, ..., zz
 * - Numeric (-d): 00, 01, ..., 99
 * - Hex (-x): 00, 01, ..., 0f, 10, ..., ff
 *
 * The suffix is zero/a-padded to the specified length.
 *
 * @param index   The chunk index (0-based).
 * @param opts    Suffix options.
 * @returns       The suffix string.
 * @throws        If the index exceeds the suffix space.
 *
 * @example
 * ```ts
 * generateSuffix(0, { suffixLength: 2, numeric: false, hex: false, additionalSuffix: "" });
 * // => "aa"
 * generateSuffix(27, { suffixLength: 2, numeric: false, hex: false, additionalSuffix: "" });
 * // => "bb"
 * generateSuffix(5, { suffixLength: 2, numeric: true, hex: false, additionalSuffix: "" });
 * // => "05"
 * ```
 */
export function generateSuffix(index: number, opts: SuffixOptions): string {
  if (opts.numeric) {
    // Numeric: base-10 encoding.
    const maxValue = Math.pow(10, opts.suffixLength);
    if (index >= maxValue) {
      throw new Error(
        `split: output file suffixes exhausted (max ${maxValue} files with -a ${opts.suffixLength})`
      );
    }
    return String(index).padStart(opts.suffixLength, "0");
  }

  if (opts.hex) {
    // Hexadecimal: base-16 encoding.
    const maxValue = Math.pow(16, opts.suffixLength);
    if (index >= maxValue) {
      throw new Error(
        `split: output file suffixes exhausted (max ${maxValue} files with -a ${opts.suffixLength})`
      );
    }
    return index.toString(16).padStart(opts.suffixLength, "0");
  }

  // Alphabetic: base-26 encoding using a-z.
  const maxValue = Math.pow(26, opts.suffixLength);
  if (index >= maxValue) {
    throw new Error(
      `split: output file suffixes exhausted (max ${maxValue} files with -a ${opts.suffixLength})`
    );
  }

  let result = "";
  let remaining = index;

  for (let pos = 0; pos < opts.suffixLength; pos++) {
    const charIndex = remaining % 26;
    result = String.fromCharCode(97 + charIndex) + result;  // 'a' = 97
    remaining = Math.floor(remaining / 26);
  }

  return result;
}

// ---------------------------------------------------------------------------
// Business Logic: Generate full output filename.
// ---------------------------------------------------------------------------

/**
 * Generate the full output filename for a chunk.
 *
 * @param prefix  The filename prefix (default "x").
 * @param index   The chunk index (0-based).
 * @param opts    Suffix options.
 * @returns       The complete filename.
 *
 * @example
 * ```ts
 * generateFilename("x", 0, { suffixLength: 2, numeric: false, hex: false, additionalSuffix: "" });
 * // => "xaa"
 * generateFilename("out_", 3, { suffixLength: 2, numeric: true, hex: false, additionalSuffix: ".txt" });
 * // => "out_03.txt"
 * ```
 */
export function generateFilename(
  prefix: string,
  index: number,
  opts: SuffixOptions
): string {
  return prefix + generateSuffix(index, opts) + opts.additionalSuffix;
}

// ---------------------------------------------------------------------------
// Business Logic: Split content by line count.
// ---------------------------------------------------------------------------

/**
 * Split text content into chunks of N lines each.
 *
 * Returns an array of [filename, content] pairs. The last chunk may
 * have fewer than N lines.
 *
 * @param content    The full text content to split.
 * @param lineCount  Number of lines per output file.
 * @param prefix     Output filename prefix.
 * @param suffixOpts Suffix generation options.
 * @returns          Array of [filename, content] pairs.
 *
 * @example
 * ```ts
 * splitByLines("a\nb\nc\nd\ne\n", 2, "x", defaultSuffixOpts);
 * // => [["xaa", "a\nb\n"], ["xab", "c\nd\n"], ["xac", "e\n"]]
 * ```
 */
export function splitByLines(
  content: string,
  lineCount: number,
  prefix: string,
  suffixOpts: SuffixOptions
): Array<[string, string]> {
  const results: Array<[string, string]> = [];

  // Split into lines, preserving line endings by re-adding them.
  const lines = content.split("\n");

  // Remove trailing empty string from split if content ends with \n.
  if (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }

  if (lines.length === 0) {
    return results;
  }

  let chunkIndex = 0;

  for (let i = 0; i < lines.length; i += lineCount) {
    const chunk = lines.slice(i, i + lineCount);
    const filename = generateFilename(prefix, chunkIndex, suffixOpts);
    const chunkContent = chunk.join("\n") + "\n";
    results.push([filename, chunkContent]);
    chunkIndex++;
  }

  return results;
}

// ---------------------------------------------------------------------------
// Business Logic: Split content by byte count.
// ---------------------------------------------------------------------------

/**
 * Parse a byte size string like "1024", "1K", "1M", "1G".
 *
 * Supported suffixes (case-insensitive):
 * - K or KB: kilobytes (1024)
 * - M or MB: megabytes (1024^2)
 * - G or GB: gigabytes (1024^3)
 *
 * @param sizeStr  The size string to parse.
 * @returns        The size in bytes.
 * @throws         If the string is not a valid size.
 */
export function parseByteSize(sizeStr: string): number {
  const match = sizeStr.match(/^(\d+)\s*([kmg]b?)?$/i);

  if (!match) {
    throw new Error(`split: invalid number of bytes: '${sizeStr}'`);
  }

  const value = parseInt(match[1], 10);
  const suffix = (match[2] || "").toUpperCase();

  switch (suffix) {
    case "K":
    case "KB":
      return value * 1024;
    case "M":
    case "MB":
      return value * 1024 * 1024;
    case "G":
    case "GB":
      return value * 1024 * 1024 * 1024;
    case "":
      return value;
    default:
      throw new Error(`split: invalid number of bytes: '${sizeStr}'`);
  }
}

/**
 * Split binary/text content into chunks of N bytes each.
 *
 * @param content    The content as a Buffer.
 * @param byteCount  Number of bytes per output file.
 * @param prefix     Output filename prefix.
 * @param suffixOpts Suffix generation options.
 * @returns          Array of [filename, content] pairs.
 */
export function splitByBytes(
  content: Buffer,
  byteCount: number,
  prefix: string,
  suffixOpts: SuffixOptions
): Array<[string, Buffer]> {
  const results: Array<[string, Buffer]> = [];

  if (content.length === 0) {
    return results;
  }

  let chunkIndex = 0;

  for (let i = 0; i < content.length; i += byteCount) {
    const chunk = content.subarray(i, i + byteCount);
    const filename = generateFilename(prefix, chunkIndex, suffixOpts);
    results.push([filename, chunk]);
    chunkIndex++;
  }

  return results;
}

// ---------------------------------------------------------------------------
// Business Logic: Split content into N equal chunks.
// ---------------------------------------------------------------------------

/**
 * Split content into exactly N chunks (by bytes).
 *
 * Each chunk gets approximately contentLength / N bytes. The last
 * chunk gets any remainder.
 *
 * @param content    The content as a Buffer.
 * @param numChunks  Number of output files.
 * @param prefix     Output filename prefix.
 * @param suffixOpts Suffix generation options.
 * @returns          Array of [filename, content] pairs.
 */
export function splitByChunks(
  content: Buffer,
  numChunks: number,
  prefix: string,
  suffixOpts: SuffixOptions
): Array<[string, Buffer]> {
  const results: Array<[string, Buffer]> = [];

  if (content.length === 0 || numChunks <= 0) {
    return results;
  }

  const chunkSize = Math.ceil(content.length / numChunks);

  for (let i = 0; i < numChunks; i++) {
    const start = i * chunkSize;
    if (start >= content.length) break;
    const end = Math.min(start + chunkSize, content.length);
    const chunk = content.subarray(start, end);
    const filename = generateFilename(prefix, i, suffixOpts);
    results.push([filename, chunk]);
  }

  return results;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then split the file.
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
        process.stderr.write(`split: ${error.message}\n`);
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
  const filePath: string = result.args?.file || "-";
  const prefix: string = result.args?.prefix || "x";

  const suffixOpts: SuffixOptions = {
    suffixLength: flags.suffix_length ?? 2,
    numeric: flags.numeric_suffixes || false,
    hex: flags.hex_suffixes || false,
    additionalSuffix: flags.additional_suffix || "",
  };

  try {
    // Read input file (or stdin).
    let content: Buffer;

    if (filePath === "-") {
      content = fs.readFileSync(0); // stdin
    } else {
      content = fs.readFileSync(filePath);
    }

    // Determine split mode and execute.
    let chunks: Array<[string, Buffer | string]>;

    if (flags.bytes) {
      const byteCount = parseByteSize(flags.bytes);
      chunks = splitByBytes(content, byteCount, prefix, suffixOpts);
    } else if (flags.number) {
      const numChunks = parseInt(flags.number, 10);
      chunks = splitByChunks(content, numChunks, prefix, suffixOpts);
    } else {
      // Default: split by lines.
      const lineCount = flags.lines ?? 1000;
      const textContent = content.toString("utf-8");
      chunks = splitByLines(textContent, lineCount, prefix, suffixOpts);
    }

    // Write output files.
    for (const [filename, chunkContent] of chunks) {
      if (flags.verbose) {
        process.stderr.write(`creating file '${filename}'\n`);
      }
      fs.writeFileSync(filename, chunkContent);
    }
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(`split: ${err.message}\n`);
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
