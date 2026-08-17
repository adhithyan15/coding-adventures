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
 * Grammar Source
 * --------------
 *
 * The parser grammar is compiled ahead of time from `ruby.grammar` (in
 * `code/grammars/ruby/`) into `./_grammar.ts`, a native TypeScript object
 * literal. This avoids reading and parsing a grammar file from disk at
 * runtime -- which would break once this package is published, since a
 * published npm package never ships the monorepo's `code/grammars/` tree.
 */

import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeRuby } from "@coding-adventures/ruby-lexer";

import { PARSER_GRAMMAR } from "./_grammar.js";

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
  const parser = new GrammarParser(tokens, PARSER_GRAMMAR);
  return parser.parse();
}
