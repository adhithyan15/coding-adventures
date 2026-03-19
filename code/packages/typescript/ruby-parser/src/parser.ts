/**
 * Ruby Parser — parses Ruby source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `ruby.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * The Ruby grammar supports method calls (like `puts("hello")`) in addition
 * to the standard assignment and expression patterns shared with Python.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `ruby.grammar` file lives in `code/grammars/` at the repository root.
 *
 *     src/ -> ruby-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeRuby } from "@coding-adventures/ruby-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const RUBY_GRAMMAR_PATH = join(GRAMMARS_DIR, "ruby.grammar");

/**
 * Parse Ruby source code and return an AST.
 *
 * @param source - The Ruby source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     const ast = parseRuby("x = 1 + 2");
 *     console.log(ast.ruleName); // "program"
 */
export function parseRuby(source: string): ASTNode {
  const tokens = tokenizeRuby(source);
  const grammarText = readFileSync(RUBY_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
