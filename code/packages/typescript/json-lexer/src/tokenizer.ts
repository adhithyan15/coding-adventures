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
 * Grammar Source
 * --------------
 *
 * The token grammar is compiled ahead of time from `json.tokens` (in
 * `code/grammars/json/`) into `./_grammar.ts`, a native TypeScript object
 * literal. This avoids reading and parsing a grammar file from disk at
 * runtime -- which would break once this package is published, since a
 * published npm package never ships the monorepo's `code/grammars/` tree.
 */

import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR } from "./_grammar.js";

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
   * Run the generic grammar-driven tokenizer against the pre-compiled
   * TOKEN_GRAMMAR constant. This is the same engine used for Starlark,
   * Python, Ruby, and other languages -- the only thing that changes
   * between languages is the grammar.
   */
  return grammarTokenize(source, TOKEN_GRAMMAR);
}
