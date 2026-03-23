/**
 * Python Lexer — tokenizes Python source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It demonstrates a core idea of the
 * grammar-driven architecture: the *same* lexer engine that tokenizes one language
 * can tokenize any other — simply by swapping the `.tokens` file.
 *
 * How It Works
 * ------------
 *
 * 1. We locate `python.tokens` in the `code/grammars/` directory.
 * 2. We parse that file into a `TokenGrammar` using `parseTokenGrammar`.
 * 3. We feed the grammar to `grammarTokenize`, which handles the actual
 *    tokenization — matching characters against regex patterns and producing
 *    `Token` objects.
 *
 * No Python-specific tokenization code exists here. The grammar file *is* the
 * specification, and the generic engine *is* the implementation. This is the
 * same pattern used by tools like Tree-sitter and TextMate grammars.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `python.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts
 *     └── python-lexer/      (parent)
 *         └── typescript/     (parent)
 *             └── packages/   (parent)
 *                 └── code/   (parent)
 *                     └── grammars/
 *                         └── python.tokens
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

// ---------------------------------------------------------------------------
// Grammar File Location
// ---------------------------------------------------------------------------
//
// We navigate from this file's directory (src/) up four levels to reach
// the code/ directory, then into grammars/.
//
//   src/ -> python-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const PYTHON_TOKENS_PATH = join(GRAMMARS_DIR, "python.tokens");

/**
 * Tokenize Python source code and return an array of tokens.
 *
 * This is the main entry point for the Python lexer. Pass in a string of
 * Python source code, and get back a flat array of `Token` objects. The
 * array always ends with an `EOF` token.
 *
 * The function handles all setup internally: locating the grammar file,
 * parsing it, and running the tokenization.
 *
 * @param source - The Python source code to tokenize.
 * @returns An array of Token objects representing the lexical structure.
 *
 * @example
 *     const tokens = tokenizePython("x = 1 + 2");
 *     // [Token(NAME, "x"), Token(EQUALS, "="), Token(NUMBER, "1"),
 *     //  Token(PLUS, "+"), Token(NUMBER, "2"), Token(EOF, "")]
 */
export function tokenizePython(source: string): Token[] {
  const grammarText = readFileSync(PYTHON_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}
