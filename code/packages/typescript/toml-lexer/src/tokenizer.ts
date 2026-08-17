/**
 * TOML Lexer -- tokenizes TOML text using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `toml.tokens` grammar
 * file and delegates all tokenization work to the generic engine.
 *
 * What Is TOML?
 * -------------
 *
 * TOML (Tom's Obvious Minimal Language, https://toml.io) is a configuration file
 * format designed for clarity. Created by Tom Preston-Werner (co-founder of GitHub),
 * TOML maps unambiguously to a hash table. It is used by Rust (Cargo.toml),
 * Python (pyproject.toml), and many other tools.
 *
 * TOML vs JSON
 * ------------
 *
 * TOML is significantly more complex to tokenize than JSON:
 *
 *   - **Newline-sensitive** -- key-value pairs are terminated by newlines.
 *     The lexer emits NEWLINE tokens, which JSON's lexer skips entirely.
 *   - **Four string types** -- basic ("..."), literal ('...'), multi-line basic
 *     ("""..."""), and multi-line literal ('''...'''). JSON has only one.
 *   - **Comments** -- lines starting with # are comments. JSON has none.
 *   - **Date/time literals** -- TOML has offset datetimes, local datetimes,
 *     local dates, and local times as first-class token types. JSON has no
 *     date type at all.
 *   - **Multiple integer formats** -- hexadecimal (0xFF), octal (0o77), binary
 *     (0b1010), and decimal. JSON only has decimal.
 *   - **Underscore separators** -- numbers can use underscores for readability:
 *     1_000_000. JSON does not allow this.
 *   - **Bare keys** -- unquoted key names like `server` or `database`. JSON
 *     requires all keys to be double-quoted strings.
 *   - **No escape processing** -- TOML has four string types with different
 *     escape semantics. The `escapes: none` directive in toml.tokens tells
 *     the lexer to strip quotes but leave escape sequences as raw text.
 *     The semantic layer in toml-parser handles type-specific escape processing.
 *
 * Token Ordering Challenges
 * -------------------------
 *
 * TOML token definitions must be carefully ordered because many patterns overlap:
 *
 *   1. **Triple-quoted strings before single-quoted** -- Without this, `"""hello"""`
 *      would match as empty string + "hello" + empty string.
 *   2. **Dates before bare keys and numbers** -- `1979-05-27` looks like three
 *      integers separated by minus signs.
 *   3. **Floats before integers** -- `3.14` would match as integer `3` then `.14`.
 *   4. **Special floats before bare keys** -- `inf` and `nan` would match as bare keys.
 *   5. **Hex/oct/bin before decimal integers** -- `0xFF` would match as `0`.
 *   6. **Bare keys last** -- they match almost anything: letters, digits, dashes.
 *
 * The `toml.tokens` grammar file handles all this ordering. This module just
 * loads the file and calls the generic engine.
 *
 * Token Types
 * -----------
 *
 *   | Token              | Example             | Description                              |
 *   |--------------------|---------------------|------------------------------------------|
 *   | ML_BASIC_STRING    | \"""hello\"""       | Triple-double-quoted, escapes allowed    |
 *   | ML_LITERAL_STRING  | '''hello'''         | Triple-single-quoted, no escapes         |
 *   | BASIC_STRING       | "hello"             | Double-quoted, escapes allowed           |
 *   | LITERAL_STRING     | 'hello'             | Single-quoted, no escapes                |
 *   | OFFSET_DATETIME    | 1979-05-27T07:32Z   | Date+time with timezone offset           |
 *   | LOCAL_DATETIME     | 1979-05-27T07:32:00 | Date+time without timezone               |
 *   | LOCAL_DATE         | 1979-05-27          | Date only                                |
 *   | LOCAL_TIME         | 07:32:00            | Time only                                |
 *   | FLOAT              | 3.14, 1e10, inf     | Decimal, scientific, or special float    |
 *   | INTEGER            | 42, 0xFF, 0b1010    | Decimal, hex, octal, or binary integer   |
 *   | TRUE               | true                | Boolean true literal                     |
 *   | FALSE              | false               | Boolean false literal                    |
 *   | BARE_KEY           | server              | Unquoted key name                        |
 *   | EQUALS             | =                   | Key-value separator                      |
 *   | DOT                | .                   | Dotted key separator                     |
 *   | COMMA              | ,                   | Array/inline-table element separator     |
 *   | LBRACKET           | [                   | Table header / array start               |
 *   | RBRACKET           | ]                   | Table header / array end                 |
 *   | LBRACE             | {                   | Inline table start                       |
 *   | RBRACE             | }                   | Inline table end                         |
 *   | NEWLINE            | \\n                 | Line break (significant in TOML)         |
 *   | EOF                |                     | End of input (always the last token)     |
 *
 * Grammar Source
 * --------------
 *
 * The token grammar is compiled ahead of time from `toml.tokens` (in
 * `code/grammars/toml/`) into `./_grammar.ts`, a native TypeScript object
 * literal. This avoids reading and parsing a grammar file from disk at
 * runtime -- which would break once this package is published, since a
 * published npm package never ships the monorepo's `code/grammars/` tree.
 */

import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR } from "./_grammar.js";

/**
 * Tokenize TOML text and return an array of tokens.
 *
 * The function reads the `toml.tokens` grammar file, parses it into a
 * `TokenGrammar` object (which contains regex patterns, literal patterns,
 * skip patterns, and the `escapes: none` directive), then passes the source
 * text and grammar to the generic `grammarTokenize` engine.
 *
 * The generic engine handles:
 *   - Pattern matching (regexes and literals, first-match-wins ordering)
 *   - Skip patterns (whitespace and comments)
 *   - Position tracking (line and column for each token)
 *   - NEWLINE token emission (TOML is newline-sensitive)
 *   - Quote stripping without escape processing (escapes: none)
 *
 * Unlike JSON, the TOML lexer:
 *   - Emits NEWLINE tokens (newlines are significant in TOML)
 *   - Skips comments (# to end of line)
 *   - Does NOT process escape sequences (the semantic layer handles this)
 *   - Recognizes four string types, date/time literals, and bare keys
 *
 * @param source - The TOML text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeTOML('title = "TOML Example"');
 *     // [Token(BARE_KEY, "title"), Token(EQUALS, "="),
 *     //  Token(BASIC_STRING, "TOML Example"), Token(EOF, "")]
 *
 * @example
 *     const tokens = tokenizeTOML('[server]\nhost = "localhost"\nport = 8080');
 *     // [Token(LBRACKET, "["), Token(BARE_KEY, "server"), Token(RBRACKET, "]"),
 *     //  Token(NEWLINE, "\\n"), Token(BARE_KEY, "host"), Token(EQUALS, "="),
 *     //  Token(BASIC_STRING, "localhost"), Token(NEWLINE, "\\n"),
 *     //  Token(BARE_KEY, "port"), Token(EQUALS, "="), Token(INTEGER, "8080"),
 *     //  Token(EOF, "")]
 *
 * @example
 *     const tokens = tokenizeTOML('colors = ["red", "green", "blue"]');
 *     // [Token(BARE_KEY, "colors"), Token(EQUALS, "="),
 *     //  Token(LBRACKET, "["), Token(BASIC_STRING, "red"), Token(COMMA, ","),
 *     //  Token(BASIC_STRING, "green"), Token(COMMA, ","),
 *     //  Token(BASIC_STRING, "blue"), Token(RBRACKET, "]"), Token(EOF, "")]
 */
export function tokenizeTOML(source: string): Token[] {
  /**
   * Run the generic grammar-driven tokenizer against the pre-compiled
   * TOKEN_GRAMMAR constant. This is the same engine used for JSON,
   * Starlark, Python, and other languages -- the only thing that changes
   * between languages is the grammar.
   *
   * For TOML, the engine will:
   *   1. Skip whitespace (spaces and tabs only -- not newlines)
   *   2. Skip comments (# to end of line)
   *   3. Emit NEWLINE tokens for line breaks
   *   4. Match token patterns in priority order (triple-quoted strings
   *      before single-quoted, dates before bare keys, etc.)
   *   5. Strip quotes from string tokens without processing escapes
   */
  return grammarTokenize(source, TOKEN_GRAMMAR);
}
