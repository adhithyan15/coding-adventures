/**
 * ECMAScript 3 (1999) Parser — parses ES3 source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `es3.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * ES3 adds structured error handling and strict equality to the ES1 grammar:
 * - try/catch/finally statements
 * - throw statement
 * - === and !== in equality_expression
 * - `instanceof` in relational_expression
 * - REGEX as a primary expression
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `es3.grammar` file lives in `code/grammars/ecmascript/` at the repository root.
 *
 *     src/ -> ecmascript-es3-parser/ -> typescript/ -> packages/ -> code/ -> grammars/ecmascript/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeEs3 } from "@coding-adventures/ecmascript-es3-lexer";

/**
 * Resolve __dirname for ES modules.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Path to the ECMAScript grammars directory.
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars", "ecmascript");
const ES3_GRAMMAR_PATH = join(GRAMMARS_DIR, "es3.grammar");

/**
 * Parse ECMAScript 3 source code and return an AST.
 *
 * @param source - The ECMAScript 3 source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     const ast = parseEs3("try { x; } catch (e) { y; }");
 *     console.log(ast.ruleName); // "program"
 */
export function parseEs3(source: string): ASTNode {
  const tokens = tokenizeEs3(source);
  const grammarText = readFileSync(ES3_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
