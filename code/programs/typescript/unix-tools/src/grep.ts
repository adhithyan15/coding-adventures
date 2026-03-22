/**
 * grep -- print lines that match patterns.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `grep` utility in TypeScript.
 * It searches for lines matching a regular expression pattern in one
 * or more files, and prints matching lines to standard output.
 *
 * === How grep Works ===
 *
 *     grep "hello" file.txt         =>  lines containing "hello"
 *     grep -i "hello" file.txt      =>  case-insensitive match
 *     grep -v "hello" file.txt      =>  lines NOT containing "hello"
 *     grep -n "hello" file.txt      =>  with line numbers
 *     grep -c "hello" file.txt      =>  count of matching lines
 *
 * === Regular Expression Modes ===
 *
 * grep supports several regex interpretation modes:
 *
 *     -G (default): Basic Regular Expressions (BRE)
 *     -E:           Extended Regular Expressions (ERE)
 *     -F:           Fixed strings (literal match, no regex)
 *     -P:           Perl-compatible regex (PCRE)
 *
 * In our TypeScript implementation, we use JavaScript's built-in
 * RegExp engine, which is closest to ERE/PCRE. For -F (fixed strings),
 * we escape all regex metacharacters.
 *
 * === Matching Modifiers ===
 *
 *     -i   Ignore case
 *     -v   Invert match (select non-matching lines)
 *     -w   Whole word match (surround pattern with word boundaries)
 *     -x   Whole line match (anchor pattern with ^ and $)
 *
 * === Output Control ===
 *
 *     -n   Print line numbers
 *     -c   Print only a count of matching lines
 *     -l   Print only filenames with matches
 *     -L   Print only filenames without matches
 *     -o   Print only the matched part of each line
 *     -q   Quiet: no output, just set exit code
 *     -m N Stop after N matches
 *
 * @module grep
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

const __filename_grep = fileURLToPath(import.meta.url);
const __dirname_grep = path.dirname(__filename_grep);
const SPEC_FILE = path.resolve(__dirname_grep, "..", "grep.json");

// ---------------------------------------------------------------------------
// Types: Options that control grep's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for grep operations.
 */
export interface GrepOptions {
  /** Treat pattern as a fixed string, not regex (-F). */
  fixedStrings: boolean;
  /** Ignore case when matching (-i). */
  ignoreCase: boolean;
  /** Select non-matching lines (-v). */
  invertMatch: boolean;
  /** Match whole words only (-w). */
  wordRegexp: boolean;
  /** Match whole lines only (-x). */
  lineRegexp: boolean;
  /** Print line numbers (-n). */
  lineNumber: boolean;
  /** Print only a count of matches (-c). */
  count: boolean;
  /** Print only names of files with matches (-l). */
  filesWithMatches: boolean;
  /** Print only names of files without matches (-L). */
  filesWithoutMatch: boolean;
  /** Print only the matched parts (-o). */
  onlyMatching: boolean;
  /** Maximum number of matches per file (-m). */
  maxCount: number | null;
}

// ---------------------------------------------------------------------------
// Types: A single grep match result.
// ---------------------------------------------------------------------------

/**
 * Represents a single matching line from grep.
 */
export interface GrepMatch {
  /** The line number (1-based). */
  lineNumber: number;
  /** The full text of the matching line. */
  line: string;
  /** The matched portions of the line (for -o mode). */
  matches: string[];
}

// ---------------------------------------------------------------------------
// Business Logic: Build regex from pattern.
// ---------------------------------------------------------------------------

/**
 * Build a RegExp from a pattern string and grep options.
 *
 * This function handles:
 * - Fixed strings (-F): escape all regex metacharacters.
 * - Word match (-w): wrap pattern in word boundary anchors.
 * - Line match (-x): wrap pattern in ^ and $ anchors.
 * - Case insensitive (-i): add the 'i' flag.
 *
 * JavaScript's RegExp is used, which behaves like ERE/PCRE for most
 * practical purposes.
 *
 * @param pattern  The search pattern.
 * @param opts     Grep options.
 * @returns        A compiled RegExp.
 *
 * @example
 * ```ts
 * buildRegex("hello", { fixedStrings: false, ignoreCase: true, wordRegexp: false, lineRegexp: false, ... });
 * // => /hello/i
 * ```
 */
export function buildRegex(pattern: string, opts: GrepOptions): RegExp {
  let regexStr = pattern;

  // Fixed strings: escape all regex metacharacters so the pattern
  // is matched literally.
  if (opts.fixedStrings) {
    regexStr = regexStr.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  // Word match: surround with word boundary anchors.
  // \b matches the boundary between a word character and a non-word character.
  if (opts.wordRegexp) {
    regexStr = `\\b${regexStr}\\b`;
  }

  // Line match: anchor to start and end of line.
  if (opts.lineRegexp) {
    regexStr = `^${regexStr}$`;
  }

  // We do NOT add the 'g' flag here. The global flag causes stateful
  // behavior in RegExp.test() and RegExp.exec() (lastIndex advances),
  // which leads to bugs when the same regex is reused across lines.
  // The 'g' flag is added only in extractMatches(), which needs to
  // find all occurrences within a single line.
  const flags = opts.ignoreCase ? "i" : "";

  return new RegExp(regexStr, flags);
}

// ---------------------------------------------------------------------------
// Business Logic: Test a single line against the pattern.
// ---------------------------------------------------------------------------

/**
 * Test whether a single line matches the grep pattern.
 *
 * This is the core matching function. It applies the regex to the
 * line and handles invert-match mode.
 *
 * @param line     The line of text to test.
 * @param pattern  The compiled regex pattern.
 * @param opts     Grep options (for invertMatch).
 * @returns        True if the line should be included in output.
 *
 * @example
 * ```ts
 * const re = /hello/i;
 * grepLine("Hello World", re, { invertMatch: false, ... });  // => true
 * grepLine("Goodbye", re, { invertMatch: false, ... });       // => false
 * ```
 */
export function grepLine(
  line: string,
  pattern: RegExp,
  opts: GrepOptions
): boolean {
  const matches = pattern.test(line);

  // Invert match: return true for NON-matching lines.
  return opts.invertMatch ? !matches : matches;
}

// ---------------------------------------------------------------------------
// Business Logic: Get all matches from a line (for -o mode).
// ---------------------------------------------------------------------------

/**
 * Extract all matches from a line.
 *
 * Used for the -o (--only-matching) flag, which prints only the
 * matched portions of each line, one per output line.
 *
 * @param line     The line to search.
 * @param pattern  The compiled regex pattern.
 * @returns        Array of matched strings.
 */
export function extractMatches(line: string, pattern: RegExp): string[] {
  // Create a global version of the pattern so exec() finds all matches.
  const globalPattern = new RegExp(pattern.source, pattern.flags + "g");
  const results: string[] = [];
  let match: RegExpExecArray | null;

  while ((match = globalPattern.exec(line)) !== null) {
    results.push(match[0]);
    // Prevent infinite loop on zero-length matches.
    if (match[0].length === 0) {
      globalPattern.lastIndex++;
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Business Logic: Search within a file's content (lines).
// ---------------------------------------------------------------------------

/**
 * Search for pattern matches across an array of lines.
 *
 * This function processes all lines and returns the matches, respecting
 * maxCount limits. It works on pre-split lines (not files) so it can
 * be tested without filesystem access.
 *
 * @param lines    Array of text lines to search.
 * @param pattern  The compiled regex pattern.
 * @param opts     Grep options.
 * @returns        Array of GrepMatch objects for matching lines.
 */
export function grepLines(
  lines: string[],
  pattern: RegExp,
  opts: GrepOptions
): GrepMatch[] {
  const results: GrepMatch[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (grepLine(line, pattern, opts)) {
      const matchObj: GrepMatch = {
        lineNumber: i + 1,
        line,
        matches: opts.onlyMatching ? extractMatches(line, pattern) : [],
      };
      results.push(matchObj);

      // Respect max count.
      if (opts.maxCount !== null && results.length >= opts.maxCount) {
        break;
      }
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Business Logic: Search a file.
// ---------------------------------------------------------------------------

/**
 * Search for pattern matches in a file.
 *
 * Reads the file, splits into lines, and delegates to grepLines.
 *
 * @param filePath  Path to the file to search.
 * @param pattern   The compiled regex pattern.
 * @param opts      Grep options.
 * @returns         Array of GrepMatch objects.
 * @throws          If the file can't be read.
 */
export function grepFile(
  filePath: string,
  pattern: RegExp,
  opts: GrepOptions
): GrepMatch[] {
  let content: string;

  try {
    content = fs.readFileSync(filePath, "utf-8");
  } catch {
    throw new Error(
      `grep: ${filePath}: No such file or directory`
    );
  }

  const lines = content.split("\n");

  // Remove trailing empty string from split (if file ends with newline).
  if (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }

  return grepLines(lines, pattern, opts);
}

// ---------------------------------------------------------------------------
// Business Logic: Format grep output.
// ---------------------------------------------------------------------------

/**
 * Format grep matches for display.
 *
 * Handles all output modes: normal, line-numbered, only-matching,
 * count, and files-with-matches.
 *
 * @param matches   Array of GrepMatch objects.
 * @param filePath  The file path (for multi-file output).
 * @param opts      Grep options.
 * @param showFile  Whether to prefix output with the filename.
 * @returns         Array of formatted output lines.
 */
export function formatMatches(
  matches: GrepMatch[],
  filePath: string,
  opts: GrepOptions,
  showFile: boolean
): string[] {
  const output: string[] = [];
  const prefix = showFile ? `${filePath}:` : "";

  if (opts.count) {
    output.push(`${prefix}${matches.length}`);
    return output;
  }

  for (const m of matches) {
    if (opts.onlyMatching) {
      for (const match of m.matches) {
        const linePrefix = opts.lineNumber ? `${m.lineNumber}:` : "";
        output.push(`${prefix}${linePrefix}${match}`);
      }
    } else {
      const linePrefix = opts.lineNumber ? `${m.lineNumber}:` : "";
      output.push(`${prefix}${linePrefix}${m.line}`);
    }
  }

  return output;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then search files.
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
        process.stderr.write(`grep: ${error.message}\n`);
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
  const patternArg: string = result.args?.pattern || "";
  const files: string[] = result.args?.files || [];

  const opts: GrepOptions = {
    fixedStrings: flags.fixed_strings || false,
    ignoreCase: flags.ignore_case || false,
    invertMatch: flags.invert_match || false,
    wordRegexp: flags.word_regexp || false,
    lineRegexp: flags.line_regexp || false,
    lineNumber: flags.line_number || false,
    count: flags.count || false,
    filesWithMatches: flags.files_with_matches || false,
    filesWithoutMatch: flags.files_without_match || false,
    onlyMatching: flags.only_matching || false,
    maxCount: flags.max_count ?? null,
  };

  try {
    const pattern = buildRegex(patternArg, opts);
    const showFile = files.length > 1;
    let anyMatch = false;

    for (const filePath of files) {
      const matches = grepFile(filePath, pattern, opts);

      if (opts.filesWithMatches) {
        if (matches.length > 0) {
          process.stdout.write(filePath + "\n");
          anyMatch = true;
        }
      } else if (opts.filesWithoutMatch) {
        if (matches.length === 0) {
          process.stdout.write(filePath + "\n");
        }
      } else {
        const output = formatMatches(matches, filePath, opts, showFile);
        for (const line of output) {
          process.stdout.write(line + "\n");
        }
        if (matches.length > 0) anyMatch = true;
      }
    }

    // grep exits with 0 if matches found, 1 if not.
    process.exit(anyMatch ? 0 : 1);
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(err.message + "\n");
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
