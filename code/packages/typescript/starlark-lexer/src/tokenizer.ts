/**
 * Starlark Lexer — tokenizes Starlark source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `starlark.tokens` grammar
 * file and delegates all tokenization work to the generic engine.
 *
 * What Is Starlark?
 * -----------------
 *
 * Starlark is a dialect of Python designed by Google for the Bazel build system.
 * It is intentionally restricted to guarantee deterministic, hermetic evaluation:
 *
 *   - No `while` loops or recursion (guarantees termination)
 *   - No `class`, `import`, `try/except` (simplifies the runtime)
 *   - No global mutable state (enables parallel evaluation)
 *   - No `is` operator (identity is an implementation detail)
 *
 * Because of these restrictions, Starlark is safe to execute in a build system
 * where untrusted configuration files must be evaluated without risk of infinite
 * loops or side effects.
 *
 * Starlark vs Python Tokenization
 * --------------------------------
 *
 * Starlark shares most of Python's lexical structure:
 *   - Significant indentation (INDENT/DEDENT tokens)
 *   - Same string literal syntax (single, double, triple-quoted, raw, bytes)
 *   - Same numeric literals (int, float, hex, octal)
 *   - Same operator set (with ** for exponentiation, // for floor division)
 *
 * The key difference is in **reserved keywords**: words like `class`, `import`,
 * `while`, `try`, etc. are not just unrecognized — they cause a lexer error.
 * This gives users immediate, clear feedback instead of a confusing parse error.
 *
 * Grammar Source
 * --------------
 *
 * The token grammar is compiled ahead of time from `starlark.tokens` (in
 * `code/grammars/starlark/`) into `./_grammar.ts`, a native TypeScript
 * object literal. This avoids reading and parsing a grammar file from disk
 * at runtime -- which would break once this package is published, since a
 * published npm package never ships the monorepo's `code/grammars/` tree.
 */

import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR } from "./_grammar.js";

/**
 * Tokenize Starlark source code and return an array of tokens.
 *
 * The function reads the `starlark.tokens` grammar file, parses it into a
 * `TokenGrammar` object (which contains regex patterns, keywords, reserved
 * words, skip patterns, and the indentation mode flag), then passes the
 * source code and grammar to the generic `grammarTokenize` engine.
 *
 * The generic engine handles:
 *   - Pattern matching (regexes and literals)
 *   - Keyword reclassification (NAME -> KEYWORD when the value matches)
 *   - Reserved word detection (NAME -> error when the value is reserved)
 *   - Indentation tracking (INDENT/DEDENT/NEWLINE emission)
 *   - Skip patterns (comments and whitespace)
 *   - Position tracking (line and column for each token)
 *
 * @param source - The Starlark source code to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeStarlark("x = 1 + 2");
 *     // [Token(NAME, "x"), Token(EQUALS, "="), Token(INT, "1"),
 *     //  Token(PLUS, "+"), Token(INT, "2"), Token(NEWLINE, ""),
 *     //  Token(EOF, "")]
 *
 * @example
 *     // Starlark uses INDENT/DEDENT for blocks, just like Python:
 *     const tokens = tokenizeStarlark("def f():\n    return 1");
 *     // Includes KEYWORD("def"), NAME("f"), ..., INDENT, KEYWORD("return"),
 *     // INT("1"), NEWLINE, DEDENT, EOF
 *
 * @example
 *     // Reserved keywords cause an error:
 *     tokenizeStarlark("class Foo:");  // throws: 'class' is reserved
 */
export function tokenizeStarlark(source: string): Token[] {
  /**
   * Run the generic grammar-driven tokenizer against the pre-compiled
   * TOKEN_GRAMMAR constant. This is the same engine used for Python,
   * Ruby, JavaScript, and TypeScript — the only thing that changes
   * between languages is the grammar.
   */
  return grammarTokenize(source, TOKEN_GRAMMAR);
}
