/**
 * Ruby Lexer — tokenizes Ruby source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `ruby.tokens` grammar
 * file and delegates all tokenization work to the generic engine.
 *
 * Ruby has operators that Python does not — like `..` (range), `=>` (hash rocket),
 * and `!=`. The grammar-driven approach handles all of these without any new
 * tokenization code: they are simply declared in the `.tokens` file.
 *
 * Grammar Source
 * --------------
 *
 * The token grammar is compiled ahead of time from `ruby.tokens` (in
 * `code/grammars/ruby/`) into `./_grammar.ts`, a native TypeScript object
 * literal. This avoids reading and parsing a grammar file from disk at
 * runtime -- which would break once this package is published, since a
 * published npm package never ships the monorepo's `code/grammars/` tree.
 */

import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR } from "./_grammar.js";

/**
 * Tokenize Ruby source code and return an array of tokens.
 *
 * @param source - The Ruby source code to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeRuby("x = 1 + 2");
 *     // [Token(NAME, "x"), Token(EQUALS, "="), Token(NUMBER, "1"),
 *     //  Token(PLUS, "+"), Token(NUMBER, "2"), Token(EOF, "")]
 */
export function tokenizeRuby(source: string): Token[] {
  return grammarTokenize(source, TOKEN_GRAMMAR);
}
