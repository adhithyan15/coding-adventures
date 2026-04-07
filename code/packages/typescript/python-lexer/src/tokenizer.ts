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
 * Grammar files live in `code/grammars/python/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts
 *     └── python-lexer/      (parent)
 *         └── typescript/     (parent)
 *             └── packages/   (parent)
 *                 └── code/   (parent)
 *                     └── grammars/
 *                         └── python/
 *                             └── python3.12.tokens
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import type { TokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** The Python version used when no version is specified. */
const DEFAULT_VERSION = "3.12";

/** All Python versions with grammar files. */
export const SUPPORTED_VERSIONS = ["2.7", "3.0", "3.6", "3.8", "3.10", "3.12"];

// ---------------------------------------------------------------------------
// Grammar File Location
// ---------------------------------------------------------------------------
//
// We navigate from this file's directory (src/) up four levels to reach
// the code/ directory, then into grammars/python/.
//
//   src/ -> python-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/python/
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars", "python");

// ---------------------------------------------------------------------------
// Grammar Cache
// ---------------------------------------------------------------------------
//
// Parsed TokenGrammar objects are cached per version string. Once a grammar
// is parsed it is reused for all subsequent calls with that version. Since
// TokenGrammar is a read-only data structure, sharing it is safe.
// ---------------------------------------------------------------------------

const grammarCache = new Map<string, TokenGrammar>();

/**
 * Resolve the version string. Empty or undefined defaults to "3.12".
 */
function resolveVersion(version?: string): string {
  return version || DEFAULT_VERSION;
}

/**
 * Return the file path for the grammar of the given Python version.
 *
 * @param version - A version string like "3.12" or "2.7".
 * @returns Absolute path to the `.tokens` file.
 */
function grammarPath(version: string): string {
  return join(GRAMMARS_DIR, `python${version}.tokens`);
}

/**
 * Load and parse (or retrieve from cache) the TokenGrammar for a Python version.
 *
 * @param version - Resolved version string (not empty).
 * @returns The parsed TokenGrammar.
 */
function loadGrammar(version: string): TokenGrammar {
  const cached = grammarCache.get(version);
  if (cached) return cached;

  const grammarText = readFileSync(grammarPath(version), "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  grammarCache.set(version, grammar);
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
