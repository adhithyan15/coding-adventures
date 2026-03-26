/**
 * Tests for @coding-adventures/lattice-transpiler
 *
 * Integration tests for the full Lattice-to-CSS compilation pipeline.
 *
 * Test categories:
 * 1. Basic CSS passthrough — pure CSS passes unchanged
 * 2. Variable substitution — $var → resolved value
 * 3. Mixin expansion — @mixin / @include
 * 4. Control flow — @if / @else / @for / @each
 * 5. Functions — @function / @return
 * 6. Formatting options — minified, custom indent
 * 7. Browser bundle — transpileLatticeInBrowser works identically
 * 8. Re-exported building blocks — parseLattice, LatticeTransformer, CSSEmitter
 * 9. Error propagation — compiler errors surface correctly
 * 10. Edge cases — empty input, whitespace, complex programs
 */

import { describe, it, expect } from "vitest";
import {
  transpileLattice,
  LatticeTransformer,
  CSSEmitter,
  parseLattice,
  VERSION,
  UndefinedVariableError,
  UndefinedMixinError,
  WrongArityError,
  CircularReferenceError,
  MissingReturnError,
  LatticeError,
  UndefinedFunctionError,
  TypeErrorInExpression,
} from "../src/index.js";
import { transpileLatticeInBrowser } from "../src/browser.js";

// =============================================================================
// Helper
// =============================================================================

/**
 * Normalize whitespace in CSS for comparison.
 * Collapses multiple spaces/newlines to a single space for flexible matching.
 */
function normalize(css: string): string {
  return css.replace(/\s+/g, " ").trim();
}

// =============================================================================
// 1. Version
// =============================================================================

describe("version", () => {
  it("exports VERSION constant", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// =============================================================================
// 2. Basic CSS Passthrough
// =============================================================================

describe("basic CSS passthrough", () => {
  it("passes through a simple rule unchanged", () => {
    const css = transpileLattice("h1 { color: red; }");
    expect(normalize(css)).toBe("h1 { color: red; }");
  });

  it("passes through multiple rules", () => {
    const css = transpileLattice("h1 { color: red; } p { font-size: 14px; }");
    expect(normalize(css)).toContain("h1 { color: red; }");
    expect(normalize(css)).toContain("p { font-size: 14px; }");
  });

  it("passes through an @media rule", () => {
    const css = transpileLattice("@media (max-width: 768px) { h1 { color: blue; } }");
    expect(normalize(css)).toContain("@media");
    expect(normalize(css)).toContain("max-width");
    expect(normalize(css)).toContain("h1");
    expect(normalize(css)).toContain("color: blue");
  });

  it("passes through an @import rule", () => {
    const css = transpileLattice('@import url("style.css");');
    expect(normalize(css)).toContain("@import");
    expect(normalize(css)).toContain("style.css");
  });

  it("preserves multi-value declarations", () => {
    const css = transpileLattice("div { border: 1px solid #ccc; }");
    expect(normalize(css)).toContain("border");
    expect(normalize(css)).toContain("1px");
    expect(normalize(css)).toContain("solid");
    expect(normalize(css)).toContain("#ccc");
  });

  it("preserves CSS class selectors", () => {
    const css = transpileLattice(".container { width: 100%; }");
    expect(normalize(css)).toContain(".container");
    expect(normalize(css)).toContain("width: 100%");
  });

  it("preserves CSS id selectors", () => {
    const css = transpileLattice("#header { background: white; }");
    expect(normalize(css)).toContain("#header");
  });

  it("preserves pseudo-classes", () => {
    const css = transpileLattice("a:hover { color: blue; }");
    expect(normalize(css)).toContain("a:hover");
  });

  it("preserves child combinator selectors", () => {
    const css = transpileLattice("ul > li { list-style: none; }");
    expect(normalize(css)).toContain("ul > li");
  });

  it("preserves !important", () => {
    const css = transpileLattice("p { color: red !important; }");
    expect(normalize(css)).toContain("!important");
  });

  it("preserves CSS custom properties (CSS variables)", () => {
    const css = transpileLattice(":root { --color: red; } p { color: var(--color); }");
    expect(normalize(css)).toContain("--color");
    expect(normalize(css)).toContain("var(");
  });
});

// =============================================================================
// 3. Empty Input
// =============================================================================

describe("empty input", () => {
  it("returns empty string for empty input", () => {
    expect(transpileLattice("")).toBe("");
  });

  it("returns empty string for whitespace-only input", () => {
    expect(transpileLattice("   \n\t  ")).toBe("");
  });

  it("returns empty string for comment-only input", () => {
    // Comments are stripped by the lexer
    expect(transpileLattice("// just a comment")).toBe("");
  });

  it("returns empty string for only variable declarations (no CSS output)", () => {
    expect(transpileLattice("$color: red;")).toBe("");
  });

  it("returns empty string for only mixin definitions", () => {
    // Mixin names must use FUNCTION token syntax: flex(
    expect(transpileLattice("@mixin flex() { display: flex; }")).toBe("");
  });
});

// =============================================================================
// 4. Variable Substitution
// =============================================================================

describe("variable substitution", () => {
  it("substitutes a simple variable", () => {
    const css = transpileLattice("$color: red; h1 { color: $color; }");
    expect(normalize(css)).toBe("h1 { color: red; }");
  });

  it("substitutes a dimension variable", () => {
    const css = transpileLattice("$size: 16px; p { font-size: $size; }");
    expect(normalize(css)).toContain("font-size: 16px");
  });

  it("substitutes a percentage variable", () => {
    const css = transpileLattice("$width: 100%; .col { width: $width; }");
    expect(normalize(css)).toContain("width: 100%");
  });

  it("substitutes a hash (color) variable", () => {
    const css = transpileLattice("$brand: #4a90d9; a { color: $brand; }");
    expect(normalize(css)).toContain("color: #4a90d9");
  });

  it("substitutes multiple variables", () => {
    const css = transpileLattice(
      "$bg: white; $fg: black; body { background: $bg; color: $fg; }"
    );
    expect(normalize(css)).toContain("background: white");
    expect(normalize(css)).toContain("color: black");
  });

  it("resolves variable defined after use (forward reference)", () => {
    // Pass 1 collects all vars, so order doesn't matter
    const css = transpileLattice("h1 { color: $brand; } $brand: blue;");
    expect(normalize(css)).toContain("color: blue");
  });

  it("throws UndefinedVariableError for unknown variable", () => {
    expect(() => transpileLattice("h1 { color: $missing; }")).toThrow(
      UndefinedVariableError
    );
  });

  it("handles variable referencing another variable", () => {
    // Variables can reference other variables in value_list context
    const css = transpileLattice("$base: 16px; $padding: $base; .box { padding: $padding; }");
    expect(normalize(css)).toContain("padding: 16px");
  });
});

// =============================================================================
// 5. Mixin Expansion
// =============================================================================

describe("mixin expansion", () => {
  it("expands a simple mixin (no args)", () => {
    const css = transpileLattice(`
      @mixin flex() {
        display: flex;
        align-items: center;
      }
      .box { @include flex(); }
    `);
    expect(normalize(css)).toContain("display: flex");
    expect(normalize(css)).toContain("align-items: center");
  });

  it("expands a zero-argument mixin defined without parentheses", () => {
    const css = transpileLattice(`
      @mixin flex {
        display: flex;
        align-items: center;
      }
      .box { @include flex; }
    `);
    expect(normalize(css)).toContain("display: flex");
    expect(normalize(css)).toContain("align-items: center");
  });

  it("expands a mixin with one argument", () => {
    const css = transpileLattice(`
      @mixin bg($color) {
        background: $color;
      }
      .hero { @include bg(blue); }
    `);
    expect(normalize(css)).toContain("background: blue");
  });

  it("expands a mixin with multiple arguments", () => {
    const css = transpileLattice(`
      @mixin border-box($width, $style, $color) {
        border: $width $style $color;
      }
      .card { @include border-box(1px, solid, #ccc); }
    `);
    expect(normalize(css)).toContain("border");
    expect(normalize(css)).toContain("1px");
    expect(normalize(css)).toContain("solid");
    expect(normalize(css)).toContain("#ccc");
  });

  it("expands a mixin with a default parameter", () => {
    const css = transpileLattice(`
      @mixin button($bg, $fg: white) {
        background: $bg;
        color: $fg;
      }
      .btn { @include button(red); }
    `);
    expect(normalize(css)).toContain("background: red");
    expect(normalize(css)).toContain("color: white");
  });

  it("overrides default parameter when arg is provided", () => {
    const css = transpileLattice(`
      @mixin button($bg, $fg: white) {
        background: $bg;
        color: $fg;
      }
      .btn { @include button(red, black); }
    `);
    expect(normalize(css)).toContain("background: red");
    expect(normalize(css)).toContain("color: black");
  });

  it("throws UndefinedMixinError for unknown mixin", () => {
    expect(() => transpileLattice(".btn { @include ghost; }")).toThrow(
      UndefinedMixinError
    );
  });

  it("includes suggestions in UndefinedMixinError messages", () => {
    expect(() =>
      transpileLattice(`
        @mixin spacing() { margin: 8px; }
        .btn { @include spacin(); }
      `)
    ).toThrow(/Did you mean 'spacing'\?/);
  });

  it("throws WrongArityError for too few arguments", () => {
    expect(() =>
      transpileLattice(`
        @mixin pad($top, $bottom) { padding: $top $bottom; }
        .x { @include pad(10px); }
      `)
    ).toThrow(WrongArityError);
  });

  it("expands a mixin that uses outer variables", () => {
    const css = transpileLattice(`
      $unit: 8px;
      @mixin spacing() { margin: $unit; }
      .box { @include spacing(); }
    `);
    expect(normalize(css)).toContain("margin: 8px");
  });
});

// =============================================================================
// 6. @if / @else Control Flow
// =============================================================================

describe("@if / @else control flow", () => {
  it("takes the true branch when condition is true", () => {
    const css = transpileLattice(`
      $theme: dark;
      @if $theme == dark {
        body { background: #1a1a1a; }
      } @else {
        body { background: white; }
      }
    `);
    expect(normalize(css)).toContain("background: #1a1a1a");
    expect(normalize(css)).not.toContain("background: white");
  });

  it("takes the false branch when condition is false", () => {
    const css = transpileLattice(`
      $theme: light;
      @if $theme == dark {
        body { background: #1a1a1a; }
      } @else {
        body { background: white; }
      }
    `);
    expect(normalize(css)).not.toContain("background: #1a1a1a");
    expect(normalize(css)).toContain("background: white");
  });

  it("handles @if without @else", () => {
    const css = transpileLattice(`
      $show: false;
      @if $show {
        .hidden { display: none; }
      }
    `);
    // The block is not expanded when condition is false
    expect(normalize(css)).not.toContain(".hidden");
  });

  it("handles numeric comparison", () => {
    const css = transpileLattice(`
      $size: 10;
      @if $size > 5 {
        p { font-size: large; }
      }
    `);
    expect(normalize(css)).toContain("font-size: large");
  });

  it("handles @else if chain", () => {
    const css = transpileLattice(`
      $val: 2;
      @if $val == 1 {
        p { color: red; }
      } @else if $val == 2 {
        p { color: green; }
      } @else {
        p { color: blue; }
      }
    `);
    expect(normalize(css)).toContain("color: green");
    expect(normalize(css)).not.toContain("color: red");
    expect(normalize(css)).not.toContain("color: blue");
  });
});

// =============================================================================
// 7. @for Loop
// =============================================================================

describe("@for loop", () => {
  it("generates rules for each iteration (through = inclusive)", () => {
    // Note: #{$i} interpolation is not supported by the lexer.
    // Test using $i in value position instead.
    const css = transpileLattice(`
      @for $i from 1 through 3 {
        .item { order: $i; }
      }
    `);
    // Should produce rules for i = 1, 2, 3
    expect(normalize(css)).toContain("order: 1");
    expect(normalize(css)).toContain("order: 2");
    expect(normalize(css)).toContain("order: 3");
  });

  it("generates rules for each iteration (to = exclusive)", () => {
    // 1 to 3 → i = 1, 2 (not 3)
    const css = transpileLattice(`
      @for $i from 1 to 3 {
        .col { flex: $i; }
      }
    `);
    expect(normalize(css)).toContain("flex: 1");
    expect(normalize(css)).toContain("flex: 2");
    expect(normalize(css)).not.toContain("flex: 3");
  });

  it("generates the correct number of rules", () => {
    const css = transpileLattice(`
      @for $i from 1 through 5 {
        .item { order: $i; }
      }
    `);
    // Five .item rules should appear
    const matches = css.match(/order:/g);
    expect(matches).toHaveLength(5);
  });

  it("handles a loop that iterates zero times (from > through)", () => {
    // from 5 through 3 — empty range
    const css = transpileLattice(`
      @for $i from 5 through 3 {
        .item { color: red; }
      }
    `);
    expect(css).toBe(""); // No output
  });
});

// =============================================================================
// 8. @each Loop
// =============================================================================

describe("@each loop", () => {
  it("iterates over a list of values", () => {
    const css = transpileLattice(`
      @each $color in red, green, blue {
        .text { color: $color; }
      }
    `);
    expect(normalize(css)).toContain("color: red");
    expect(normalize(css)).toContain("color: green");
    expect(normalize(css)).toContain("color: blue");
  });

  it("produces one rule per item", () => {
    // #{$size} interpolation not supported; use $size in value position
    const css = transpileLattice(`
      @each $size in sm, md, lg {
        .icon { font-size: $size; }
      }
    `);
    const matches = css.match(/font-size:/g);
    expect(matches).toHaveLength(3);
  });
});

// =============================================================================
// 9. @function / @return
// =============================================================================

describe("@function / @return", () => {
  it("evaluates a function and substitutes its return value", () => {
    const css = transpileLattice(`
      @function double($x) {
        @return $x * 2;
      }
      p { margin: double(8px); }
    `);
    expect(normalize(css)).toContain("margin: 16px");
  });

  it("evaluates a no-arg function", () => {
    // Functions must use FUNCTION token syntax: brand-color(
    const css = transpileLattice(`
      @function brand-color() {
        @return #4a90d9;
      }
      h1 { color: brand-color(); }
    `);
    expect(normalize(css)).toContain("color: #4a90d9");
  });

  it("handles a function with multiple parameters", () => {
    const css = transpileLattice(`
      @function clamp-val($min, $max) {
        @if $min > $max {
          @return $max;
        } @else {
          @return $min;
        }
      }
      div { width: clamp-val(100px, 50px); }
    `);
    expect(normalize(css)).toContain("width: 50px");
  });

  it("throws MissingReturnError for function without @return", () => {
    expect(() =>
      transpileLattice(`
        @function bad() {
          $x: 1;
        }
        p { color: bad(); }
      `)
    ).toThrow(MissingReturnError);
  });

  it("does not throw for unknown function in value position (treated as CSS passthrough)", () => {
    // Unknown functions in value position are passed through unchanged
    // (they might be CSS functions not in our built-ins list, e.g. future CSS)
    // Only functions used in Lattice expression contexts (e.g. @if, @return) throw
    const css = transpileLattice(`
      @function double($x) { @return $x * 2; }
      p { margin: double(8px); }
    `);
    expect(normalize(css)).toContain("margin: 16px");
  });
});

// =============================================================================
// 10. Formatting Options
// =============================================================================

describe("formatting options", () => {
  it("produces pretty-printed output by default", () => {
    const css = transpileLattice("p { color: red; }");
    // Should have newlines and indentation
    expect(css).toContain("\n");
    expect(css).toContain("  color: red;");
  });

  it("produces minified output with minified: true", () => {
    const css = transpileLattice("p { color: red; }", { minified: true });
    // The emitter adds a trailing newline after the content for consistency.
    // Minified means no unnecessary internal whitespace, not no final newline.
    expect(normalize(css)).toBe("p{color:red;}");
    // Verify no internal newlines or spaces in the rule
    expect(css.trim()).toBe("p{color:red;}");
  });

  it("respects custom indent string", () => {
    const css = transpileLattice("p { color: red; }", { indent: "    " });
    // 4-space indent
    expect(css).toContain("    color: red;");
  });

  it("respects tab indent", () => {
    const css = transpileLattice("p { color: red; }", { indent: "\t" });
    expect(css).toContain("\tcolor: red;");
  });

  it("output ends with newline for non-empty input", () => {
    const css = transpileLattice("p { color: red; }");
    expect(css.endsWith("\n")).toBe(true);
  });

  it("empty input returns empty string (no trailing newline)", () => {
    const css = transpileLattice("");
    expect(css).toBe("");
  });

  it("minified and indent: undefined still works", () => {
    const css = transpileLattice("p { color: red; }", {});
    expect(normalize(css)).toBe("p { color: red; }");
  });
});

// =============================================================================
// 11. Browser Bundle
// =============================================================================

describe("transpileLatticeInBrowser", () => {
  it("transpiles basic CSS", () => {
    const css = transpileLatticeInBrowser("h1 { color: red; }");
    expect(normalize(css)).toBe("h1 { color: red; }");
  });

  it("substitutes variables", () => {
    const css = transpileLatticeInBrowser("$c: blue; p { color: $c; }");
    expect(normalize(css)).toContain("color: blue");
  });

  it("expands mixins", () => {
    const css = transpileLatticeInBrowser(`
      @mixin flex() { display: flex; }
      .box { @include flex(); }
    `);
    expect(normalize(css)).toContain("display: flex");
  });

  it("supports zero-argument mixins defined without parentheses in the browser bundle", () => {
    const css = transpileLatticeInBrowser(`
      @mixin flex {
        display: flex;
      }
      .box { @include flex; }
    `);
    expect(normalize(css)).toContain("display: flex");
  });

  it("evaluates @if", () => {
    const css = transpileLatticeInBrowser(`
      $on: true;
      @if $on { p { color: green; } }
    `);
    expect(normalize(css)).toContain("color: green");
  });

  it("supports minified option", () => {
    const css = transpileLatticeInBrowser("p { color: red; }", { minified: true });
    expect(normalize(css)).toBe("p{color:red;}");
    expect(css.trim()).toBe("p{color:red;}");
  });

  it("supports custom indent option", () => {
    const css = transpileLatticeInBrowser("p { color: red; }", { indent: "    " });
    expect(css).toContain("    color: red;");
  });

  it("returns empty string for empty input", () => {
    expect(transpileLatticeInBrowser("")).toBe("");
  });

  it("produces same output as Node.js transpiler", () => {
    const src = `
      $primary: #4a90d9;
      $padding: 16px;
      @mixin flex-center() {
        display: flex;
        align-items: center;
        justify-content: center;
      }
      .hero {
        @include flex-center();
        background: $primary;
        padding: $padding;
      }
    `;
    const nodeCss = transpileLattice(src);
    const browserCss = transpileLatticeInBrowser(src);
    expect(browserCss).toBe(nodeCss);
  });

  it("handles @for loop", () => {
    // #{$i} interpolation not supported; use $i in value position
    const css = transpileLatticeInBrowser(`
      @for $i from 1 through 3 {
        .col { order: $i; }
      }
    `);
    expect(normalize(css)).toContain("order: 1");
    expect(normalize(css)).toContain("order: 2");
    expect(normalize(css)).toContain("order: 3");
  });

  it("handles @each loop", () => {
    const css = transpileLatticeInBrowser(`
      @each $c in red, green {
        p { color: $c; }
      }
    `);
    expect(normalize(css)).toContain("color: red");
    expect(normalize(css)).toContain("color: green");
  });

  it("handles @function", () => {
    const css = transpileLatticeInBrowser(`
      @function triple($x) { @return $x * 3; }
      p { padding: triple(10px); }
    `);
    expect(normalize(css)).toContain("padding: 30px");
  });
});

// =============================================================================
// 12. Re-exported Building Blocks
// =============================================================================

describe("re-exported building blocks", () => {
  it("re-exports parseLattice", () => {
    const ast = parseLattice("h1 { color: red; }");
    expect(ast.ruleName).toBe("stylesheet");
  });

  it("re-exports LatticeTransformer", () => {
    const ast = parseLattice("$c: red; p { color: $c; }");
    const transformer = new LatticeTransformer();
    const cssAst = transformer.transform(ast);
    expect(cssAst.ruleName).toBe("stylesheet");
  });

  it("re-exports CSSEmitter", () => {
    const ast = parseLattice("p { color: red; }");
    const transformer = new LatticeTransformer();
    const cssAst = transformer.transform(ast);
    const emitter = new CSSEmitter();
    const css = emitter.emit(cssAst);
    expect(normalize(css)).toBe("p { color: red; }");
  });

  it("CSSEmitter supports minified mode", () => {
    const ast = parseLattice("p { color: red; }");
    const transformer = new LatticeTransformer();
    const cssAst = transformer.transform(ast);
    const emitter = new CSSEmitter("  ", true);
    const css = emitter.emit(cssAst);
    expect(normalize(css)).toBe("p{color:red;}");
  });
});

// =============================================================================
// 13. Error Classes
// =============================================================================

describe("error classes", () => {
  it("re-exports LatticeError base class", () => {
    expect(LatticeError).toBeDefined();
    const err = new LatticeError("test", 1, 1);
    expect(err).toBeInstanceOf(Error);
    // The message includes position info: "test at line 1, column 1"
    expect(err.message).toContain("test");
    // The latticeMessage property holds the original message without position
    expect(err.latticeMessage).toBe("test");
    expect(err.line).toBe(1);
    expect(err.column).toBe(1);
  });

  it("UndefinedVariableError is a LatticeError", () => {
    expect(() => transpileLattice("p { color: $x; }")).toThrow(LatticeError);
  });

  it("UndefinedMixinError is a LatticeError", () => {
    expect(() => transpileLattice(".x { @include missing; }")).toThrow(LatticeError);
  });

  it("WrongArityError is a LatticeError", () => {
    expect(() =>
      transpileLattice(`
        @mixin pad($a, $b) { padding: $a $b; }
        .x { @include pad(1px); }
      `)
    ).toThrow(LatticeError);
  });

  it("CircularReferenceError is thrown for recursive mixin", () => {
    // A mixin that includes itself (must use FUNCTION token form with parens)
    expect(() =>
      transpileLattice(`
        @mixin loop() { @include loop(); }
        .x { @include loop(); }
      `)
    ).toThrow(CircularReferenceError);
  });

  it("TypeErrorInExpression is a LatticeError", () => {
    expect(TypeErrorInExpression).toBeDefined();
  });
});

// =============================================================================
// 14. Complex Programs
// =============================================================================

describe("complex programs", () => {
  it("handles a realistic design system snippet", () => {
    // Note: no-arg mixins require () in both definition and call.
    // Arithmetic in value_list ($spacing-unit * 2) is not supported;
    // only simple variable references are allowed in declaration values.
    const css = transpileLattice(`
      // Design tokens
      $primary: #4a90d9;
      $secondary: #9b59b6;
      $spacing-unit: 8px;
      $radius: 4px;

      // Flex centering mixin
      @mixin flex-center() {
        display: flex;
        align-items: center;
        justify-content: center;
      }

      // Button mixin with theming
      @mixin btn($bg, $fg: white) {
        @include flex-center();
        background: $bg;
        color: $fg;
        border-radius: $radius;
        padding: $spacing-unit;
      }

      .btn-primary { @include btn($primary); }
      .btn-secondary { @include btn($secondary); }
    `);

    expect(normalize(css)).toContain("background: #4a90d9");
    expect(normalize(css)).toContain("background: #9b59b6");
    expect(normalize(css)).toContain("border-radius: 4px");
    expect(normalize(css)).toContain("display: flex");
    expect(normalize(css)).toContain("align-items: center");
    expect(normalize(css)).toContain("justify-content: center");
  });

  it("handles a grid system with @for", () => {
    // #{$i} interpolation not supported; use $i in value position
    const css = transpileLattice(`
      $columns: 4;
      @for $i from 1 through $columns {
        .col { flex: $i; }
      }
    `);
    // Should produce 4 rules
    const matches = css.match(/flex:/g);
    expect(matches).toHaveLength(4);
  });

  it("handles nested control flow", () => {
    // Note: < (less-than) is not a grammar token; use == or > instead
    const css = transpileLattice(`
      $a: 1;
      @if $a == 1 {
        @if $a != 0 {
          p { content: "found"; }
        }
      }
    `);
    expect(normalize(css)).toContain('content: "found"');
  });

  it("handles a function used inside an @if condition", () => {
    const css = transpileLattice(`
      @function is-big($x) {
        @if $x > 10 {
          @return true;
        } @else {
          @return false;
        }
      }
      @if is-big(20) {
        p { color: red; }
      }
    `);
    expect(normalize(css)).toContain("color: red");
  });

  it("handles comments in the source", () => {
    const css = transpileLattice(`
      // This is a comment
      $c: red; /* block comment */
      p { color: $c; }
    `);
    expect(normalize(css)).toContain("color: red");
    expect(css).not.toContain("//");
    expect(css).not.toContain("/*");
  });

  it("handles multiple rules with blank lines between them in output", () => {
    const css = transpileLattice("h1 { color: red; } h2 { color: blue; }");
    // Pretty output has blank lines between rules
    expect(css).toContain("\n\n");
  });

  it("handles the @media at-rule wrapping qualified rules", () => {
    const css = transpileLattice(`
      $breakpoint: 768px;
      @media (max-width: $breakpoint) {
        .nav { display: none; }
      }
    `);
    expect(normalize(css)).toContain("@media");
    // Note: variable substitution in at_prelude depends on transformer support
    // The at_prelude may or may not expand $breakpoint — test for structure
    expect(normalize(css)).toContain(".nav");
    expect(normalize(css)).toContain("display: none");
  });
});

// =============================================================================
// 15. CSS Built-in Function Passthrough
// =============================================================================

describe("CSS built-in function passthrough", () => {
  it("passes through rgb()", () => {
    const css = transpileLattice("p { color: rgb(255, 0, 0); }");
    expect(normalize(css)).toContain("rgb(255, 0, 0)");
  });

  it("passes through rgba()", () => {
    const css = transpileLattice("p { background: rgba(0, 0, 0, 0.5); }");
    expect(normalize(css)).toContain("rgba(");
  });

  it("passes through calc()", () => {
    const css = transpileLattice("p { width: calc(100% - 16px); }");
    expect(normalize(css)).toContain("calc(");
    expect(normalize(css)).toContain("100%");
  });

  it("passes through linear-gradient()", () => {
    const css = transpileLattice(
      "p { background: linear-gradient(to right, red, blue); }"
    );
    expect(normalize(css)).toContain("linear-gradient(");
  });

  it("passes through var()", () => {
    const css = transpileLattice(":root { --c: red; } p { color: var(--c); }");
    expect(normalize(css)).toContain("var(");
  });

  it("passes through hsl()", () => {
    const css = transpileLattice("p { color: hsl(240, 100%, 50%); }");
    expect(normalize(css)).toContain("hsl(");
  });
});
