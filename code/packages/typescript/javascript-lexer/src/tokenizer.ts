/**
 * JavaScript Lexer — tokenizes JavaScript source code using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It selects a precompiled JavaScript
 * `TokenGrammar` and delegates all tokenization work to the generic engine.
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
 * | Version string | Compiled grammar module |
 * |-----------------|-------------------------|
 * | `"es1"`        | `./_grammar_es1.js`     |
 * | `"es3"`        | `./_grammar_es3.js`     |
 * | `"es5"`        | `./_grammar_es5.js`     |
 * | `"es2015"`     | `./_grammar_es2015.js`  |
 * | `"es2016"`     | `./_grammar_es2016.js`  |
 * | `"es2017"`     | `./_grammar_es2017.js`  |
 * | `"es2018"`     | `./_grammar_es2018.js`  |
 * | `"es2019"`     | `./_grammar_es2019.js`  |
 * | `"es2020"`     | `./_grammar_es2020.js`  |
 * | `"es2021"`     | `./_grammar_es2021.js`  |
 * | `"es2022"`     | `./_grammar_es2022.js`  |
 * | `"es2023"`     | `./_grammar_es2023.js`  |
 * | `"es2024"`     | `./_grammar_es2024.js`  |
 * | `"es2025"`     | `./_grammar_es2025.js`  |
 * | `undefined`/`""`| `./_grammar.js` (generic) |
 *
 * When no version is supplied the generic grammar is used, which covers the
 * broad intersection of JavaScript syntax — the same behaviour as v0.1.x.
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
import { TOKEN_GRAMMAR as ES1 } from "./_grammar_es1.js";
import { TOKEN_GRAMMAR as ES3 } from "./_grammar_es3.js";
import { TOKEN_GRAMMAR as ES5 } from "./_grammar_es5.js";
import { TOKEN_GRAMMAR as ES2015 } from "./_grammar_es2015.js";
import { TOKEN_GRAMMAR as ES2016 } from "./_grammar_es2016.js";
import { TOKEN_GRAMMAR as ES2017 } from "./_grammar_es2017.js";
import { TOKEN_GRAMMAR as ES2018 } from "./_grammar_es2018.js";
import { TOKEN_GRAMMAR as ES2019 } from "./_grammar_es2019.js";
import { TOKEN_GRAMMAR as ES2020 } from "./_grammar_es2020.js";
import { TOKEN_GRAMMAR as ES2021 } from "./_grammar_es2021.js";
import { TOKEN_GRAMMAR as ES2022 } from "./_grammar_es2022.js";
import { TOKEN_GRAMMAR as ES2023 } from "./_grammar_es2023.js";
import { TOKEN_GRAMMAR as ES2024 } from "./_grammar_es2024.js";
import { TOKEN_GRAMMAR as ES2025 } from "./_grammar_es2025.js";

/**
 * Valid ECMAScript version strings accepted by this module.
 *
 * ES1 through ES5 use the older "esN" naming convention. ES2015 and later
 * use the four-digit year naming introduced by TC39 when they moved to annual
 * releases.
 */
const VERSIONED_GRAMMARS: Record<string, TokenGrammar> = {
  es1: ES1,
  es3: ES3,
  es5: ES5,
  es2015: ES2015,
  es2016: ES2016,
  es2017: ES2017,
  es2018: ES2018,
  es2019: ES2019,
  es2020: ES2020,
  es2021: ES2021,
  es2022: ES2022,
  es2023: ES2023,
  es2024: ES2024,
  es2025: ES2025,
};

/**
 * Resolve the compiled `TokenGrammar` for the given version.
 *
 * @param version - An optional ECMAScript version string (e.g. `"es2015"`,
 *   `"es5"`). Pass `undefined` or `""` to use the generic grammar.
 * @returns The precompiled `TokenGrammar` object.
 * @throws Error if `version` is a non-empty string that is not a recognised
 *   ECMAScript edition identifier.
 *
 * @example
 *   resolveTokenGrammar("es2015")
 *   // => the compiled es2015 TokenGrammar
 *
 *   resolveTokenGrammar()
 *   // => the compiled generic TokenGrammar
 */
function resolveTokenGrammar(version?: string): TokenGrammar {
  if (!version) {
    // Generic grammar — same behaviour as v0.1.x.
    return GENERIC;
  }

  const grammar = VERSIONED_GRAMMARS[version];
  if (!grammar) {
    throw new Error(
      `Unknown JavaScript/ECMAScript version "${version}". ` +
        `Valid values: ${Object.keys(VERSIONED_GRAMMARS).join(", ")}`
    );
  }

  return grammar;
}

/**
 * Tokenize JavaScript source code and return an array of tokens.
 *
 * @param source  - The JavaScript source code to tokenize.
 * @param version - Optional ECMAScript edition string. When omitted (or the
 *   empty string) the generic grammar is used, which covers the union of
 *   all modern JS keyword sets and is backwards-compatible with v0.1.x.
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
  const grammar = resolveTokenGrammar(version);
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
  const grammar = resolveTokenGrammar(version);
  return new GrammarLexer(source, grammar);
}
