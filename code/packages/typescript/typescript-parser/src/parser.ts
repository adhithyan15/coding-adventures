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
 * | Version string  | Lexer tokens file                                  |
 * |-----------------|----------------------------------------------------|
 * | `"ts1.0"`       | `src/tokens/typescript/ts1.0.tokens`               |
 * | `"ts2.0"`       | `src/tokens/typescript/ts2.0.tokens`               |
 * | `"ts3.0"`       | `src/tokens/typescript/ts3.0.tokens`               |
 * | `"ts4.0"`       | `src/tokens/typescript/ts4.0.tokens`               |
 * | `"ts5.0"`       | `src/tokens/typescript/ts5.0.tokens`               |
 * | `"ts5.8"`       | `src/tokens/typescript/ts5.8.tokens`               |
 * | `undefined`/`""`| `src/tokens/typescript.tokens` (generic)           |
 *
 * The parser grammar is always the generic `typescript.grammar`, which uses
 * simple rules (`var_declaration`, `expression`, etc.) regardless of TypeScript
 * version. The version parameter only selects the lexer's token set — different
 * TypeScript versions have different keyword sets, but the parser AST shape
 * remains stable across versions.
 *
 * When no version is supplied the generic grammar is used, which is backwards-
 * compatible with v0.1.x.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `typescript.grammar` file lives in `code/grammars/` at the repository root.
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
 * The generic TypeScript parser grammar path.
 *
 * The parser always uses this single grammar file. Version strings only
 * affect which *lexer* token set is loaded (via `tokenizeTypescript`), keeping
 * the AST shape consistent across TypeScript versions.
 */
const TS_GRAMMAR_PATH = join(GRAMMARS_DIR, "typescript.grammar");

/**
 * Parse TypeScript source code and return an AST.
 *
 * @param source  - The TypeScript source code to parse.
 * @param version - Optional TypeScript version (e.g. `"ts5.8"`). When omitted
 *   (or the empty string) the generic token set is used — backwards-compatible
 *   with v0.1.x. The version affects which *lexer* grammar is loaded; the parser
 *   grammar is always the generic `typescript.grammar`.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     // Generic (backwards-compatible)
 *     const ast = parseTypescript("let x = 1 + 2;");
 *
 *     // Version-specific lexer, generic parser rules
 *     const ast = parseTypescript("let x: number = 1;", "ts5.8");
 *     console.log(ast.ruleName); // "program"
 */
export function parseTypescript(source: string, version?: string): ASTNode {
  const tokens = tokenizeTypescript(source, version);
  const grammarText = readFileSync(TS_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
