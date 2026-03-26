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
 * Locating the Grammar File
 * -------------------------
 *
 * The `toml.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts -> toml-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
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
 *   __dirname = .../toml-lexer/src/
 *   ..         = .../toml-lexer/
 *   ../..      = .../typescript/
 *   ../../..   = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const TOML_TOKENS_PATH = join(GRAMMARS_DIR, "toml.tokens");

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
   * Read the grammar file from disk. In a production system, you would
   * cache this -- but for an educational codebase, reading on every call
   * keeps the code simple and makes the data flow obvious.
   */
  const grammarText = readFileSync(TOML_TOKENS_PATH, "utf-8");

  /**
   * Parse the grammar text into a structured TokenGrammar object.
   * This extracts:
   *   - Token patterns (regex and literal, ordered for first-match-wins)
   *   - Skip patterns (whitespace and comments)
   *   - Escape mode ("none" -- strip quotes, skip escape processing)
   *
   * TOML has no keywords or reserved words (unlike programming languages).
   * Booleans (true/false) are literal tokens, not keyword-reclassified
   * identifiers. Bare keys have their own token type (BARE_KEY).
   */
  const grammar = parseTokenGrammar(grammarText);

  /**
   * Run the generic grammar-driven tokenizer. This is the same engine
   * used for JSON, Starlark, Python, and other languages -- the only thing
   * that changes between languages is the grammar file.
   *
   * For TOML, the engine will:
   *   1. Skip whitespace (spaces and tabs only -- not newlines)
   *   2. Skip comments (# to end of line)
   *   3. Emit NEWLINE tokens for line breaks
   *   4. Match token patterns in priority order (triple-quoted strings
   *      before single-quoted, dates before bare keys, etc.)
   *   5. Strip quotes from string tokens without processing escapes
   */
  return grammarTokenize(source, grammar);
}
