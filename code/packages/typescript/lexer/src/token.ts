/**
 * Token — The smallest meaningful unit of source code.
 * =====================================================
 *
 * Before a computer can execute code like `x = 1 + 2`, it needs to break
 * that raw text into meaningful chunks. Those chunks are called **tokens**.
 *
 * Think of it like reading a sentence in English. When you see:
 *
 *     The cat sat on the mat.
 *
 * Your brain automatically groups the letters into words: "The", "cat",
 * "sat", "on", "the", "mat", and the period ".". You don't think about
 * individual letters — you think about *words* and *punctuation*. A lexer
 * does the same thing for source code.
 *
 * Given the input `x = 1 + 2`, the lexer produces:
 *
 *     NAME("x")  EQUALS("=")  NUMBER("1")  PLUS("+")  NUMBER("2")  EOF
 *
 * Each of these is a **Token** — a small labeled piece of text. The label
 * (like NAME or NUMBER) is called the **token type**, and the text itself
 * (like "x" or "1") is called the **token value**.
 *
 * In TypeScript we represent tokens as plain objects conforming to the
 * `Token` interface. Unlike the Python version which uses an Enum for
 * token types, we use plain strings — this makes the grammar-driven lexer
 * more flexible since it can produce any token type defined in a .tokens
 * file without needing to extend an enum.
 */

// ---------------------------------------------------------------------------
// Token Interface
// ---------------------------------------------------------------------------

/**
 * A single token — the smallest meaningful unit of source code.
 *
 * A token pairs a **type** (what kind of thing it is) with a **value**
 * (the actual text from the source code), plus position information for
 * error reporting.
 *
 * Think of a token like a labeled sticky note attached to a piece of text:
 *
 *     ┌──────────┐
 *     │ NAME     │  ← type (what kind of token)
 *     │ "x"      │  ← value (the actual text)
 *     │ line 1   │  ← where it appeared
 *     │ col 1    │
 *     └──────────┘
 *
 * Why use an interface instead of a class? Tokens are simple data — they
 * don't need methods, inheritance, or encapsulation. A plain object with
 * a known shape is the lightest-weight representation in TypeScript, and
 * it's trivially serializable to JSON for debugging or testing.
 *
 * Properties:
 *   type: The kind of token (e.g., "NAME", "NUMBER", "PLUS").
 *   value: The actual text from the source code that this token represents.
 *   line: The 1-based line number where this token starts.
 *   column: The 1-based column number where this token starts.
 */
export interface Trivia {
  readonly type: string;
  readonly value: string;
  readonly line: number;
  readonly column: number;
  readonly endLine: number;
  readonly endColumn: number;
  readonly startOffset: number;
  readonly endOffset: number;
}

export interface Token {
  readonly type: string;
  readonly value: string;
  readonly line: number;
  readonly column: number;
  readonly typeName?: string;
  readonly flags?: number;
  readonly endLine?: number;
  readonly endColumn?: number;
  readonly startOffset?: number;
  readonly endOffset?: number;
  readonly tokenIndex?: number;
  readonly leadingTrivia?: readonly Trivia[];
}

// ---------------------------------------------------------------------------
// Token Flag Constants
// ---------------------------------------------------------------------------

/**
 * Bitmask flags for token metadata.
 *
 * Flags carry information that is neither type nor value but affects
 * how downstream consumers (parsers, formatters, linters) interpret
 * a token. For example, JavaScript's automatic semicolon insertion
 * rule depends on whether a newline appeared before certain tokens.
 *
 * Flags are optional — when `flags` is undefined, all flags are off.
 * Use bitwise AND to test: `(token.flags ?? 0) & TOKEN_PRECEDED_BY_NEWLINE`
 */

/**
 * Set when a line break appeared between this token and the previous one.
 *
 * Languages with automatic semicolon insertion (JavaScript, Go) use
 * this to decide whether an implicit semicolon should be inserted.
 * The lexer itself does not insert semicolons — that is a language-
 * specific concern handled in language packages via post-tokenize
 * hooks or parser pre-parse hooks.
 */
export const TOKEN_PRECEDED_BY_NEWLINE = 1;

/**
 * Set for context-sensitive keywords — words that are keywords in some
 * syntactic positions but identifiers in others.
 *
 * For example, JavaScript's `async`, `yield`, `await`, `get`, `set`
 * are sometimes keywords (in function declarations, property accessors)
 * and sometimes plain identifiers (`let get = 5`). The lexer emits
 * these as NAME tokens with this flag set, leaving the final
 * keyword-vs-identifier decision to the language-specific parser.
 */
export const TOKEN_CONTEXT_KEYWORD = 2;
