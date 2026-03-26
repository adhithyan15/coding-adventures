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
} from "../../../src/typescript/grammar-tools/token-grammar.js";
import {
  ParserGrammarError,
  parseParserGrammar,
  validateParserGrammar,
} from "../../../src/typescript/grammar-tools/parser-grammar.js";
import { crossValidate } from "../../../src/typescript/grammar-tools/cross-validator.js";
import { compileTokensToTypeScript, compileParserToTypeScript } from "../../../src/typescript/grammar-tools/compiler.js";

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

import { resolve, join, extname, dirname } from "path";
import { readdirSync, statSync, writeFileSync } from "fs";
import {
  Parser,
  ParseErrors,
  HelpResult,
  VersionResult,
  ParseResult,
} from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

export function main(): number {
  const specPath = resolve(join(__dirname, "../../../specs/grammar-tools.cli.json"));
  let parser: Parser;
  try {
    parser = new Parser(specPath, process.argv);
  } catch (e) {
    process.stdout.write(`Spec Error: ${e instanceof Error ? e.message : String(e)}\n`);
    return 1;
  }

  let result: ParseResult | HelpResult | VersionResult;
  try {
    result = parser.parse();
  } catch (e) {
    if (e instanceof ParseErrors) {
      for (const err of e.errors) {
        process.stdout.write(`Error: ${err.message}\n`);
        if (err.suggestion) {
          process.stdout.write(`  ${err.suggestion}\n`);
        }
      }
    } else {
      process.stdout.write(`Error: ${e instanceof Error ? e.message : String(e)}\n`);
    }
    return 2;
  }

  if ("text" in result) {
    process.stdout.write(result.text + "\n");
    return 0;
  }
  if ("version" in result && !("flags" in result)) {
    process.stdout.write(result.version + "\n");
    return 0;
  }

  const parseResult = result as ParseResult;
  const cmd = parseResult.commandPath[parseResult.commandPath.length - 1];

  switch (cmd) {
    case "validate":
      return validateCommand(
        parseResult.arguments["tokens_file"] as string,
        parseResult.arguments["grammar_file"] as string
      );
    case "validate-tokens":
      return validateTokensOnly(parseResult.arguments["tokens_file"] as string);
    case "validate-grammar":
      return validateGrammarOnly(parseResult.arguments["grammar_file"] as string);
    case "compile-tokens":
      return compileTokensOnly(
        parseResult.arguments["tokens_file"] as string,
        parseResult.arguments["export_name"] as string
      );
    case "compile-grammar":
      return compileGrammarOnly(
        parseResult.arguments["grammar_file"] as string,
        parseResult.arguments["export_name"] as string
      );
    case "generate":
      return generateCommand();
    default:
      process.stdout.write(`Error: Unknown command '${cmd}'\n`);
      return 2;
  }
}

function compileTokensOnly(tokensPath: string, exportName: string): number {
  const tokensText = readFile(tokensPath);
  if (tokensText === null) return 1;
  let tg;
  try {
    tg = parseTokenGrammar(tokensText);
    const issues = validateTokenGrammar(tg);
    if (countErrors(issues) > 0) {
      process.stdout.write("Error: Cannot compile invalid grammar file.\n");
      printIssues(issues);
      return 1;
    }
    process.stdout.write(compileTokensToTypeScript(tg, exportName));
    return 0;
  } catch (e) {
    process.stdout.write(`PARSE ERROR\n  ${e instanceof Error ? e.message : String(e)}\n`);
    return 1;
  }
}

function compileGrammarOnly(grammarPath: string, exportName: string): number {
  const grammarText = readFile(grammarPath);
  if (grammarText === null) return 1;
  let pg;
  try {
    pg = parseParserGrammar(grammarText);
    const issues = validateParserGrammar(pg, null);
    if (countErrors(issues) > 0) {
      process.stdout.write("Error: Cannot compile invalid grammar file.\n");
      printIssues(issues);
      return 1;
    }
    process.stdout.write(compileParserToTypeScript(pg, exportName));
    return 0;
  } catch (e) {
    process.stdout.write(`PARSE ERROR\n  ${e instanceof Error ? e.message : String(e)}\n`);
    return 1;
  }
}

function toCamelCase(snakeStr: string): string {
  const components = snakeStr.replace(/-/g, "_").split("_");
  return components
    .map((c) => (c.length > 0 ? c.charAt(0).toUpperCase() + c.slice(1) : ""))
    .join("");
}

function findMonorepoRoot(): string | null {
  let currentDir = process.cwd();
  while (true) {
    try {
      if (statSync(join(currentDir, "code", "grammars")).isDirectory()) {
        return currentDir;
      }
    } catch {
      // Ignore
    }
    const parent = dirname(currentDir);
    if (parent === currentDir) break;
    currentDir = parent;
  }
  return null;
}

export function generateCommand(): number {
  let hasErrors = false;
  const monorepoRoot = findMonorepoRoot();
  if (!monorepoRoot) {
    process.stdout.write("Error: could not find monorepo root\n");
    return 1;
  }

  const grammarsDir = join(monorepoRoot, "code", "grammars");
  const langDir = join(monorepoRoot, "code", "packages", "typescript");

  let files: string[];
  try {
    files = readdirSync(grammarsDir);
  } catch (e) {
    process.stdout.write(`Error reading grammars dir: ${e instanceof Error ? e.message : String(e)}\n`);
    return 1;
  }

  for (const file of files) {
    const filePath = join(grammarsDir, file);
    if (!statSync(filePath).isFile()) continue;

    const ext = extname(file);
    if (ext !== ".tokens" && ext !== ".grammar") continue;

    const isTokens = ext === ".tokens";
    const kind = isTokens ? "lexer" : "parser";
    const gn = basename(file, ext);

    const possibleDirs = [
      join(langDir, `${gn}-${kind}`),
      join(langDir, `${gn}_${kind}`),
    ];

    let targetDir: string | null = null;
    for (const pd of possibleDirs) {
      try {
        if (statSync(pd).isDirectory()) {
          targetDir = pd;
          break;
        }
      } catch {
        continue;
      }
    }

    if (!targetDir) continue;

    process.stdout.write(`Generating for ${file} ...\n`);

    const varSuffix = isTokens ? "Tokens" : "Grammar";
    const exportName = toCamelCase(gn) + varSuffix;
    const fnameBase = isTokens ? `${gn}-tokens.ts` : `${gn}-grammar.ts`;
    const outPath = join(targetDir, "src", fnameBase);

    const source = readFile(filePath);
    if (source === null) {
      hasErrors = true;
      continue;
    }

    let code = "";
    if (isTokens) {
      try {
        const tg = parseTokenGrammar(source);
        const issues = validateTokenGrammar(tg);
        if (countErrors(issues) > 0) {
          process.stdout.write(`Error: Cannot compile invalid grammar file ${file}\n`);
          printIssues(issues);
          hasErrors = true;
          continue;
        }
        code = compileTokensToTypeScript(tg, exportName);
      } catch (e) {
        process.stdout.write(`Error: parse failed for ${file}: ${e instanceof Error ? e.message : String(e)}\n`);
        hasErrors = true;
        continue;
      }
    } else {
      try {
        const pg = parseParserGrammar(source);
        const issues = validateParserGrammar(pg, null);
        if (countErrors(issues) > 0) {
          process.stdout.write(`Error: Cannot compile invalid grammar file ${file}\n`);
          printIssues(issues);
          hasErrors = true;
          continue;
        }
        code = compileParserToTypeScript(pg, exportName);
      } catch (e) {
        process.stdout.write(`Error: parse failed for ${file}: ${e instanceof Error ? e.message : String(e)}\n`);
        hasErrors = true;
        continue;
      }
    }

    try {
      writeFileSync(outPath, code);
      process.stdout.write(`  -> Saved ${outPath}\n`);
    } catch (e) {
      process.stdout.write(`Error writing ${outPath}: ${e instanceof Error ? e.message : String(e)}\n`);
      hasErrors = true;
    }
  }

  return hasErrors ? 1 : 0;
}

if (typeof require !== "undefined" && require.main === module) {
  process.exit(main());
}
