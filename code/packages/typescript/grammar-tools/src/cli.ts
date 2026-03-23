#!/usr/bin/env node
/**
 * cli.ts — Command-line entry point for grammar-tools validation.
 *
 * Run with::
 *
 *     grammar-tools validate tokens.file grammar.file
 *     grammar-tools validate-tokens tokens.file
 *     grammar-tools validate-grammar grammar.file
 *     grammar-tools --help
 *
 * Why a CLI tool?
 * --------------
 *
 * The validation functions (``validateTokenGrammar``, ``validateParserGrammar``,
 * ``crossValidate``) exist as library code, but when you are writing or editing
 * ``.tokens`` and ``.grammar`` files, you need a quick way to check for typos
 * and consistency errors without writing TypeScript code. This CLI tool is that
 * quick check.
 *
 * Think of it like a compiler's ``-fsyntax-only`` flag — it parses and validates
 * without generating any output, and tells you exactly what is wrong.
 *
 * Exit codes
 * ----------
 *
 * - 0: all checks passed
 * - 1: one or more validation errors found
 * - 2: usage error (wrong number of arguments, unknown command)
 *
 * Output format
 * -------------
 *
 * On success::
 *
 *     Validating lattice.tokens ... OK (N tokens, M skip, K error)
 *     Validating lattice.grammar ... OK (P rules)
 *     Cross-validating ... OK
 *
 *     All checks passed.
 *
 * On failure::
 *
 *     Validating broken.tokens ... 2 error(s)
 *       Line 5: Duplicate token name 'IDENT' ...
 *     Found 4 error(s). Fix them and try again.
 */

import { readFileSync } from "fs";
import { basename } from "path";

import {
  TokenGrammarError,
  parseTokenGrammar,
  validateTokenGrammar,
} from "./token-grammar.js";
import {
  ParserGrammarError,
  parseParserGrammar,
  validateParserGrammar,
} from "./parser-grammar.js";
import { crossValidate } from "./cross-validator.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Count how many issues are actual errors (not warnings).
 *
 * Issues starting with "Warning:" are informational and do not cause the tool
 * to fail. Everything else (undefined references, duplicates, etc.) is an error.
 */
function countErrors(issues: string[]): number {
  return issues.filter((i) => !i.startsWith("Warning:")).length;
}

/**
 * Print a list of issues indented by two spaces.
 *
 * Each issue is printed on its own line so that CI logs remain scannable.
 */
function printIssues(issues: string[], indent = "  "): void {
  for (const issue of issues) {
    process.stdout.write(`${indent}${issue}\n`);
  }
}

/**
 * Read a file, returning its text content.
 *
 * Returns null and prints an error message if the file cannot be opened.
 * Using ``readFileSync`` keeps the control flow linear — no callbacks needed
 * for a simple CLI tool that reads one or two files and exits.
 */
function readFile(filePath: string): string | null {
  try {
    return readFileSync(filePath, "utf-8");
  } catch (e: unknown) {
    const message = e instanceof Error ? e.message : String(e);
    process.stdout.write(`Error: Cannot read file '${filePath}': ${message}\n`);
    return null;
  }
}

// ---------------------------------------------------------------------------
// Validate command — full pair validation
// ---------------------------------------------------------------------------

/**
 * Validate a .tokens and .grammar file pair.
 *
 * This is the core of the ``validate`` subcommand. It:
 *   1. Parses the .tokens file and runs ``validateTokenGrammar``
 *   2. Parses the .grammar file and runs ``validateParserGrammar``
 *   3. Cross-validates the two with ``crossValidate``
 *
 * @param tokensPath - Path to the .tokens file.
 * @param grammarPath - Path to the .grammar file.
 * @returns 0 if all checks pass, 1 if any errors are found.
 */
export function validateCommand(tokensPath: string, grammarPath: string): number {
  let totalErrors = 0;

  // --- Parse and validate the .tokens file ---
  const tokensText = readFile(tokensPath);
  if (tokensText === null) return 1;

  const tokensName = basename(tokensPath);
  process.stdout.write(`Validating ${tokensName} ... `);

  let tokenGrammar;
  try {
    tokenGrammar = parseTokenGrammar(tokensText);
  } catch (e: unknown) {
    const message = e instanceof TokenGrammarError ? e.message : String(e);
    process.stdout.write(`PARSE ERROR\n  ${message}\n`);
    return 1;
  }

  const tokenIssues = validateTokenGrammar(tokenGrammar);
  const nTokens = tokenGrammar.definitions.length;
  const nSkip = tokenGrammar.skipDefinitions?.length ?? 0;
  const tokenErrors = countErrors(tokenIssues);

  if (tokenErrors > 0) {
    process.stdout.write(`${tokenErrors} error(s)\n`);
    printIssues(tokenIssues);
    totalErrors += tokenErrors;
  } else {
    // Build the OK summary: "N tokens", optionally "M skip"
    const parts = [`${nTokens} tokens`];
    if (nSkip > 0) parts.push(`${nSkip} skip`);
    process.stdout.write(`OK (${parts.join(", ")})\n`);
  }

  // --- Parse and validate the .grammar file ---
  const grammarText = readFile(grammarPath);
  if (grammarText === null) return 1;

  const grammarName = basename(grammarPath);
  process.stdout.write(`Validating ${grammarName} ... `);

  let parserGrammar;
  try {
    parserGrammar = parseParserGrammar(grammarText);
  } catch (e: unknown) {
    const message = e instanceof ParserGrammarError ? e.message : String(e);
    process.stdout.write(`PARSE ERROR\n  ${message}\n`);
    return 1;
  }

  // Pass the token names from the .tokens file so that undefined token
  // references in the grammar can be caught during validation.
  const tokenNamesSet = new Set(tokenGrammar.definitions.map((d) => d.alias ?? d.name));
  const parserIssues = validateParserGrammar(parserGrammar, tokenNamesSet);
  const nRules = parserGrammar.rules.length;
  const parserErrors = countErrors(parserIssues);

  if (parserErrors > 0) {
    process.stdout.write(`${parserErrors} error(s)\n`);
    printIssues(parserIssues);
    totalErrors += parserErrors;
  } else {
    process.stdout.write(`OK (${nRules} rules)\n`);
  }

  // --- Cross-validate ---
  process.stdout.write(`Cross-validating ... `);
  const crossIssues = crossValidate(tokenGrammar, parserGrammar);
  const crossErrors = countErrors(crossIssues);
  const crossWarnings = crossIssues.length - crossErrors;

  if (crossErrors > 0) {
    process.stdout.write(`${crossErrors} error(s)\n`);
    printIssues(crossIssues);
    totalErrors += crossErrors;
  } else if (crossWarnings > 0) {
    process.stdout.write(`OK (${crossWarnings} warning(s))\n`);
    printIssues(crossIssues);
  } else {
    process.stdout.write(`OK\n`);
  }

  // --- Summary ---
  if (totalErrors > 0) {
    process.stdout.write(`\nFound ${totalErrors} error(s). Fix them and try again.\n`);
    return 1;
  } else {
    process.stdout.write(`\nAll checks passed.\n`);
    return 0;
  }
}

// ---------------------------------------------------------------------------
// Validate-tokens command — tokens-only validation
// ---------------------------------------------------------------------------

/**
 * Validate just a .tokens file (no grammar file required).
 *
 * Useful when you are still writing the grammar or want a quick sanity-check
 * on just the lexer definition.
 *
 * @param tokensPath - Path to the .tokens file.
 * @returns 0 if all checks pass, 1 if any errors are found.
 */
export function validateTokensOnly(tokensPath: string): number {
  const tokensText = readFile(tokensPath);
  if (tokensText === null) return 1;

  const tokensName = basename(tokensPath);
  process.stdout.write(`Validating ${tokensName} ... `);

  let tokenGrammar;
  try {
    tokenGrammar = parseTokenGrammar(tokensText);
  } catch (e: unknown) {
    const message = e instanceof TokenGrammarError ? e.message : String(e);
    process.stdout.write(`PARSE ERROR\n  ${message}\n`);
    return 1;
  }

  const issues = validateTokenGrammar(tokenGrammar);
  const nTokens = tokenGrammar.definitions.length;
  const errors = countErrors(issues);

  if (errors > 0) {
    process.stdout.write(`${errors} error(s)\n`);
    printIssues(issues);
    process.stdout.write(`\nFound ${errors} error(s). Fix them and try again.\n`);
    return 1;
  } else {
    process.stdout.write(`OK (${nTokens} tokens)\n`);
    process.stdout.write(`\nAll checks passed.\n`);
    return 0;
  }
}

// ---------------------------------------------------------------------------
// Validate-grammar command — grammar-only validation
// ---------------------------------------------------------------------------

/**
 * Validate just a .grammar file (no tokens file required).
 *
 * Without a tokens file, undefined token references cannot be detected, but
 * structural issues (duplicate rules, unreachable rules, bad rule names) are
 * still caught.
 *
 * @param grammarPath - Path to the .grammar file.
 * @returns 0 if all checks pass, 1 if any errors are found.
 */
export function validateGrammarOnly(grammarPath: string): number {
  const grammarText = readFile(grammarPath);
  if (grammarText === null) return 1;

  const grammarName = basename(grammarPath);
  process.stdout.write(`Validating ${grammarName} ... `);

  let parserGrammar;
  try {
    parserGrammar = parseParserGrammar(grammarText);
  } catch (e: unknown) {
    const message = e instanceof ParserGrammarError ? e.message : String(e);
    process.stdout.write(`PARSE ERROR\n  ${message}\n`);
    return 1;
  }

  // Without a tokens file, pass null so token-reference checks are skipped.
  const issues = validateParserGrammar(parserGrammar, null);
  const nRules = parserGrammar.rules.length;
  const errors = countErrors(issues);

  if (errors > 0) {
    process.stdout.write(`${errors} error(s)\n`);
    printIssues(issues);
    process.stdout.write(`\nFound ${errors} error(s). Fix them and try again.\n`);
    return 1;
  } else {
    process.stdout.write(`OK (${nRules} rules)\n`);
    process.stdout.write(`\nAll checks passed.\n`);
    return 0;
  }
}

// ---------------------------------------------------------------------------
// Usage / help
// ---------------------------------------------------------------------------

/**
 * Print usage information to stdout.
 *
 * Mirrors the Python grammar-tools CLI output format so that users who know
 * the Python version can use the TypeScript version without re-reading docs.
 */
export function printUsage(): void {
  process.stdout.write("Usage: grammar-tools <command> [args...]\n");
  process.stdout.write("\n");
  process.stdout.write("Commands:\n");
  process.stdout.write("  validate <file.tokens> <file.grammar>  Validate a token/grammar pair\n");
  process.stdout.write("  validate-tokens <file.tokens>           Validate just a .tokens file\n");
  process.stdout.write("  validate-grammar <file.grammar>         Validate just a .grammar file\n");
  process.stdout.write("\n");
  process.stdout.write("Examples:\n");
  process.stdout.write("  grammar-tools validate css.tokens css.grammar\n");
  process.stdout.write("  grammar-tools validate-tokens css.tokens\n");
  process.stdout.write("  grammar-tools validate-grammar css.grammar\n");
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/**
 * Main entry point for the grammar-tools CLI.
 *
 * Parses command-line arguments (from ``process.argv``) and dispatches to the
 * appropriate validation function.
 *
 * Why ``process.argv``? It is the idiomatic way to read CLI arguments in
 * Node.js. Unlike ``argv[0]`` (node binary) and ``argv[1]`` (script path),
 * ``argv[2...]`` are the actual user-supplied arguments.
 *
 * @returns Exit code: 0 for success, 1 for errors, 2 for usage errors.
 */
export function main(): number {
  // Skip the first two elements: "node" and the script path.
  const args = process.argv.slice(2);

  if (args.length === 0 || args[0] === "--help" || args[0] === "-h" || args[0] === "help") {
    printUsage();
    return 0;
  }

  const command = args[0];

  if (command === "validate") {
    if (args.length !== 3) {
      process.stdout.write("Error: 'validate' requires two arguments: <tokens> <grammar>\n\n");
      printUsage();
      return 2;
    }
    return validateCommand(args[1], args[2]);
  }

  if (command === "validate-tokens") {
    if (args.length !== 2) {
      process.stdout.write("Error: 'validate-tokens' requires one argument: <tokens>\n\n");
      printUsage();
      return 2;
    }
    return validateTokensOnly(args[1]);
  }

  if (command === "validate-grammar") {
    if (args.length !== 2) {
      process.stdout.write("Error: 'validate-grammar' requires one argument: <grammar>\n\n");
      printUsage();
      return 2;
    }
    return validateGrammarOnly(args[1]);
  }

  process.stdout.write(`Error: Unknown command '${command}'\n\n`);
  printUsage();
  return 2;
}

// Run when executed directly (not imported as a module).
// ``import.meta.url`` holds the URL of the current module. Comparing it with
// ``process.argv[1]`` (converted to a URL) tells us if we are the entry point.
// This pattern is the ESM equivalent of Python's ``if __name__ == "__main__"``.
const scriptUrl = new URL(import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1");
const argPath = process.argv[1]?.replace(/\\/g, "/") ?? "";

if (
  scriptUrl === argPath ||
  scriptUrl.replace(/\.ts$/, ".js") === argPath ||
  argPath.endsWith("/cli.js") ||
  argPath.endsWith("\\cli.js")
) {
  process.exit(main());
}
