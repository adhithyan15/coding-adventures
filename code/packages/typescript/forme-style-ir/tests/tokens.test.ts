/**
 * tokens.test.ts — `TokenSet` / `Color` / `Length` / `Shadow` / `TokenRef`
 * shape and constructor coverage.
 *
 * Most token-shape semantics are exercised through the validator
 * (validate.test.ts).  This file pins:
 *   - the runtime helpers (`emptyTokenSet`, `isTokenRef`)
 *   - the frozen `LENGTH_UNITS` tuple
 *   - the discriminated unions at the type level (compile-time)
 */

import { describe, it, expect } from "vitest";
import {
  LENGTH_UNITS, isTokenRef, emptyTokenSet,
  type Color, type Length, type Shadow, type TokenRef,
} from "../src/index.js";

describe("LENGTH_UNITS", () => {
  it("includes every unit named in the spec", () => {
    expect(LENGTH_UNITS).toEqual([
      "px", "rem", "em", "%", "vh", "vw", "pt", "mm", "in", "ch", "ex",
    ]);
  });

  it("is frozen", () => {
    expect(() => (LENGTH_UNITS as unknown as string[]).push("zz")).toThrow();
  });
});

describe("isTokenRef", () => {
  it("recognises a well-formed TokenRef", () => {
    const t: TokenRef = { kind: "token-ref", path: "colors.primary" };
    expect(isTokenRef(t)).toBe(true);
  });

  it("rejects plain objects without kind", () => {
    expect(isTokenRef({ path: "colors.x" })).toBe(false);
  });

  it("rejects wrong-kinded objects", () => {
    expect(isTokenRef({ kind: "rgb", r: 1, g: 1, b: 1 })).toBe(false);
  });

  it("rejects non-string paths", () => {
    expect(isTokenRef({ kind: "token-ref", path: 42 })).toBe(false);
  });

  it("rejects null and primitives", () => {
    expect(isTokenRef(null)).toBe(false);
    expect(isTokenRef("colors.primary")).toBe(false);
    expect(isTokenRef(42)).toBe(false);
    expect(isTokenRef(undefined)).toBe(false);
  });
});

describe("emptyTokenSet", () => {
  it("returns all five required buckets, each empty", () => {
    const ts = emptyTokenSet();
    expect(ts.colors).toEqual({});
    expect(ts.space).toEqual({});
    expect(ts.radii).toEqual({});
    expect(ts.shadows).toEqual({});
    expect(ts.typography.families).toEqual({});
    expect(ts.typography.scale).toEqual({});
    expect(ts.typography.weights).toEqual({});
    expect(ts.typography.leading).toEqual({});
    expect(ts.typography.tracking).toEqual({});
  });

  it("does not include the optional extensions field", () => {
    const ts = emptyTokenSet();
    expect("extensions" in ts).toBe(false);
  });
});

describe("Color / Length / Shadow value-shape sanity", () => {
  it("Color rgb literal compiles and is structurally sound", () => {
    const c: Color = { kind: "rgb", r: 0, g: 128, b: 255, a: 0.5 };
    expect(c.kind).toBe("rgb");
    expect((c as { r: number }).r).toBe(0);
  });

  it("Color named literal compiles", () => {
    const c: Color = { kind: "named", name: "red" };
    expect(c.kind).toBe("named");
  });

  it("Length rem literal compiles", () => {
    const l: Length = { unit: "rem", value: 1.25 };
    expect(l.unit).toBe("rem");
    expect(l.value).toBe(1.25);
  });

  it("Shadow literal compiles with TokenRef color", () => {
    const s: Shadow = {
      offsetX: { unit: "px", value: 0 },
      offsetY: { unit: "px", value: 2 },
      blur:    { unit: "px", value: 4 },
      spread:  { unit: "px", value: 0 },
      color:   { kind: "token-ref", path: "colors.shadow" },
    };
    expect(s.inset).toBeUndefined();
  });
});
