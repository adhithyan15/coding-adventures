/**
 * Haskell Parser — parses Haskell source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads a Haskell `.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * The Haskell grammar differs from HaskellScript, Python, and Ruby grammars in
 * several ways:
 * - Everything lives inside classes — `class` is the fundamental unit
 * - Static typing: `int x = 1;` instead of `let x = 1;`
 * - Access modifiers: `public`, `private`, `protected`
 * - Statements end with semicolons
 * - No function-level declarations outside classes (in standard Haskell)
 *
 * Version Support
 * ---------------
 *
 * This parser accepts the same version strings as `@coding-adventures/haskell-lexer`:
 *
 * | Version string  | Grammar files                              |
 * |-----------------|--------------------------------------------|
 * | `"1.0"`         | `grammars/haskell/haskell1.0.{tokens,grammar}`   |
 * | `"1.1"`         | `grammars/haskell/haskell1.1.{tokens,grammar}`   |
 * | `"1.4"`         | `grammars/haskell/haskell1.4.{tokens,grammar}`   |
 * | `"5"`           | `grammars/haskell/haskell5.{tokens,grammar}`     |
 * | `"7"`           | `grammars/haskell/haskell7.{tokens,grammar}`     |
 * | `"8"`           | `grammars/haskell/haskell8.{tokens,grammar}`     |
 * | `"10"`          | `grammars/haskell/haskell10.{tokens,grammar}`    |
 * | `"14"`          | `grammars/haskell/haskell14.{tokens,grammar}`    |
 * | `"17"`          | `grammars/haskell/haskell17.{tokens,grammar}`    |
 * | `"21"`          | `grammars/haskell/haskell21.{tokens,grammar}`    |
 * | `undefined`     | Haskell 21 (default)                          |
 *
 * Both the lexer tokens file and the parser grammar file are selected by
 * the version string, so tokens and grammar rules always come from the
 * same Haskell edition.
 *
 * When no version is supplied, Haskell 21 (the latest LTS) is used as the default.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `haskell*.grammar` files live in `code/grammars/haskell/` at the repository root.
 *
 *     src/ -> haskell-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeHaskell } from "@coding-adventures/haskell-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Root of the grammars directory.
 * Walk up: src/ -> haskell-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid Haskell version strings accepted by this module.
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
 * Resolve the path to the Haskell parser grammar for the given version.
 *
 * @param version - An optional Haskell version string. Pass `undefined` or `""`
 *   to use the default (Haskell 21).
 * @returns Absolute path to the `.grammar` file.
 * @throws Error if `version` is not a recognised Haskell version.
 */
function resolveGrammarPath(version?: string): string {
  const effectiveVersion = version || DEFAULT_HASKELL_VERSION;

  if (!VALID_HASKELL_VERSIONS.has(effectiveVersion)) {
    throw new Error(
      `Unknown Haskell version "${effectiveVersion}". ` +
        `Valid values: ${[...VALID_HASKELL_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "haskell", `haskell${effectiveVersion}.grammar`);
}

/**
 * Create a `GrammarParser` configured for Haskell source code.
 *
 * Unlike `parseHaskell`, which eagerly parses the full source, `createHaskellParser`
 * returns the configured `GrammarParser` object before parsing begins. This is
 * useful when you need more control over the parsing process.
 *
 * @param source  - The Haskell source code to parse.
 * @param version - Optional Haskell version string (same semantics as `parseHaskell`).
 * @returns A `GrammarParser` instance ready to call `.parse()` on.
 *
 * @example
 *     const parser = createHaskellParser("int x = 42;", "21");
 *     const ast = parser.parse();
 *     console.log(ast.ruleName); // "program"
 */
export function createHaskellParser(
  source: string,
  version?: string
): GrammarParser {
  const tokens = tokenizeHaskell(source, version);
  const grammarPath = resolveGrammarPath(version);
  const grammarText = readFileSync(grammarPath, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  return new GrammarParser(tokens, grammar);
}

/**
 * Parse Haskell source code and return an AST.
 *
 * @param source  - The Haskell source code to parse.
 * @param version - Optional Haskell version string (e.g. `"21"`, `"8"`, `"1.4"`).
 *   When omitted (or the empty string) Haskell 21 is used as the default — the
 *   latest Long-Term Support release. The version selects both the lexer tokens
 *   file and the parser grammar file, so they always match.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     // Default (Haskell 21)
 *     const ast = parseHaskell("class Hello { }");
 *
 *     // Version-specific
 *     const ast = parseHaskell("int x = 1 + 2;", "8");
 *     console.log(ast.ruleName); // "program"
 */
export function parseHaskell(source: string, version?: string): ASTNode {
  const parser = createHaskellParser(source, version);
  return parser.parse();
}
