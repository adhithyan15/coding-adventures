/**
 * JavaScript Lexer — tokenizes JavaScript source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `javascript.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * JavaScript has features that Python and Ruby do not:
 * - `let`, `const`, `var` for variable declarations
 * - `===` and `!==` for strict equality
 * - Semicolons terminate statements
 * - Curly braces `{}` for blocks
 * - `null` and `undefined` (not `None` or `nil`)
 * - `$` is valid in identifiers
 * - `=>` for arrow functions
 *
 * All of these are handled by the grammar file — no JavaScript-specific
 * tokenization code exists in this module.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `javascript.tokens` file lives in `code/grammars/` at the repository root.
 *
 *     src/ -> javascript-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const JS_TOKENS_PATH = join(GRAMMARS_DIR, "javascript.tokens");

/**
 * Tokenize JavaScript source code and return an array of tokens.
 *
 * @param source - The JavaScript source code to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeJavascript("let x = 1 + 2;");
 *     // [Token(KEYWORD, "let"), Token(NAME, "x"), Token(EQUALS, "="),
 *     //  Token(NUMBER, "1"), Token(PLUS, "+"), Token(NUMBER, "2"),
 *     //  Token(SEMICOLON, ";"), Token(EOF, "")]
 */
export function tokenizeJavascript(source: string): Token[] {
  const grammarText = readFileSync(JS_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}
