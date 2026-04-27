/**
 * Java Lexer — tokenizes Java source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads a Java `.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * Java has features that differ from JavaScript, Python, and Ruby:
 * - Static typing with explicit type annotations (`int`, `String`, `boolean`)
 * - Access modifiers (`public`, `private`, `protected`)
 * - `class` is the fundamental organizational unit
 * - Semicolons terminate statements
 * - Curly braces `{}` for blocks
 * - `null` (not `None` or `nil` or `undefined`)
 * - No `$` in identifiers (unlike JavaScript)
 * - `==` for equality (no `===` strict equality)
 * - Annotations with `@` prefix
 *
 * All of these are handled by the grammar file — no Java-specific
 * tokenization code exists in this module.
 *
 * Version Support
 * ---------------
 *
 * Java has evolved significantly since JDK 1.0 (1996). This module
 * supports selecting a specific edition grammar by version string:
 *
 * | Version string | Grammar file                       |
 * |----------------|------------------------------------|
 * | `"1.0"`        | `grammars/java/java1.0.tokens`     |
 * | `"1.1"`        | `grammars/java/java1.1.tokens`     |
 * | `"1.4"`        | `grammars/java/java1.4.tokens`     |
 * | `"5"`          | `grammars/java/java5.tokens`       |
 * | `"7"`          | `grammars/java/java7.tokens`       |
 * | `"8"`          | `grammars/java/java8.tokens`       |
 * | `"10"`         | `grammars/java/java10.tokens`      |
 * | `"14"`         | `grammars/java/java14.tokens`      |
 * | `"17"`         | `grammars/java/java17.tokens`      |
 * | `"21"`         | `grammars/java/java21.tokens`      |
 * | `undefined`    | `grammars/java/java21.tokens` (default) |
 *
 * When no version is supplied, Java 21 (the latest LTS) is used as the
 * default — this is the recommended grammar for most use cases.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `java*.tokens` files live in `code/grammars/java/` at the repository root.
 *
 *     src/ -> java-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
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
 * Walk up: src/ -> java-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid Java version strings accepted by this module.
 *
 * Early Java releases (1.0, 1.1, 1.4) use the "1.x" naming convention.
 * Starting with Java 5, Sun dropped the "1." prefix. Modern Java uses
 * just the major version number (5, 7, 8, 10, 14, 17, 21).
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
 * Resolve the path to the Java token grammar for the given version.
 *
 * @param version - An optional Java version string (e.g. `"21"`, `"8"`,
 *   `"1.4"`). Pass `undefined` or `""` to use the default (Java 21).
 * @returns Absolute path to the `.tokens` grammar file.
 * @throws Error if `version` is a non-empty string that is not a recognised
 *   Java version identifier.
 *
 * @example
 *   resolveTokensPath("8")
 *   // => ".../code/grammars/java/java8.tokens"
 *
 *   resolveTokensPath()
 *   // => ".../code/grammars/java/java21.tokens"
 */
function resolveTokensPath(version?: string): string {
  // Default to Java 21 when no version is specified.
  const effectiveVersion = version || DEFAULT_JAVA_VERSION;

  if (!VALID_JAVA_VERSIONS.has(effectiveVersion)) {
    throw new Error(
      `Unknown Java version "${effectiveVersion}". ` +
        `Valid values: ${[...VALID_JAVA_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "java", `java${effectiveVersion}.tokens`);
}

/**
 * Tokenize Java source code and return an array of tokens.
 *
 * @param source  - The Java source code to tokenize.
 * @param version - Optional Java version string. When omitted (or the
 *   empty string) Java 21 is used as the default, which is the latest
 *   Long-Term Support release.
 *   Pass a version like `"8"` or `"17"` to use an edition-exact grammar.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     // Default (Java 21)
 *     const tokens = tokenizeJava("class Hello { }");
 *
 *     // Version-specific
 *     const tokens = tokenizeJava("int x = 1;", "8");
 *     const tokens = tokenizeJava("var x = 1;", "10");
 */
export function tokenizeJava(source: string, version?: string): Token[] {
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}

/**
 * Create a `GrammarLexer` instance for Java source code.
 *
 * Unlike `tokenizeJava`, which eagerly produces the full token array,
 * `createJavaLexer` returns the configured `GrammarLexer` object before
 * tokenization begins. This is useful when you need to attach an on-token
 * callback for context-sensitive lexing.
 *
 * @param source  - The Java source code to tokenize.
 * @param version - Optional Java version string (same semantics as
 *   `tokenizeJava`).
 * @returns A `GrammarLexer` instance ready to call `.tokenize()` on.
 *
 * @example
 *     const lexer = createJavaLexer("class Hello { }", "21");
 *     lexer.setOnToken((token, ctx) => { /* custom logic *\/ });
 *     const tokens = lexer.tokenize();
 */
export function createJavaLexer(
  source: string,
  version?: string
): GrammarLexer {
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return new GrammarLexer(source, grammar);
}
