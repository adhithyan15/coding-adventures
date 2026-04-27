/**
 * Java Parser — parses Java source code into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads a Java `.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * The Java grammar differs from JavaScript, Python, and Ruby grammars in
 * several ways:
 * - Everything lives inside classes — `class` is the fundamental unit
 * - Static typing: `int x = 1;` instead of `let x = 1;`
 * - Access modifiers: `public`, `private`, `protected`
 * - Statements end with semicolons
 * - No function-level declarations outside classes (in standard Java)
 *
 * Version Support
 * ---------------
 *
 * This parser accepts the same version strings as `@coding-adventures/java-lexer`:
 *
 * | Version string  | Grammar files                              |
 * |-----------------|--------------------------------------------|
 * | `"1.0"`         | `grammars/java/java1.0.{tokens,grammar}`   |
 * | `"1.1"`         | `grammars/java/java1.1.{tokens,grammar}`   |
 * | `"1.4"`         | `grammars/java/java1.4.{tokens,grammar}`   |
 * | `"5"`           | `grammars/java/java5.{tokens,grammar}`     |
 * | `"7"`           | `grammars/java/java7.{tokens,grammar}`     |
 * | `"8"`           | `grammars/java/java8.{tokens,grammar}`     |
 * | `"10"`          | `grammars/java/java10.{tokens,grammar}`    |
 * | `"14"`          | `grammars/java/java14.{tokens,grammar}`    |
 * | `"17"`          | `grammars/java/java17.{tokens,grammar}`    |
 * | `"21"`          | `grammars/java/java21.{tokens,grammar}`    |
 * | `undefined`     | Java 21 (default)                          |
 *
 * Both the lexer tokens file and the parser grammar file are selected by
 * the version string, so tokens and grammar rules always come from the
 * same Java edition.
 *
 * When no version is supplied, Java 21 (the latest LTS) is used as the default.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `java*.grammar` files live in `code/grammars/java/` at the repository root.
 *
 *     src/ -> java-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeJava } from "@coding-adventures/java-lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Root of the grammars directory.
 * Walk up: src/ -> java-parser/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid Java version strings accepted by this module.
 */
const VALID_JAVA_VERSIONS = new Set([
  "1.0",
  "1.1",
  "1.4",
  "5",
  "7",
  "8",
  "10",
  "14",
  "17",
  "21",
]);

/**
 * The default Java version used when no version is specified.
 * Java 21 is the latest Long-Term Support (LTS) release.
 */
const DEFAULT_JAVA_VERSION = "21";

/**
 * Resolve the path to the Java parser grammar for the given version.
 *
 * @param version - An optional Java version string. Pass `undefined` or `""`
 *   to use the default (Java 21).
 * @returns Absolute path to the `.grammar` file.
 * @throws Error if `version` is not a recognised Java version.
 */
function resolveGrammarPath(version?: string): string {
  const effectiveVersion = version || DEFAULT_JAVA_VERSION;

  if (!VALID_JAVA_VERSIONS.has(effectiveVersion)) {
    throw new Error(
      `Unknown Java version "${effectiveVersion}". ` +
        `Valid values: ${[...VALID_JAVA_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "java", `java${effectiveVersion}.grammar`);
}

/**
 * Create a `GrammarParser` configured for Java source code.
 *
 * Unlike `parseJava`, which eagerly parses the full source, `createJavaParser`
 * returns the configured `GrammarParser` object before parsing begins. This is
 * useful when you need more control over the parsing process.
 *
 * @param source  - The Java source code to parse.
 * @param version - Optional Java version string (same semantics as `parseJava`).
 * @returns A `GrammarParser` instance ready to call `.parse()` on.
 *
 * @example
 *     const parser = createJavaParser("int x = 42;", "21");
 *     const ast = parser.parse();
 *     console.log(ast.ruleName); // "program"
 */
export function createJavaParser(
  source: string,
  version?: string
): GrammarParser {
  const tokens = tokenizeJava(source, version);
  const grammarPath = resolveGrammarPath(version);
  const grammarText = readFileSync(grammarPath, "utf-8");
  const grammar = parseParserGrammar(grammarText);
  return new GrammarParser(tokens, grammar);
}

/**
 * Parse Java source code and return an AST.
 *
 * @param source  - The Java source code to parse.
 * @param version - Optional Java version string (e.g. `"21"`, `"8"`, `"1.4"`).
 *   When omitted (or the empty string) Java 21 is used as the default — the
 *   latest Long-Term Support release. The version selects both the lexer tokens
 *   file and the parser grammar file, so they always match.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     // Default (Java 21)
 *     const ast = parseJava("class Hello { }");
 *
 *     // Version-specific
 *     const ast = parseJava("int x = 1 + 2;", "8");
 *     console.log(ast.ruleName); // "program"
 */
export function parseJava(source: string, version?: string): ASTNode {
  const parser = createJavaParser(source, version);
  return parser.parse();
}
