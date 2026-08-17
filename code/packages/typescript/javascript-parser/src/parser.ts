/**
 * JavaScript Parser — parses JavaScript source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It selects a precompiled JavaScript
 * `ParserGrammar` and delegates all parsing work to the generic engine.
 *
 * The JavaScript grammar differs from Python and Ruby grammars in several ways:
 * - Variable declarations use `let`, `const`, or `var` keywords
 * - Statements end with semicolons (not newlines)
 * - The grammar includes a `var_declaration` rule for `KEYWORD NAME EQUALS expression SEMICOLON`
 *
 * Version Support
 * ---------------
 *
 * This parser accepts the same version strings as `@coding-adventures/javascript-lexer`:
 *
 * | Version string  | Lexer version used                     |
 * |-----------------|-----------------------------------------|
 * | `"es1"`         | `es1`                                    |
 * | `"es3"`         | `es3`                                    |
 * | `"es5"`         | `es5`                                    |
 * | `"es2015"`…     | `es2015` …                                |
 * | `"es2025"`      | `es2025`                                 |
 * | `undefined`/`""`| generic                                  |
 *
 * The parser grammar is always the generic compiled `ParserGrammar`, which uses
 * simple rules (`var_declaration`, `expression`, etc.) regardless of ECMAScript
 * version. The version parameter only selects the lexer's token set — different
 * ECMAScript editions have different keyword sets (e.g. es2015 adds `let`,
 * `const`, `class`), but the parser AST shape remains stable across versions.
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
import { tokenizeJavascript } from "@coding-adventures/javascript-lexer";

import { PARSER_GRAMMAR } from "./_grammar.js";

/**
 * The generic JavaScript parser grammar.
 *
 * The parser always uses this single compiled grammar. Version strings only
 * affect which *lexer* token set is loaded (via `tokenizeJavascript`), keeping
 * the AST shape consistent across ECMAScript editions.
 */
const JS_GRAMMAR = PARSER_GRAMMAR;

/**
 * Parse JavaScript source code and return an AST.
 *
 * @param source  - The JavaScript source code to parse.
 * @param version - Optional ECMAScript edition string (e.g. `"es2015"`, `"es5"`).
 *   When omitted (or the empty string) the generic token set is used — backwards-
 *   compatible with v0.1.x. The version affects which *lexer* grammar is loaded;
 *   the parser grammar is always the generic compiled `ParserGrammar`.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     // Generic (backwards-compatible)
 *     const ast = parseJavascript("let x = 1 + 2;");
 *
 *     // Version-specific lexer, generic parser rules
 *     const ast = parseJavascript("var x = 1 + 2;", "es5");
 *     console.log(ast.ruleName); // "program"
 */
export function parseJavascript(source: string, version?: string): ASTNode {
  const tokens = tokenizeJavascript(source, version);
  const parser = new GrammarParser(tokens, JS_GRAMMAR);
  return parser.parse();
}
