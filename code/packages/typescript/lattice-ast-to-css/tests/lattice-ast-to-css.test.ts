/**
 * Tests for @coding-adventures/lattice-ast-to-css
 *
 * Tests cover:
 * 1. ScopeChain — lexical scoping for variables
 * 2. Value types (LatticeNumber, LatticeDimension, etc.)
 * 3. isTruthy / arithmetic / comparison
 * 4. ExpressionEvaluator — compile-time expression evaluation
 * 5. LatticeTransformer — three-pass transformation
 * 6. CSSEmitter — AST to CSS text
 * 7. Error classes
 */

import { describe, it, expect } from "vitest";
import {
  ScopeChain,
  LatticeNumber,
  LatticeDimension,
  LatticePercentage,
  LatticeString,
  LatticeIdent,
  LatticeColor,
  LatticeBool,
  LatticeNull,
  LatticeList,
  isTruthy,
  tokenToValue,
  valueToCss,
  addValues,
  subtractValues,
  multiplyValues,
  negateValue,
  compareValues,
  ExpressionEvaluator,
  LatticeTransformer,
  CSSEmitter,
  LatticeError,
  UndefinedVariableError,
  UndefinedMixinError,
  UndefinedFunctionError,
  WrongArityError,
  CircularReferenceError,
  TypeErrorInExpression,
  MissingReturnError,
} from "../src/index.js";
import { parseLattice } from "@coding-adventures/lattice-parser";

// =============================================================================
// Helper: transpile Lattice to CSS
// =============================================================================

function transpile(source: string, minified = false): string {
  const ast = parseLattice(source);
  const transformer = new LatticeTransformer();
  const cssAst = transformer.transform(ast);
  const emitter = new CSSEmitter("  ", minified);
  return emitter.emit(cssAst);
}

// =============================================================================
// ScopeChain
// =============================================================================

describe("ScopeChain", () => {
  it("global scope has depth 0", () => {
    const scope = new ScopeChain();
    expect(scope.depth).toBe(0);
  });

  it("child scope has depth 1", () => {
    const global = new ScopeChain();
    const child = global.child();
    expect(child.depth).toBe(1);
  });

  it("nested scope has depth 2", () => {
    const global = new ScopeChain();
    const child = global.child();
    const nested = child.child();
    expect(nested.depth).toBe(2);
  });

  it("set and get a value in same scope", () => {
    const scope = new ScopeChain();
    scope.set("$color", "red");
    expect(scope.get("$color")).toBe("red");
  });

  it("get inherits from parent scope", () => {
    const parent = new ScopeChain();
    parent.set("$color", "blue");
    const child = parent.child();
    expect(child.get("$color")).toBe("blue");
  });

  it("child scope shadows parent scope", () => {
    const parent = new ScopeChain();
    parent.set("$color", "red");
    const child = parent.child();
    child.set("$color", "blue");
    expect(child.get("$color")).toBe("blue");
    expect(parent.get("$color")).toBe("red"); // Parent unchanged
  });

  it("get returns undefined for undeclared name", () => {
    const scope = new ScopeChain();
    expect(scope.get("$nonexistent")).toBeUndefined();
  });

  it("has() returns true for bound name", () => {
    const scope = new ScopeChain();
    scope.set("$x", 1);
    expect(scope.has("$x")).toBe(true);
  });

  it("has() returns false for unbound name", () => {
    const scope = new ScopeChain();
    expect(scope.has("$x")).toBe(false);
  });

  it("has() checks parent chain", () => {
    const parent = new ScopeChain();
    parent.set("$x", 1);
    const child = parent.child();
    expect(child.has("$x")).toBe(true);
  });

  it("hasLocal() returns false for parent binding", () => {
    const parent = new ScopeChain();
    parent.set("$x", 1);
    const child = parent.child();
    expect(child.hasLocal("$x")).toBe(false);
  });

  it("hasLocal() returns true for local binding", () => {
    const scope = new ScopeChain();
    scope.set("$x", 1);
    expect(scope.hasLocal("$x")).toBe(true);
  });
});

// =============================================================================
// Value Types
// =============================================================================

describe("LatticeNumber", () => {
  it("stores value", () => {
    const n = new LatticeNumber(42);
    expect(n.value).toBe(42);
    expect(n.kind).toBe("number");
  });

  it("toString emits integer without decimal", () => {
    expect(new LatticeNumber(42).toString()).toBe("42");
  });

  it("toString emits decimal for non-integer", () => {
    expect(new LatticeNumber(3.14).toString()).toBe("3.14");
  });
});

describe("LatticeDimension", () => {
  it("stores value and unit", () => {
    const d = new LatticeDimension(16, "px");
    expect(d.value).toBe(16);
    expect(d.unit).toBe("px");
    expect(d.kind).toBe("dimension");
  });

  it("toString emits integer dimension", () => {
    expect(new LatticeDimension(16, "px").toString()).toBe("16px");
  });

  it("toString emits decimal dimension", () => {
    expect(new LatticeDimension(1.5, "rem").toString()).toBe("1.5rem");
  });
});

describe("LatticePercentage", () => {
  it("stores value", () => {
    const p = new LatticePercentage(50);
    expect(p.value).toBe(50);
    expect(p.kind).toBe("percentage");
  });

  it("toString emits with percent sign", () => {
    expect(new LatticePercentage(50).toString()).toBe("50%");
  });
});

describe("LatticeString", () => {
  it("stores value without quotes", () => {
    const s = new LatticeString("hello");
    expect(s.value).toBe("hello");
    expect(s.kind).toBe("string");
  });

  it("toString emits with double quotes", () => {
    expect(new LatticeString("hello").toString()).toBe('"hello"');
  });
});

describe("LatticeIdent", () => {
  it("stores value", () => {
    const i = new LatticeIdent("red");
    expect(i.value).toBe("red");
    expect(i.kind).toBe("ident");
  });

  it("toString emits value unchanged", () => {
    expect(new LatticeIdent("red").toString()).toBe("red");
  });
});

describe("LatticeColor", () => {
  it("stores value with # prefix", () => {
    const c = new LatticeColor("#4a90d9");
    expect(c.value).toBe("#4a90d9");
    expect(c.kind).toBe("color");
  });

  it("toString emits value unchanged", () => {
    expect(new LatticeColor("#fff").toString()).toBe("#fff");
  });
});

describe("LatticeBool", () => {
  it("stores true", () => {
    const b = new LatticeBool(true);
    expect(b.value).toBe(true);
    expect(b.kind).toBe("bool");
  });

  it("toString emits 'true'", () => {
    expect(new LatticeBool(true).toString()).toBe("true");
  });

  it("toString emits 'false'", () => {
    expect(new LatticeBool(false).toString()).toBe("false");
  });
});

describe("LatticeNull", () => {
  it("has kind 'null'", () => {
    const n = new LatticeNull();
    expect(n.kind).toBe("null");
  });

  it("toString emits empty string", () => {
    expect(new LatticeNull().toString()).toBe("");
  });
});

describe("LatticeList", () => {
  it("stores items", () => {
    const list = new LatticeList([new LatticeIdent("red"), new LatticeIdent("blue")]);
    expect(list.items).toHaveLength(2);
    expect(list.kind).toBe("list");
  });

  it("toString emits comma-separated items", () => {
    const list = new LatticeList([new LatticeIdent("red"), new LatticeIdent("blue")]);
    expect(list.toString()).toBe("red, blue");
  });
});

// =============================================================================
// Truthiness
// =============================================================================

describe("isTruthy", () => {
  it("false is falsy", () => {
    expect(isTruthy(new LatticeBool(false))).toBe(false);
  });

  it("true is truthy", () => {
    expect(isTruthy(new LatticeBool(true))).toBe(true);
  });

  it("null is falsy", () => {
    expect(isTruthy(new LatticeNull())).toBe(false);
  });

  it("LatticeNumber(0) is falsy", () => {
    expect(isTruthy(new LatticeNumber(0))).toBe(false);
  });

  it("LatticeNumber(1) is truthy", () => {
    expect(isTruthy(new LatticeNumber(1))).toBe(true);
  });

  it("ident is truthy", () => {
    expect(isTruthy(new LatticeIdent("red"))).toBe(true);
  });

  it("dimension is truthy", () => {
    expect(isTruthy(new LatticeDimension(10, "px"))).toBe(true);
  });

  it("empty string is truthy (Sass convention)", () => {
    expect(isTruthy(new LatticeString(""))).toBe(true);
  });
});

// =============================================================================
// Arithmetic
// =============================================================================

describe("addValues", () => {
  it("Number + Number = Number", () => {
    const result = addValues(new LatticeNumber(3), new LatticeNumber(4));
    expect(result).toEqual(new LatticeNumber(7));
  });

  it("Dimension + Dimension (same unit)", () => {
    const result = addValues(
      new LatticeDimension(10, "px"),
      new LatticeDimension(5, "px")
    );
    expect(result).toEqual(new LatticeDimension(15, "px"));
  });

  it("Dimension + Dimension (different unit) throws", () => {
    expect(() =>
      addValues(new LatticeDimension(10, "px"), new LatticeDimension(5, "em"))
    ).toThrow(TypeErrorInExpression);
  });

  it("Percentage + Percentage", () => {
    const result = addValues(new LatticePercentage(30), new LatticePercentage(70));
    expect(result).toEqual(new LatticePercentage(100));
  });

  it("String + String = concatenation", () => {
    const result = addValues(new LatticeString("hello"), new LatticeString(" world"));
    expect(result).toEqual(new LatticeString("hello world"));
  });

  it("Number + String throws", () => {
    expect(() =>
      addValues(new LatticeNumber(1), new LatticeString("x"))
    ).toThrow(TypeErrorInExpression);
  });
});

describe("subtractValues", () => {
  it("Number - Number = Number", () => {
    const result = subtractValues(new LatticeNumber(10), new LatticeNumber(3));
    expect(result).toEqual(new LatticeNumber(7));
  });

  it("Dimension - Dimension (same unit)", () => {
    const result = subtractValues(
      new LatticeDimension(20, "px"),
      new LatticeDimension(5, "px")
    );
    expect(result).toEqual(new LatticeDimension(15, "px"));
  });
});

describe("multiplyValues", () => {
  it("Number * Number = Number", () => {
    const result = multiplyValues(new LatticeNumber(3), new LatticeNumber(4));
    expect(result).toEqual(new LatticeNumber(12));
  });

  it("Number * Dimension = Dimension", () => {
    const result = multiplyValues(new LatticeNumber(2), new LatticeDimension(8, "px"));
    expect(result).toEqual(new LatticeDimension(16, "px"));
  });

  it("Dimension * Number = Dimension", () => {
    const result = multiplyValues(new LatticeDimension(8, "px"), new LatticeNumber(2));
    expect(result).toEqual(new LatticeDimension(16, "px"));
  });

  it("Number * Percentage = Percentage", () => {
    const result = multiplyValues(new LatticeNumber(2), new LatticePercentage(50));
    expect(result).toEqual(new LatticePercentage(100));
  });

  it("Dimension * Dimension throws", () => {
    expect(() =>
      multiplyValues(new LatticeDimension(2, "px"), new LatticeDimension(3, "px"))
    ).toThrow(TypeErrorInExpression);
  });
});

describe("negateValue", () => {
  it("negates Number", () => {
    expect(negateValue(new LatticeNumber(5))).toEqual(new LatticeNumber(-5));
  });

  it("negates Dimension", () => {
    expect(negateValue(new LatticeDimension(10, "px"))).toEqual(
      new LatticeDimension(-10, "px")
    );
  });

  it("negates Percentage", () => {
    expect(negateValue(new LatticePercentage(50))).toEqual(
      new LatticePercentage(-50)
    );
  });

  it("negating Ident throws", () => {
    expect(() => negateValue(new LatticeIdent("red"))).toThrow(TypeErrorInExpression);
  });
});

describe("compareValues", () => {
  it("== for equal numbers", () => {
    expect(compareValues(new LatticeNumber(5), new LatticeNumber(5), "EQUALS_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("== for different numbers", () => {
    expect(compareValues(new LatticeNumber(5), new LatticeNumber(6), "EQUALS_EQUALS"))
      .toEqual(new LatticeBool(false));
  });

  it("!= for different numbers", () => {
    expect(compareValues(new LatticeNumber(5), new LatticeNumber(6), "NOT_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("> for greater number", () => {
    expect(compareValues(new LatticeNumber(10), new LatticeNumber(5), "GREATER"))
      .toEqual(new LatticeBool(true));
  });

  it(">= for equal numbers", () => {
    expect(compareValues(new LatticeNumber(5), new LatticeNumber(5), "GREATER_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("<= for lesser number", () => {
    expect(compareValues(new LatticeNumber(3), new LatticeNumber(5), "LESS_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("== for idents (string comparison)", () => {
    expect(compareValues(new LatticeIdent("dark"), new LatticeIdent("dark"), "EQUALS_EQUALS"))
      .toEqual(new LatticeBool(true));
  });
});

// =============================================================================
// tokenToValue
// =============================================================================

describe("tokenToValue", () => {
  it("NUMBER → LatticeNumber", () => {
    const tok = { type: "NUMBER", value: "42", line: 1, column: 1 };
    expect(tokenToValue(tok)).toEqual(new LatticeNumber(42));
  });

  it("DIMENSION → LatticeDimension", () => {
    const tok = { type: "DIMENSION", value: "16px", line: 1, column: 1 };
    const result = tokenToValue(tok);
    expect(result).toBeInstanceOf(LatticeDimension);
    expect((result as LatticeDimension).value).toBe(16);
    expect((result as LatticeDimension).unit).toBe("px");
  });

  it("PERCENTAGE → LatticePercentage", () => {
    const tok = { type: "PERCENTAGE", value: "50%", line: 1, column: 1 };
    expect(tokenToValue(tok)).toEqual(new LatticePercentage(50));
  });

  it("STRING → LatticeString", () => {
    const tok = { type: "STRING", value: "hello", line: 1, column: 1 };
    expect(tokenToValue(tok)).toEqual(new LatticeString("hello"));
  });

  it("HASH → LatticeColor", () => {
    const tok = { type: "HASH", value: "#4a90d9", line: 1, column: 1 };
    expect(tokenToValue(tok)).toEqual(new LatticeColor("#4a90d9"));
  });

  it("IDENT 'true' → LatticeBool(true)", () => {
    const tok = { type: "IDENT", value: "true", line: 1, column: 1 };
    expect(tokenToValue(tok)).toEqual(new LatticeBool(true));
  });

  it("IDENT 'false' → LatticeBool(false)", () => {
    const tok = { type: "IDENT", value: "false", line: 1, column: 1 };
    expect(tokenToValue(tok)).toEqual(new LatticeBool(false));
  });

  it("IDENT 'null' → LatticeNull", () => {
    const tok = { type: "IDENT", value: "null", line: 1, column: 1 };
    expect(tokenToValue(tok)).toEqual(new LatticeNull());
  });

  it("IDENT other → LatticeIdent", () => {
    const tok = { type: "IDENT", value: "red", line: 1, column: 1 };
    expect(tokenToValue(tok)).toEqual(new LatticeIdent("red"));
  });
});

// =============================================================================
// LatticeTransformer — Variable Resolution
// =============================================================================

describe("LatticeTransformer — variables", () => {
  it("substitutes a simple variable", () => {
    const css = transpile("$color: red; h1 { color: $color; }");
    expect(css).toContain("color: red");
    expect(css).not.toContain("$color");
  });

  it("substitutes a dimension variable", () => {
    const css = transpile("$size: 16px; p { font-size: $size; }");
    expect(css).toContain("font-size: 16px");
    expect(css).not.toContain("$size");
  });

  it("substitutes a color variable", () => {
    const css = transpile("$primary: #4a90d9; a { color: $primary; }");
    expect(css).toContain("color: #4a90d9");
  });

  it("handles variable defined after use (forward reference)", () => {
    // Pass 1 collects all top-level variables before expansion
    const css = transpile("h1 { color: $color; } $color: blue;");
    expect(css).toContain("color: blue");
  });

  it("throws UndefinedVariableError for undeclared variable", () => {
    expect(() =>
      transpile("h1 { color: $nonexistent; }")
    ).toThrow(UndefinedVariableError);
  });

  it("variable in block scope does not leak", () => {
    // Variables declared inside rules are scoped to that block
    const css = transpile("$x: global; .a { $x: local; color: $x; } .b { color: $x; }");
    // .a should get "local", .b should get "global"
    expect(css).toContain("color: local");
    expect(css).toContain("color: global");
  });
});

// =============================================================================
// LatticeTransformer — Mixin Expansion
// =============================================================================

describe("LatticeTransformer — mixins", () => {
  it("expands a simple mixin", () => {
    const css = transpile(`
      @mixin clearfix() {
        overflow: hidden;
      }
      .container { @include clearfix(); }
    `);
    expect(css).toContain("overflow: hidden");
    expect(css).not.toContain("@include");
    expect(css).not.toContain("@mixin");
  });

  it("expands mixin with parameter", () => {
    const css = transpile(`
      @mixin button($bg) {
        background: $bg;
      }
      .btn { @include button(red); }
    `);
    expect(css).toContain("background: red");
  });

  it("expands mixin with multiple parameters", () => {
    const css = transpile(`
      @mixin btn($bg, $fg) {
        background: $bg;
        color: $fg;
      }
      .primary { @include btn(blue, white); }
    `);
    expect(css).toContain("background: blue");
    expect(css).toContain("color: white");
  });

  it("throws UndefinedMixinError for unknown mixin", () => {
    expect(() =>
      transpile(".btn { @include nonexistent(); }")
    ).toThrow(UndefinedMixinError);
  });

  it("throws CircularReferenceError for recursive mixin", () => {
    expect(() =>
      transpile(`
        @mixin a() { @include b(); }
        @mixin b() { @include a(); }
        .x { @include a(); }
      `)
    ).toThrow(CircularReferenceError);
  });

  it("throws WrongArityError for wrong argument count", () => {
    expect(() =>
      transpile(`
        @mixin btn($bg, $fg) { color: $fg; }
        .x { @include btn(red); }
      `)
    ).toThrow(WrongArityError);
  });
});

// =============================================================================
// LatticeTransformer — @if Control Flow
// =============================================================================

describe("LatticeTransformer — @if", () => {
  it("expands @if when condition is true", () => {
    const css = transpile(`
      @mixin theme($t) {
        @if $t == dark {
          background: black;
        }
      }
      body { @include theme(dark); }
    `);
    expect(css).toContain("background: black");
  });

  it("does not expand @if when condition is false", () => {
    const css = transpile(`
      @mixin theme($t) {
        @if $t == dark {
          background: black;
        }
      }
      body { @include theme(light); }
    `);
    expect(css).not.toContain("background: black");
  });

  it("expands @else branch when condition is false", () => {
    const css = transpile(`
      @mixin theme($t) {
        @if $t == dark {
          background: black;
        } @else {
          background: white;
        }
      }
      body { @include theme(light); }
    `);
    expect(css).toContain("background: white");
    expect(css).not.toContain("background: black");
  });
});

// =============================================================================
// LatticeTransformer — @for Loop
// =============================================================================

describe("LatticeTransformer — @for", () => {
  it("expands @for through loop", () => {
    const css = transpile(`
      @for $i from 1 through 3 {
        .item { color: red; }
      }
    `);
    // Should produce 3 .item rules
    const matches = css.match(/color: red/g);
    expect(matches).toHaveLength(3);
  });

  it("expands @for to loop (exclusive)", () => {
    const css = transpile(`
      @for $i from 1 to 3 {
        .item { color: red; }
      }
    `);
    // Should produce 2 rules (1 and 2, not 3)
    const matches = css.match(/color: red/g);
    expect(matches).toHaveLength(2);
  });
});

// =============================================================================
// LatticeTransformer — @each Loop
// =============================================================================

describe("LatticeTransformer — @each", () => {
  it("expands @each loop", () => {
    const css = transpile(`
      @each $color in red, blue, green {
        .dot { background: $color; }
      }
    `);
    expect(css).toContain("background: red");
    expect(css).toContain("background: blue");
    expect(css).toContain("background: green");
  });
});

// =============================================================================
// LatticeTransformer — Functions
// =============================================================================

describe("LatticeTransformer — @function", () => {
  it("evaluates a simple function", () => {
    const css = transpile(`
      @function double($n) {
        @return $n * 2;
      }
      .box { width: double(8px); }
    `);
    expect(css).toContain("width: 16px");
  });

  it("evaluates function with return value", () => {
    const css = transpile(`
      @function brand() {
        @return red;
      }
      .x { color: brand(); }
    `);
    expect(css).toContain("color: red");
  });

  it("throws MissingReturnError for function without @return", () => {
    expect(() =>
      transpile(`
        @function noop($x) { $y: $x; }
        .x { color: noop(red); }
      `)
    ).toThrow(MissingReturnError);
  });

  it("throws UndefinedFunctionError... or passes through for truly unknown functions", () => {
    // Unknown functions are passed through as CSS (like CSS builtins we don't know)
    // This is the expected behavior
    const css = transpile(".x { color: rgb(255, 0, 0); }");
    expect(css).toContain("rgb(255, 0, 0)");
  });

  it("resolves loop variable passed as function argument", () => {
    // Regression: calling space($i) inside @for previously failed with
    // "Cannot multiply '$i' and '0.25rem'" because the VARIABLE token was
    // converted to LatticeIdent("$i") instead of being resolved to its value
    // in the caller scope before being passed to the function.
    const css = transpile(`
      @function space($n) {
        @return $n * 0.25rem;
      }
      @for $i from 1 through 3 {
        .box { padding: space($i); }
      }
    `);
    // All three iterations should compile without error and produce correct values
    expect(css).toContain("padding: 0.25rem");
    expect(css).toContain("padding: 0.5rem");
    expect(css).toContain("padding: 0.75rem");
  });

  it("resolves variable argument passed to function from outer scope", () => {
    // Variables defined in the surrounding scope should also be resolved
    // when passed as function arguments.
    const css = transpile(`
      @function double($n) {
        @return $n * 2;
      }
      $size: 4px;
      .box { width: double($size); }
    `);
    expect(css).toContain("width: 8px");
  });
});

// =============================================================================
// LatticeTransformer — CSS Passthrough
// =============================================================================

describe("LatticeTransformer — CSS passthrough", () => {
  it("passes through plain CSS unchanged", () => {
    const css = transpile("h1 { color: red; }");
    expect(css).toContain("h1");
    expect(css).toContain("color: red");
  });

  it("passes through @media queries", () => {
    const css = transpile("@media (max-width: 768px) { h1 { color: red; } }");
    expect(css).toContain("@media");
  });

  it("passes through CSS functions unchanged", () => {
    const css = transpile("h1 { color: rgb(255, 0, 0); }");
    expect(css).toContain("rgb(255, 0, 0)");
  });

  it("passes through !important", () => {
    const css = transpile("h1 { color: red !important; }");
    expect(css).toContain("!important");
  });
});

// =============================================================================
// CSSEmitter
// =============================================================================

describe("CSSEmitter", () => {
  it("emits minified CSS with minified=true", () => {
    const css = transpile("h1 { color: red; }", true);
    // The emitter adds a trailing newline but no internal newlines in minified mode
    expect(css.trim()).not.toContain("\n");
    expect(css).toContain("color:red");
  });

  it("emits pretty-printed CSS with default settings", () => {
    const css = transpile("h1 { color: red; }");
    expect(css).toContain("\n");
    expect(css).toContain("  color: red;");
  });

  it("emits selector correctly", () => {
    const css = transpile(".btn { color: blue; }");
    expect(css).toContain(".btn");
  });

  it("emits empty output for source with only variable declarations", () => {
    const css = transpile("$x: 1; $y: 2;");
    // No CSS output — only variable definitions
    expect(css.trim()).toBe("");
  });

  it("CSSEmitter can be created with custom indent", () => {
    const ast = parseLattice("h1 { color: red; }");
    const transformer = new LatticeTransformer();
    const cssAst = transformer.transform(ast);
    const emitter = new CSSEmitter("    ", false); // 4-space indent
    const css = emitter.emit(cssAst);
    expect(css).toContain("    color: red;");
  });
});

// =============================================================================
// Error Classes
// =============================================================================

describe("Error classes", () => {
  it("LatticeError has line and column", () => {
    const err = new LatticeError("test", 5, 10);
    expect(err.line).toBe(5);
    expect(err.column).toBe(10);
    expect(err.message).toContain("line 5");
  });

  it("UndefinedVariableError stores name", () => {
    const err = new UndefinedVariableError("$color", 1, 1);
    expect(err.name).toBe("$color");
    expect(err instanceof LatticeError).toBe(true);
  });

  it("UndefinedMixinError stores name", () => {
    const err = new UndefinedMixinError("button");
    expect(err.name).toBe("button");
  });

  it("WrongArityError stores expected and got", () => {
    const err = new WrongArityError("Mixin", "button", 2, 1);
    expect(err.expected).toBe(2);
    expect(err.got).toBe(1);
  });

  it("CircularReferenceError stores chain", () => {
    const err = new CircularReferenceError("mixin", ["a", "b", "a"]);
    expect(err.chain).toEqual(["a", "b", "a"]);
    expect(err.message).toContain("a → b → a");
  });

  it("TypeErrorInExpression stores op and types", () => {
    const err = new TypeErrorInExpression("add", "10px", "red");
    expect(err.op).toBe("add");
    expect(err.leftType).toBe("10px");
  });

  it("MissingReturnError stores function name", () => {
    const err = new MissingReturnError("spacing");
    expect(err.name).toBe("spacing");
  });

  it("all errors inherit from LatticeError", () => {
    expect(new UndefinedVariableError("$x")).toBeInstanceOf(LatticeError);
    expect(new UndefinedMixinError("m")).toBeInstanceOf(LatticeError);
    expect(new UndefinedFunctionError("f")).toBeInstanceOf(LatticeError);
    expect(new WrongArityError("M", "m", 1, 2)).toBeInstanceOf(LatticeError);
    expect(new CircularReferenceError("mixin", ["a", "b"])).toBeInstanceOf(LatticeError);
    expect(new TypeErrorInExpression("add", "a", "b")).toBeInstanceOf(LatticeError);
    expect(new MissingReturnError("f")).toBeInstanceOf(LatticeError);
  });
});

// =============================================================================
// Additional tests to reach 80%+ coverage
// =============================================================================

import {
  LatticeModuleNotFoundError,
  ReturnOutsideFunctionError,
  UnitMismatchError,
  extractValueFromAst,
} from "../src/index.js";

describe("LatticeModuleNotFoundError", () => {
  it("stores moduleName", () => {
    const err = new LatticeModuleNotFoundError("theme");
    expect(err.moduleName).toBe("theme");
    expect(err instanceof LatticeError).toBe(true);
  });

  it("has the right message with line info", () => {
    const err = new LatticeModuleNotFoundError("buttons", 3, 5);
    expect(err.message).toContain("buttons");
    expect(err.line).toBe(3);
    expect(err.column).toBe(5);
  });
});

describe("ReturnOutsideFunctionError", () => {
  it("constructs with default line/column", () => {
    const err = new ReturnOutsideFunctionError();
    expect(err instanceof LatticeError).toBe(true);
    expect(err.message).toContain("return");
  });

  it("constructs with line and column", () => {
    const err = new ReturnOutsideFunctionError(7, 3);
    expect(err.line).toBe(7);
    expect(err.column).toBe(3);
  });
});

describe("UnitMismatchError", () => {
  it("stores left and right units", () => {
    const err = new UnitMismatchError("px", "s");
    expect(err.leftUnit).toBe("px");
    expect(err.rightUnit).toBe("s");
    expect(err instanceof LatticeError).toBe(true);
  });

  it("has the right message", () => {
    const err = new UnitMismatchError("em", "ms");
    expect(err.message).toContain("em");
    expect(err.message).toContain("ms");
  });
});

describe("compareValues — dimension NOT_EQUALS", () => {
  it("!= for same-unit dimensions with different values", () => {
    expect(
      compareValues(new LatticeDimension(10, "px"), new LatticeDimension(5, "px"), "NOT_EQUALS")
    ).toEqual(new LatticeBool(true));
  });

  it("!= for same-unit same-value dimensions returns false", () => {
    expect(
      compareValues(new LatticeDimension(10, "px"), new LatticeDimension(10, "px"), "NOT_EQUALS")
    ).toEqual(new LatticeBool(false));
  });

  it("!= for different-unit dimensions returns true", () => {
    expect(
      compareValues(new LatticeDimension(10, "px"), new LatticeDimension(10, "em"), "NOT_EQUALS")
    ).toEqual(new LatticeBool(true));
  });

  it("== for different-unit dimensions returns false", () => {
    expect(
      compareValues(new LatticeDimension(10, "px"), new LatticeDimension(10, "em"), "EQUALS_EQUALS")
    ).toEqual(new LatticeBool(false));
  });

  it("GREATER with incompatible dimension units returns false", () => {
    expect(
      compareValues(new LatticeDimension(10, "px"), new LatticeDimension(5, "em"), "GREATER")
    ).toEqual(new LatticeBool(false));
  });
});

describe("compareValues — non-numeric string fallback", () => {
  it("== for equal colors via string comparison", () => {
    expect(
      compareValues(new LatticeColor("#fff"), new LatticeColor("#fff"), "EQUALS_EQUALS")
    ).toEqual(new LatticeBool(true));
  });

  it("!= for different colors", () => {
    expect(
      compareValues(new LatticeColor("#fff"), new LatticeColor("#000"), "NOT_EQUALS")
    ).toEqual(new LatticeBool(true));
  });

  it("GREATER on non-numeric types returns false", () => {
    expect(
      compareValues(new LatticeColor("#fff"), new LatticeColor("#000"), "GREATER")
    ).toEqual(new LatticeBool(false));
  });

  it("LESS_EQUALS on idents returns false", () => {
    expect(
      compareValues(new LatticeIdent("bold"), new LatticeIdent("normal"), "LESS_EQUALS")
    ).toEqual(new LatticeBool(false));
  });

  it("!= for different idents", () => {
    expect(
      compareValues(new LatticeIdent("dark"), new LatticeIdent("light"), "NOT_EQUALS")
    ).toEqual(new LatticeBool(true));
  });
});

describe("subtractValues — additional paths", () => {
  it("Percentage - Percentage", () => {
    expect(subtractValues(new LatticePercentage(80), new LatticePercentage(30)))
      .toEqual(new LatticePercentage(50));
  });

  it("Dimension - Dimension different units throws", () => {
    expect(() =>
      subtractValues(new LatticeDimension(10, "px"), new LatticeDimension(5, "em"))
    ).toThrow(TypeErrorInExpression);
  });

  it("mismatched types throws", () => {
    expect(() =>
      subtractValues(new LatticeNumber(5), new LatticeString("x"))
    ).toThrow(TypeErrorInExpression);
  });
});

describe("multiplyValues — Percentage * Number", () => {
  it("Percentage * Number = Percentage", () => {
    expect(multiplyValues(new LatticePercentage(50), new LatticeNumber(2)))
      .toEqual(new LatticePercentage(100));
  });
});

describe("valueToCss", () => {
  it("converts LatticeNumber to CSS string", () => {
    expect(valueToCss(new LatticeNumber(42))).toBe("42");
  });

  it("converts LatticeDimension to CSS string", () => {
    expect(valueToCss(new LatticeDimension(16, "px"))).toBe("16px");
  });

  it("converts LatticeIdent to CSS string", () => {
    expect(valueToCss(new LatticeIdent("red"))).toBe("red");
  });
});

describe("extractValueFromAst", () => {
  it("extracts value from a raw token directly", () => {
    const token = { type: "NUMBER", value: "10", line: 1, column: 1 } as any;
    const result = extractValueFromAst(token);
    expect(result).toEqual(new LatticeNumber(10));
  });

  it("extracts value from a nested ASTNode", () => {
    const numToken = { type: "NUMBER", value: "5", line: 1, column: 1 };
    const valueNode = { ruleName: "value", children: [numToken] };
    const listNode = { ruleName: "value_list", children: [valueNode] } as any;
    const result = extractValueFromAst(listNode);
    expect(result).toEqual(new LatticeNumber(5));
  });

  it("returns LatticeNull for empty ASTNode", () => {
    const emptyNode = { ruleName: "value_list", children: [] } as any;
    const result = extractValueFromAst(emptyNode);
    expect(result).toBeInstanceOf(LatticeNull);
  });
});

describe("ExpressionEvaluator — direct evaluation", () => {
  it("evaluates a raw token directly", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    const token = { type: "NUMBER", value: "42", line: 1, column: 1 };
    const result = evaluator.evaluate(token as any);
    expect(result).toEqual(new LatticeNumber(42));
  });

  it("looks up variable stored as LatticeValue in scope", () => {
    const scope = new ScopeChain();
    scope.set("$x", new LatticeNumber(99));
    const evaluator = new ExpressionEvaluator(scope);
    const varToken = { type: "VARIABLE", value: "$x", line: 1, column: 1 };
    const primaryNode = { ruleName: "lattice_primary", children: [varToken] } as any;
    const result = evaluator.evaluate(primaryNode);
    expect(result).toEqual(new LatticeNumber(99));
  });

  it("looks up undefined variable returns LatticeIdent", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    const varToken = { type: "VARIABLE", value: "$missing", line: 1, column: 1 };
    const primaryNode = { ruleName: "lattice_primary", children: [varToken] } as any;
    const result = evaluator.evaluate(primaryNode);
    expect(result).toBeInstanceOf(LatticeIdent);
  });

  it("looks up variable stored as raw Token", () => {
    const scope = new ScopeChain();
    const rawToken = { type: "NUMBER", value: "77", line: 1, column: 1 };
    scope.set("$t", rawToken);
    const evaluator = new ExpressionEvaluator(scope);
    const varToken = { type: "VARIABLE", value: "$t", line: 1, column: 1 };
    const primaryNode = { ruleName: "lattice_primary", children: [varToken] } as any;
    const result = evaluator.evaluate(primaryNode);
    expect(result).toEqual(new LatticeNumber(77));
  });

  it("looks up variable stored as ASTNode", () => {
    const scope = new ScopeChain();
    const identToken = { type: "IDENT", value: "blue", line: 1, column: 1 };
    const valueNode = { ruleName: "value", children: [identToken] };
    const valueListNode = { ruleName: "value_list", children: [valueNode] };
    scope.set("$c", valueListNode);
    const evaluator = new ExpressionEvaluator(scope);
    const varToken = { type: "VARIABLE", value: "$c", line: 1, column: 1 };
    const primaryNode = { ruleName: "lattice_primary", children: [varToken] } as any;
    const result = evaluator.evaluate(primaryNode);
    expect(result).toBeInstanceOf(LatticeIdent);
    expect((result as LatticeIdent).value).toBe("blue");
  });

  it("evaluates lattice_primary with ASTNode child recurses", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    const innerToken = { type: "NUMBER", value: "5", line: 1, column: 1 };
    const innerNode = { ruleName: "lattice_primary", children: [innerToken] } as any;
    const outerNode = { ruleName: "lattice_primary", children: [innerNode] } as any;
    const result = evaluator.evaluate(outerNode);
    expect(result).toEqual(new LatticeNumber(5));
  });

  it("evaluates empty primary node returns LatticeNull", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    const emptyNode = { ruleName: "lattice_primary", children: [] } as any;
    const result = evaluator.evaluate(emptyNode);
    expect(result).toBeInstanceOf(LatticeNull);
  });
});

describe("ExpressionEvaluator — via transpile", () => {
  it("evaluates or logic", () => {
    const css = transpile(`
      @mixin check($v) {
        @if $v == a or $v == b {
          color: red;
        }
      }
      .x { @include check(a); }
    `);
    expect(css).toContain("color: red");
  });

  it("evaluates and logic", () => {
    const css = transpile(`
      @mixin check($a, $b) {
        @if $a == 1 and $b == 2 {
          color: green;
        }
      }
      .x { @include check(1, 2); }
    `);
    expect(css).toContain("color: green");
  });

  it("evaluates != comparison", () => {
    const css = transpile(`
      @mixin check($v) {
        @if $v != dark {
          color: white;
        }
      }
      .x { @include check(light); }
    `);
    expect(css).toContain("color: white");
  });

  it("evaluates > comparison", () => {
    const css = transpile(`
      @mixin check($n) {
        @if $n > 5 {
          display: block;
        }
      }
      .x { @include check(10); }
    `);
    expect(css).toContain("display: block");
  });

  it("evaluates >= comparison", () => {
    const css = transpile(`
      @mixin check($n) {
        @if $n >= 10 {
          display: block;
        }
      }
      .x { @include check(10); }
    `);
    expect(css).toContain("display: block");
  });

  it("evaluates <= comparison", () => {
    const css = transpile(`
      @mixin check($n) {
        @if $n <= 5 {
          font-size: small;
        }
      }
      .x { @include check(3); }
    `);
    expect(css).toContain("font-size: small");
  });

  it("evaluates unary negation", () => {
    const css = transpile(`
      @function neg($n) {
        @return -$n;
      }
      .x { margin: neg(10px); }
    `);
    expect(css).toContain("margin: -10px");
  });

  it("evaluates subtraction in function", () => {
    const css = transpile(`
      @function sub($a, $b) {
        @return $a - $b;
      }
      .x { width: sub(20px, 5px); }
    `);
    expect(css).toContain("width: 15px");
  });
});

describe("CSSEmitter — advanced CSS features", () => {
  it("emits @keyframes", () => {
    const css = transpile(`
      @keyframes fade {
        from { opacity: 0; }
        to { opacity: 1; }
      }
    `);
    expect(css).toContain("@keyframes");
    expect(css).toContain("opacity");
  });

  it("emits child combinator selector", () => {
    const css = transpile("ul > li { color: red; }");
    expect(css).toContain("color: red");
  });

  it("emits pseudo-class", () => {
    const css = transpile("a:hover { color: red; }");
    expect(css).toContain(":hover");
  });

  it("emits pseudo-element", () => {
    const css = transpile("p::before { color: red; }");
    expect(css).toContain("::before");
  });

  it("emits id selector", () => {
    const css = transpile("#main { display: block; }");
    expect(css).toContain("#main");
  });

  it("emits rgba function (exercises RPAREN path)", () => {
    const css = transpile("body { background: rgba(0, 0, 0, 0.5); }");
    expect(css).toContain("rgba(0, 0, 0, 0.5)");
  });

  it("emits minified @media rule", () => {
    const css = transpile("@media (max-width: 600px) { h1 { color: red; } }", true);
    expect(css).toContain("@media");
  });

  it("emits comma-separated selector list", () => {
    const css = transpile("h1, h2 { font-weight: bold; }");
    expect(css).toContain("h1");
    expect(css).toContain("h2");
  });

  it("emits nested rules inside @media", () => {
    const css = transpile(`
      @media screen {
        .container { width: 100%; }
      }
    `);
    expect(css).toContain("@media");
    expect(css).toContain(".container");
  });

  it("emits attribute selector", () => {
    const css = transpile("input[type] { border: none; }");
    expect(css).toContain("[type]");
  });

  it("emits attribute selector with value matcher", () => {
    const css = transpile("a[href] { color: blue; }");
    expect(css).toContain("[href]");
  });
});

describe("LatticeTransformer — additional paths", () => {
  it("@if with boolean variable true", () => {
    const css = transpile(`
      $show: true;
      .x {
        @if $show {
          display: block;
        }
      }
    `);
    expect(css).toContain("display: block");
  });

  it("@if with boolean variable false skips block", () => {
    const css = transpile(`
      $show: false;
      .x {
        @if $show {
          display: block;
        }
      }
    `);
    expect(css).not.toContain("display: block");
  });

  it("mixin with default parameter value", () => {
    const css = transpile(`
      @mixin border($color: red) {
        border-color: $color;
      }
      .x { @include border(); }
    `);
    expect(css).toContain("border-color: red");
  });

  it("function returning arithmetic on dimension", () => {
    const css = transpile(`
      @function triple($n) {
        @return $n * 3;
      }
      .x { margin: triple(4px); }
    `);
    expect(css).toContain("margin: 12px");
  });

  it("multiple variables in one rule", () => {
    const css = transpile(`
      $pad: 8px;
      $margin: 16px;
      .x {
        padding: $pad;
        margin: $margin;
      }
    `);
    expect(css).toContain("padding: 8px");
    expect(css).toContain("margin: 16px");
  });
});
