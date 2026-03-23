/**
 * Browser-Compatible Lattice Transpiler
 * ======================================
 *
 * The Node.js lattice-transpiler package reads grammar files from disk using
 * `fs.readFileSync`. That works perfectly in Node.js but breaks in a browser
 * environment where the file system is not accessible.
 *
 * This module solves the problem by importing the grammar files as raw strings
 * at Vite build time (via the `?raw` suffix). Vite replaces the import with
 * an inline string constant, so at runtime in the browser there are no file
 * system reads — the grammar is already embedded in the JavaScript bundle.
 *
 * Architecture:
 *
 *   lattice.tokens  ──(Vite ?raw)──► string constant LATTICE_TOKENS_TEXT
 *   lattice.grammar ──(Vite ?raw)──► string constant LATTICE_GRAMMAR_TEXT
 *       │
 *       ▼
 *   parseTokenGrammar / parseParserGrammar  (grammar-tools, pure JS)
 *       │
 *       ▼
 *   GrammarLexer → GrammarParser  (lexer/parser packages, pure JS)
 *       │
 *       ▼
 *   LatticeTransformer → CSSEmitter  (lattice-ast-to-css, pure JS)
 *       │
 *       ▼
 *   CSS string  ◄────────────────────────────────────────────────────
 *
 * The key insight: the transformation engine (GrammarLexer, GrammarParser,
 * LatticeTransformer, CSSEmitter) is pure JavaScript/TypeScript with no
 * Node.js-specific APIs. Only the thin wrapper functions that *locate* and
 * *read* the grammar files are Node.js-specific. This module replaces those
 * wrappers with Vite-inlined constants.
 *
 * Note on @use directives:
 *
 * The @use module system reads external .lattice files from disk (Pass 1 of
 * the transformer). This is not supported in the browser playground because
 * there is no file system. Any @use directive will produce a LatticeError at
 * compile time. All other Lattice features work fully.
 */

// ── Grammar files inlined at build time by Vite ──────────────────────────────
//
// These imports look like normal file imports, but the `?raw` suffix tells
// Vite to treat the file as a raw text string and embed it as a string
// constant in the bundle. At runtime, LATTICE_TOKENS_TEXT and
// LATTICE_GRAMMAR_TEXT are already strings — no network requests needed.

import LATTICE_TOKENS_TEXT from "../../../../grammars/lattice.tokens?raw";
import LATTICE_GRAMMAR_TEXT from "../../../../grammars/lattice.grammar?raw";

// ── Grammar-tools: parse the text into structured grammar objects ─────────────

import {
  parseTokenGrammar,
  parseParserGrammar,
} from "@coding-adventures/grammar-tools";

// ── Lexer / Parser: generic grammar-driven engines ───────────────────────────

import { GrammarLexer } from "@coding-adventures/lexer";
import { GrammarParser } from "@coding-adventures/parser";

// ── Lattice-specific transformation engine ────────────────────────────────────
//
// These classes are pure TypeScript — no file I/O — so they work in browsers.

import {
  LatticeTransformer,
  CSSEmitter,
} from "@coding-adventures/lattice-ast-to-css";

// ── Pre-parse the grammars once at module load ────────────────────────────────
//
// Parsing a grammar is O(grammar_size) and the result is reused for every
// transpilation. Pre-parsing at module initialization means the first call
// to transpileLatticeBrowser() is just as fast as subsequent calls.

const tokenGrammar = parseTokenGrammar(LATTICE_TOKENS_TEXT);
const parserGrammar = parseParserGrammar(LATTICE_GRAMMAR_TEXT);

// ── Public API ────────────────────────────────────────────────────────────────

/** Options for the browser transpiler. */
export interface TranspileOptions {
  /** Emit minified CSS with no unnecessary whitespace. Default: false. */
  minified?: boolean;
  /** Indentation string for pretty-printed output. Default: "  " (2 spaces). */
  indent?: string;
}

/** Result returned by transpileLatticeBrowser(). */
export type TranspileResult =
  | { success: true; css: string }
  | { success: false; error: string };

/**
 * Transpile Lattice source text to CSS in the browser.
 *
 * This function is equivalent to the Node.js `transpileLattice()` function
 * from `@coding-adventures/lattice-transpiler`, but works without a file
 * system because the grammar files are embedded at build time.
 *
 * Three-pass compilation:
 *   1. Tokenize with the embedded lattice.tokens grammar
 *   2. Parse into an AST with the embedded lattice.grammar rules
 *   3. Transform: collect symbols, expand variables/mixins/control-flow
 *   4. Emit: render the clean CSS AST as a string
 *
 * @param source  - Lattice source text.
 * @param options - Formatting options (minified, indent).
 * @returns       An object with `success: true` and `css`, or `success: false`
 *                and an error message string suitable for display.
 *
 * @example
 *   const result = transpileLatticeBrowser("$color: red; .btn { color: $color; }");
 *   if (result.success) console.log(result.css);
 *   else console.error(result.error);
 */
export function transpileLatticeBrowser(
  source: string,
  options: TranspileOptions = {}
): TranspileResult {
  const { minified = false, indent = "  " } = options;

  try {
    // ── Step 1: Tokenize ─────────────────────────────────────────────────────
    //
    // The GrammarLexer converts the raw Lattice source text into a flat array
    // of Token objects. Whitespace and comments are automatically skipped by
    // the lexer's skip patterns defined in lattice.tokens.

    const tokens = new GrammarLexer(source, tokenGrammar).tokenize();

    // ── Step 2: Parse ────────────────────────────────────────────────────────
    //
    // The GrammarParser converts the token array into an Abstract Syntax Tree
    // (AST). The root node has rule_name "stylesheet" and contains child nodes
    // for every CSS rule and Lattice construct in the source.

    const ast = new GrammarParser(tokens, parserGrammar).parse();

    // ── Step 3: Transform ────────────────────────────────────────────────────
    //
    // The LatticeTransformer performs three passes over the AST:
    //   Pass 1 (Module Resolution): Resolve @use directives. In the browser
    //           playground, @use will raise LatticeError — no file system.
    //   Pass 2 (Symbol Collection): Extract variable/mixin/function definitions
    //           into registries. Remove definition nodes from the tree.
    //   Pass 3 (Expansion): Substitute $variables, inline @include mixins,
    //           evaluate @if/@for/@each control flow, call @function bodies.
    // The result is a clean CSS AST with no Lattice nodes.

    const transformer = new LatticeTransformer();
    const cssAst = transformer.transform(ast);

    // ── Step 4: Emit ─────────────────────────────────────────────────────────
    //
    // The CSSEmitter renders the clean CSS AST as a formatted string. Two modes:
    //   Pretty-print: 2-space indentation, blank lines between rules (default)
    //   Minified: no unnecessary whitespace, no line breaks

    const emitter = new CSSEmitter({ indent, minified });
    const css = emitter.emit(cssAst);

    return { success: true, css };
  } catch (err) {
    // Any LatticeError (undefined variable, wrong arity, circular reference,
    // etc.) or ParseError from the grammar engine is caught here and returned
    // as a human-readable error message.
    const message =
      err instanceof Error ? err.message : "Unknown compilation error";
    return { success: false, error: message };
  }
}
