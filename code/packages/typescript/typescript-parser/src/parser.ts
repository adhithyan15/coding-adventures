/**
 * TypeScript Parser — parses TypeScript source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It selects a precompiled TypeScript
 * `ParserGrammar` and delegates all parsing work to the generic engine.
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
 * | Version string  | Lexer version used |
 * |------------------|---------------------|
 * | `"ts1.0"`        | `ts1.0`             |
 * | `"ts2.0"`        | `ts2.0`             |
 * | `"ts3.0"`        | `ts3.0`             |
 * | `"ts4.0"`        | `ts4.0`             |
 * | `"ts5.0"`        | `ts5.0`             |
 * | `"ts5.8"`        | `ts5.8`             |
 * | `undefined`/`""` | generic             |
 *
 * The parser grammar is always the generic compiled `ParserGrammar`, which uses
 * simple rules (`var_declaration`, `expression`, etc.) regardless of TypeScript
 * version. The version parameter only selects the lexer's token set — different
 * TypeScript versions have different keyword sets, but the parser AST shape
 * remains stable across versions.
 *
 * When no version is supplied the generic grammar is used, which is backwards-
 * compatible with v0.1.x.
 *
 * Locating the Grammar
 * ---------------------
 *
 * Grammars are no longer read from disk at runtime. Each `.grammar` source
 * file under `code/grammars/` is compiled ahead of time into a sibling
 * `_grammar*.ts` module (via `code/scripts/_ts_grammar_compile.ts`) that
 * embeds the `ParserGrammar` as a native TypeScript object literal. This
 * keeps the package self-contained — a published npm package never needs to
 * reach outside its own directory — and avoids repeated file I/O and
 * re-parsing on every call.
 */

import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeTypescript } from "@coding-adventures/typescript-lexer";

import { PARSER_GRAMMAR } from "./_grammar.js";

/**
 * The generic TypeScript parser grammar.
 *
 * The parser always uses this single compiled grammar. Version strings only
 * affect which *lexer* token set is loaded (via `tokenizeTypescript`), keeping
 * the AST shape consistent across TypeScript versions.
 */
const TS_GRAMMAR = PARSER_GRAMMAR;

/**
 * Parse TypeScript source code and return an AST.
 *
 * @param source  - The TypeScript source code to parse.
 * @param version - Optional TypeScript version (e.g. `"ts5.8"`). When omitted
 *   (or the empty string) the generic token set is used — backwards-compatible
 *   with v0.1.x. The version affects which *lexer* grammar is loaded; the parser
 *   grammar is always the generic compiled `ParserGrammar`.
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
  const parser = new GrammarParser(tokens, TS_GRAMMAR);
  return parser.parse();
}
