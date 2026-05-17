/**
 * token-resolver.test.ts — walker + proto-pollution guards.
 */

import { describe, it, expect } from "vitest";
import type { TokenSet } from "@coding-adventures/forme-style-ir";
import {
  resolveRef, resolveColor, resolveLength, resolveNumber,
  resolveShadow, resolveFontStack,
} from "../src/index.js";

const tokens: TokenSet = {
  colors: {
    text: { kind: "rgb", r: 0, g: 0, b: 0 },
    link: { kind: "token-ref", path: "colors.text" },
    a: { kind: "token-ref", path: "colors.b" },
    b: { kind: "token-ref", path: "colors.a" },
  },
  typography: {
    families: { body: ["Inter"] },
    scale:    { md: { unit: "pt", value: 12 } },
    weights:  { regular: 400 },
    leading:  { normal: 1.5 },
    tracking: { normal: { unit: "em", value: 0 } },
  },
  space:   { md: { unit: "pt", value: 6 } },
  radii:   {},
  shadows: {
    soft: {
      offsetX: { unit: "pt", value: 0 },
      offsetY: { unit: "pt", value: 2 },
      blur:    { unit: "pt", value: 4 },
      spread:  { unit: "pt", value: 0 },
      color:   { kind: "rgb", r: 0, g: 0, b: 0 },
    },
  },
};

describe("resolveRef", () => {
  it("resolves direct ref", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.text" }, tokens)).toEqual({ kind: "rgb", r: 0, g: 0, b: 0 });
  });

  it("follows chain", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.link" }, tokens)).toEqual({ kind: "rgb", r: 0, g: 0, b: 0 });
  });

  it("cycle returns null", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.a" }, tokens)).toBeNull();
  });

  it("missing path returns null", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.gone" }, tokens)).toBeNull();
  });
});

describe("resolveRef — proto-pollution defence", () => {
  it("refuses __proto__", () => {
    expect(resolveRef({ kind: "token-ref", path: "__proto__.toString" }, tokens)).toBeNull();
  });

  it("refuses constructor", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.constructor.name" }, tokens)).toBeNull();
  });

  it("refuses prototype", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.prototype" }, tokens)).toBeNull();
  });

  it("inherited keys are not followed", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.toString" }, tokens)).toBeNull();
  });
});

describe("typed wrappers", () => {
  it("resolveColor", () => {
    expect(resolveColor({ kind: "token-ref", path: "colors.text" }, tokens))
      .toEqual({ kind: "rgb", r: 0, g: 0, b: 0 });
    expect(resolveColor({ kind: "rgb", r: 1, g: 2, b: 3 }, tokens))
      .toEqual({ kind: "rgb", r: 1, g: 2, b: 3 });
    expect(resolveColor({ kind: "token-ref", path: "colors.gone" }, tokens)).toBeNull();
  });

  it("resolveLength", () => {
    expect(resolveLength({ kind: "token-ref", path: "space.md" }, tokens)).toEqual({ unit: "pt", value: 6 });
    expect(resolveLength({ unit: "em", value: 1 }, tokens)).toEqual({ unit: "em", value: 1 });
  });

  it("resolveNumber rejects NaN", () => {
    const broken: TokenSet = {
      ...tokens,
      typography: { ...tokens.typography, weights: { broken: NaN } },
    };
    expect(resolveNumber({ kind: "token-ref", path: "typography.weights.broken" }, broken)).toBeNull();
  });

  it("typed wrapper rejects type-mismatched leaves", () => {
    expect(resolveColor({ kind: "token-ref", path: "space.md" }, tokens)).toBeNull();
  });

  it("resolveShadow follows the ref", () => {
    const s = resolveShadow({ kind: "token-ref", path: "shadows.soft" }, tokens);
    expect(s).not.toBeNull();
    expect(s!.offsetY).toEqual({ unit: "pt", value: 2 });
  });

  it("resolveShadow passes through a concrete value", () => {
    const concrete = {
      offsetX: { unit: "pt", value: 1 } as const,
      offsetY: { unit: "pt", value: 1 } as const,
      blur:    { unit: "pt", value: 1 } as const,
      spread:  { unit: "pt", value: 1 } as const,
      color:   { kind: "named" as const, name: "red" },
    };
    expect(resolveShadow(concrete, tokens)).toBe(concrete);
  });

  it("resolveShadow rejects type-mismatched leaves", () => {
    expect(resolveShadow({ kind: "token-ref", path: "colors.text" }, tokens)).toBeNull();
  });

  it("resolveFontStack follows the ref", () => {
    expect(resolveFontStack({ kind: "token-ref", path: "typography.families.body" }, tokens))
      .toEqual(["Inter"]);
  });

  it("resolveFontStack passes through a concrete array", () => {
    const arr = ["Inter", "sans-serif"];
    expect(resolveFontStack(arr, tokens)).toBe(arr);
  });

  it("resolveFontStack rejects type-mismatched leaves", () => {
    expect(resolveFontStack({ kind: "token-ref", path: "colors.text" }, tokens)).toBeNull();
  });
});
