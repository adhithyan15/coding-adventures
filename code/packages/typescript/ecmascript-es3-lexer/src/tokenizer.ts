/**
 * ECMAScript 3 (1999) Lexer — tokenizes ES3 source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `es3.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * ECMAScript 3 — The Real Language
 * ---------------------------------
 *
 * ES3 was published in December 1999, two years after ES1. It transformed
 * JavaScript from a simple scripting language into a complete programming language
 * by adding critical features:
 *
 * - === and !== (strict equality — no type coercion)
 * - try/catch/finally/throw (structured error handling)
 * - Regular expression literals (/pattern/flags)
 * - `instanceof` operator
 * - Expanded future-reserved words
 *
 * What ES3 does NOT have:
 *   - No getters/setters in object literals (added in ES5)
 *   - No strict mode (added in ES5)
 *   - No let/const/class/arrow functions (added in ES2015)
 *
 * Regex vs Division Ambiguity
 * ---------------------------
 * The `/` character is ambiguous: it could start a regex literal or be the
 * division operator. The grammar file includes a REGEX token pattern, but
 * context-sensitive disambiguation is needed in production lexers.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `es3.tokens` file lives in `code/grammars/ecmascript/` at the repository root.
 *
 *     src/ -> ecmascript-es3-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/ecmascript/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

/**
 * Resolve __dirname for ES modules.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Path to the ECMAScript grammars directory.
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars", "ecmascript");
const ES3_TOKENS_PATH = join(GRAMMARS_DIR, "es3.tokens");

/**
 * Tokenize ECMAScript 3 source code and return an array of tokens.
 *
 * The function reads the `es3.tokens` grammar file, parses it into a
 * token grammar structure, then runs the generic `grammarTokenize` engine
 * over the source code.
 *
 * @param source - The ECMAScript 3 source code to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeEs3("var x = 1 === 2;");
 *     // [Token(KEYWORD, "var"), Token(NAME, "x"), Token(EQUALS, "="),
 *     //  Token(NUMBER, "1"), Token(STRICT_EQUALS, "==="),
 *     //  Token(NUMBER, "2"), Token(SEMICOLON, ";"), Token(EOF, "")]
 */
export function tokenizeEs3(source: string): Token[] {
  const grammarText = readFileSync(ES3_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}
