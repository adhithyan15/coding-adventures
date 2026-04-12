/**
 * C# Lexer — tokenizes C# source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads a C# `.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * C# has features that differ from JavaScript, Python, Java, and Ruby:
 * - Static typing with explicit type annotations (`int`, `string`, `bool`)
 * - Access modifiers (`public`, `private`, `protected`, `internal`)
 * - `class` and `namespace` as fundamental organizational units
 * - Semicolons terminate statements
 * - Curly braces `{}` for blocks
 * - `null` (C# also has nullable types: `int?`)
 * - Properties with `get` and `set` accessors
 * - LINQ query syntax (`from`, `where`, `select`)
 * - Attributes with `[AttributeName]` syntax
 * - `delegate`, `event`, `interface`, `struct`, `enum` keywords
 * - Null-coalescing operator `??` and null-conditional operator `?.`
 * - Lambda expressions with `=>` (fat arrow)
 * - `async`/`await` for asynchronous programming
 * - `var` for type inference
 * - String interpolation with `$"Hello {name}"`
 * - Pattern matching with `is` and `switch` expressions
 *
 * All of these are handled by the grammar file — no C#-specific
 * tokenization code exists in this module.
 *
 * Version Support
 * ---------------
 *
 * C# has evolved significantly since version 1.0 (2002). This module
 * supports selecting a specific edition grammar by version string:
 *
 * | Version string | Grammar file                            |
 * |----------------|-----------------------------------------|
 * | `"1.0"`        | `grammars/csharp/csharp1.0.tokens`     |
 * | `"2.0"`        | `grammars/csharp/csharp2.0.tokens`     |
 * | `"3.0"`        | `grammars/csharp/csharp3.0.tokens`     |
 * | `"4.0"`        | `grammars/csharp/csharp4.0.tokens`     |
 * | `"5.0"`        | `grammars/csharp/csharp5.0.tokens`     |
 * | `"6.0"`        | `grammars/csharp/csharp6.0.tokens`     |
 * | `"7.0"`        | `grammars/csharp/csharp7.0.tokens`     |
 * | `"8.0"`        | `grammars/csharp/csharp8.0.tokens`     |
 * | `"9.0"`        | `grammars/csharp/csharp9.0.tokens`     |
 * | `"10.0"`       | `grammars/csharp/csharp10.0.tokens`    |
 * | `"11.0"`       | `grammars/csharp/csharp11.0.tokens`    |
 * | `"12.0"`       | `grammars/csharp/csharp12.0.tokens`    |
 * | `undefined`    | `grammars/csharp/csharp12.0.tokens` (default) |
 *
 * When no version is supplied, C# 12.0 (the latest stable release as of
 * 2023, shipping with .NET 8.0 LTS) is used as the default.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `csharp*.tokens` files live in `code/grammars/csharp/` at the
 * repository root.
 *
 *     src/ -> csharp-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
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
 * Walk up: src/ -> csharp-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid C# version strings accepted by this module.
 *
 * C# follows a major.minor versioning scheme tied to .NET releases:
 *
 *   1.0  — .NET Framework 1.0 (2002): The original C#.
 *   2.0  — .NET Framework 2.0 (2005): Generics, nullable types, iterators.
 *   3.0  — .NET Framework 3.5 (2007): LINQ, lambda, anonymous types, `var`.
 *   4.0  — .NET Framework 4.0 (2010): `dynamic`, named/optional params.
 *   5.0  — .NET Framework 4.5 (2012): `async`/`await`.
 *   6.0  — .NET Framework 4.6 (2015): String interpolation, null-conditional.
 *   7.0  — .NET Framework 4.7 (2017): Tuples, pattern matching, `out` vars.
 *   8.0  — .NET Core 3.0 (2019): Nullable reference types, async streams.
 *   9.0  — .NET 5.0 (2020): Records, init-only properties, top-level programs.
 *   10.0 — .NET 6.0 LTS (2021): Global usings, file-scoped namespaces.
 *   11.0 — .NET 7.0 (2022): Required members, list patterns, raw strings.
 *   12.0 — .NET 8.0 LTS (2023): Primary constructors, collection expressions.
 */
const VALID_CSHARP_VERSIONS = new Set([
  "1.0",
  "2.0",
  "3.0",
  "4.0",
  "5.0",
  "6.0",
  "7.0",
  "8.0",
  "9.0",
  "10.0",
  "11.0",
  "12.0",
]);

/**
 * The default C# version used when no version is specified.
 * C# 12.0 ships with .NET 8.0, the latest Long-Term Support (LTS) release.
 */
const DEFAULT_CSHARP_VERSION = "12.0";

/**
 * Resolve the path to the C# token grammar for the given version.
 *
 * @param version - An optional C# version string (e.g. `"12.0"`, `"8.0"`,
 *   `"3.0"`). Pass `undefined` or `""` to use the default (C# 12.0).
 * @returns Absolute path to the `.tokens` grammar file.
 * @throws Error if `version` is a non-empty string that is not a recognised
 *   C# version identifier.
 *
 * @example
 *   resolveTokensPath("8.0")
 *   // => ".../code/grammars/csharp/csharp8.0.tokens"
 *
 *   resolveTokensPath()
 *   // => ".../code/grammars/csharp/csharp12.0.tokens"
 */
function resolveTokensPath(version?: string): string {
  // Default to C# 12.0 when no version is specified.
  const effectiveVersion = version || DEFAULT_CSHARP_VERSION;

  if (!VALID_CSHARP_VERSIONS.has(effectiveVersion)) {
    throw new Error(
      `Unknown C# version "${effectiveVersion}". ` +
        `Valid values: ${[...VALID_CSHARP_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "csharp", `csharp${effectiveVersion}.tokens`);
}

/**
 * Tokenize C# source code and return an array of tokens.
 *
 * @param source  - The C# source code to tokenize.
 * @param version - Optional C# version string. When omitted (or the
 *   empty string) C# 12.0 is used as the default, which ships with
 *   .NET 8.0 LTS. Pass a version like `"8.0"` or `"10.0"` to use an
 *   edition-exact grammar.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     // Default (C# 12.0)
 *     const tokens = tokenizeCSharp("class Hello { }");
 *
 *     // Version-specific
 *     const tokens = tokenizeCSharp("int x = 1;", "8.0");
 *     const tokens = tokenizeCSharp("var x = 1;", "3.0");
 *     const tokens = tokenizeCSharp("async Task M() { await Task.Delay(0); }", "5.0");
 */
export function tokenizeCSharp(source: string, version?: string): Token[] {
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}

/**
 * Create a `GrammarLexer` instance for C# source code.
 *
 * Unlike `tokenizeCSharp`, which eagerly produces the full token array,
 * `createCSharpLexer` returns the configured `GrammarLexer` object before
 * tokenization begins. This is useful when you need to attach an on-token
 * callback for context-sensitive lexing — for example, to track whether
 * you are inside a string interpolation expression `$"... {expr} ..."`.
 *
 * @param source  - The C# source code to tokenize.
 * @param version - Optional C# version string (same semantics as
 *   `tokenizeCSharp`).
 * @returns A `GrammarLexer` instance ready to call `.tokenize()` on.
 *
 * @example
 *     const lexer = createCSharpLexer("class Hello { }", "12.0");
 *     lexer.setOnToken((token, ctx) => { /* custom logic *\/ });
 *     const tokens = lexer.tokenize();
 */
export function createCSharpLexer(
  source: string,
  version?: string
): GrammarLexer {
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return new GrammarLexer(source, grammar);
}
