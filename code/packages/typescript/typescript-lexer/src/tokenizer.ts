/**
 * TypeScript Lexer — tokenizes TypeScript source code using the grammar-driven approach.
 *
 * This module is a thin wrapper around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads a TypeScript `.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
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
 * | Version string  | Grammar file                               |
 * |-----------------|--------------------------------------------|
 * | `"ts1.0"`       | `grammars/typescript/ts1.0.tokens`         |
 * | `"ts2.0"`       | `grammars/typescript/ts2.0.tokens`         |
 * | `"ts3.0"`       | `grammars/typescript/ts3.0.tokens`         |
 * | `"ts4.0"`       | `grammars/typescript/ts4.0.tokens`         |
 * | `"ts5.0"`       | `grammars/typescript/ts5.0.tokens`         |
 * | `"ts5.8"`       | `grammars/typescript/ts5.8.tokens`         |
 * | `undefined`/`""`| `grammars/typescript.tokens` (generic)     |
 *
 * When no version is supplied the generic `typescript.tokens` grammar is used,
 * which covers the broad intersection of TypeScript syntax — the same behaviour
 * as v0.1.x.
 *
 * Locating the Grammar Files
 * --------------------------
 *
 * Grammar files live in `code/grammars/` at the repository root.
 *
 *     src/ -> typescript-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
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
 * Walk up: src/ -> typescript-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid TypeScript version strings accepted by this module.
 *
 * Each version corresponds to a versioned grammar file in
 * `code/grammars/typescript/`. Omitting the version uses the generic
 * `typescript.tokens` grammar.
 */
const VALID_TS_VERSIONS = new Set([
  "ts1.0",
  "ts2.0",
  "ts3.0",
  "ts4.0",
  "ts5.0",
  "ts5.8",
]);

/**
 * Resolve the path to the TypeScript token grammar for the given version.
 *
 * @param version - An optional TypeScript version string (e.g. `"ts5.8"`).
 *   Pass `undefined` or `""` to get the generic grammar.
 * @returns Absolute path to the `.tokens` grammar file.
 * @throws Error if `version` is a non-empty string that is not a recognised
 *   TypeScript version identifier.
 *
 * @example
 *   resolveTokensPath("ts5.8")
 *   // => ".../code/grammars/typescript/ts5.8.tokens"
 *
 *   resolveTokensPath()
 *   // => ".../code/grammars/typescript.tokens"
 */
function resolveTokensPath(version?: string): string {
  if (!version) {
    return join(GRAMMARS_DIR, "typescript.tokens");
  }

  if (!VALID_TS_VERSIONS.has(version)) {
    throw new Error(
      `Unknown TypeScript version "${version}". ` +
        `Valid values: ${[...VALID_TS_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "typescript", `${version}.tokens`);
}

/**
 * Tokenize TypeScript source code and return an array of tokens.
 *
 * @param source  - The TypeScript source code to tokenize.
 * @param version - Optional TypeScript version. When omitted (or empty string)
 *   the generic `typescript.tokens` grammar is used, backward-compatible with
 *   v0.1.x. Pass a specific version like `"ts5.8"` to use a version-exact
 *   grammar.
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
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
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
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return new GrammarLexer(source, grammar);
}
