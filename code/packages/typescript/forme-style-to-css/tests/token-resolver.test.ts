/**
 * token-resolver.test.ts — TokenRef path walking + cycle detection.
 */

import { describe, it, expect } from "vitest";
import {
  resolveRef, resolveColor, resolveLength, resolveShadow,
  resolveFontStack, resolveNumber,
} from "../src/index.js";
import {
  emptyTokenSet, type TokenSet, type Color,
} from "@coding-adventures/forme-style-ir";

function ts(): TokenSet {
  const t = emptyTokenSet();
  return {
    ...t,
    colors: {
      text: { kind: "rgb", r: 31, g: 35, b: 40 },
      link: { kind: "token-ref", path: "colors.text" },
      cycle: { kind: "token-ref", path: "colors.cycle" },
    } as Record<string, Color | { kind: "token-ref"; path: string }>,
    typography: {
      ...t.typography,
      families: { body: ["Inter", "sans-serif"] },
      scale:    { md: { unit: "rem", value: 1 } },
      weights:  { regular: 400 },
      leading:  { normal: 1.5 },
      tracking: { normal: { unit: "em", value: 0 } },
    },
    space: { md: { unit: "rem", value: 1 } },
  };
}

describe("resolveRef", () => {
  it("resolves a direct path to a concrete value", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.text" }, ts()))
      .toEqual({ kind: "rgb", r: 31, g: 35, b: 40 });
  });

  it("chains through nested TokenRefs", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.link" }, ts()))
      .toEqual({ kind: "rgb", r: 31, g: 35, b: 40 });
  });

  it("returns null on missing path", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.nope" }, ts())).toBeNull();
  });

  it("returns null on cycle (depth guard)", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.cycle" }, ts())).toBeNull();
  });

  it("walks deeply nested paths", () => {
    expect(resolveRef({ kind: "token-ref", path: "typography.scale.md" }, ts()))
      .toEqual({ unit: "rem", value: 1 });
  });

  it("returns null when path segment encounters a primitive", () => {
    // typography.weights.regular is a number — walking ".more" past it should fail
    expect(resolveRef({ kind: "token-ref", path: "typography.weights.regular.more" }, ts()))
      .toBeNull();
  });

  it("refuses prototype-pollution traversal vectors", () => {
    // Hand-rolled refs bypassing the IR validator's path grammar
    // shouldn't surface Function.prototype / Object.prototype members.
    expect(resolveRef({ kind: "token-ref", path: "__proto__" }, ts())).toBeNull();
    expect(resolveRef({ kind: "token-ref", path: "constructor" }, ts())).toBeNull();
    expect(resolveRef({ kind: "token-ref", path: "colors.__proto__" }, ts())).toBeNull();
    expect(resolveRef({ kind: "token-ref", path: "colors.constructor.prototype" }, ts())).toBeNull();
  });

  it("only walks own enumerable properties (not inherited)", () => {
    // Define a token set whose prototype carries an `inherited`
    // color.  The walker should NOT find it via `colors.inherited`.
    const proto = { colors: { inherited: { kind: "rgb", r: 1, g: 1, b: 1 } } };
    const custom = Object.create(proto) as TokenSet;
    expect(resolveRef({ kind: "token-ref", path: "colors.inherited" }, custom))
      .toBeNull();
  });
});

describe("typed wrappers", () => {
  it("resolveColor returns concrete or null", () => {
    expect(resolveColor({ kind: "named", name: "red" }, ts()))
      .toEqual({ kind: "named", name: "red" });
    expect(resolveColor({ kind: "token-ref", path: "colors.text" }, ts()))
      .toEqual({ kind: "rgb", r: 31, g: 35, b: 40 });
    expect(resolveColor({ kind: "token-ref", path: "colors.nope" }, ts())).toBeNull();
  });

  it("resolveLength returns concrete or null", () => {
    expect(resolveLength({ unit: "px", value: 4 }, ts())).toEqual({ unit: "px", value: 4 });
    expect(resolveLength({ kind: "token-ref", path: "space.md" }, ts()))
      .toEqual({ unit: "rem", value: 1 });
    expect(resolveLength({ kind: "token-ref", path: "space.nope" }, ts())).toBeNull();
  });

  it("resolveLength rejects type-mismatched ref (color path → Length expected)", () => {
    expect(resolveLength({ kind: "token-ref", path: "colors.text" }, ts())).toBeNull();
  });

  it("resolveNumber for font-weight literal vs ref", () => {
    expect(resolveNumber(700, ts())).toBe(700);
    expect(resolveNumber({ kind: "token-ref", path: "typography.weights.regular" }, ts()))
      .toBe(400);
    expect(resolveNumber({ kind: "token-ref", path: "typography.weights.nope" }, ts())).toBeNull();
  });

  it("resolveFontStack returns array or null", () => {
    expect(resolveFontStack(["Inter"], ts())).toEqual(["Inter"]);
    expect(resolveFontStack({ kind: "token-ref", path: "typography.families.body" }, ts()))
      .toEqual(["Inter", "sans-serif"]);
    expect(resolveFontStack({ kind: "token-ref", path: "typography.families.nope" }, ts()))
      .toBeNull();
  });

  it("resolveShadow returns object or null", () => {
    const literal = {
      offsetX: { unit: "px", value: 0 } as const,
      offsetY: { unit: "px", value: 1 } as const,
      blur:    { unit: "px", value: 2 } as const,
      spread:  { unit: "px", value: 0 } as const,
      color:   { kind: "named", name: "red" } as const,
    };
    expect(resolveShadow(literal, ts())).toEqual(literal);
    expect(resolveShadow({ kind: "token-ref", path: "colors.text" }, ts())).toBeNull();
  });
});
