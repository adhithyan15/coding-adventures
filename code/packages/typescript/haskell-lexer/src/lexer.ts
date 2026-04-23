/**
 * Haskell Lexer — tokenizes Haskell source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads a Haskell `.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * Haskell has features that differ from HaskellScript, Python, and Ruby:
 * - Static typing with explicit type annotations (`int`, `String`, `boolean`)
 * - Access modifiers (`public`, `private`, `protected`)
 * - `class` is the fundamental organizational unit
 * - Semicolons terminate statements
 * - Curly braces `{}` for blocks
 * - `null` (not `None` or `nil` or `undefined`)
 * - No `$` in identifiers (unlike HaskellScript)
 * - `==` for equality (no `===` strict equality)
 * - Annotations with `@` prefix
 *
 * All of these are handled by the grammar file — no Haskell-specific
 * tokenization code exists in this module.
 *
 * Version Support
 * ---------------
 *
 * Haskell has evolved significantly since JDK 1.0 (1996). This module
 * supports selecting a specific edition grammar by version string:
 *
 * | Version string | Grammar file                       |
 * |----------------|------------------------------------|
 * | `"1.0"`        | `grammars/haskell/haskell1.0.tokens`     |
 * | `"1.1"`        | `grammars/haskell/haskell1.1.tokens`     |
 * | `"1.4"`        | `grammars/haskell/haskell1.4.tokens`     |
 * | `"5"`          | `grammars/haskell/haskell5.tokens`       |
 * | `"7"`          | `grammars/haskell/haskell7.tokens`       |
 * | `"8"`          | `grammars/haskell/haskell8.tokens`       |
 * | `"10"`         | `grammars/haskell/haskell10.tokens`      |
 * | `"14"`         | `grammars/haskell/haskell14.tokens`      |
 * | `"17"`         | `grammars/haskell/haskell17.tokens`      |
 * | `"21"`         | `grammars/haskell/haskell21.tokens`      |
 * | `undefined`    | `grammars/haskell/haskell21.tokens` (default) |
 *
 * When no version is supplied, Haskell 21 (the latest LTS) is used as the
 * default — this is the recommended grammar for most use cases.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `haskell*.tokens` files live in `code/grammars/haskell/` at the repository root.
 *
 *     src/ -> haskell-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Root of the grammars directory.
 * Walk up: src/ -> haskell-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid Haskell version strings accepted by this module.
 *
 * Early Haskell releases (1.0, 1.1, 1.4) use the "1.x" naming convention.
 * Starting with Haskell 5, Sun dropped the "1." prefix. Modern Haskell uses
 * just the major version number (5, 7, 8, 10, 14, 17, 21).
 */
const VALID_HASKELL_VERSIONS = new Set([
  "1.0",
  "1.1",
  "1.2",
  "1.3",
  "1.4",
  "98",
  "2010",
]);

/**
 * The default Haskell version used when no version is specified.
 * Haskell 21 is the latest Long-Term Support (LTS) release.
 */
const DEFAULT_HASKELL_VERSION = "2010";

/**
 * Resolve the path to the Haskell token grammar for the given version.
 *
 * @param version - An optional Haskell version string (e.g. `"21"`, `"8"`,
 *   `"1.4"`). Pass `undefined` or `""` to use the default (Haskell 21).
 * @returns Absolute path to the `.tokens` grammar file.
 * @throws Error if `version` is a non-empty string that is not a recognised
 *   Haskell version identifier.
 *
 * @example
 *   resolveTokensPath("8")
 *   // => ".../code/grammars/haskell/haskell8.tokens"
 *
 *   resolveTokensPath()
 *   // => ".../code/grammars/haskell/haskell21.tokens"
 */
function resolveTokensPath(version?: string): string {
  // Default to Haskell 21 when no version is specified.
  const effectiveVersion = version || DEFAULT_HASKELL_VERSION;

  if (!VALID_HASKELL_VERSIONS.has(effectiveVersion)) {
    throw new Error(
      `Unknown Haskell version "${effectiveVersion}". ` +
        `Valid values: ${[...VALID_HASKELL_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "haskell", `haskell${effectiveVersion}.tokens`);
}

/**
 * Tokenize Haskell source code and return an array of tokens.
 *
 * @param source  - The Haskell source code to tokenize.
 * @param version - Optional Haskell version string. When omitted (or the
 *   empty string) Haskell 21 is used as the default, which is the latest
 *   Long-Term Support release.
 *   Pass a version like `"8"` or `"17"` to use an edition-exact grammar.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     // Default (Haskell 21)
 *     const tokens = tokenizeHaskell("class Hello { }");
 *
 *     // Version-specific
 *     const tokens = tokenizeHaskell("int x = 1;", "8");
 *     const tokens = tokenizeHaskell("var x = 1;", "10");
 */
export function tokenizeHaskell(source: string, version?: string): Token[] {
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}

/**
 * Create a `GrammarLexer` instance for Haskell source code.
 *
 * Unlike `tokenizeHaskell`, which eagerly produces the full token array,
 * `createHaskellLexer` returns the configured `GrammarLexer` object before
 * tokenization begins. This is useful when you need to attach an on-token
 * callback for context-sensitive lexing.
 *
 * @param source  - The Haskell source code to tokenize.
 * @param version - Optional Haskell version string (same semantics as
 *   `tokenizeHaskell`).
 * @returns A `GrammarLexer` instance ready to call `.tokenize()` on.
 *
 * @example
 *     const lexer = createHaskellLexer("class Hello { }", "21");
 *     lexer.setOnToken((token, ctx) => { /* custom logic *\/ });
 *     const tokens = lexer.tokenize();
 */
export function createHaskellLexer(
  source: string,
  version?: string
): GrammarLexer {
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return new GrammarLexer(source, grammar);
}
