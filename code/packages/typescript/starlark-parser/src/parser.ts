/**
 * Starlark Parser — parses Starlark source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `starlark.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * How It Works
 * ------------
 *
 * The parsing pipeline has two stages:
 *
 *   1. **Lexing** — The starlark-lexer reads the source code and produces a flat
 *      array of tokens. Each token has a type (NAME, INT, KEYWORD, INDENT, etc.)
 *      and a value (the actual text). The lexer also handles indentation tracking,
 *      producing synthetic INDENT/DEDENT tokens for block structure.
 *
 *   2. **Parsing** — The GrammarParser reads the starlark.grammar file, which
 *      defines the syntax rules (file, statement, expression, etc.) using
 *      EBNF-like notation. It then applies recursive descent with backtracking
 *      to match the token stream against these rules, producing an AST.
 *
 * The AST Structure
 * -----------------
 *
 * The resulting AST is a tree of `ASTNode` objects. Each node has:
 *   - `ruleName` — the grammar rule that produced this node (e.g., "file",
 *     "assign_stmt", "expression")
 *   - `children` — an array of child nodes or leaf tokens
 *
 * For example, parsing `x = 1` produces roughly:
 *
 *     ASTNode("file", [
 *       ASTNode("statement", [
 *         ASTNode("simple_stmt", [
 *           ASTNode("assign_stmt", [
 *             ASTNode("expression_list", [ASTNode("expression", [...NAME("x")...])]),
 *             Token(EQUALS, "="),
 *             ASTNode("expression_list", [ASTNode("expression", [...INT("1")...])])
 *           ]),
 *           Token(NEWLINE, "")
 *         ])
 *       ])
 *     ])
 *
 * Starlark Grammar Highlights
 * ---------------------------
 *
 * The Starlark grammar (starlark.grammar) supports:
 *   - **Assignments** — simple (x = 1) and augmented (x += 1)
 *   - **Function definitions** — def f(a, b=1, *args, **kwargs): ...
 *   - **If/elif/else** — conditional blocks with indentation
 *   - **For loops** — iteration over collections (no while loops!)
 *   - **Load statements** — load("//path", "symbol") for imports
 *   - **Expressions** — full operator precedence from lambda to primary
 *   - **Comprehensions** — [x for x in lst if cond]
 *   - **Function calls** — f(arg, key=val, *args, **kwargs)
 *
 * The top-level rule is `file` (not `program` as in the Ruby grammar),
 * reflecting that Starlark files are configuration files, not programs.
 *
 * Grammar Source
 * --------------
 *
 * The parser grammar is compiled ahead of time from `starlark.grammar` (in
 * `code/grammars/starlark/`) into `./_grammar.ts`, a native TypeScript
 * object literal. This avoids reading and parsing a grammar file from disk
 * at runtime -- which would break once this package is published, since a
 * published npm package never ships the monorepo's `code/grammars/` tree.
 */

import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeStarlark } from "@coding-adventures/starlark-lexer";

import { PARSER_GRAMMAR } from "./_grammar.js";

/**
 * Parse Starlark source code and return an AST.
 *
 * This function orchestrates the full parsing pipeline:
 *   1. Tokenize the source using the starlark-lexer
 *   2. Read and parse the starlark.grammar file
 *   3. Run the grammar-driven parser to produce an AST
 *
 * @param source - The Starlark source code to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"file"`.
 *
 * @example
 *     const ast = parseStarlark("x = 1 + 2");
 *     console.log(ast.ruleName); // "file"
 *
 * @example
 *     // Parse a BUILD-file style function call:
 *     const ast = parseStarlark('cc_library(name = "foo", srcs = ["foo.cc"])');
 *
 * @example
 *     // Parse a function definition:
 *     const ast = parseStarlark("def greet(name):\n    return 'Hello, ' + name");
 */
export function parseStarlark(source: string): ASTNode {
  /**
   * Step 1: Tokenize.
   * The starlark-lexer handles indentation tracking, keyword recognition,
   * reserved word detection, and all the lexical details.
   */
  const tokens = tokenizeStarlark(source);

  /**
   * Step 2: Parse.
   * The GrammarParser takes the token array and the pre-compiled
   * PARSER_GRAMMAR constant, then performs recursive descent with
   * backtracking to produce an AST. The starting rule is determined by
   * the grammar (usually the first rule defined, which for
   * starlark.grammar is "file").
   */
  const parser = new GrammarParser(tokens, PARSER_GRAMMAR);
  return parser.parse();
}
