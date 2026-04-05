/**
 * JavaScript Parser — parses JavaScript source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads a JavaScript `.grammar` file and
 * delegates all parsing work to the generic engine.
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
 * | Version string  | Lexer tokens file                            |
 * |-----------------|----------------------------------------------|
 * | `"es1"`         | `grammars/ecmascript/es1.tokens`             |
 * | `"es3"`         | `grammars/ecmascript/es3.tokens`             |
 * | `"es5"`         | `grammars/ecmascript/es5.tokens`             |
 * | `"es2015"`…     | `grammars/ecmascript/es2015.tokens` …        |
 * | `"es2025"`      | `grammars/ecmascript/es2025.tokens`          |
 * | `undefined`/`""`| `grammars/javascript.tokens` (generic)       |
 *
 * The parser grammar is always the generic `javascript.grammar`, which uses
 * simple rules (`var_declaration`, `expression`, etc.) regardless of ECMAScript
 * version. The version parameter only selects the lexer's token set — different
 * ECMAScript editions have different keyword sets (e.g. es2015 adds `let`,
 * `const`, `class`), but the parser AST shape remains stable across versions.
 *
 * When no version is supplied the generic grammar is used, which is backwards-
 * compatible with v0.1.x.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `javascript.grammar` file lives in `code/grammars/` at the repository root.
 * Versioned grammars live in `code/grammars/ecmascript/`.
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

/**
 * Root of the grammars directory.
 * Walk up: src/ -> javascript-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * The generic JavaScript parser grammar path.
 *
 * The parser always uses this single grammar file. Version strings only
 * affect which *lexer* token set is loaded (via `tokenizeJavascript`), keeping
 * the AST shape consistent across ECMAScript editions.
 */
const JS_GRAMMAR_PATH = join(GRAMMARS_DIR, "javascript.grammar");

/**
 * Parse JavaScript source code and return an AST.
 *
 * @param source  - The JavaScript source code to parse.
 * @param version - Optional ECMAScript edition string (e.g. `"es2015"`, `"es5"`).
 *   When omitted (or the empty string) the generic token set is used — backwards-
 *   compatible with v0.1.x. The version affects which *lexer* grammar is loaded;
 *   the parser grammar is always the generic `javascript.grammar`.
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
  const grammarText = readFileSync(JS_GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
