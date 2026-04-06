/**
 * TypeScript Lexer — tokenizes TypeScript source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the shared `lexer` package source. It loads a TypeScript `.tokens`
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
 * | Version string | Grammar file                          |
 * |----------------|---------------------------------------|
 * | `"ts1.0"`      | `grammars/typescript/ts1.0.tokens`    |
 * | `"ts2.0"`      | `grammars/typescript/ts2.0.tokens`    |
 * | `"ts3.0"`      | `grammars/typescript/ts3.0.tokens`    |
 * | `"ts4.0"`      | `grammars/typescript/ts4.0.tokens`    |
 * | `"ts5.0"`      | `grammars/typescript/ts5.0.tokens`    |
 * | `"ts5.8"`      | `grammars/typescript/ts5.8.tokens`    |
 * | `undefined`/`""`| `grammars/typescript.tokens` (generic)|
 *
 * When no version is supplied the generic `typescript.tokens` grammar is used,
 * which covers the broad intersection of TypeScript syntax — the same behaviour
 * as v0.1.x.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The grammar files live in `code/src/tokens/` (generic) and
 * `code/src/tokens/typescript/` (versioned) in the shared source tree.
 *
 *     tokenizer.ts -> typescript-lexer/ -> typescript/ -> src/ -> code/ -> src/tokens/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "../grammar-tools/index.js";
import { grammarTokenize, GrammarLexer } from "../lexer/index.js";
import type { Token } from "../lexer/index.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Root directory of all token grammar files.
 * Walk up: tokenizer.ts -> typescript-lexer/ -> typescript/ -> src/ -> code/ -> src/tokens/
 */
const TOKENS_DIR = join(__dirname, "..", "..", "tokens");

/**
 * Valid TypeScript version strings.
 *
 * Each version corresponds to a versioned grammar file in
 * `code/src/tokens/typescript/`. Omitting the version uses the generic
 * `typescript.tokens` grammar, which is suitable when you do not need
 * version-specific keyword sets.
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
 *   // => ".../code/src/tokens/typescript/ts5.8.tokens"
 *
 *   resolveTokensPath()
 *   // => ".../code/src/tokens/typescript.tokens"
 */
function resolveTokensPath(version?: string): string {
  if (!version) {
    // Generic grammar — same behaviour as v0.1.x.
    return join(TOKENS_DIR, "typescript.tokens");
  }

  if (!VALID_TS_VERSIONS.has(version)) {
    throw new Error(
      `Unknown TypeScript version "${version}". ` +
        `Valid values: ${[...VALID_TS_VERSIONS].join(", ")}`
    );
  }

  return join(TOKENS_DIR, "typescript", `${version}.tokens`);
}

/**
 * Tokenize TypeScript source code and return an array of tokens.
 *
 * @param source  - The TypeScript source code to tokenize.
 * @param version - Optional TypeScript version. When omitted (or empty string)
 *   the generic `typescript.tokens` grammar is used, which covers the union of
 *   all TypeScript keyword sets and is backwards-compatible with v0.1.x.
 *   Pass a specific version like `"ts5.8"` to use a version-exact grammar.
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
 * tokenization begins. This is useful when you need to attach an on-token
 * callback for context-sensitive lexing (e.g. distinguishing type-argument
 * `<` from less-than `<`).
 *
 * @param source  - The TypeScript source code to tokenize.
 * @param version - Optional TypeScript version (same semantics as
 *   `tokenizeTypescript`).
 * @returns A `GrammarLexer` instance ready to call `.tokenize()` on.
 *
 * @example
 *     const lexer = createTypescriptLexer("let x: T<number> = f<T>();", "ts5.8");
 *     lexer.setOnToken((token, ctx) => {
 *       // custom context-sensitive logic here
 *     });
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
