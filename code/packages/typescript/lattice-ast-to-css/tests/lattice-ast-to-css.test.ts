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
