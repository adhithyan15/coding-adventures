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
 * | Version string  | Grammar files                                       |
 * |-----------------|-----------------------------------------------------|
 * | `"es1"`         | `grammars/ecmascript/es1.{tokens,grammar}`          |
 * | `"es3"`         | `grammars/ecmascript/es3.{tokens,grammar}`          |
 * | `"es5"`         | `grammars/ecmascript/es5.{tokens,grammar}`          |
 * | `"es2015"`…     | `grammars/ecmascript/es2015.{tokens,grammar}` …     |
 * | `"es2025"`      | `grammars/ecmascript/es2025.{tokens,grammar}`       |
 * | `undefined`/`""`| `grammars/javascript.{tokens,grammar}` (generic)    |
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
 * Valid ECMAScript version strings — mirrors the set accepted by the lexer.
 */
const VALID_ES_VERSIONS = new Set([
  "es1",
  "es3",
  "es5",
  "es2015",
  "es2016",
  "es2017",
  "es2018",
  "es2019",
  "es2020",
  "es2021",
  "es2022",
  "es2023",
  "es2024",
  "es2025",
]);

/**
 * Resolve the path to the JavaScript parser grammar for the given version.
 *
 * @param version - Optional ECMAScript version string.
 * @returns Absolute path to the `.grammar` file.
 */
function resolveGrammarPath(version?: string): string {
  if (!version) {
    return join(GRAMMARS_DIR, "javascript.grammar");
  }

  if (!VALID_ES_VERSIONS.has(version)) {
    throw new Error(
      `Unknown JavaScript/ECMAScript version "${version}". ` +
        `Valid values: ${[...VALID_ES_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "ecmascript", `${version}.grammar`);
}

/**
 * Parse JavaScript source code and return an AST.
 *
 * @param source  - The JavaScript source code to parse.
 * @param version - Optional ECMAScript edition string (e.g. `"es2015"`, `"es5"`).
 *   When omitted (or the empty string) the generic grammars are used — backwards-
 *   compatible with v0.1.x.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     // Generic (backwards-compatible)
 *     const ast = parseJavascript("let x = 1 + 2;");
 *
 *     // Version-specific
 *     const ast = parseJavascript("var x = 1 + 2;", "es5");
 *     console.log(ast.ruleName); // "program"
 */
export function parseJavascript(source: string, version?: string): ASTNode {
  const tokens = tokenizeJavascript(source, version);
  const grammarText = readFileSync(resolveGrammarPath(version), "utf-8");
  const grammar = parseParserGrammar(grammarText);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
