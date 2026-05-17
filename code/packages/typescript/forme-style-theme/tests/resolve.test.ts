/**
 * resolve.test.ts — `resolveTokenRefs` semantics.
 *
 * Invariants:
 * - Concrete leaves (Color, Length, Shadow, FontStack, number) resolve
 *   to their value.
 * - Token chains follow refs to their concrete leaf.
 * - Cycles return null (depth-capped).
 * - Unresolvable paths (missing segment) return null.
 * - Non-leaf landings (path stops at an intermediate object) return null.
 * - Prototype-pollution paths are refused.
 * - Bulk: duplicate paths in input collapse to one map entry.
 * - Empty input → empty map.
 */

import { describe, it, expect } from "vitest";
import {
  styleRuleId, sel,
  type StyleDocument, type TokenRef,
} from "@coding-adventures/forme-style-ir";
import { resolveTokenRefs } from "../src/index.js";

function doc(): StyleDocument {
  return {
    kind: "StyleDocument",
    tokens: {
      colors: {
        text: { kind: "rgb", r: 31, g: 35, b: 40 },
        // Chain: link → text → concrete rgb.
        link: { kind: "token-ref", path: "colors.text" },
        // Cycle: a ↔ b.
        a: { kind: "token-ref", path: "colors.b" },
        b: { kind: "token-ref", path: "colors.a" },
      },
      typography: {
        families: { body: ["Inter", "sans-serif"] },
        scale:    { md: { unit: "rem", value: 1 } },
        weights:  { regular: 400 },
        leading:  { normal: 1.5 },
        tracking: { normal: { unit: "em", value: 0 } },
      },
      space:   { md: { unit: "rem", value: 1 } },
      radii:   { sm: { unit: "px", value: 4 } },
      shadows: {
        soft: {
          offsetX: { unit: "px", value: 0 },
          offsetY: { unit: "px", value: 2 },
          blur:    { unit: "px", value: 4 },
          spread:  { unit: "px", value: 0 },
          color:   { kind: "rgb", r: 0, g: 0, b: 0, a: 0.1 },
        },
      },
    },
    rules: [{
      id: styleRuleId("noop"), selector: sel.type("paragraph"), properties: [],
    }],
    contexts: [],
    theme: null,
  };
}

function ref(path: string): TokenRef {
  return { kind: "token-ref", path };
}

describe("resolveTokenRefs — concrete leaves", () => {
  it("resolves a Color", () => {
    const out = resolveTokenRefs(doc(), [ref("colors.text")]);
    expect(out.get("colors.text")).toEqual({ kind: "rgb", r: 31, g: 35, b: 40 });
  });

  it("resolves a Length", () => {
    const out = resolveTokenRefs(doc(), [ref("space.md")]);
    expect(out.get("space.md")).toEqual({ unit: "rem", value: 1 });
  });

  it("resolves a Shadow", () => {
    const out = resolveTokenRefs(doc(), [ref("shadows.soft")]);
    const v = out.get("shadows.soft");
    expect(v).toBeDefined();
    expect((v as { offsetY?: unknown }).offsetY).toEqual({ unit: "px", value: 2 });
  });

  it("resolves a FontStack", () => {
    const out = resolveTokenRefs(doc(), [ref("typography.families.body")]);
    expect(out.get("typography.families.body")).toEqual(["Inter", "sans-serif"]);
  });

  it("resolves a number (font weight)", () => {
    const out = resolveTokenRefs(doc(), [ref("typography.weights.regular")]);
    expect(out.get("typography.weights.regular")).toBe(400);
  });

  it("resolves a number (leading)", () => {
    const out = resolveTokenRefs(doc(), [ref("typography.leading.normal")]);
    expect(out.get("typography.leading.normal")).toBe(1.5);
  });
});

describe("resolveTokenRefs — chains", () => {
  it("follows a one-hop ref chain", () => {
    const out = resolveTokenRefs(doc(), [ref("colors.link")]);
    // link → text → rgb(31,35,40)
    expect(out.get("colors.link")).toEqual({ kind: "rgb", r: 31, g: 35, b: 40 });
  });
});

describe("resolveTokenRefs — failures", () => {
  it("missing path → null", () => {
    const out = resolveTokenRefs(doc(), [ref("colors.nonexistent")]);
    expect(out.get("colors.nonexistent")).toBeNull();
  });

  it("missing top-level bucket → null", () => {
    const out = resolveTokenRefs(doc(), [ref("nope.also-nope")]);
    expect(out.get("nope.also-nope")).toBeNull();
  });

  it("descending past a primitive in the middle of a path → null", () => {
    // Path attempts to descend "into" a number — e.g. `colors.text.r.x`
    // takes the `r` channel (a number) and tries to step further.
    // walkPath should bail when cursor becomes non-object.
    const out = resolveTokenRefs(doc(), [ref("typography.weights.regular.foo")]);
    expect(out.get("typography.weights.regular.foo")).toBeNull();
  });

  it("descending past an explicit-undefined own property → null", () => {
    // A token bucket that has an own property with `undefined` value.
    // walkPath's "value === undefined" guard kicks in.
    const d = doc();
    const dWithUndef: StyleDocument = {
      ...d,
      tokens: {
        ...d.tokens,
        space: { ...d.tokens.space, ghost: undefined as unknown as never },
      },
    };
    const out = resolveTokenRefs(dWithUndef, [ref("space.ghost")]);
    expect(out.get("space.ghost")).toBeNull();
  });

  it("non-leaf landing (path stops at an intermediate object) → null", () => {
    // `colors` itself is an object — it's not a recognised leaf value.
    const out = resolveTokenRefs(doc(), [ref("colors")]);
    expect(out.get("colors")).toBeNull();
  });

  it("cycle → null (depth-capped)", () => {
    const out = resolveTokenRefs(doc(), [ref("colors.a")]);
    expect(out.get("colors.a")).toBeNull();
  });

  it("NaN as a numeric leaf → null (defensive)", () => {
    const d: StyleDocument = doc();
    const dWithNaN: StyleDocument = {
      ...d,
      tokens: {
        ...d.tokens,
        typography: {
          ...d.tokens.typography,
          weights: { broken: NaN },
        },
      },
    };
    const out = resolveTokenRefs(dWithNaN, [ref("typography.weights.broken")]);
    expect(out.get("typography.weights.broken")).toBeNull();
  });
});

describe("resolveTokenRefs — bulk semantics", () => {
  it("empty input → empty map", () => {
    expect(resolveTokenRefs(doc(), []).size).toBe(0);
  });

  it("duplicate paths collapse to one entry (idempotent — same result)", () => {
    const out = resolveTokenRefs(doc(), [ref("space.md"), ref("space.md")]);
    expect(out.size).toBe(1);
    expect(out.get("space.md")).toEqual({ unit: "rem", value: 1 });
  });

  it("mixed resolvable + unresolvable input — every input has an entry", () => {
    const out = resolveTokenRefs(doc(), [
      ref("colors.text"),
      ref("colors.nonexistent"),
      ref("space.md"),
    ]);
    expect(out.size).toBe(3);
    expect(out.get("colors.text")).not.toBeNull();
    expect(out.get("colors.nonexistent")).toBeNull();
    expect(out.get("space.md")).not.toBeNull();
  });
});

describe("resolveTokenRefs — security (prototype traversal)", () => {
  it("refuses __proto__ in a path segment", () => {
    const out = resolveTokenRefs(doc(), [ref("__proto__.toString")]);
    expect(out.get("__proto__.toString")).toBeNull();
  });

  it("refuses constructor in a path segment", () => {
    const out = resolveTokenRefs(doc(), [ref("colors.constructor.name")]);
    expect(out.get("colors.constructor.name")).toBeNull();
  });

  it("refuses prototype in a path segment", () => {
    const out = resolveTokenRefs(doc(), [ref("colors.prototype")]);
    expect(out.get("colors.prototype")).toBeNull();
  });
});
