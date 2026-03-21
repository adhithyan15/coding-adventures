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
 * Locating the Grammar File
 * -------------------------
 *
 * The `starlark.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts -> starlark-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
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
 * In CommonJS, __dirname is a global. In ESM, it does not exist — we must
 * derive it from import.meta.url, which gives the file URL of the current
 * module (e.g., "file:///path/to/tokenizer.ts"). The fileURLToPath + dirname
 * pattern converts this to a directory path.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname = .../starlark-lexer/src/
 *   ..         = .../starlark-lexer/
 *   ../..      = .../typescript/
 *   ../../..   = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const STARLARK_TOKENS_PATH = join(GRAMMARS_DIR, "starlark.tokens");

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
   * Read the grammar file from disk. In a production system, you would
   * cache this — but for an educational codebase, reading on every call
   * keeps the code simple and makes the data flow obvious.
   */
  const grammarText = readFileSync(STARLARK_TOKENS_PATH, "utf-8");

  /**
   * Parse the grammar text into a structured TokenGrammar object.
   * This extracts:
   *   - Token patterns (regex and literal)
   *   - Keywords list (and, break, continue, def, elif, else, for, ...)
   *   - Reserved words list (class, import, while, try, ...)
   *   - Skip patterns (comments, whitespace)
   *   - Mode flags (indentation: true)
   */
  const grammar = parseTokenGrammar(grammarText);

  /**
   * Run the generic grammar-driven tokenizer. This is the same engine
   * used for Python, Ruby, JavaScript, and TypeScript — the only thing
   * that changes between languages is the grammar file.
   */
  return grammarTokenize(source, grammar);
}
