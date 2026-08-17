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
 * Grammar Data
 * ------------
 *
 * The `haskell*.tokens` files under `code/grammars/haskell/` at the
 * repository root are compiled ahead of time into `./_grammar.ts` (the
 * default) and one `./_grammar_<version>.ts` per supported edition (see
 * `code/scripts/_ts_grammar_compile.ts`). This module statically imports
 * all of them and looks up the right one at call time — it never reads
 * the monorepo's `code/grammars/` tree at runtime, so a published npm
 * package works standalone.
 */

import type { TokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR as DEFAULT_GRAMMAR } from "./_grammar.js";
import { TOKEN_GRAMMAR as V1_0 } from "./_grammar_1_0.js";
import { TOKEN_GRAMMAR as V1_1 } from "./_grammar_1_1.js";
import { TOKEN_GRAMMAR as V1_2 } from "./_grammar_1_2.js";
import { TOKEN_GRAMMAR as V1_3 } from "./_grammar_1_3.js";
import { TOKEN_GRAMMAR as V1_4 } from "./_grammar_1_4.js";
import { TOKEN_GRAMMAR as V98 } from "./_grammar_98.js";
import { TOKEN_GRAMMAR as V2010 } from "./_grammar_2010.js";

/**
 * Every supported Haskell edition's token grammar, pre-compiled at build
 * time from the `.tokens` files in `code/grammars/haskell/`. ESM static
 * imports can't be conditional on a runtime string, so every version is
 * imported up front and looked up by key in `resolveTokenGrammar`.
 */
const VERSIONED_GRAMMARS: Record<string, TokenGrammar> = {
  "1.0": V1_0,
  "1.1": V1_1,
  "1.2": V1_2,
  "1.3": V1_3,
  "1.4": V1_4,
  "98": V98,
  "2010": V2010,
};

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
 * Resolve the compiled Haskell token grammar for the given version.
 *
 * @param version - An optional Haskell version string (e.g. `"2010"`,
 *   `"98"`, `"1.4"`). Pass `undefined` or `""` to use the default
 *   (Haskell 2010).
 * @returns The pre-compiled `TokenGrammar` for that version.
 * @throws Error if `version` is a non-empty string that is not a recognised
 *   Haskell version identifier.
 *
 * @example
 *   resolveTokenGrammar("98")
 *   // => TOKEN_GRAMMAR compiled from haskell98.tokens
 *
 *   resolveTokenGrammar()
 *   // => TOKEN_GRAMMAR compiled from haskell2010.tokens (the default)
 */
function resolveTokenGrammar(version?: string): TokenGrammar {
  if (!version) {
    return DEFAULT_GRAMMAR;
  }

  if (!VALID_HASKELL_VERSIONS.has(version)) {
    throw new Error(
      `Unknown Haskell version "${version}". ` +
        `Valid values: ${[...VALID_HASKELL_VERSIONS].join(", ")}`
    );
  }

  return VERSIONED_GRAMMARS[version];
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
  const grammar = resolveTokenGrammar(version);
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
  const grammar = resolveTokenGrammar(version);
  return new GrammarLexer(source, grammar);
}
