/**
 * xargs -- build and execute command lines from standard input.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `xargs` utility in TypeScript.
 * It reads items from standard input (or a file), constructs command
 * lines from those items, and executes them.
 *
 * === How xargs Works ===
 *
 * xargs solves a common problem: many commands accept arguments on
 * the command line, but you have a list of items coming from a pipe.
 * xargs bridges this gap:
 *
 *     $ find . -name "*.tmp" | xargs rm
 *
 * This is equivalent to:
 *
 *     $ rm ./a.tmp ./b.tmp ./c.tmp
 *
 * Without xargs, you'd need:
 *
 *     $ find . -name "*.tmp" -exec rm {} \;
 *
 * === Input Parsing ===
 *
 * By default, xargs splits input on whitespace (spaces, tabs, newlines).
 * Items can be quoted with single or double quotes:
 *
 *     $ echo '"hello world" foo bar' | xargs echo
 *     hello world foo bar
 *
 * The -0 flag changes the delimiter to NUL (\\0), which is useful
 * for filenames that contain spaces:
 *
 *     $ find . -name "*.txt" -print0 | xargs -0 rm
 *
 * === Batching ===
 *
 * The -n flag limits how many arguments per command invocation:
 *
 *     $ echo "a b c d" | xargs -n 2 echo
 *     a b
 *     c d
 *
 * This runs echo twice: once with "a b" and once with "c d".
 *
 * === Replacement ===
 *
 * The -I flag enables replacement mode. Each input item replaces
 * the placeholder in the command template:
 *
 *     $ echo -e "foo\nbar" | xargs -I {} echo "item: {}"
 *     item: foo
 *     item: bar
 *
 * @module xargs
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------

const __filename_xargs = fileURLToPath(import.meta.url);
const __dirname_xargs = path.dirname(__filename_xargs);
const SPEC_FILE = path.resolve(__dirname_xargs, "..", "xargs.json");

// ---------------------------------------------------------------------------
// Types: Options that control xargs's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for xargs.
 *
 *     Flag   Option          Meaning
 *     ----   ------          -------
 *     -0     nullDelimiter   Split on NUL instead of whitespace
 *     -d     delimiter       Custom single-character delimiter
 *     -n     maxArgs         Max arguments per command invocation
 *     -I     replaceStr      Replacement string in command template
 *     -t     verbose         Print commands to stderr before execution
 *     -r     noRunIfEmpty    Don't run if input is empty
 */
export interface XargsOptions {
  /** Use NUL as the input delimiter (-0). */
  nullDelimiter: boolean;
  /** Custom input delimiter (-d). */
  delimiter: string | null;
  /** Maximum number of arguments per command invocation (-n). */
  maxArgs: number;
  /** Replacement string for -I mode. */
  replaceStr: string | null;
  /** Print each command to stderr before executing (-t). */
  verbose: boolean;
  /** Don't run the command if input is empty (-r). */
  noRunIfEmpty: boolean;
}

// ---------------------------------------------------------------------------
// Business Logic: Parse input into items.
// ---------------------------------------------------------------------------

/**
 * Parse input text into individual items.
 *
 * The parsing behavior depends on the delimiter mode:
 *
 * **Default mode** (whitespace splitting with quote handling):
 *
 *     "hello world 'foo bar' baz"
 *     => ["hello", "world", "foo bar", "baz"]
 *
 * Supports single quotes, double quotes, and backslash escaping.
 *
 * **NUL mode** (-0):
 *
 *     "hello\0world\0"
 *     => ["hello", "world"]
 *
 * **Custom delimiter** (-d):
 *
 *     "hello,world,foo"  with delimiter ","
 *     => ["hello", "world", "foo"]
 *
 * @param input      The raw input text.
 * @param opts       xargs options (delimiter mode).
 * @returns          Array of parsed items.
 *
 * @example
 * ```ts
 * parseItems("hello world", { nullDelimiter: false, delimiter: null, ... });
 * // => ["hello", "world"]
 *
 * parseItems("hello\0world\0", { nullDelimiter: true, delimiter: null, ... });
 * // => ["hello", "world"]
 * ```
 */
export function parseItems(input: string, opts: XargsOptions): string[] {
  // --- NUL delimiter mode ----------------------------------------------
  if (opts.nullDelimiter) {
    return input.split("\0").filter(item => item.length > 0);
  }

  // --- Custom delimiter mode -------------------------------------------
  if (opts.delimiter !== null) {
    return input.split(opts.delimiter).filter(item => item.length > 0);
  }

  // --- Default mode: whitespace splitting with quote handling ----------
  // This is the most complex parsing mode. We need to handle:
  // - Whitespace as delimiters (space, tab, newline)
  // - Single-quoted strings ('foo bar')
  // - Double-quoted strings ("foo bar")
  // - Backslash escaping (foo\ bar)

  const items: string[] = [];
  let current = "";
  let inSingleQuote = false;
  let inDoubleQuote = false;
  let escaped = false;

  for (let i = 0; i < input.length; i++) {
    const ch = input[i];

    if (escaped) {
      current += ch;
      escaped = false;
      continue;
    }

    if (ch === "\\") {
      escaped = true;
      continue;
    }

    if (ch === "'" && !inDoubleQuote) {
      inSingleQuote = !inSingleQuote;
      continue;
    }

    if (ch === '"' && !inSingleQuote) {
      inDoubleQuote = !inDoubleQuote;
      continue;
    }

    if (!inSingleQuote && !inDoubleQuote && /\s/.test(ch)) {
      if (current.length > 0) {
        items.push(current);
        current = "";
      }
      continue;
    }

    current += ch;
  }

  // Don't forget the last item if input doesn't end with whitespace.
  if (current.length > 0) {
    items.push(current);
  }

  return items;
}

// ---------------------------------------------------------------------------
// Business Logic: Build command batches.
// ---------------------------------------------------------------------------

/**
 * Split items into batches for execution.
 *
 * Without -n (maxArgs), all items go into a single batch:
 *
 *     items: ["a", "b", "c", "d"], maxArgs: 0
 *     => [["a", "b", "c", "d"]]
 *
 * With -n, items are chunked:
 *
 *     items: ["a", "b", "c", "d"], maxArgs: 2
 *     => [["a", "b"], ["c", "d"]]
 *
 * @param items    The parsed input items.
 * @param maxArgs  Maximum items per batch (0 = all at once).
 * @returns        Array of batches.
 */
export function buildBatches(items: string[], maxArgs: number): string[][] {
  if (maxArgs <= 0 || items.length === 0) {
    return items.length > 0 ? [items] : [];
  }

  const batches: string[][] = [];
  for (let i = 0; i < items.length; i += maxArgs) {
    batches.push(items.slice(i, i + maxArgs));
  }

  return batches;
}

// ---------------------------------------------------------------------------
// Business Logic: Build command strings.
// ---------------------------------------------------------------------------

/**
 * Build the command string for a batch of items.
 *
 * In normal mode, items are appended to the base command:
 *
 *     command: ["echo"], items: ["hello", "world"]
 *     => "echo hello world"
 *
 * In replacement mode (-I), each item replaces the placeholder:
 *
 *     command: ["echo", "item: {}"], replaceStr: "{}", item: "foo"
 *     => "echo item: foo"
 *
 * @param baseCommand  The base command and its initial arguments.
 * @param items        The items to append or substitute.
 * @param replaceStr   The replacement string (null for normal mode).
 * @returns            Array of command strings to execute.
 */
export function buildCommands(
  baseCommand: string[],
  items: string[],
  replaceStr: string | null
): string[] {
  if (replaceStr !== null) {
    // Replacement mode: one command per item.
    return items.map(item => {
      const args = baseCommand.map(arg => arg.split(replaceStr).join(item));
      return args.map(a => a.includes(" ") ? `"${a}"` : a).join(" ");
    });
  }

  // Normal mode: append all items to the command.
  const quotedItems = items.map(item =>
    item.includes(" ") ? `"${item}"` : item
  );
  const cmd = [...baseCommand, ...quotedItems].join(" ");
  return [cmd];
}

// ---------------------------------------------------------------------------
// Business Logic: Execute a command.
// ---------------------------------------------------------------------------

/**
 * Execute a single command string and return the result.
 *
 * @param cmdStr    The command string to execute.
 * @param verbose   Whether to print the command to stderr.
 * @returns         An object with stdout and exit code.
 */
export function executeCommand(
  cmdStr: string,
  verbose: boolean
): { stdout: string; exitCode: number } {
  if (verbose) {
    process.stderr.write(cmdStr + "\n");
  }

  try {
    const output = execSync(cmdStr, { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] });
    return { stdout: output, exitCode: 0 };
  } catch (err: unknown) {
    if (err && typeof err === "object" && "status" in err) {
      const execErr = err as { status: number; stdout?: string };
      return {
        stdout: execErr.stdout || "",
        exitCode: execErr.status || 1,
      };
    }
    return { stdout: "", exitCode: 1 };
  }
}

// ---------------------------------------------------------------------------
// Business Logic: Run the full xargs pipeline.
// ---------------------------------------------------------------------------

/**
 * Run the complete xargs pipeline: parse input, build commands, execute.
 *
 * This is the main orchestrator function that ties together all the
 * business logic:
 *
 * 1. Parse the input into items.
 * 2. If no items and -r is set, do nothing.
 * 3. Build batches based on -n.
 * 4. Build command strings for each batch.
 * 5. Execute each command.
 *
 * @param input        The raw input text.
 * @param baseCommand  The base command (defaults to ["echo"]).
 * @param opts         xargs options.
 * @returns            Combined stdout from all commands and final exit code.
 */
export function runXargs(
  input: string,
  baseCommand: string[],
  opts: XargsOptions
): { output: string; exitCode: number } {
  // --- Step 1: Parse items from input ----------------------------------

  const items = parseItems(input, opts);

  // --- Step 2: Check for empty input -----------------------------------

  if (items.length === 0) {
    if (opts.noRunIfEmpty) {
      return { output: "", exitCode: 0 };
    }
    // Default behavior: still run the command with no arguments.
    // But only in normal mode (not replacement mode).
    if (opts.replaceStr !== null) {
      return { output: "", exitCode: 0 };
    }
  }

  // --- Step 3: Handle replacement mode ---------------------------------

  if (opts.replaceStr !== null) {
    const commands = buildCommands(baseCommand, items, opts.replaceStr);
    let output = "";
    let exitCode = 0;

    for (const cmd of commands) {
      const result = executeCommand(cmd, opts.verbose);
      output += result.stdout;
      if (result.exitCode !== 0) exitCode = result.exitCode;
    }

    return { output, exitCode };
  }

  // --- Step 4: Build batches and execute -------------------------------

  const batches = buildBatches(items, opts.maxArgs);

  if (batches.length === 0) {
    // No items, but we should still run the command once.
    const cmd = baseCommand.join(" ");
    const result = executeCommand(cmd, opts.verbose);
    return { output: result.stdout, exitCode: result.exitCode };
  }

  let output = "";
  let exitCode = 0;

  for (const batch of batches) {
    const commands = buildCommands(baseCommand, batch, null);
    for (const cmd of commands) {
      const result = executeCommand(cmd, opts.verbose);
      output += result.stdout;
      if (result.exitCode !== 0) exitCode = result.exitCode;
    }
  }

  return { output, exitCode };
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then run xargs.
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
        process.stderr.write(`xargs: ${error.message}\n`);
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
  const baseCommand: string[] = result.args?.command || ["echo"];

  const opts: XargsOptions = {
    nullDelimiter: flags.null || false,
    delimiter: (flags.delimiter as string) || null,
    maxArgs: (flags.max_args as number) || 0,
    replaceStr: (flags.replace as string) || null,
    verbose: flags.verbose || false,
    noRunIfEmpty: flags.no_run_if_empty || false,
  };

  // Read from stdin or arg-file.
  let input: string;
  if (flags.arg_file) {
    input = fs.readFileSync(flags.arg_file as string, "utf-8");
  } else {
    input = fs.readFileSync("/dev/stdin", "utf-8");
  }

  const { output, exitCode } = runXargs(input, baseCommand, opts);
  if (output) {
    process.stdout.write(output);
  }
  process.exit(exitCode);
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
