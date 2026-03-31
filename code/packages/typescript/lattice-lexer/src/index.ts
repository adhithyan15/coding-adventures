/**
 * Lattice Lexer — Tokenizes Lattice source text.
 *
 * Lattice is a CSS superset language that extends CSS with:
 *   - Variables ($color, $font-size)
 *   - Mixins (@mixin / @include)
 *   - Control flow (@if / @else / @for / @each)
 *   - Functions (@function / @return)
 *   - Modules (@use)
 *
 * This module is a thin wrapper around the generic GrammarLexer. It uses a
 * pre-compiled grammar object (from `_grammar.ts`) and delegates tokenization
 * to the generic engine. The design mirrors the json-lexer: language-specific
 * behavior lives in the grammar file, not in TypeScript code.
 *
 * Token Types (new to Lattice, not in CSS):
 *   VARIABLE       — $color, $font-size ($ never appears in valid CSS values)
 *   EQUALS_EQUALS  — == (equality in @if conditions)
 *   NOT_EQUALS     — != (inequality)
 *   GREATER_EQUALS — >= (greater-or-equal)
 *   LESS_EQUALS    — <= (less-or-equal)
 *
 * All CSS token types are preserved unchanged. The grammar file also adds
 * // line comment support (not in standard CSS).
 *
 * Browser Compatibility
 * ---------------------
 *
 * This module uses a pre-compiled grammar object imported from `_grammar.ts`.
 * No file system access is needed at runtime — it works in Node.js, browsers,
 * edge runtimes, and any other JavaScript environment.
 *
 * Usage:
 *
 *     import { tokenizeLatticeLexer } from "@coding-adventures/lattice-lexer";
 *
 *     const tokens = tokenizeLatticeLexer("$color: red;");
 *     // [Token(VARIABLE, "$color"), Token(COLON, ":"),
 *     //  Token(IDENT, "red"), Token(SEMICOLON, ";"), Token(EOF, "")]
 */

import { GrammarLexer, grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";
import { TOKEN_GRAMMAR } from "./_grammar.js";

/**
 * Create a GrammarLexer instance configured for Lattice source text.
 *
 * This function:
 * 1. Creates a GrammarLexer with the pre-compiled grammar object.
 *
 * The returned lexer has a .tokenize() method that produces a Token array.
 * For advanced use cases (on-token callbacks, pattern groups), use this
 * factory function; for simple tokenization, use tokenizeLatticeLexer().
 *
 * @param source - The Lattice source text to tokenize.
 * @returns A GrammarLexer instance ready to call .tokenize() on.
 *
 * @example
 *     const lexer = createLatticeLexer("$color: red;");
 *     const tokens = lexer.tokenize();
 */
export function createLatticeLexer(source: string): GrammarLexer {
  return new GrammarLexer(source, TOKEN_GRAMMAR);
}

/**
 * Tokenize Lattice source text and return an array of Token objects.
 *
 * This is the main entry point for the Lattice tokenizer. Pass in a string
 * of Lattice source, get back a flat list of Token objects. The list always
 * ends with a synthetic EOF token.
 *
 * Token types produced:
 *   - All CSS token types (IDENT, NUMBER, DIMENSION, PERCENTAGE, STRING,
 *     HASH, AT_KEYWORD, FUNCTION, URL_TOKEN, COLON, SEMICOLON, COMMA,
 *     LBRACE, RBRACE, LPAREN, RPAREN, etc.)
 *   - VARIABLE ($color, $font-size)
 *   - EQUALS_EQUALS, NOT_EQUALS, GREATER_EQUALS, LESS_EQUALS
 *
 * Whitespace and comments (//, /* ... * /) are automatically skipped
 * and do not appear in the output.
 *
 * @param source - The Lattice source text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 * @throws LexerError if the source contains characters that match no token pattern.
 *
 * @example
 *     const tokens = tokenizeLatticeLexer("$color: red;");
 *     // [Token(VARIABLE, "$color"), Token(COLON, ":"),
 *     //  Token(IDENT, "red"), Token(SEMICOLON, ";"), Token(EOF, "")]
 *
 * @example
 *     const tokens = tokenizeLatticeLexer("h1 { color: #4a90d9; }");
 *     // [Token(IDENT, "h1"), Token(LBRACE, "{"), Token(IDENT, "color"),
 *     //  Token(COLON, ":"), Token(HASH, "#4a90d9"), Token(SEMICOLON, ";"),
 *     //  Token(RBRACE, "}"), Token(EOF, "")]
 *
 * @example
 *     // Line and block comments are skipped:
 *     const tokens = tokenizeLatticeLexer("// comment\n$x: 1;");
 *     // [Token(VARIABLE, "$x"), Token(COLON, ":"),
 *     //  Token(NUMBER, "1"), Token(SEMICOLON, ";"), Token(EOF, "")]
 */
export function tokenizeLatticeLexer(source: string): Token[] {
  return grammarTokenize(source, TOKEN_GRAMMAR);
}

// Re-export Token type for consumers that need it
export type { Token };
