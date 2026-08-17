/**
 * TypeScript Lexer — tokenizes TypeScript source code using the grammar-driven approach.
 *
 * This module is a thin wrapper around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It selects a precompiled TypeScript
 * `TokenGrammar` and delegates all tokenization work to the generic engine.
 *
 * TypeScript extends JavaScript with additional features:
 * - `interface`, `type`, `enum`, `namespace`, `declare` keywords
 * - Type annotations like `: number`, `: string`, `: boolean`
 * - Generic syntax with `<` and `>`
 * - `readonly`, `abstract`, `implements`, `extends` keywords
 * - All JavaScript features are also supported
 *
 * Version Support
 * ---------------
 *
 * TypeScript has evolved significantly across major versions. This module
 * supports selecting a specific grammar by version:
 *
 * | Version string  | Compiled grammar module     |
 * |------------------|-----------------------------|
 * | `"ts1.0"`        | `./_grammar_ts1_0.js`       |
 * | `"ts2.0"`        | `./_grammar_ts2_0.js`       |
 * | `"ts3.0"`        | `./_grammar_ts3_0.js`       |
 * | `"ts4.0"`        | `./_grammar_ts4_0.js`       |
 * | `"ts5.0"`        | `./_grammar_ts5_0.js`       |
 * | `"ts5.8"`        | `./_grammar_ts5_8.js`       |
 * | `undefined`/`""` | `./_grammar.js` (generic)   |
 *
 * When no version is supplied the generic grammar is used, which covers the
 * broad intersection of TypeScript syntax — the same behaviour as v0.1.x.
 *
 * Locating the Grammar
 * ---------------------
 *
 * Grammars are no longer read from disk at runtime. Each `.tokens` source
 * file under `code/grammars/` is compiled ahead of time into a sibling
 * `_grammar*.ts` module (via `code/scripts/_ts_grammar_compile.ts`) that
 * embeds the `TokenGrammar` as a native TypeScript object literal. This
 * keeps the package self-contained — a published npm package never needs to
 * reach outside its own directory — and avoids repeated file I/O and
 * re-parsing on every call.
 */

import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";
import type { TokenGrammar } from "@coding-adventures/grammar-tools";

import { TOKEN_GRAMMAR as GENERIC } from "./_grammar.js";
import { TOKEN_GRAMMAR as TS1_0 } from "./_grammar_ts1_0.js";
import { TOKEN_GRAMMAR as TS2_0 } from "./_grammar_ts2_0.js";
import { TOKEN_GRAMMAR as TS3_0 } from "./_grammar_ts3_0.js";
import { TOKEN_GRAMMAR as TS4_0 } from "./_grammar_ts4_0.js";
import { TOKEN_GRAMMAR as TS5_0 } from "./_grammar_ts5_0.js";
import { TOKEN_GRAMMAR as TS5_8 } from "./_grammar_ts5_8.js";

/**
 * Valid TypeScript version strings accepted by this module.
 *
 * Each version corresponds to a precompiled grammar module. Omitting the
 * version uses the generic grammar.
 */
const VERSIONED_GRAMMARS: Record<string, TokenGrammar> = {
  "ts1.0": TS1_0,
  "ts2.0": TS2_0,
  "ts3.0": TS3_0,
  "ts4.0": TS4_0,
  "ts5.0": TS5_0,
  "ts5.8": TS5_8,
};

/**
 * Resolve the compiled `TokenGrammar` for the given version.
 *
 * @param version - An optional TypeScript version string (e.g. `"ts5.8"`).
 *   Pass `undefined` or `""` to get the generic grammar.
 * @returns The precompiled `TokenGrammar` object.
 * @throws Error if `version` is a non-empty string that is not a recognised
 *   TypeScript version identifier.
 *
 * @example
 *   resolveTokenGrammar("ts5.8")
 *   // => the compiled ts5.8 TokenGrammar
 *
 *   resolveTokenGrammar()
 *   // => the compiled generic TokenGrammar
 */
function resolveTokenGrammar(version?: string): TokenGrammar {
  if (!version) {
    return GENERIC;
  }

  const grammar = VERSIONED_GRAMMARS[version];
  if (!grammar) {
    throw new Error(
      `Unknown TypeScript version "${version}". ` +
        `Valid values: ${Object.keys(VERSIONED_GRAMMARS).join(", ")}`
    );
  }

  return grammar;
}

/**
 * Tokenize TypeScript source code and return an array of tokens.
 *
 * @param source  - The TypeScript source code to tokenize.
 * @param version - Optional TypeScript version. When omitted (or empty string)
 *   the generic grammar is used, backward-compatible with v0.1.x. Pass a
 *   specific version like `"ts5.8"` to use a version-exact grammar.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     // Generic (backwards-compatible)
 *     const tokens = tokenizeTypescript("let x: number = 1 + 2;");
 *
 *     // Version-specific
 *     const tokens = tokenizeTypescript("let x: number = 1 + 2;", "ts5.8");
 */
export function tokenizeTypescript(source: string, version?: string): Token[] {
  const grammar = resolveTokenGrammar(version);
  return grammarTokenize(source, grammar);
}

/**
 * Create a `GrammarLexer` instance for TypeScript source code.
 *
 * Unlike `tokenizeTypescript`, which eagerly produces the full token array,
 * `createTypescriptLexer` returns the configured `GrammarLexer` object before
 * tokenization begins.
 *
 * @param source  - The TypeScript source code to tokenize.
 * @param version - Optional TypeScript version (same semantics as
 *   `tokenizeTypescript`).
 * @returns A `GrammarLexer` instance ready to call `.tokenize()` on.
 *
 * @example
 *     const lexer = createTypescriptLexer("let x: number = 1;", "ts5.8");
 *     const tokens = lexer.tokenize();
 */
export function createTypescriptLexer(
  source: string,
  version?: string
): GrammarLexer {
  const grammar = resolveTokenGrammar(version);
  return new GrammarLexer(source, grammar);
}
