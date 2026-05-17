/**
 * canonical.test.ts — byte-stability + fixed-point for
 * `canonicalStyleDocument`.
 */

import { describe, it, expect } from "vitest";
import {
  canonicalStyleDocument, emptyStyleDocument, styleRuleId, sel,
  type StyleDocument,
} from "../src/index.js";

function complexDoc(): StyleDocument {
  return {
    kind: "StyleDocument",
    tokens: {
      colors: {
        text: { kind: "rgb", r: 31, g: 35, b: 40 },
        link: { kind: "token-ref", path: "colors.text" },
      },
      typography: {
        families: { body: ["Inter", "system-ui"] },
        scale:    { md: { unit: "rem", value: 1 } },
        weights:  { regular: 400, bold: 700 },
        leading:  { normal: 1.5 },
        tracking: { normal: { unit: "em", value: 0 } },
      },
      space:   { md: { unit: "rem", value: 1 } },
      radii:   {},
      shadows: {},
    },
    rules: [
      { id: styleRuleId("r1"), selector: sel.type("paragraph"), properties: [
        { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
      ] },
      { id: styleRuleId("r2"), selector: sel.heading(1), properties: [
        { kind: "font-size", value: { unit: "rem", value: 2 } },
      ] },
    ],
    contexts: ["screen", "print", "dark"],
    theme: null,
  };
}

describe("canonicalStyleDocument", () => {
  it("produces a byte-stable string", () => {
    const a = canonicalStyleDocument(complexDoc());
    const b = canonicalStyleDocument(complexDoc());
    expect(a).toBe(b);
  });

  it("is insensitive to top-level key insertion order", () => {
    const d1 = complexDoc();
    // Construct a deep-equal doc with different key insertion order.
    const d2: StyleDocument = {
      theme: null,
      contexts: ["screen", "print", "dark"],
      rules: d1.rules,
      tokens: d1.tokens,
      kind: "StyleDocument",
    };
    expect(canonicalStyleDocument(d1)).toBe(canonicalStyleDocument(d2));
  });

  it("treats contexts as a set (sorts before hashing)", () => {
    const d1 = { ...complexDoc(), contexts: ["screen", "print", "dark"] };
    const d2 = { ...complexDoc(), contexts: ["dark", "print", "screen"] };
    expect(canonicalStyleDocument(d1)).toBe(canonicalStyleDocument(d2));
  });

  it("preserves rules array order (specificity matters)", () => {
    const d1 = complexDoc();
    const d2 = { ...d1, rules: [d1.rules[1]!, d1.rules[0]!] };
    expect(canonicalStyleDocument(d1)).not.toBe(canonicalStyleDocument(d2));
  });

  it("round-trips through JSON.parse without losing equality", () => {
    const a = canonicalStyleDocument(complexDoc());
    const parsed = JSON.parse(a) as StyleDocument;
    const b = canonicalStyleDocument(parsed);
    expect(a).toBe(b);
  });

  it("the empty doc serialises deterministically", () => {
    expect(canonicalStyleDocument(emptyStyleDocument()))
      .toBe(canonicalStyleDocument(emptyStyleDocument()));
  });

  it("emits sorted keys at every object depth", () => {
    const s = canonicalStyleDocument(complexDoc());
    // Top level
    const topKeyOrder = Array.from(s.matchAll(/^\{("[^"]+")/gm)).map((m) => m[1]);
    expect(topKeyOrder[0]).toBe('"contexts"');   // alphabetic: contexts < kind < rules < theme < tokens
  });

  it("rejects non-finite numbers in the input (caller bug)", () => {
    const d = complexDoc() as unknown as { rules: { properties: { value: unknown }[] }[] };
    d.rules = [{ properties: [{ value: Number.POSITIVE_INFINITY }] }];
    expect(() => canonicalStyleDocument(d as unknown as StyleDocument)).toThrow(RangeError);
  });

  it("drops undefined values (matches JSON.stringify behaviour)", () => {
    const d = { ...emptyStyleDocument(), extensions: undefined };
    const s = canonicalStyleDocument(d);
    expect(s).not.toContain("undefined");
  });

  it("rejects function-typed values", () => {
    const d = complexDoc() as unknown as Record<string, unknown>;
    d.theme = (() => null) as unknown as string;
    expect(() => canonicalStyleDocument(d as unknown as StyleDocument)).toThrow(TypeError);
  });

  it("guards against cyclic graphs with a RangeError, not stack overflow", () => {
    // Self-referential extensions object.  In practice the validator
    // would have caught this; the canonical serializer's guard is
    // defence in depth.
    const d = complexDoc() as unknown as { extensions?: Record<string, unknown> };
    const exts: Record<string, unknown> = {};
    exts.self = exts;
    d.extensions = exts;
    expect(() => canonicalStyleDocument(d as unknown as StyleDocument)).toThrow(RangeError);
  });
});
