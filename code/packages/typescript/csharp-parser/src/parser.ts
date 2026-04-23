/**
 * C# Parser — parses C# source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads a C# `.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * The C# grammar differs from JavaScript, Python, Java, and Ruby grammars in
 * several ways:
 * - Everything lives inside namespaces and classes
 * - Static typing: `int x = 1;` instead of `let x = 1;` or `var x = 1` (Ruby)
 * - Access modifiers: `public`, `private`, `protected`, `internal`
 * - Statements end with semicolons
 * - Properties with `get` / `set` accessors: `public int Age { get; set; }`
 * - LINQ query syntax: `from item in items where item > 0 select item`
 * - Delegates and events: `delegate void Handler(object sender, EventArgs e);`
 * - Generics: `List<T>`, `Dictionary<TKey, TValue>`
 * - Attributes on declarations: `[Serializable]`, `[HttpGet]`
 *
 * Version Support
 * ---------------
 *
 * This parser accepts the same version strings as `@coding-adventures/csharp-lexer`:
 *
 * | Version string  | Grammar files                                   |
 * |-----------------|-------------------------------------------------|
 * | `"1.0"`         | `grammars/csharp/csharp1.0.{tokens,grammar}`   |
 * | `"2.0"`         | `grammars/csharp/csharp2.0.{tokens,grammar}`   |
 * | `"3.0"`         | `grammars/csharp/csharp3.0.{tokens,grammar}`   |
 * | `"4.0"`         | `grammars/csharp/csharp4.0.{tokens,grammar}`   |
 * | `"5.0"`         | `grammars/csharp/csharp5.0.{tokens,grammar}`   |
 * | `"6.0"`         | `grammars/csharp/csharp6.0.{tokens,grammar}`   |
 * | `"7.0"`         | `grammars/csharp/csharp7.0.{tokens,grammar}`   |
 * | `"8.0"`         | `grammars/csharp/csharp8.0.{tokens,grammar}`   |
 * | `"9.0"`         | `grammars/csharp/csharp9.0.{tokens,grammar}`   |
 * | `"10.0"`        | `grammars/csharp/csharp10.0.{tokens,grammar}`  |
 * | `"11.0"`        | `grammars/csharp/csharp11.0.{tokens,grammar}`  |
 * | `"12.0"`        | `grammars/csharp/csharp12.0.{tokens,grammar}`  |
 * | `undefined`     | C# 12.0 (default)                               |
 *
 * Both the lexer tokens file and the parser grammar file are selected by
 * the version string, so tokens and grammar rules always come from the
 * same C# edition. This is important because, for example, the `record`
 * keyword (C# 9.0) or `required` modifier (C# 11.0) would not be
 * recognised by an earlier grammar version.
 *
 * When no version is supplied, C# 12.0 (shipping with .NET 8.0 LTS) is
 * used as the default.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `csharp*.grammar` files live in `code/grammars/csharp/` at the
 * repository root.
 *
 *     src/ -> csharp-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeCSharp } from "@coding-adventures/csharp-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Root of the grammars directory.
 * Walk up: src/ -> csharp-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid C# version strings accepted by this module.
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
 * Resolve the path to the C# parser grammar for the given version.
 *
 * @param version - An optional C# version string. Pass `undefined` or `""`
 *   to use the default (C# 12.0).
 * @returns Absolute path to the `.grammar` file.
 * @throws Error if `version` is not a recognised C# version.
 */
function resolveGrammarPath(version?: string): string {
  const effectiveVersion = version || DEFAULT_CSHARP_VERSION;

  if (!VALID_CSHARP_VERSIONS.has(effectiveVersion)) {
    throw new Error(
      `Unknown C# version "${effectiveVersion}". ` +
        `Valid values: ${[...VALID_CSHARP_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "csharp", `csharp${effectiveVersion}.grammar`);
}

/**
 * Create a `GrammarParser` configured for C# source code.
 *
 * Unlike `parseCSharp`, which eagerly parses the full source, `createCSharpParser`
 * returns the configured `GrammarParser` object before parsing begins. This is
 * useful when you need more control over the parsing process — for example, to
 * inspect the token stream before walking the AST.
 *
 * @param source  - The C# source code to parse.
 * @param version - Optional C# version string (same semantics as `parseCSharp`).
 * @returns A `GrammarParser` instance ready to call `.parse()` on.
 *
 * @example
 *     const parser = createCSharpParser("int x = 42;", "12.0");
 *     const ast = parser.parse();
 *     console.log(ast.ruleName); // "program"
 */
export function createCSharpParser(
  source: string,
  version?: string
): GrammarParser {
  const tokens = tokenizeCSharp(source, version);
  const grammarPath = resolveGrammarPath(version);
  const grammarText = readFileSync(grammarPath, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  return new GrammarParser(tokens, grammar);
}

/**
 * Parse C# source code and return an AST.
 *
 * @param source  - The C# source code to parse.
 * @param version - Optional C# version string (e.g. `"12.0"`, `"8.0"`, `"3.0"`).
 *   When omitted (or the empty string) C# 12.0 is used as the default — the
 *   latest .NET LTS release. The version selects both the lexer tokens file
 *   and the parser grammar file, so they always match.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     // Default (C# 12.0)
 *     const ast = parseCSharp("class Hello { }");
 *
 *     // Version-specific
 *     const ast = parseCSharp("int x = 1 + 2;", "8.0");
 *     console.log(ast.ruleName); // "program"
 */
export function parseCSharp(source: string, version?: string): ASTNode {
  const parser = createCSharpParser(source, version);
  return parser.parse();
}
