/**
 * TypeScript Parser — parses TypeScript source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads a TypeScript `.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * The TypeScript grammar extends the JavaScript grammar with:
 * - Type annotations (`: number`, `: string`, `: boolean`)
 * - Interface and type alias declarations
 * - Generic syntax
 * - All JavaScript grammar rules carry over (var_declaration, assignment, etc.)
 *
 * Version Support
 * ---------------
 *
 * This parser accepts the same version strings as `@coding-adventures/typescript-lexer`:
 *
 * | Version string  | Grammar files                                      |
 * |-----------------|----------------------------------------------------|
 * | `"ts1.0"`       | `grammars/typescript/ts1.0.{tokens,grammar}`       |
 * | `"ts2.0"`       | `grammars/typescript/ts2.0.{tokens,grammar}`       |
 * | `"ts3.0"`       | `grammars/typescript/ts3.0.{tokens,grammar}`       |
 * | `"ts4.0"`       | `grammars/typescript/ts4.0.{tokens,grammar}`       |
 * | `"ts5.0"`       | `grammars/typescript/ts5.0.{tokens,grammar}`       |
 * | `"ts5.8"`       | `grammars/typescript/ts5.8.{tokens,grammar}`       |
 * | `undefined`/`""`| `grammars/typescript.{tokens,grammar}` (generic)   |
 *
 * When no version is supplied the generic grammar is used, which is backwards-
 * compatible with v0.1.x.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `typescript.grammar` file lives in `code/grammars/` at the repository root.
 * Versioned grammars live in `code/grammars/typescript/`.
 *
 *     src/ -> typescript-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeTypescript } from "@coding-adventures/typescript-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Root of the grammars directory.
 * Walk up: src/ -> typescript-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid TypeScript version strings — mirrors the set accepted by the lexer.
 */
const VALID_TS_VERSIONS = new Set([
  "ts1.0",
  "ts2.0",
  "ts3.0",
  "ts4.0",
  "ts5.0",
  "ts5.8",
]);

/**
 * Resolve the path to the TypeScript parser grammar for the given version.
 *
 * @param version - Optional TypeScript version string.
 * @returns Absolute path to the `.grammar` file.
 */
function resolveGrammarPath(version?: string): string {
  if (!version) {
    return join(GRAMMARS_DIR, "typescript.grammar");
  }

  if (!VALID_TS_VERSIONS.has(version)) {
    throw new Error(
      `Unknown TypeScript version "${version}". ` +
        `Valid values: ${[...VALID_TS_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "typescript", `${version}.grammar`);
}

/**
 * Parse TypeScript source code and return an AST.
 *
 * @param source  - The TypeScript source code to parse.
 * @param version - Optional TypeScript version (e.g. `"ts5.8"`). When omitted
 *   (or the empty string) the generic grammars are used — backwards-compatible
 *   with v0.1.x.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     // Generic (backwards-compatible)
 *     const ast = parseTypescript("let x = 1 + 2;");
 *
 *     // Version-specific
 *     const ast = parseTypescript("let x: number = 1;", "ts5.8");
 *     console.log(ast.ruleName); // "program"
 */
export function parseTypescript(source: string, version?: string): ASTNode {
  const tokens = tokenizeTypescript(source, version);
  const grammarText = readFileSync(resolveGrammarPath(version), "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
