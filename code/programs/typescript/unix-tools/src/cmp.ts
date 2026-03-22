/**
 * cmp -- compare two files byte by byte.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `cmp` utility in TypeScript.
 * It reads two files and compares them byte by byte, reporting the
 * first position where they differ.
 *
 * === How cmp Works ===
 *
 * The simplest comparison tool: read both files simultaneously, one
 * byte at a time, and stop at the first difference:
 *
 *     $ cmp file1.txt file2.txt
 *     file1.txt file2.txt differ: byte 42, line 3
 *
 * If the files are identical, cmp produces no output and exits with
 * status 0. If they differ, it exits with status 1. If an error
 * occurs (e.g., file not found), it exits with status 2.
 *
 * === Modes of Operation ===
 *
 *     Default:  Report first difference (byte number, line number)
 *     -l:       List ALL differences (byte number, byte1, byte2)
 *     -s:       Silent -- no output, just exit status
 *     -b:       Print differing bytes as characters
 *
 * === Byte Position Counting ===
 *
 * cmp counts bytes starting from 1 (not 0). Line numbers also start
 * from 1. Every newline (0x0A) increments the line counter.
 *
 *     Byte:    H  e  l  l  o  \n  W  o  r  l  d
 *     Position: 1  2  3  4  5   6  7  8  9 10 11
 *     Line:    1  1  1  1  1   1  2  2  2  2  2
 *
 * Note that the newline byte itself is on the line it terminates,
 * and the line counter increments AFTER it.
 *
 * @module cmp
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

const __filename_cmp = fileURLToPath(import.meta.url);
const __dirname_cmp = path.dirname(__filename_cmp);
const SPEC_FILE = path.resolve(__dirname_cmp, "..", "cmp.json");

// ---------------------------------------------------------------------------
// Types: Options that control cmp's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for byte-level file comparison.
 *
 * These map directly to the cmp command-line flags:
 *
 *     Flag   Option        Meaning
 *     ----   ------        -------
 *     -l     list          List all differences (not just first)
 *     -s     silent        No output, exit status only
 *     -b     printBytes    Print differing bytes as characters
 *     -i     skipBytes     Skip the first N bytes of each file
 *     -n     maxBytes      Compare at most N bytes
 */
export interface CmpOptions {
  /** List all differences, not just the first (-l). */
  list: boolean;
  /** Silent mode -- produce no output (-s). */
  silent: boolean;
  /** Print differing bytes as printable characters (-b). */
  printBytes: boolean;
  /** Number of bytes to skip at the start of each file (-i). */
  skipBytes: number;
  /** Maximum number of bytes to compare (-n). Zero means no limit. */
  maxBytes: number;
}

// ---------------------------------------------------------------------------
// Result Types: Structured output from comparison.
// ---------------------------------------------------------------------------

/**
 * A single byte difference between two files.
 *
 * Used when running in list mode (-l) to record every position
 * where the files disagree.
 */
export interface ByteDifference {
  /** 1-based byte position where the difference occurs. */
  byteNumber: number;
  /** Line number (1-based) at this position in file 1. */
  lineNumber: number;
  /** The byte value from file 1 (0-255). */
  byte1: number;
  /** The byte value from file 2 (0-255). */
  byte2: number;
}

/**
 * The result of comparing two files byte by byte.
 *
 * This structure captures everything needed to produce cmp's output
 * in any mode (default, list, silent, print-bytes).
 */
export interface CmpResult {
  /** Are the compared regions identical? */
  identical: boolean;
  /** The first difference found (undefined if identical). */
  firstDiff?: ByteDifference;
  /** All differences found (populated only in list mode). */
  allDiffs: ByteDifference[];
  /** True if one file is a prefix of the other (EOF reached early). */
  eofReached: boolean;
  /** Which file hit EOF first: 1 or 2 (0 if neither). */
  eofFile: 0 | 1 | 2;
}

// ---------------------------------------------------------------------------
// Business Logic: Compare two buffers byte by byte.
// ---------------------------------------------------------------------------

/**
 * Compare two byte buffers and report differences.
 *
 * This is the core comparison engine. It works on Buffers rather
 * than files so that it can be easily tested without touching the
 * filesystem.
 *
 * The algorithm is simple: walk both buffers in lockstep, comparing
 * each byte. Track the current line number by counting newlines.
 *
 *     buf1:  [72, 101, 108, 108, 111]   "Hello"
 *     buf2:  [72, 101, 76,  108, 111]   "HeLlo"
 *                        ^
 *                   byte 3, line 1 -- first difference
 *
 * @param buf1      First buffer to compare.
 * @param buf2      Second buffer to compare.
 * @param opts      Comparison options (skip, max, list mode).
 * @returns         A CmpResult describing the comparison outcome.
 *
 * @example
 * ```ts
 * const a = Buffer.from("hello\nworld");
 * const b = Buffer.from("hello\nWORLD");
 * const result = compareBuffers(a, b, { list: false, silent: false, printBytes: false, skipBytes: 0, maxBytes: 0 });
 * // result.identical === false
 * // result.firstDiff.byteNumber === 7 (the 'w' vs 'W')
 * // result.firstDiff.lineNumber === 2
 * ```
 */
export function compareBuffers(
  buf1: Buffer,
  buf2: Buffer,
  opts: CmpOptions
): CmpResult {
  // --- Step 1: Apply the skip offset -----------------------------------
  // The -i flag tells cmp to skip the first N bytes of BOTH files.
  // This is useful for ignoring headers in binary files.

  const start = opts.skipBytes;
  const effective1 = buf1.subarray(start);
  const effective2 = buf2.subarray(start);

  // --- Step 2: Determine comparison length -----------------------------
  // We compare the shorter of the two buffers, then check if one
  // is longer (indicating an EOF difference).

  const len1 = effective1.length;
  const len2 = effective2.length;
  let compareLen = Math.min(len1, len2);

  // If -n was given, cap the comparison length.
  if (opts.maxBytes > 0) {
    compareLen = Math.min(compareLen, opts.maxBytes);
  }

  // --- Step 3: Walk both buffers in lockstep ---------------------------

  const allDiffs: ByteDifference[] = [];
  let firstDiff: ByteDifference | undefined;
  let lineNumber = 1;

  for (let i = 0; i < compareLen; i++) {
    const b1 = effective1[i];
    const b2 = effective2[i];

    // Check for a difference at this position.
    if (b1 !== b2) {
      const diff: ByteDifference = {
        byteNumber: start + i + 1, // 1-based
        lineNumber,
        byte1: b1,
        byte2: b2,
      };

      if (!firstDiff) {
        firstDiff = diff;
      }

      if (opts.list) {
        allDiffs.push(diff);
      } else if (!opts.silent) {
        // In default mode, we only need the first difference.
        // We can stop early.
        return {
          identical: false,
          firstDiff,
          allDiffs,
          eofReached: false,
          eofFile: 0,
        };
      } else {
        // Silent mode: we know they differ, exit immediately.
        return {
          identical: false,
          firstDiff,
          allDiffs,
          eofReached: false,
          eofFile: 0,
        };
      }
    }

    // Track line numbers: newline (0x0A) increments the counter.
    if (b1 === 0x0a) {
      lineNumber++;
    }
  }

  // --- Step 4: Check for EOF differences -------------------------------
  // If we've compared all bytes in the overlap region but one file
  // is longer, cmp reports "EOF on file X".

  // Only check EOF if maxBytes wasn't limiting us, or if we compared
  // fewer bytes than maxBytes requested.
  const maxEffective = opts.maxBytes > 0 ? opts.maxBytes : Infinity;
  if (len1 !== len2 && compareLen < maxEffective) {
    const eofFile: 1 | 2 = len1 < len2 ? 1 : 2;
    return {
      identical: false,
      firstDiff,
      allDiffs,
      eofReached: true,
      eofFile,
    };
  }

  // --- Step 5: Return the result ---------------------------------------

  if (firstDiff) {
    // We found differences (list mode collected them all).
    return {
      identical: false,
      firstDiff,
      allDiffs,
      eofReached: false,
      eofFile: 0,
    };
  }

  // Files are identical in the compared region.
  return {
    identical: true,
    allDiffs: [],
    eofReached: false,
    eofFile: 0,
  };
}

// ---------------------------------------------------------------------------
// Business Logic: Format output for display.
// ---------------------------------------------------------------------------

/**
 * Format a byte value for display with -b (print-bytes) flag.
 *
 * Printable ASCII characters (0x20-0x7E) are shown as themselves.
 * Non-printable bytes are shown in octal notation.
 *
 *     formatByte(65)  => "A"      (printable)
 *     formatByte(10)  => "\\n"    (newline -- special case)
 *     formatByte(0)   => "\\0"    (null -- special case)
 *     formatByte(128) => "200"    (octal)
 *
 * @param byte  A byte value (0-255).
 * @returns     A human-readable string representation.
 */
export function formatByte(byte: number): string {
  // Special named characters that cmp traditionally shows by name.
  const specialNames: Record<number, string> = {
    0x00: "\\0",   // null
    0x07: "\\a",   // bell
    0x08: "\\b",   // backspace
    0x09: "\\t",   // tab
    0x0a: "\\n",   // newline
    0x0b: "\\v",   // vertical tab
    0x0c: "\\f",   // form feed
    0x0d: "\\r",   // carriage return
  };

  if (byte in specialNames) {
    return specialNames[byte];
  }

  // Printable ASCII range: space (0x20) through tilde (0x7E).
  if (byte >= 0x20 && byte <= 0x7e) {
    return String.fromCharCode(byte);
  }

  // Everything else: show in octal.
  return "\\" + byte.toString(8).padStart(3, "0");
}

/**
 * Format the default-mode output line.
 *
 * The standard format for cmp's first-difference report:
 *
 *     file1 file2 differ: byte NN, line NN
 *
 * With -b, each byte value is appended in parentheses:
 *
 *     file1 file2 differ: byte NN, line NN is OOO CHAR1 OOO CHAR2
 *
 * @param file1Name  Name of the first file.
 * @param file2Name  Name of the second file.
 * @param diff       The byte difference to report.
 * @param printBytes Whether to include byte representations.
 * @returns          The formatted output line.
 */
export function formatDefaultDiff(
  file1Name: string,
  file2Name: string,
  diff: ByteDifference,
  printBytes: boolean
): string {
  let line = `${file1Name} ${file2Name} differ: byte ${diff.byteNumber}, line ${diff.lineNumber}`;
  if (printBytes) {
    const oct1 = diff.byte1.toString(8);
    const oct2 = diff.byte2.toString(8);
    line += ` is ${oct1} ${formatByte(diff.byte1)} ${oct2} ${formatByte(diff.byte2)}`;
  }
  return line;
}

/**
 * Format a list-mode (-l) output line.
 *
 * In list mode, each difference is reported as:
 *
 *     BYTE_NUMBER OCTAL1 OCTAL2
 *
 * With -b, character representations are added:
 *
 *     BYTE_NUMBER OCTAL1 CHAR1 OCTAL2 CHAR2
 *
 * @param diff       The byte difference.
 * @param printBytes Whether to include character representations.
 * @returns          The formatted line.
 */
export function formatListDiff(
  diff: ByteDifference,
  printBytes: boolean
): string {
  const oct1 = diff.byte1.toString(8).padStart(3, " ");
  const oct2 = diff.byte2.toString(8).padStart(3, " ");

  if (printBytes) {
    return `${diff.byteNumber.toString().padStart(6, " ")} ${oct1} ${formatByte(diff.byte1)}  ${oct2} ${formatByte(diff.byte2)}`;
  }

  return `${diff.byteNumber.toString().padStart(6, " ")} ${oct1} ${oct2}`;
}

/**
 * Format an EOF message.
 *
 * When one file is shorter than the other, cmp reports:
 *
 *     cmp: EOF on fileName after byte NN, line NN
 *
 * @param fileName   The file that ended early.
 * @param byteCount  Total bytes compared before EOF.
 * @param lineCount  Line number at EOF.
 * @returns          The formatted EOF message.
 */
export function formatEof(
  fileName: string,
  byteCount: number,
  lineCount: number
): string {
  return `cmp: EOF on ${fileName} after byte ${byteCount}, line ${lineCount}`;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then compare files.
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
        process.stderr.write(`cmp: ${error.message}\n`);
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

  const file1Path = args.file1 as string;
  const file2Path = args.file2 as string || "-";

  const opts: CmpOptions = {
    list: flags.list || false,
    silent: flags.silent || false,
    printBytes: flags.print_bytes || false,
    skipBytes: flags.ignore_initial ? parseInt(flags.ignore_initial as string, 10) : 0,
    maxBytes: (flags.max_bytes as number) || 0,
  };

  try {
    const buf1 = fs.readFileSync(file1Path);
    const buf2 = file2Path === "-"
      ? fs.readFileSync("/dev/stdin")
      : fs.readFileSync(file2Path);

    const cmpResult = compareBuffers(buf1, buf2, opts);

    if (cmpResult.identical) {
      process.exit(0);
    }

    if (opts.silent) {
      process.exit(1);
    }

    if (cmpResult.eofReached) {
      const eofName = cmpResult.eofFile === 1 ? file1Path : file2Path;
      const minLen = Math.min(buf1.length, buf2.length);
      let lines = 1;
      const eof = cmpResult.eofFile === 1 ? buf1 : buf2;
      for (let i = 0; i < eof.length; i++) {
        if (eof[i] === 0x0a) lines++;
      }
      process.stderr.write(formatEof(eofName, minLen, lines) + "\n");

      if (cmpResult.firstDiff) {
        process.stdout.write(
          formatDefaultDiff(file1Path, file2Path, cmpResult.firstDiff, opts.printBytes) + "\n"
        );
      }

      process.exit(1);
    }

    if (opts.list) {
      for (const diff of cmpResult.allDiffs) {
        process.stdout.write(formatListDiff(diff, opts.printBytes) + "\n");
      }
    } else if (cmpResult.firstDiff) {
      process.stdout.write(
        formatDefaultDiff(file1Path, file2Path, cmpResult.firstDiff, opts.printBytes) + "\n"
      );
    }

    process.exit(1);
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(`cmp: ${err.message}\n`);
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
