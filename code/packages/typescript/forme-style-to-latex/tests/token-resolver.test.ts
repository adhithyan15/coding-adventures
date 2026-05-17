/**
 * token-resolver.test.ts — TokenRef walker + prototype-pollution guards.
 */

import { describe, it, expect } from "vitest";
import type { TokenSet } from "@coding-adventures/forme-style-ir";
import {
  resolveRef, resolveColor, resolveLength, resolveShadow,
  resolveFontStack, resolveNumber,
} from "../src/index.js";

const tokens: TokenSet = {
  colors: {
    text: { kind: "rgb", r: 0, g: 0, b: 0 },
    link: { kind: "token-ref", path: "colors.text" },
    // Cycle: a ↔ b.
    a: { kind: "token-ref", path: "colors.b" },
    b: { kind: "token-ref", path: "colors.a" },
  },
  typography: {
    families: { body: ["Inter", "sans-serif"] },
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

describe("resolveRef — happy path", () => {
  it("resolves a direct ref", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.text" }, tokens))
      .toEqual({ kind: "rgb", r: 0, g: 0, b: 0 });
  });

  it("follows a one-hop chain", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.link" }, tokens))
      .toEqual({ kind: "rgb", r: 0, g: 0, b: 0 });
  });

  it("returns null on a cycle (depth-capped)", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.a" }, tokens)).toBeNull();
  });

  it("returns null on missing path", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.gone" }, tokens)).toBeNull();
  });
});

describe("resolveRef — prototype-pollution defence", () => {
  it("refuses __proto__ in path", () => {
    expect(resolveRef({ kind: "token-ref", path: "__proto__.toString" }, tokens)).toBeNull();
  });

  it("refuses constructor", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.constructor.name" }, tokens)).toBeNull();
  });

  it("refuses prototype", () => {
    expect(resolveRef({ kind: "token-ref", path: "colors.prototype" }, tokens)).toBeNull();
  });

  it("hasOwnProperty defence: inherited keys are not followed", () => {
    // `colors.toString` is inherited from Object.prototype.
    expect(resolveRef({ kind: "token-ref", path: "colors.toString" }, tokens)).toBeNull();
  });
});

describe("resolveRef — typed wrappers", () => {
  it("resolveColor", () => {
    expect(resolveColor({ kind: "token-ref", path: "colors.text" }, tokens))
      .toEqual({ kind: "rgb", r: 0, g: 0, b: 0 });
    expect(resolveColor({ kind: "rgb", r: 1, g: 2, b: 3 }, tokens))
      .toEqual({ kind: "rgb", r: 1, g: 2, b: 3 });
    expect(resolveColor({ kind: "token-ref", path: "colors.gone" }, tokens))
      .toBeNull();
  });

  it("resolveLength", () => {
    expect(resolveLength({ kind: "token-ref", path: "space.md" }, tokens))
      .toEqual({ unit: "pt", value: 6 });
    expect(resolveLength({ unit: "em", value: 1 }, tokens))
      .toEqual({ unit: "em", value: 1 });
  });

  it("resolveShadow", () => {
    const s = resolveShadow({ kind: "token-ref", path: "shadows.soft" }, tokens);
    expect(s).not.toBeNull();
    expect(s!.offsetY).toEqual({ unit: "pt", value: 2 });
  });

  it("resolveFontStack", () => {
    expect(resolveFontStack({ kind: "token-ref", path: "typography.families.body" }, tokens))
      .toEqual(["Inter", "sans-serif"]);
  });

  it("resolveNumber", () => {
    expect(resolveNumber({ kind: "token-ref", path: "typography.weights.regular" }, tokens))
      .toBe(400);
  });

  it("resolveNumber rejects NaN / Infinity", () => {
    // Construct a token set with a broken weight value.
    const broken: TokenSet = {
      ...tokens,
      typography: {
        ...tokens.typography,
        weights: { broken: NaN },
      },
    };
    expect(resolveNumber({ kind: "token-ref", path: "typography.weights.broken" }, broken))
      .toBeNull();
  });

  it("typed wrapper rejects type-mismatched leaves", () => {
    // ask for a Color but path lands on a Length
    expect(resolveColor({ kind: "token-ref", path: "space.md" }, tokens)).toBeNull();
  });
});
