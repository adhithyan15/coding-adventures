/**
 * JavaScript Parser — parses JavaScript source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `javascript.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * The JavaScript grammar differs from Python and Ruby grammars in several ways:
 * - Variable declarations use `let`, `const`, or `var` keywords
 * - Statements end with semicolons (not newlines)
 * - The grammar includes a `var_declaration` rule for `KEYWORD NAME EQUALS expression SEMICOLON`
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `javascript.grammar` file lives in `code/grammars/` at the repository root.
 *
 *     src/ -> javascript-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeJavascript } from "@coding-adventures/javascript-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const JS_GRAMMAR_PATH = join(GRAMMARS_DIR, "javascript.grammar");

/**
 * Parse JavaScript source code and return an AST.
 *
 * @param source - The JavaScript source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     const ast = parseJavascript("let x = 1 + 2;");
 *     console.log(ast.ruleName); // "program"
 */
export function parseJavascript(source: string): ASTNode {
  const tokens = tokenizeJavascript(source);
  const grammarText = readFileSync(JS_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
