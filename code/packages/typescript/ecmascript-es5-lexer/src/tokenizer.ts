/**
 * ECMAScript 5 (2009) Lexer — tokenizes ES5 source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `es5.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * ECMAScript 5 — The Decade Gap
 * ------------------------------
 *
 * ES5 landed in December 2009, a full decade after ES3 (ES4 was abandoned).
 * The syntactic changes are modest — the real innovations were strict mode
 * semantics, native JSON support, and property descriptors.
 *
 * What ES5 adds over ES3:
 *   - `debugger` keyword (moved from future-reserved to keyword)
 *   - Getter/setter syntax in object literals
 *   - String line continuation (backslash before newline)
 *   - Trailing commas in object literals
 *
 * What ES5 does NOT have:
 *   - No let/const (added in ES2015)
 *   - No class syntax (added in ES2015)
 *   - No arrow functions (added in ES2015)
 *   - No template literals (added in ES2015)
 *   - No modules (added in ES2015)
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `es5.tokens` file lives in `code/grammars/ecmascript/` at the repository root.
 *
 *     src/ -> ecmascript-es5-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/ecmascript/
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
const ES5_TOKENS_PATH = join(GRAMMARS_DIR, "es5.tokens");

/**
 * Tokenize ECMAScript 5 source code and return an array of tokens.
 *
 * The function reads the `es5.tokens` grammar file, parses it into a
 * token grammar structure, then runs the generic `grammarTokenize` engine
 * over the source code.
 *
 * @param source - The ECMAScript 5 source code to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeEs5('debugger;');
 *     // [Token(KEYWORD, "debugger"), Token(SEMICOLON, ";"), Token(EOF, "")]
 */
export function tokenizeEs5(source: string): Token[] {
  const grammarText = readFileSync(ES5_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}
