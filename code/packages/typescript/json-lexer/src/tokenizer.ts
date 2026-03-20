/**
 * JSON Lexer -- tokenizes JSON text using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `json.tokens` grammar
 * file and delegates all tokenization work to the generic engine.
 *
 * What Is JSON?
 * -------------
 *
 * JSON (JavaScript Object Notation) is the most widely used data interchange
 * format on the web. Defined by RFC 8259, it is a simple, text-based format
 * for representing structured data. JSON has exactly six structural characters,
 * three literal names, and two value types (strings and numbers).
 *
 * JSON vs Programming Languages
 * -----------------------------
 *
 * JSON is far simpler to tokenize than programming languages like Starlark or
 * Python:
 *
 *   - **No keywords** -- `true`, `false`, and `null` are literal values, not
 *     keyword-reclassified identifiers. Each gets its own token type.
 *   - **No identifiers** -- there is no NAME token. Object keys are strings.
 *   - **No operators** -- no `+`, `-`, `*`, etc. The minus sign is part of
 *     the NUMBER token, not a separate operator.
 *   - **No comments** -- JSON has no comment syntax (unlike JSON5 or JSONC).
 *   - **No indentation** -- whitespace is insignificant; no INDENT/DEDENT.
 *   - **No newlines** -- line breaks are just whitespace, not statement
 *     terminators. No NEWLINE tokens are emitted.
 *
 * This simplicity makes JSON an excellent first grammar for the grammar-driven
 * tokenization infrastructure. If the generic engine can tokenize JSON correctly,
 * the fundamentals work.
 *
 * Token Types
 * -----------
 *
 * The `json.tokens` file defines these token types:
 *
 *   | Token     | Example     | Description                                |
 *   |-----------|-------------|--------------------------------------------|
 *   | STRING    | "hello"     | Double-quoted string with escape sequences |
 *   | NUMBER    | -42, 3.14   | Integer, decimal, or scientific notation   |
 *   | TRUE      | true        | Boolean true literal                       |
 *   | FALSE     | false       | Boolean false literal                      |
 *   | NULL      | null        | Null literal                               |
 *   | LBRACE    | {           | Start of object                            |
 *   | RBRACE    | }           | End of object                              |
 *   | LBRACKET  | [           | Start of array                             |
 *   | RBRACKET  | ]           | End of array                               |
 *   | COLON     | :           | Key-value separator in objects             |
 *   | COMMA     | ,           | Element separator                          |
 *   | EOF       | (synthetic) | End of input                               |
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `json.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts -> json-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

/**
 * Resolve __dirname for ESM modules.
 *
 * In CommonJS, __dirname is a global. In ESM, it does not exist -- we must
 * derive it from import.meta.url, which gives the file URL of the current
 * module (e.g., "file:///path/to/tokenizer.ts"). The fileURLToPath + dirname
 * pattern converts this to a directory path.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname = .../json-lexer/src/
 *   ..         = .../json-lexer/
 *   ../..      = .../typescript/
 *   ../../..   = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const JSON_TOKENS_PATH = join(GRAMMARS_DIR, "json.tokens");

/**
 * Tokenize JSON text and return an array of tokens.
 *
 * The function reads the `json.tokens` grammar file, parses it into a
 * `TokenGrammar` object (which contains regex patterns, literal patterns,
 * and skip patterns), then passes the source text and grammar to the
 * generic `grammarTokenize` engine.
 *
 * The generic engine handles:
 *   - Pattern matching (regexes and literals)
 *   - Skip patterns (whitespace)
 *   - Position tracking (line and column for each token)
 *
 * Unlike programming language lexers, the JSON lexer does not need:
 *   - Keyword reclassification (no NAME token exists)
 *   - Reserved word detection (no reserved words)
 *   - Indentation tracking (whitespace is insignificant)
 *
 * @param source - The JSON text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeJSON('{"name": "Alice", "age": 30}');
 *     // [Token(LBRACE, "{"), Token(STRING, '"name"'), Token(COLON, ":"),
 *     //  Token(STRING, '"Alice"'), Token(COMMA, ","), Token(STRING, '"age"'),
 *     //  Token(COLON, ":"), Token(NUMBER, "30"), Token(RBRACE, "}"),
 *     //  Token(EOF, "")]
 *
 * @example
 *     const tokens = tokenizeJSON("[1, 2, 3]");
 *     // [Token(LBRACKET, "["), Token(NUMBER, "1"), Token(COMMA, ","),
 *     //  Token(NUMBER, "2"), Token(COMMA, ","), Token(NUMBER, "3"),
 *     //  Token(RBRACKET, "]"), Token(EOF, "")]
 *
 * @example
 *     const tokens = tokenizeJSON("true");
 *     // [Token(TRUE, "true"), Token(EOF, "")]
 */
export function tokenizeJSON(source: string): Token[] {
  /**
   * Read the grammar file from disk. In a production system, you would
   * cache this -- but for an educational codebase, reading on every call
   * keeps the code simple and makes the data flow obvious.
   */
  const grammarText = readFileSync(JSON_TOKENS_PATH, "utf-8");

  /**
   * Parse the grammar text into a structured TokenGrammar object.
   * This extracts:
   *   - Token patterns (regex and literal)
   *   - Skip patterns (whitespace)
   *
   * JSON has no keywords, reserved words, or indentation mode.
   */
  const grammar = parseTokenGrammar(grammarText);

  /**
   * Run the generic grammar-driven tokenizer. This is the same engine
   * used for Starlark, Python, Ruby, and other languages -- the only thing
   * that changes between languages is the grammar file.
   */
  return grammarTokenize(source, grammar);
}
