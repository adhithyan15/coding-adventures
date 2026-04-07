/**
 * JavaScript Lexer — tokenizes JavaScript source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads a JavaScript `.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * JavaScript has features that Python and Ruby do not:
 * - `let`, `const`, `var` for variable declarations
 * - `===` and `!==` for strict equality
 * - Semicolons terminate statements
 * - Curly braces `{}` for blocks
 * - `null` and `undefined` (not `None` or `nil`)
 * - `$` is valid in identifiers
 * - `=>` for arrow functions
 *
 * All of these are handled by the grammar file — no JavaScript-specific
 * tokenization code exists in this module.
 *
 * Version Support
 * ---------------
 *
 * ECMAScript has gone through many editions since ES1 (1997). This module
 * supports selecting a specific edition grammar by version string:
 *
 * | Version string | Grammar file                           |
 * |----------------|----------------------------------------|
 * | `"es1"`        | `grammars/ecmascript/es1.tokens`       |
 * | `"es3"`        | `grammars/ecmascript/es3.tokens`       |
 * | `"es5"`        | `grammars/ecmascript/es5.tokens`       |
 * | `"es2015"`     | `grammars/ecmascript/es2015.tokens`    |
 * | `"es2016"`     | `grammars/ecmascript/es2016.tokens`    |
 * | `"es2017"`     | `grammars/ecmascript/es2017.tokens`    |
 * | `"es2018"`     | `grammars/ecmascript/es2018.tokens`    |
 * | `"es2019"`     | `grammars/ecmascript/es2019.tokens`    |
 * | `"es2020"`     | `grammars/ecmascript/es2020.tokens`    |
 * | `"es2021"`     | `grammars/ecmascript/es2021.tokens`    |
 * | `"es2022"`     | `grammars/ecmascript/es2022.tokens`    |
 * | `"es2023"`     | `grammars/ecmascript/es2023.tokens`    |
 * | `"es2024"`     | `grammars/ecmascript/es2024.tokens`    |
 * | `"es2025"`     | `grammars/ecmascript/es2025.tokens`    |
 * | `undefined`/`""`| `grammars/javascript.tokens` (generic) |
 *
 * When no version is supplied the generic `javascript.tokens` grammar is used,
 * which covers the broad intersection of JavaScript syntax — the same behaviour
 * as v0.1.x.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `javascript.tokens` file lives in `code/grammars/` at the repository root.
 * Versioned grammars live in `code/grammars/ecmascript/`.
 *
 *     src/ -> javascript-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
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
 * Walk up: src/ -> javascript-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

/**
 * Valid ECMAScript version strings accepted by this module.
 *
 * ES1 through ES5 use the older "esN" naming convention. ES2015 and later
 * use the four-digit year naming introduced by TC39 when they moved to annual
 * releases.
 */
const VALID_ES_VERSIONS = new Set([
  "es1",
  "es3",
  "es5",
  "es2015",
  "es2016",
  "es2017",
  "es2018",
  "es2019",
  "es2020",
  "es2021",
  "es2022",
  "es2023",
  "es2024",
  "es2025",
]);

/**
 * Resolve the path to the JavaScript token grammar for the given version.
 *
 * @param version - An optional ECMAScript version string (e.g. `"es2015"`,
 *   `"es5"`). Pass `undefined` or `""` to use the generic grammar.
 * @returns Absolute path to the `.tokens` grammar file.
 * @throws Error if `version` is a non-empty string that is not a recognised
 *   ECMAScript edition identifier.
 *
 * @example
 *   resolveTokensPath("es2015")
 *   // => ".../code/grammars/ecmascript/es2015.tokens"
 *
 *   resolveTokensPath()
 *   // => ".../code/grammars/javascript.tokens"
 */
function resolveTokensPath(version?: string): string {
  if (!version) {
    // Generic grammar — same behaviour as v0.1.x.
    return join(GRAMMARS_DIR, "javascript.tokens");
  }

  if (!VALID_ES_VERSIONS.has(version)) {
    throw new Error(
      `Unknown JavaScript/ECMAScript version "${version}". ` +
        `Valid values: ${[...VALID_ES_VERSIONS].join(", ")}`
    );
  }

  return join(GRAMMARS_DIR, "ecmascript", `${version}.tokens`);
}

/**
 * Tokenize JavaScript source code and return an array of tokens.
 *
 * @param source  - The JavaScript source code to tokenize.
 * @param version - Optional ECMAScript edition string. When omitted (or the
 *   empty string) the generic `javascript.tokens` grammar is used, which
 *   covers the union of all modern JS keyword sets and is backwards-compatible
 *   with v0.1.x.
 *   Pass a version like `"es2015"` or `"es5"` to use an edition-exact grammar.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     // Generic (backwards-compatible)
 *     const tokens = tokenizeJavascript("let x = 1 + 2;");
 *
 *     // Version-specific
 *     const tokens = tokenizeJavascript("var x = 1 + 2;", "es5");
 *     const tokens = tokenizeJavascript("let x = 1 + 2;", "es2015");
 */
export function tokenizeJavascript(source: string, version?: string): Token[] {
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return grammarTokenize(source, grammar);
}

/**
 * Create a `GrammarLexer` instance for JavaScript source code.
 *
 * Unlike `tokenizeJavascript`, which eagerly produces the full token array,
 * `createJavascriptLexer` returns the configured `GrammarLexer` object before
 * tokenization begins. This is useful when you need to attach an on-token
 * callback for context-sensitive lexing.
 *
 * @param source  - The JavaScript source code to tokenize.
 * @param version - Optional ECMAScript edition string (same semantics as
 *   `tokenizeJavascript`).
 * @returns A `GrammarLexer` instance ready to call `.tokenize()` on.
 *
 * @example
 *     const lexer = createJavascriptLexer("let x = 1;", "es2015");
 *     lexer.setOnToken((token, ctx) => { /* custom logic *\/ });
 *     const tokens = lexer.tokenize();
 */
export function createJavascriptLexer(
  source: string,
  version?: string
): GrammarLexer {
  const tokensPath = resolveTokensPath(version);
  const grammarText = readFileSync(tokensPath, "utf-8");
  const grammar = parseTokenGrammar(grammarText);
  return new GrammarLexer(source, grammar);
}
