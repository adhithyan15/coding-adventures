/**
 * ECMAScript 1 (1997) Parser — parses ES1 source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `es1.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * The ES1 grammar defines the syntactic structure of the first standardized
 * JavaScript. It covers:
 * - Variable declarations (`var` only — no `let` or `const`)
 * - Function declarations and expressions
 * - All 14 statement types (no try/catch — that is ES3)
 * - The full expression precedence chain from comma to primary
 * - Object and array literals
 *
 * The parser uses PEG (Parsing Expression Grammar) semantics with packrat
 * memoization. Operator precedence is encoded by rule nesting, from lowest
 * (comma) to highest (primary).
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `es1.grammar` file lives in `code/grammars/ecmascript/` at the repository root.
 *
 *     src/ -> ecmascript-es1-parser/ -> typescript/ -> packages/ -> code/ -> grammars/ecmascript/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeEs1 } from "@coding-adventures/ecmascript-es1-lexer";

/**
 * Resolve __dirname for ES modules.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Path to the ECMAScript grammars directory.
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars", "ecmascript");
const ES1_GRAMMAR_PATH = join(GRAMMARS_DIR, "es1.grammar");

/**
 * Parse ECMAScript 1 source code and return an AST.
 *
 * The function first tokenizes the source using the ES1 lexer, then reads
 * the `es1.grammar` file, parses it into a grammar structure, and runs
 * the generic `GrammarParser` engine over the token stream.
 *
 * @param source - The ECMAScript 1 source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     const ast = parseEs1("var x = 1 + 2;");
 *     console.log(ast.ruleName); // "program"
 */
export function parseEs1(source: string): ASTNode {
  const tokens = tokenizeEs1(source);
  const grammarText = readFileSync(ES1_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
