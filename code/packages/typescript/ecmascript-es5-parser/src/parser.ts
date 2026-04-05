/**
 * ECMAScript 5 (2009) Parser — parses ES5 source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `es5.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * ES5 adds getter/setter properties and the debugger statement to the ES3 grammar:
 * - debugger_statement: `debugger;`
 * - getter_property: `get name() { return value; }` in object literals
 * - setter_property: `set name(param) { this._x = param; }` in object literals
 *
 * Note: `get` and `set` are NOT keywords in ES5 — they are contextual. The lexer
 * emits them as NAME tokens and the grammar recognizes the pattern contextually.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `es5.grammar` file lives in `code/grammars/ecmascript/` at the repository root.
 *
 *     src/ -> ecmascript-es5-parser/ -> typescript/ -> packages/ -> code/ -> grammars/ecmascript/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeEs5 } from "@coding-adventures/ecmascript-es5-lexer";

/**
 * Resolve __dirname for ES modules.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Path to the ECMAScript grammars directory.
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars", "ecmascript");
const ES5_GRAMMAR_PATH = join(GRAMMARS_DIR, "es5.grammar");

/**
 * Parse ECMAScript 5 source code and return an AST.
 *
 * @param source - The ECMAScript 5 source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     const ast = parseEs5("debugger;");
 *     console.log(ast.ruleName); // "program"
 */
export function parseEs5(source: string): ASTNode {
  const tokens = tokenizeEs5(source);
  const grammarText = readFileSync(ES5_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
