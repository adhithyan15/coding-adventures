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
 * Locating the Grammar File
 * -------------------------
 *
 * The `ruby.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts -> ruby-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const RUBY_TOKENS_PATH = join(GRAMMARS_DIR, "ruby.tokens");

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
  const grammarText = readFileSync(RUBY_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}
