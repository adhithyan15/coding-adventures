#!/usr/bin/env node
/**
 * grammar-tools CLI — validate .tokens and .grammar files.
 *
 * This program wraps the @coding-adventures/grammar-tools library behind a
 * @coding-adventures/cli-builder-powered interface. It is the TypeScript
 * counterpart of the Python, Elixir, Go, Rust, and Ruby implementations.
 * All produce identical output so CI scripts can use any implementation.
 *
 * Usage:
 *
 *   grammar-tools validate <file.tokens> <file.grammar>
 *   grammar-tools validate-tokens <file.tokens>
 *   grammar-tools validate-grammar <file.grammar>
 *   grammar-tools --help
 *
 * Exit codes:
 *
 *   0  All checks passed.
 *   1  One or more validation errors found.
 *   2  Usage error (wrong number of arguments, unknown command).
 */

import { readFileSync, existsSync } from "fs";
import { basename, join, dirname, resolve } from "path";
import { fileURLToPath } from "url";
import { Parser } from "@coding-adventures/cli-builder";
import {
  TokenGrammarError,
  parseTokenGrammar,
  validateTokenGrammar,
  tokenNames,
} from "@coding-adventures/grammar-tools";
import {
  ParserGrammarError,
  parseParserGrammar,
  validateParserGrammar,
} from "@coding-adventures/grammar-tools";
import { crossValidate } from "@coding-adventures/grammar-tools";

const __dirname = dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Count issues that are actual errors, not informational warnings. */
function countErrors(issues: string[]): number {
  return issues.filter((i) => !i.startsWith("Warning:")).length;
}

/** Print issues with two-space indentation. */
function printIssues(issues: string[]): void {
  for (const issue of issues) {
    process.stdout.write(`  ${issue}\n`);
  }
}

function printUsage(): void {
  process.stderr.write("Usage: grammar-tools <command> [args...]\n");
  process.stderr.write("\n");
  process.stderr.write("Commands:\n");
  process.stderr.write("  validate <file.tokens> <file.grammar>  Validate a token/grammar pair\n");
  process.stderr.write("  validate-tokens <file.tokens>           Validate just a .tokens file\n");
  process.stderr.write("  validate-grammar <file.grammar>         Validate just a .grammar file\n");
  process.stderr.write("\n");
  process.stderr.write("Run 'grammar-tools --help' for full help text.\n");
}

/**
 * Walk up the directory tree from __dirname until we find code/specs/grammar-tools.json.
 * This identifies the repo root so we can locate the spec file.
 */
function findRoot(): string {
  let current = resolve(__dirname);
  for (let i = 0; i < 20; i++) {
    if (existsSync(join(current, "code", "specs", "grammar-tools.json"))) {
      return current;
    }
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
  return resolve(__dirname);
}

// ---------------------------------------------------------------------------
// validate — cross-validate a .tokens/.grammar pair
// ---------------------------------------------------------------------------

export function validateCommand(tokensPath: string, grammarPath: string): number {
  let totalIssues = 0;

  // Step 1: parse and validate .tokens
  if (!existsSync(tokensPath)) {
    process.stderr.write(`Error: File not found: ${tokensPath}\n`);
    return 1;
  }

  process.stdout.write(`Validating ${basename(tokensPath)} ... `);
  let tokenGrammar;
  try {
    tokenGrammar = parseTokenGrammar(readFileSync(tokensPath, "utf-8"));
  } catch (e) {
    if (e instanceof TokenGrammarError) {
      process.stdout.write("PARSE ERROR\n");
      process.stdout.write(`  ${e.message}\n`);
      return 1;
    }
    throw e;
  }

  const tokenIssues = validateTokenGrammar(tokenGrammar);
  const nTokens = tokenGrammar.definitions.length;
  const nSkip = tokenGrammar.skipDefinitions?.length ?? 0;
  const tokenErrors = countErrors(tokenIssues);

  if (tokenErrors > 0) {
    process.stdout.write(`${tokenErrors} error(s)\n`);
    printIssues(tokenIssues);
    totalIssues += tokenErrors;
  } else {
    const parts = [`${nTokens} tokens`];
    if (nSkip > 0) parts.push(`${nSkip} skip`);
    process.stdout.write(`OK (${parts.join(", ")})\n`);
  }

  // Step 2: parse and validate .grammar
  if (!existsSync(grammarPath)) {
    process.stderr.write(`Error: File not found: ${grammarPath}\n`);
    return 1;
  }

  process.stdout.write(`Validating ${basename(grammarPath)} ... `);
  let parserGrammar;
  try {
    parserGrammar = parseParserGrammar(readFileSync(grammarPath, "utf-8"));
  } catch (e) {
    if (e instanceof ParserGrammarError) {
      process.stdout.write("PARSE ERROR\n");
      process.stdout.write(`  ${e.message}\n`);
      return 1;
    }
    throw e;
  }

  const parserIssues = validateParserGrammar(parserGrammar, tokenNames(tokenGrammar));
  const nRules = parserGrammar.rules.length;
  const parserErrors = countErrors(parserIssues);

  if (parserErrors > 0) {
    process.stdout.write(`${parserErrors} error(s)\n`);
    printIssues(parserIssues);
    totalIssues += parserErrors;
  } else {
    process.stdout.write(`OK (${nRules} rules)\n`);
  }

  // Step 3: cross-validate
  process.stdout.write("Cross-validating ... ");
  const crossIssues = crossValidate(tokenGrammar, parserGrammar);
  const crossErrors = countErrors(crossIssues);
  const crossWarnings = crossIssues.length - crossErrors;

  if (crossErrors > 0) {
    process.stdout.write(`${crossErrors} error(s)\n`);
    printIssues(crossIssues);
    totalIssues += crossErrors;
  } else if (crossWarnings > 0) {
    process.stdout.write(`OK (${crossWarnings} warning(s))\n`);
    printIssues(crossIssues);
  } else {
    process.stdout.write("OK\n");
  }

  process.stdout.write("\n");
  if (totalIssues > 0) {
    process.stdout.write(`Found ${totalIssues} error(s). Fix them and try again.\n`);
    return 1;
  }
  process.stdout.write("All checks passed.\n");
  return 0;
}

// ---------------------------------------------------------------------------
// validate-tokens
// ---------------------------------------------------------------------------

export function validateTokensOnly(tokensPath: string): number {
  if (!existsSync(tokensPath)) {
    process.stderr.write(`Error: File not found: ${tokensPath}\n`);
    return 1;
  }

  process.stdout.write(`Validating ${basename(tokensPath)} ... `);
  let tokenGrammar;
  try {
    tokenGrammar = parseTokenGrammar(readFileSync(tokensPath, "utf-8"));
  } catch (e) {
    if (e instanceof TokenGrammarError) {
      process.stdout.write("PARSE ERROR\n");
      process.stdout.write(`  ${e.message}\n`);
      return 1;
    }
    throw e;
  }

  const issues = validateTokenGrammar(tokenGrammar);
  const nTokens = tokenGrammar.definitions.length;
  const nSkip = tokenGrammar.skipDefinitions?.length ?? 0;
  const errors = countErrors(issues);

  if (errors > 0) {
    process.stdout.write(`${errors} error(s)\n`);
    printIssues(issues);
    process.stdout.write("\n");
    process.stdout.write(`Found ${errors} error(s). Fix them and try again.\n`);
    return 1;
  }

  const parts = [`${nTokens} tokens`];
  if (nSkip > 0) parts.push(`${nSkip} skip`);
  process.stdout.write(`OK (${parts.join(", ")})\n`);
  process.stdout.write("\n");
  process.stdout.write("All checks passed.\n");
  return 0;
}

// ---------------------------------------------------------------------------
// validate-grammar
// ---------------------------------------------------------------------------

export function validateGrammarOnly(grammarPath: string): number {
  if (!existsSync(grammarPath)) {
    process.stderr.write(`Error: File not found: ${grammarPath}\n`);
    return 1;
  }

  process.stdout.write(`Validating ${basename(grammarPath)} ... `);
  let parserGrammar;
  try {
    parserGrammar = parseParserGrammar(readFileSync(grammarPath, "utf-8"));
  } catch (e) {
    if (e instanceof ParserGrammarError) {
      process.stdout.write("PARSE ERROR\n");
      process.stdout.write(`  ${e.message}\n`);
      return 1;
    }
    throw e;
  }

  // No tokenNames — only rule-level checks.
  const issues = validateParserGrammar(parserGrammar, undefined);
  const nRules = parserGrammar.rules.length;
  const errors = countErrors(issues);

  if (errors > 0) {
    process.stdout.write(`${errors} error(s)\n`);
    printIssues(issues);
    process.stdout.write("\n");
    process.stdout.write(`Found ${errors} error(s). Fix them and try again.\n`);
    return 1;
  }

  process.stdout.write(`OK (${nRules} rules)\n`);
  process.stdout.write("\n");
  process.stdout.write("All checks passed.\n");
  return 0;
}

// ---------------------------------------------------------------------------
// dispatch
// ---------------------------------------------------------------------------

export function dispatch(command: string, files: string[]): number {
  switch (command) {
    case "validate":
      if (files.length !== 2) {
        process.stderr.write("Error: 'validate' requires two arguments: <tokens> <grammar>\n");
        process.stderr.write("\n");
        printUsage();
        return 2;
      }
      return validateCommand(files[0], files[1]);

    case "validate-tokens":
      if (files.length !== 1) {
        process.stderr.write("Error: 'validate-tokens' requires one argument: <tokens>\n");
        process.stderr.write("\n");
        printUsage();
        return 2;
      }
      return validateTokensOnly(files[0]);

    case "validate-grammar":
      if (files.length !== 1) {
        process.stderr.write("Error: 'validate-grammar' requires one argument: <grammar>\n");
        process.stderr.write("\n");
        printUsage();
        return 2;
      }
      return validateGrammarOnly(files[0]);

    default:
      process.stderr.write(`Error: Unknown command '${command}'\n`);
      process.stderr.write("\n");
      printUsage();
      return 2;
  }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

// Only run the CLI when this file is the entry point, not when imported as a
// module by tests. In ESM, `import.meta.url` matches `process.argv[1]` (after
// resolving the file URL) when executed directly via Node.
if (
  process.argv[1] !== undefined &&
  resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url))
) {
  const root = findRoot();
  const specPath = join(root, "code", "specs", "grammar-tools.json");

  const parser = new Parser(specPath, process.argv);

  try {
    const result = parser.parse();

    // HelpResult: has `text` but no `flags`
    if ("text" in result && !("flags" in result)) {
      process.stdout.write((result as { text: string }).text + "\n");
      process.exit(0);
    }

    // VersionResult: has `version` but no `flags`
    if ("version" in result && !("flags" in result)) {
      process.stdout.write((result as { version: string }).version + "\n");
      process.exit(0);
    }

    // ParseResult
    const r = result as { flags: Record<string, unknown>; arguments: Record<string, unknown> };
    const command = String(r.arguments["command"] ?? "");
    const rawFiles = r.arguments["files"];
    let files: string[] = [];
    if (Array.isArray(rawFiles)) {
      files = rawFiles.map(String);
    } else if (typeof rawFiles === "string") {
      files = [rawFiles];
    }

    process.exit(dispatch(command, files));
  } catch (e: unknown) {
    const message = e instanceof Error ? e.message : String(e);
    process.stderr.write(`error: ${message}\n`);
    process.exit(2);
  }
}
