/**
 * @coding-adventures/lattice-transpiler
 *
 * End-to-end Lattice source-to-CSS pipeline.
 *
 * This package is the public face of the Lattice compiler. It chains together
 * the three underlying packages to produce CSS from Lattice source text in a
 * single function call:
 *
 *     Lattice source
 *         ↓  parseLattice()            (lattice-parser)
 *     Lattice AST
 *         ↓  LatticeTransformer        (lattice-ast-to-css)
 *     CSS AST
 *         ↓  CSSEmitter                (lattice-ast-to-css)
 *     CSS text
 *
 * Architecture Overview
 * ---------------------
 *
 * lattice-transpiler
 *   └─ lattice-parser      — tokenize + parse Lattice source → ASTNode tree
 *       └─ lattice-lexer   — tokenize using lattice.tokens grammar file
 *   └─ lattice-ast-to-css
 *       └─ LatticeTransformer — three-pass expansion (vars, mixins, control flow)
 *       └─ CSSEmitter         — AST → formatted CSS text
 *
 * The transpiler is intentionally thin. All the real work happens in the
 * packages above. The transpiler simply wires them together and exposes a
 * clean, minimal public API.
 *
 * Design Decision: Single Function vs Class
 * -----------------------------------------
 *
 * The main export is a function `transpileLattice()`, not a class. This
 * follows the "function for simple cases, class for stateful cases" principle.
 * The transpiler has no meaningful state — each call is independent.
 *
 * Options
 * -------
 *
 * The `TranspileOptions` interface controls output formatting:
 *
 *   minified — Produce compact CSS with no whitespace (default: false).
 *              Useful for production builds where file size matters.
 *
 *   indent   — The indentation string per nesting level (default: "  ").
 *              Ignored when minified is true. Set to "\t" for tab indentation.
 *
 * Usage
 * -----
 *
 *     import { transpileLattice } from "@coding-adventures/lattice-transpiler";
 *
 *     const css = transpileLattice(`
 *       $brand: #4a90d9;
 *       h1 { color: $brand; }
 *     `);
 *     // "h1 {\n  color: #4a90d9;\n}\n"
 *
 *     const minified = transpileLattice("h1 { color: red; }", { minified: true });
 *     // "h1{color:red;}"
 *
 * Browser Usage
 * -------------
 *
 * This module uses Node.js `fs.readFileSync()` to load the grammar files at
 * runtime. It does NOT work in a browser environment without bundling.
 *
 * For browser usage, import from the browser entry point instead:
 *
 *     import { transpileLatticeInBrowser } from
 *       "@coding-adventures/lattice-transpiler/src/browser.js";
 *
 * The browser entry point embeds the grammar strings directly, so no file
 * system access is needed.
 */

import { parseLattice } from "@coding-adventures/lattice-parser";
import { LatticeTransformer, CSSEmitter } from "@coding-adventures/lattice-ast-to-css";

// =============================================================================
// Public API Types
// =============================================================================

/**
 * Options controlling the CSS output format.
 *
 * Both options are optional and have sensible defaults for development use.
 * Production builds typically want `minified: true`.
 */
export interface TranspileOptions {
  /**
   * If true, emit compact CSS with no unnecessary whitespace.
   *
   * Pretty-print (default):
   *   h1 {
   *     color: red;
   *   }
   *
   * Minified:
   *   h1{color:red;}
   */
  minified?: boolean;

  /**
   * Indentation string per nesting level. Default: "  " (two spaces).
   *
   * Examples:
   *   "  "   — 2-space indent (default, matches most CSS style guides)
   *   "    " — 4-space indent
   *   "\t"   — tab indent
   *
   * Ignored when `minified` is true.
   */
  indent?: string;
}

// =============================================================================
// Main Entry Point
// =============================================================================

/**
 * Transpile Lattice source text to CSS.
 *
 * This function runs the full Lattice compiler pipeline:
 *
 * 1. **Parse**: Tokenize the source with the Lattice lexer, then parse with
 *    the grammar-driven recursive descent parser to produce an ASTNode tree.
 *
 * 2. **Transform**: Run the three-pass transformer over the AST:
 *    - Pass 1: Collect all variable/mixin/function definitions.
 *    - Pass 2: Expand all Lattice constructs (variable refs, @include, @if,
 *              @for, @each, @function calls) into pure CSS.
 *    - Pass 3: Remove any empty nodes left by the expansion.
 *
 * 3. **Emit**: Walk the clean CSS AST and produce formatted CSS text.
 *
 * @param source - Lattice source text. May be empty ("" → returns "").
 * @param options - Optional output formatting options.
 * @returns A CSS string. Always ends with a newline if non-empty.
 *          Returns "" for empty or whitespace-only input.
 *
 * @throws {GrammarParseError}      If the Lattice source has syntax errors.
 * @throws {LexerError}             If the source contains unrecognized characters.
 * @throws {UndefinedVariableError} If a variable is used but not defined.
 * @throws {UndefinedMixinError}    If @include references an undefined mixin.
 * @throws {UndefinedFunctionError} If a function call is not defined and not a CSS built-in.
 * @throws {WrongArityError}        If a mixin/function is called with the wrong number of args.
 * @throws {CircularReferenceError} If variable/mixin definitions form a cycle.
 * @throws {TypeErrorInExpression}  If an expression contains a type mismatch.
 * @throws {UnitMismatchError}      If arithmetic mixes incompatible units (e.g., px + em).
 * @throws {MissingReturnError}     If a @function body has no @return statement.
 *
 * @example
 *     // Basic variable substitution
 *     const css = transpileLattice("$color: red; h1 { color: $color; }");
 *     // → "h1 {\n  color: red;\n}\n"
 *
 * @example
 *     // Mixin expansion
 *     const css = transpileLattice(`
 *       @mixin flex-center {
 *         display: flex;
 *         align-items: center;
 *         justify-content: center;
 *       }
 *       .hero { @include flex-center; }
 *     `);
 *     // → ".hero {\n  display: flex;\n  align-items: center;\n  ...\n}\n"
 *
 * @example
 *     // Minified output
 *     const css = transpileLattice("$c: red; p { color: $c; }", { minified: true });
 *     // → "p{color:red;}"
 *
 * @example
 *     // Custom indent (4 spaces)
 *     const css = transpileLattice("div { color: red; }", { indent: "    " });
 *     // → "div {\n    color: red;\n}\n"
 *
 * @example
 *     // Control flow
 *     const css = transpileLattice(`
 *       $theme: dark;
 *       @if $theme == dark {
 *         body { background: #1a1a1a; }
 *       } @else {
 *         body { background: white; }
 *       }
 *     `);
 *     // → "body {\n  background: #1a1a1a;\n}\n"
 *
 * @example
 *     // Loop
 *     const css = transpileLattice(`
 *       @for $i from 1 through 3 {
 *         .item-#{$i} { order: $i; }
 *       }
 *     `);
 *     // → ".item-1 { order: 1; }\n\n.item-2 { order: 2; }\n\n.item-3 { order: 3; }\n"
 */
export function transpileLattice(
  source: string,
  options: TranspileOptions = {}
): string {
  // --- Step 1: Parse ---
  //
  // parseLattice() chains the lexer and parser:
  //   source → lattice-lexer (lattice.tokens) → tokens
  //   tokens → GrammarParser (lattice.grammar) → ASTNode
  //
  // The result is a "stylesheet" ASTNode whose children are "rule" nodes.
  const ast = parseLattice(source);

  // --- Step 2: Transform ---
  //
  // LatticeTransformer runs three passes over the AST, mutating it in place.
  // After this step, the AST contains only CSS nodes (no Lattice constructs).
  const transformer = new LatticeTransformer();
  const cssAst = transformer.transform(ast);

  // --- Step 3: Emit ---
  //
  // CSSEmitter walks the clean CSS AST and produces formatted CSS text.
  // The indent and minified options control formatting.
  const indent = options.indent ?? "  ";
  const minified = options.minified ?? false;
  const emitter = new CSSEmitter(indent, minified);

  return emitter.emit(cssAst);
}

// =============================================================================
// Re-exports for convenience
// =============================================================================

/**
 * Re-export all error classes so consumers can do:
 *
 *     import { transpileLattice, UndefinedVariableError } from
 *       "@coding-adventures/lattice-transpiler";
 *
 * without having to import from lattice-ast-to-css directly.
 */
export {
  LatticeError,
  LatticeModuleNotFoundError,
  ReturnOutsideFunctionError,
  UndefinedVariableError,
  UndefinedMixinError,
  UndefinedFunctionError,
  WrongArityError,
  CircularReferenceError,
  TypeErrorInExpression,
  UnitMismatchError,
  MissingReturnError,
} from "@coding-adventures/lattice-ast-to-css";

/**
 * Re-export the lower-level building blocks for consumers that need
 * fine-grained control over the pipeline.
 */
export { LatticeTransformer, CSSEmitter } from "@coding-adventures/lattice-ast-to-css";
export { parseLattice } from "@coding-adventures/lattice-parser";

export const VERSION = "0.1.0";
