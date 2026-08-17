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
 * 1. We locate the versioned `pythonX.Y.tokens` file in `code/grammars/python/`.
 * 2. We parse that file into a `TokenGrammar` using `parseTokenGrammar`.
 * 3. We feed the grammar to `grammarTokenize`, which handles the actual
 *    tokenization — matching characters against regex patterns and producing
 *    `Token` objects.
 *
 * No Python-specific tokenization code exists here. The grammar file *is* the
 * specification, and the generic engine *is* the implementation. This is the
 * same pattern used by tools like Tree-sitter and TextMate grammars.
 *
 * Version Support
 * ---------------
 *
 * The lexer supports multiple Python versions, each with its own grammar file:
 *   - "2.7", "3.0", "3.6", "3.8", "3.10", "3.12"
 *
 * Versioned grammar files live at `code/grammars/python/pythonX.Y.tokens`.
 * Parsed grammars are cached per version so repeated calls avoid re-parsing.
 *
 * Locating the Grammar Files
 * --------------------------
 *
 * Grammar files live in `code/grammars/python/` at the repository root, but a
 * published npm package would never ship that monorepo directory. Instead,
 * every version's grammar is pre-compiled to a native TypeScript module
 * (`_grammar_<version>.ts`) at build time and statically imported below, so
 * no filesystem access happens at runtime.
 */

import type { TokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR as GRAMMAR_2_7 } from "./_grammar_2_7.js";
import { TOKEN_GRAMMAR as GRAMMAR_3_0 } from "./_grammar_3_0.js";
import { TOKEN_GRAMMAR as GRAMMAR_3_6 } from "./_grammar_3_6.js";
import { TOKEN_GRAMMAR as GRAMMAR_3_8 } from "./_grammar_3_8.js";
import { TOKEN_GRAMMAR as GRAMMAR_3_10 } from "./_grammar_3_10.js";
import { TOKEN_GRAMMAR as GRAMMAR_3_12 } from "./_grammar_3_12.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** The Python version used when no version is specified. */
const DEFAULT_VERSION = "3.12";

/** All Python versions with grammar files. */
export const SUPPORTED_VERSIONS = ["2.7", "3.0", "3.6", "3.8", "3.10", "3.12"];

// ---------------------------------------------------------------------------
// Grammar Map
// ---------------------------------------------------------------------------
//
// All Python version grammars, keyed by version string. Populated via static
// imports above so every version ships inside the published package with no
// runtime filesystem access.
// ---------------------------------------------------------------------------

const VERSIONED_GRAMMARS: Record<string, TokenGrammar> = {
  "2.7": GRAMMAR_2_7,
  "3.0": GRAMMAR_3_0,
  "3.6": GRAMMAR_3_6,
  "3.8": GRAMMAR_3_8,
  "3.10": GRAMMAR_3_10,
  "3.12": GRAMMAR_3_12,
};

/**
 * Resolve the version string. Empty or undefined defaults to "3.12".
 */
function resolveVersion(version?: string): string {
  return version || DEFAULT_VERSION;
}

/**
 * Look up the pre-compiled TokenGrammar for a Python version.
 *
 * @param version - Resolved version string (not empty).
 * @returns The TokenGrammar for that version.
 * @throws Error if `version` is not one of `SUPPORTED_VERSIONS`.
 */
function loadGrammar(version: string): TokenGrammar {
  const grammar = VERSIONED_GRAMMARS[version];
  if (!grammar) {
    throw new Error(
      `Unknown Python version "${version}". ` +
        `Valid values: ${SUPPORTED_VERSIONS.join(", ")}`
    );
  }
  return grammar;
}

/**
 * Tokenize Python source code and return an array of tokens.
 *
 * This is the main entry point for the Python lexer. Pass in a string of
 * Python source code and an optional version, and get back a flat array of
 * `Token` objects. The array always ends with an `EOF` token.
 *
 * The function handles all setup internally: locating the versioned grammar
 * file, parsing it (with caching), and running the tokenization.
 *
 * @param source - The Python source code to tokenize.
 * @param version - Python version string (e.g. "3.12", "2.7"). Defaults to "3.12".
 * @returns An array of Token objects representing the lexical structure.
 *
 * @example
 *     const tokens = tokenizePython("x = 1 + 2");
 *     // [Token(NAME, "x"), Token(EQUALS, "="), Token(NUMBER, "1"),
 *     //  Token(PLUS, "+"), Token(NUMBER, "2"), Token(EOF, "")]
 *
 * @example
 *     // Use a specific Python version
 *     const tokens = tokenizePython("match x:\n  case 1: pass", "3.10");
 */
export function tokenizePython(source: string, version?: string): Token[] {
  const v = resolveVersion(version);
  const grammar = loadGrammar(v);
  return grammarTokenize(source, grammar);
}
