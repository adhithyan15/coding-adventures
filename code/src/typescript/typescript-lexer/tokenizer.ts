/**
 * TypeScript Lexer — tokenizes TypeScript source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the shared `lexer` package source. It loads the `typescript.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * TypeScript extends JavaScript with additional features:
 * - `interface`, `type`, `enum`, `namespace`, `declare` keywords
 * - Type annotations like `: number`, `: string`, `: boolean`
 * - Generic syntax with `<` and `>`
 * - `readonly`, `abstract`, `implements`, `extends` keywords
 * - All JavaScript features are also supported
 *
 * All of these are handled by the grammar file — no TypeScript-specific
 * tokenization code exists in this module.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `typescript.tokens` file lives in `code/src/tokens/` in the shared source tree.
 *
 *     src/ -> typescript-lexer/ -> typescript/ -> src/ -> code/ -> src/tokens/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "../grammar-tools/index.js";
import { grammarTokenize } from "../lexer/index.js";
import type { Token } from "../lexer/index.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const TOKENS_DIR = join(__dirname, "..", "..", "tokens");
const TS_TOKENS_PATH = join(TOKENS_DIR, "typescript.tokens");

/**
 * Tokenize TypeScript source code and return an array of tokens.
 *
 * @param source - The TypeScript source code to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeTypescript("let x: number = 1 + 2;");
 *     // [Token(KEYWORD, "let"), Token(NAME, "x"), Token(COLON, ":"),
 *     //  Token(KEYWORD, "number"), Token(EQUALS, "="), Token(NUMBER, "1"),
 *     //  Token(PLUS, "+"), Token(NUMBER, "2"), Token(SEMICOLON, ";"),
 *     //  Token(EOF, "")]
 */
export function tokenizeTypescript(source: string): Token[] {
  const grammarText = readFileSync(TS_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}
