/**
 * ECMAScript 1 (1997) Lexer — tokenizes ES1 source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `es1.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * ECMAScript 1 — The Beginning
 * ----------------------------
 *
 * ES1 was published in June 1997, two years after Brendan Eich created
 * JavaScript for Netscape Navigator. It standardized the core language:
 *
 * - 26 keywords (break, case, continue, default, delete, do, else, for,
 *   function, if, in, new, return, switch, this, typeof, var, void, while, with)
 * - Basic operators: arithmetic, bitwise, logical, comparison, assignment
 * - String literals (single and double quoted)
 * - Numeric literals (decimal integers, floats, hex with 0x prefix)
 * - The $ character is valid in identifiers
 *
 * What ES1 does NOT have:
 *   - No === or !== (strict equality — added in ES3)
 *   - No try/catch/finally/throw (error handling — added in ES3)
 *   - No regex literals (implementation-defined in ES1 — formalized in ES3)
 *   - No let/const (added in ES2015)
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `es1.tokens` file lives in `code/grammars/ecmascript/` at the repository root.
 *
 *     src/ -> ecmascript-es1-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/ecmascript/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

/**
 * Resolve __dirname for ES modules — `import.meta.url` gives us the file URL,
 * then we convert to a filesystem path and extract the directory.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Path to the ECMAScript grammars directory. We walk up from:
 *   src/ -> ecmascript-es1-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/ecmascript/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars", "ecmascript");
const ES1_TOKENS_PATH = join(GRAMMARS_DIR, "es1.tokens");

/**
 * Tokenize ECMAScript 1 source code and return an array of tokens.
 *
 * The function reads the `es1.tokens` grammar file, parses it into a
 * token grammar structure, then runs the generic `grammarTokenize` engine
 * over the source code.
 *
 * @param source - The ECMAScript 1 source code to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeEs1("var x = 1 + 2;");
 *     // [Token(KEYWORD, "var"), Token(NAME, "x"), Token(EQUALS, "="),
 *     //  Token(NUMBER, "1"), Token(PLUS, "+"), Token(NUMBER, "2"),
 *     //  Token(SEMICOLON, ";"), Token(EOF, "")]
 */
export function tokenizeEs1(source: string): Token[] {
  const grammarText = readFileSync(ES1_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}
