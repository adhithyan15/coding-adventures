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
 * Grammar Data
 * ------------
 *
 * The `haskell*.grammar` files under `code/grammars/haskell/` at the
 * repository root are compiled ahead of time into `./_grammar.ts` (the
 * default) and one `./_grammar_<version>.ts` per supported edition (see
 * `code/scripts/_ts_grammar_compile.ts`). This module statically imports
 * all of them and looks up the right one at call time — it never reads
 * the monorepo's `code/grammars/` tree at runtime, so a published npm
 * package works standalone.
 */

import type { ParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeHaskell } from "@coding-adventures/haskell-lexer";

import { PARSER_GRAMMAR as DEFAULT_GRAMMAR } from "./_grammar.js";
import { PARSER_GRAMMAR as V1_0 } from "./_grammar_1_0.js";
import { PARSER_GRAMMAR as V1_1 } from "./_grammar_1_1.js";
import { PARSER_GRAMMAR as V1_2 } from "./_grammar_1_2.js";
import { PARSER_GRAMMAR as V1_3 } from "./_grammar_1_3.js";
import { PARSER_GRAMMAR as V1_4 } from "./_grammar_1_4.js";
import { PARSER_GRAMMAR as V98 } from "./_grammar_98.js";
import { PARSER_GRAMMAR as V2010 } from "./_grammar_2010.js";

/**
 * Every supported Haskell edition's parser grammar, pre-compiled at build
 * time from the `.grammar` files in `code/grammars/haskell/`. ESM static
 * imports can't be conditional on a runtime string, so every version is
 * imported up front and looked up by key in `resolveParserGrammar`.
 */
const VERSIONED_GRAMMARS: Record<string, ParserGrammar> = {
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
 * Resolve the compiled Haskell parser grammar for the given version.
 *
 * @param version - An optional Haskell version string. Pass `undefined` or `""`
 *   to use the default (Haskell 2010).
 * @returns The pre-compiled `ParserGrammar` for that version.
 * @throws Error if `version` is not a recognised Haskell version.
 */
function resolveParserGrammar(version?: string): ParserGrammar {
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
  const grammar = resolveParserGrammar(version);
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
