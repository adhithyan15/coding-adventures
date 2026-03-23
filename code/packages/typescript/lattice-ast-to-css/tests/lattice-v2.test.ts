/**
 * Lattice v2 Feature Tests
 *
 * Tests for all new features ported from the Python reference implementation:
 * - v2 error types (MaxIterationError, ExtendTargetNotFoundError, etc.)
 * - ScopeChain.setGlobal
 * - LatticeMap value type
 * - LatticeColor.toRgb/toHsl/fromRgb/fromHsl
 * - Built-in functions (map, color, list, type, math)
 * - @while loops
 * - !default and !global variable flags
 * - Variables in selectors
 * - @content blocks
 * - @at-root
 * - @extend and %placeholder
 * - Property nesting
 */

import { describe, it, expect } from "vitest";
import {
  // Errors
  LatticeError,
  MaxIterationError,
  ExtendTargetNotFoundError,
  LatticeRangeError,
  ZeroDivisionInExpressionError,
  TypeErrorInExpression,
  // Scope
  ScopeChain,
  // Values
  LatticeNumber,
  LatticeDimension,
  LatticePercentage,
  LatticeString,
  LatticeIdent,
  LatticeColor,
  LatticeBool,
  LatticeNull,
  LatticeList,
  LatticeMap,
  isTruthy,
  typeNameOf,
  getNumericValue,
  BUILTIN_FUNCTIONS,
} from "../src/index.js";

// =============================================================================
// v2 Error Types
// =============================================================================

describe("Lattice v2 Error Types", () => {
  it("MaxIterationError stores max iterations and extends LatticeError", () => {
    const err = new MaxIterationError(500, 10, 5);
    expect(err).toBeInstanceOf(LatticeError);
    expect(err.maxIterations).toBe(500);
    expect(err.line).toBe(10);
    expect(err.column).toBe(5);
    expect(err.message).toContain("500");
    expect(err.message).toContain("@while");
  });

  it("MaxIterationError defaults to 1000", () => {
    const err = new MaxIterationError();
    expect(err.maxIterations).toBe(1000);
  });

  it("ExtendTargetNotFoundError stores target", () => {
    const err = new ExtendTargetNotFoundError("%missing", 3, 7);
    expect(err).toBeInstanceOf(LatticeError);
    expect(err.target).toBe("%missing");
    expect(err.message).toContain("%missing");
  });

  it("LatticeRangeError stores message", () => {
    const err = new LatticeRangeError("Index 5 out of bounds", 1, 1);
    expect(err).toBeInstanceOf(LatticeError);
    expect(err.message).toContain("Index 5");
  });

  it("ZeroDivisionInExpressionError", () => {
    const err = new ZeroDivisionInExpressionError(2, 3);
    expect(err).toBeInstanceOf(LatticeError);
    expect(err.message).toContain("Division by zero");
    expect(err.line).toBe(2);
  });
});

// =============================================================================
// ScopeChain.setGlobal
// =============================================================================

describe("ScopeChain.setGlobal", () => {
  it("sets a variable in the root scope from a deeply nested scope", () => {
    const root = new ScopeChain();
    const child = root.child();
    const grandchild = child.child();

    grandchild.setGlobal("$color", new LatticeIdent("blue"));

    // Should be visible from the root
    expect(root.get("$color")).toEqual(new LatticeIdent("blue"));
    // Should be visible from any descendant
    expect(child.get("$color")).toEqual(new LatticeIdent("blue"));
    expect(grandchild.get("$color")).toEqual(new LatticeIdent("blue"));
  });

  it("overwrites existing global variable", () => {
    const root = new ScopeChain();
    root.set("$theme", new LatticeIdent("light"));
    const child = root.child();

    child.setGlobal("$theme", new LatticeIdent("dark"));

    expect(root.get("$theme")).toEqual(new LatticeIdent("dark"));
  });
});

// =============================================================================
// LatticeMap
// =============================================================================

describe("LatticeMap", () => {
  const map = new LatticeMap([
    ["primary", new LatticeColor("#4a90d9")],
    ["secondary", new LatticeColor("#7b68ee")],
    ["bg", new LatticeColor("#ffffff")],
  ]);

  it("has kind 'map'", () => {
    expect(map.kind).toBe("map");
  });

  it("get() returns value for existing key", () => {
    expect(map.get("primary")).toEqual(new LatticeColor("#4a90d9"));
  });

  it("get() returns undefined for missing key", () => {
    expect(map.get("missing")).toBeUndefined();
  });

  it("keys() returns all keys in insertion order", () => {
    expect(map.keys()).toEqual(["primary", "secondary", "bg"]);
  });

  it("values() returns all values in insertion order", () => {
    expect(map.values()).toEqual([
      new LatticeColor("#4a90d9"),
      new LatticeColor("#7b68ee"),
      new LatticeColor("#ffffff"),
    ]);
  });

  it("hasKey() returns true for existing key, false otherwise", () => {
    expect(map.hasKey("primary")).toBe(true);
    expect(map.hasKey("missing")).toBe(false);
  });

  it("toString() produces parenthesized key: value pairs", () => {
    const small = new LatticeMap([["a", new LatticeNumber(1)]]);
    expect(small.toString()).toBe("(a: 1)");
  });

  it("is truthy even when empty", () => {
    expect(isTruthy(new LatticeMap([]))).toBe(true);
  });
});

// =============================================================================
// LatticeColor v2 (toRgb, toHsl, fromRgb, fromHsl)
// =============================================================================

describe("LatticeColor v2 conversions", () => {
  it("toRgb parses #RGB shorthand", () => {
    const [r, g, b, a] = new LatticeColor("#f00").toRgb();
    expect(r).toBe(255);
    expect(g).toBe(0);
    expect(b).toBe(0);
    expect(a).toBe(1.0);
  });

  it("toRgb parses #RRGGBB", () => {
    const [r, g, b, a] = new LatticeColor("#4a90d9").toRgb();
    expect(r).toBe(74);
    expect(g).toBe(144);
    expect(b).toBe(217);
    expect(a).toBe(1.0);
  });

  it("toRgb parses #RRGGBBAA", () => {
    const [r, g, b, a] = new LatticeColor("#ff000080").toRgb();
    expect(r).toBe(255);
    expect(g).toBe(0);
    expect(b).toBe(0);
    expect(a).toBeCloseTo(0.502, 2);
  });

  it("toHsl converts black correctly", () => {
    const [h, s, l] = new LatticeColor("#000000").toHsl();
    expect(h).toBe(0);
    expect(s).toBe(0);
    expect(l).toBe(0);
  });

  it("toHsl converts white correctly", () => {
    const [h, s, l] = new LatticeColor("#ffffff").toHsl();
    expect(h).toBe(0);
    expect(s).toBe(0);
    expect(l).toBeCloseTo(100, 0);
  });

  it("toHsl converts pure red", () => {
    const [h, s, l] = new LatticeColor("#ff0000").toHsl();
    expect(h).toBeCloseTo(0, 0);
    expect(s).toBeCloseTo(100, 0);
    expect(l).toBeCloseTo(50, 0);
  });

  it("fromRgb creates hex color", () => {
    const color = LatticeColor.fromRgb(255, 0, 0);
    expect(color.value).toBe("#ff0000");
  });

  it("fromRgb with alpha creates rgba notation", () => {
    const color = LatticeColor.fromRgb(255, 0, 0, 0.5);
    expect(color.value).toBe("rgba(255, 0, 0, 0.5)");
  });

  it("fromHsl creates correct color", () => {
    const color = LatticeColor.fromHsl(0, 100, 50);
    expect(color.value).toBe("#ff0000");
  });

  it("fromHsl achromatic (s=0)", () => {
    const color = LatticeColor.fromHsl(0, 0, 50);
    const [r, g, b] = color.toRgb();
    expect(r).toBe(g);
    expect(g).toBe(b);
    expect(r).toBeCloseTo(128, 0);
  });

  it("round-trip: toHsl -> fromHsl preserves color", () => {
    const original = new LatticeColor("#4a90d9");
    const [h, s, l, a] = original.toHsl();
    const reconstructed = LatticeColor.fromHsl(h, s, l, a);
    const [r1, g1, b1] = original.toRgb();
    const [r2, g2, b2] = reconstructed.toRgb();
    expect(Math.abs(r1 - r2)).toBeLessThanOrEqual(1);
    expect(Math.abs(g1 - g2)).toBeLessThanOrEqual(1);
    expect(Math.abs(b1 - b2)).toBeLessThanOrEqual(1);
  });
});

// =============================================================================
// typeNameOf and getNumericValue
// =============================================================================

describe("typeNameOf", () => {
  it("returns correct type names", () => {
    expect(typeNameOf(new LatticeNumber(1))).toBe("number");
    expect(typeNameOf(new LatticeDimension(1, "px"))).toBe("number");
    expect(typeNameOf(new LatticePercentage(50))).toBe("number");
    expect(typeNameOf(new LatticeString("hi"))).toBe("string");
    expect(typeNameOf(new LatticeIdent("red"))).toBe("string");
    expect(typeNameOf(new LatticeColor("#000"))).toBe("color");
    expect(typeNameOf(new LatticeBool(true))).toBe("bool");
    expect(typeNameOf(new LatticeNull())).toBe("null");
    expect(typeNameOf(new LatticeList([]))).toBe("list");
    expect(typeNameOf(new LatticeMap([]))).toBe("map");
  });
});

describe("getNumericValue", () => {
  it("extracts number from LatticeNumber", () => {
    expect(getNumericValue(new LatticeNumber(42))).toBe(42);
  });

  it("extracts number from LatticeDimension", () => {
    expect(getNumericValue(new LatticeDimension(16, "px"))).toBe(16);
  });

  it("extracts number from LatticePercentage", () => {
    expect(getNumericValue(new LatticePercentage(50))).toBe(50);
  });

  it("throws for non-numeric value", () => {
    expect(() => getNumericValue(new LatticeIdent("red"))).toThrow(TypeErrorInExpression);
  });
});

// =============================================================================
// Built-in Functions
// =============================================================================

describe("Built-in Map Functions", () => {
  const testMap = new LatticeMap([
    ["primary", new LatticeColor("#4a90d9")],
    ["size", new LatticeDimension(16, "px")],
  ]);

  it("map-get returns value for key", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-get")!;
    expect(fn([testMap, new LatticeIdent("primary")])).toEqual(new LatticeColor("#4a90d9"));
  });

  it("map-get returns null for missing key", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-get")!;
    expect(fn([testMap, new LatticeIdent("missing")]).kind).toBe("null");
  });

  it("map-keys returns all keys", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-keys")!;
    const result = fn([testMap]);
    expect(result.kind).toBe("list");
    expect((result as LatticeList).items.length).toBe(2);
  });

  it("map-values returns all values", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-values")!;
    const result = fn([testMap]);
    expect(result.kind).toBe("list");
    expect((result as LatticeList).items.length).toBe(2);
  });

  it("map-has-key returns true/false", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-has-key")!;
    expect(fn([testMap, new LatticeIdent("primary")])).toEqual(new LatticeBool(true));
    expect(fn([testMap, new LatticeIdent("missing")])).toEqual(new LatticeBool(false));
  });

  it("map-merge combines two maps", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-merge")!;
    const map2 = new LatticeMap([["tertiary", new LatticeColor("#333")]]);
    const result = fn([testMap, map2]) as LatticeMap;
    expect(result.kind).toBe("map");
    expect(result.items.length).toBe(3);
  });

  it("map-remove removes keys", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-remove")!;
    const result = fn([testMap, new LatticeIdent("primary")]) as LatticeMap;
    expect(result.items.length).toBe(1);
    expect(result.get("primary")).toBeUndefined();
  });
});

describe("Built-in Color Functions", () => {
  const red = new LatticeColor("#ff0000");
  const blue = new LatticeColor("#0000ff");

  it("lighten increases lightness", () => {
    const fn = BUILTIN_FUNCTIONS.get("lighten")!;
    const result = fn([red, new LatticeNumber(20)]);
    expect(result.kind).toBe("color");
    // Red (L=50%) + 20% = L=70%
    const [, , l] = (result as LatticeColor).toHsl();
    expect(l).toBeCloseTo(70, 0);
  });

  it("darken decreases lightness", () => {
    const fn = BUILTIN_FUNCTIONS.get("darken")!;
    const result = fn([red, new LatticeNumber(20)]);
    const [, , l] = (result as LatticeColor).toHsl();
    expect(l).toBeCloseTo(30, 0);
  });

  it("mix blends two colors", () => {
    const fn = BUILTIN_FUNCTIONS.get("mix")!;
    const result = fn([red, blue, new LatticeNumber(50)]);
    expect(result.kind).toBe("color");
  });

  it("complement rotates hue by 180", () => {
    const fn = BUILTIN_FUNCTIONS.get("complement")!;
    const result = fn([red]);
    const [h] = (result as LatticeColor).toHsl();
    expect(h).toBeCloseTo(180, 0);
  });

  it("red/green/blue extract channels", () => {
    expect(BUILTIN_FUNCTIONS.get("red")!([red])).toEqual(new LatticeNumber(255));
    expect(BUILTIN_FUNCTIONS.get("green")!([red])).toEqual(new LatticeNumber(0));
    expect(BUILTIN_FUNCTIONS.get("blue")!([red])).toEqual(new LatticeNumber(0));
  });

  it("hue/saturation/lightness extract HSL", () => {
    const hue = BUILTIN_FUNCTIONS.get("hue")!([red]);
    expect(hue.kind).toBe("dimension");
    const sat = BUILTIN_FUNCTIONS.get("saturation")!([red]);
    expect(sat.kind).toBe("percentage");
    const light = BUILTIN_FUNCTIONS.get("lightness")!([red]);
    expect(light.kind).toBe("percentage");
  });

  it("saturate increases saturation", () => {
    const grey = LatticeColor.fromHsl(0, 50, 50);
    const fn = BUILTIN_FUNCTIONS.get("saturate")!;
    const result = fn([grey, new LatticeNumber(20)]);
    const [, s] = (result as LatticeColor).toHsl();
    expect(s).toBeCloseTo(70, 0);
  });

  it("desaturate decreases saturation", () => {
    const fn = BUILTIN_FUNCTIONS.get("desaturate")!;
    const result = fn([red, new LatticeNumber(30)]);
    const [, s] = (result as LatticeColor).toHsl();
    expect(s).toBeCloseTo(70, 0);
  });

  it("adjust-hue rotates hue", () => {
    const fn = BUILTIN_FUNCTIONS.get("adjust-hue")!;
    const result = fn([red, new LatticeNumber(120)]);
    const [h] = (result as LatticeColor).toHsl();
    expect(h).toBeCloseTo(120, 0);
  });

  it("rgba with color + alpha", () => {
    const fn = BUILTIN_FUNCTIONS.get("rgba")!;
    const result = fn([red, new LatticeNumber(0.5)]);
    expect(result.kind).toBe("color");
    expect(result.toString()).toContain("rgba");
  });

  it("rgba with 4 args", () => {
    const fn = BUILTIN_FUNCTIONS.get("rgba")!;
    const result = fn([
      new LatticeNumber(255),
      new LatticeNumber(0),
      new LatticeNumber(0),
      new LatticeNumber(0.5),
    ]);
    expect(result.kind).toBe("color");
  });
});

describe("Built-in List Functions", () => {
  const list = new LatticeList([
    new LatticeIdent("a"),
    new LatticeIdent("b"),
    new LatticeIdent("c"),
  ]);

  it("nth returns the nth item (1-indexed)", () => {
    const fn = BUILTIN_FUNCTIONS.get("nth")!;
    expect(fn([list, new LatticeNumber(1)])).toEqual(new LatticeIdent("a"));
    expect(fn([list, new LatticeNumber(3)])).toEqual(new LatticeIdent("c"));
  });

  it("nth throws for out-of-bounds index", () => {
    const fn = BUILTIN_FUNCTIONS.get("nth")!;
    expect(() => fn([list, new LatticeNumber(5)])).toThrow(LatticeRangeError);
  });

  it("length returns list length", () => {
    const fn = BUILTIN_FUNCTIONS.get("length")!;
    expect(fn([list])).toEqual(new LatticeNumber(3));
  });

  it("length returns 1 for non-list values", () => {
    const fn = BUILTIN_FUNCTIONS.get("length")!;
    expect(fn([new LatticeNumber(42)])).toEqual(new LatticeNumber(1));
  });

  it("join concatenates two lists", () => {
    const fn = BUILTIN_FUNCTIONS.get("join")!;
    const result = fn([list, new LatticeList([new LatticeIdent("d")])]);
    expect((result as LatticeList).items.length).toBe(4);
  });

  it("append adds to end", () => {
    const fn = BUILTIN_FUNCTIONS.get("append")!;
    const result = fn([list, new LatticeIdent("d")]);
    expect((result as LatticeList).items.length).toBe(4);
  });

  it("index finds position (1-indexed)", () => {
    const fn = BUILTIN_FUNCTIONS.get("index")!;
    expect(fn([list, new LatticeIdent("b")])).toEqual(new LatticeNumber(2));
  });

  it("index returns null for missing value", () => {
    const fn = BUILTIN_FUNCTIONS.get("index")!;
    expect(fn([list, new LatticeIdent("z")]).kind).toBe("null");
  });
});

describe("Built-in Type Functions", () => {
  it("type-of returns type name", () => {
    const fn = BUILTIN_FUNCTIONS.get("type-of")!;
    expect(fn([new LatticeNumber(1)])).toEqual(new LatticeString("number"));
    expect(fn([new LatticeColor("#fff")])).toEqual(new LatticeString("color"));
    expect(fn([new LatticeMap([])])).toEqual(new LatticeString("map"));
  });

  it("unit returns unit string", () => {
    const fn = BUILTIN_FUNCTIONS.get("unit")!;
    expect(fn([new LatticeDimension(16, "px")])).toEqual(new LatticeString("px"));
    expect(fn([new LatticePercentage(50)])).toEqual(new LatticeString("%"));
    expect(fn([new LatticeNumber(42)])).toEqual(new LatticeString(""));
  });

  it("unitless returns true for numbers without units", () => {
    const fn = BUILTIN_FUNCTIONS.get("unitless")!;
    expect(fn([new LatticeNumber(42)])).toEqual(new LatticeBool(true));
    expect(fn([new LatticeDimension(16, "px")])).toEqual(new LatticeBool(false));
  });

  it("comparable checks if two numbers can be compared", () => {
    const fn = BUILTIN_FUNCTIONS.get("comparable")!;
    expect(fn([new LatticeDimension(10, "px"), new LatticeDimension(5, "px")])).toEqual(new LatticeBool(true));
    expect(fn([new LatticeDimension(10, "px"), new LatticeDimension(5, "em")])).toEqual(new LatticeBool(false));
    expect(fn([new LatticeNumber(10), new LatticeDimension(5, "px")])).toEqual(new LatticeBool(true));
  });
});

describe("Built-in Math Functions", () => {
  it("math.div divides numbers", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.div")!;
    expect(fn([new LatticeNumber(10), new LatticeNumber(3)])).toEqual(new LatticeNumber(10 / 3));
  });

  it("math.div dimension by number preserves unit", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.div")!;
    const result = fn([new LatticeDimension(100, "px"), new LatticeNumber(2)]);
    expect(result.kind).toBe("dimension");
    expect((result as LatticeDimension).value).toBe(50);
    expect((result as LatticeDimension).unit).toBe("px");
  });

  it("math.div dimension by same unit gives number", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.div")!;
    const result = fn([new LatticeDimension(100, "px"), new LatticeDimension(5, "px")]);
    expect(result.kind).toBe("number");
    expect((result as LatticeNumber).value).toBe(20);
  });

  it("math.div throws on zero divisor", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.div")!;
    expect(() => fn([new LatticeNumber(10), new LatticeNumber(0)])).toThrow(ZeroDivisionInExpressionError);
  });

  it("math.floor rounds down", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.floor")!;
    expect(fn([new LatticeNumber(3.7)])).toEqual(new LatticeNumber(3));
    expect(fn([new LatticeNumber(-3.2)])).toEqual(new LatticeNumber(-4));
  });

  it("math.ceil rounds up", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.ceil")!;
    expect(fn([new LatticeNumber(3.2)])).toEqual(new LatticeNumber(4));
  });

  it("math.round rounds to nearest", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.round")!;
    expect(fn([new LatticeNumber(3.5)])).toEqual(new LatticeNumber(4));
    expect(fn([new LatticeNumber(3.4)])).toEqual(new LatticeNumber(3));
  });

  it("math.abs returns absolute value", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.abs")!;
    expect(fn([new LatticeNumber(-5)])).toEqual(new LatticeNumber(5));
    expect(fn([new LatticeNumber(5)])).toEqual(new LatticeNumber(5));
  });

  it("math.floor preserves dimension unit", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.floor")!;
    const result = fn([new LatticeDimension(3.7, "px")]);
    expect(result.kind).toBe("dimension");
    expect((result as LatticeDimension).value).toBe(3);
    expect((result as LatticeDimension).unit).toBe("px");
  });

  it("math.min returns smallest", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.min")!;
    expect(fn([new LatticeNumber(5), new LatticeNumber(3), new LatticeNumber(7)])).toEqual(new LatticeNumber(3));
  });

  it("math.max returns largest", () => {
    const fn = BUILTIN_FUNCTIONS.get("math.max")!;
    expect(fn([new LatticeNumber(5), new LatticeNumber(3), new LatticeNumber(7)])).toEqual(new LatticeNumber(7));
  });
});

// =============================================================================
// Built-in function error cases
// =============================================================================

describe("Built-in function error handling", () => {
  it("map-get throws on non-map argument", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-get")!;
    expect(() => fn([new LatticeNumber(1), new LatticeIdent("key")])).toThrow(TypeErrorInExpression);
  });

  it("map-get throws on too few arguments", () => {
    const fn = BUILTIN_FUNCTIONS.get("map-get")!;
    expect(() => fn([new LatticeMap([])])).toThrow(TypeErrorInExpression);
  });

  it("nth throws on index < 1", () => {
    const fn = BUILTIN_FUNCTIONS.get("nth")!;
    expect(() => fn([new LatticeList([new LatticeIdent("a")]), new LatticeNumber(0)])).toThrow(LatticeRangeError);
  });

  it("lighten throws on amount > 100", () => {
    const fn = BUILTIN_FUNCTIONS.get("lighten")!;
    expect(() => fn([new LatticeColor("#ff0000"), new LatticeNumber(150)])).toThrow(LatticeRangeError);
  });

  it("unit throws on non-numeric value", () => {
    const fn = BUILTIN_FUNCTIONS.get("unit")!;
    expect(() => fn([new LatticeIdent("red")])).toThrow(TypeErrorInExpression);
  });
});
