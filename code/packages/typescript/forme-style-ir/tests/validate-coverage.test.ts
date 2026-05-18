/**
 * validate-coverage.test.ts — additional tests targeting branches
 * not exercised by validate.test.ts.  Mostly happy-path coverage
 * for selector/property variants that the negative tests only
 * touched on the error side.
 *
 * Keeps the validator package above the FM04 §14.4 95% target by
 * exercising the every-variant-happy-path side of the matrix.
 */

import { describe, it, expect } from "vitest";
import {
  StyleError, validateStyleDocument,
  emptyStyleDocument, styleRuleId, sel,
  type StyleDocument, type StyleErrorCode,
} from "../src/index.js";

function codesOf(value: unknown): StyleErrorCode[] {
  try {
    validateStyleDocument(value);
    return [];
  } catch (e) {
    if (e instanceof StyleError) return e.errors.map((x) => x.code);
    throw e;
  }
}

function docWithRules(rules: unknown[]): StyleDocument {
  return { ...emptyStyleDocument(), rules: rules as never };
}

describe("happy-path variants (coverage)", () => {
  it("HSL color literal validates", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "color", value: { kind: "hsl", h: 180, s: 50, l: 50, a: 0.9 } }],
    }]))).toEqual([]);
  });

  it("OKLCH color literal validates", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "color", value: { kind: "oklch", l: 0.5, c: 0.2, h: 90 } }],
    }]))).toEqual([]);
  });

  it("named color validates", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "color", value: { kind: "named", name: "tomato" } }],
    }]))).toEqual([]);
  });

  it("padding with all-TokenRef sides validates", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "padding", value: {
        top:    { kind: "token-ref", path: "space.md" },
        right:  { kind: "token-ref", path: "space.md" },
        bottom: { kind: "token-ref", path: "space.md" },
        left:   { kind: "token-ref", path: "space.md" },
      } }],
    }]))).toEqual([]);
  });

  it("text-decoration with style + color + thickness validates", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "text-decoration", value: {
        line: "underline",
        style: "wavy",
        color: { kind: "named", name: "red" },
        thickness: { unit: "px", value: 2 },
      } }],
    }]))).toEqual([]);
  });

  it("border with sides validates", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "border", value: {
        width: { unit: "px", value: 1 },
        style: "solid",
        color: { kind: "rgb", r: 0, g: 0, b: 0 },
        sides: ["top", "bottom"],
      } }],
    }]))).toEqual([]);
  });

  it("inset shadow with all required fields validates", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, shadows: { ring: {
      offsetX: { unit: "px", value: 0 }, offsetY: { unit: "px", value: 0 },
      blur:    { unit: "px", value: 0 }, spread:  { unit: "px", value: 2 },
      color:   { kind: "rgb", r: 100, g: 100, b: 100 }, inset: true,
    } } } };
    expect(codesOf(doc)).toEqual([]);
  });

  it("non-string id throws MALFORMED", () => {
    expect(codesOf(docWithRules([{
      id: 42, selector: sel.type("p"), properties: [],
    }]))).toContain("MALFORMED");
  });

  it("nth with both number and formula variants validates", () => {
    expect(codesOf(docWithRules([
      { id: "r1", selector: sel.nth(sel.type("p"), 3), properties: [] },
      { id: "r2", selector: sel.nth(sel.type("p"), { a: 2, b: 1 }), properties: [] },
      { id: "r3", selector: sel.nth(sel.type("p"), { a: 2, b: 0, fromEnd: true }), properties: [] },
    ]))).toEqual([]);
  });

  it("nth formula non-object n throws MALFORMED", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: { kind: "nth", of: sel.type("p"), n: "first" },
      properties: [],
    }]))).toContain("MALFORMED");
  });

  it("nth formula fromEnd non-boolean throws MALFORMED", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: { kind: "nth", of: sel.type("p"), n: { a: 1, b: 0, fromEnd: "yes" } },
      properties: [],
    }]))).toContain("MALFORMED");
  });

  it("and / or / not all valid composes succeed", () => {
    expect(codesOf(docWithRules([
      { id: "r1", selector: sel.and(sel.type("p"), sel.tag("intro")), properties: [] },
      { id: "r2", selector: sel.or(sel.type("p"), sel.type("h1")), properties: [] },
      { id: "r3", selector: sel.not(sel.id("excluded")), properties: [] },
    ]))).toEqual([]);
  });

  it("and / or with non-array body throws MALFORMED", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: { kind: "and", all: "not-array" }, properties: [],
    }]))).toContain("MALFORMED");
    expect(codesOf(docWithRules([{
      id: "r", selector: { kind: "or", any: "not-array" }, properties: [],
    }]))).toContain("MALFORMED");
  });

  it("structural relation selectors all pass when valid", () => {
    expect(codesOf(docWithRules([
      { id: "r1", selector: sel.descendantOf(sel.type("blockquote"), sel.type("paragraph")), properties: [] },
      { id: "r2", selector: sel.adjacent(sel.heading(2), sel.type("paragraph")), properties: [] },
    ]))).toEqual([]);
  });

  it("typography weight non-number is caught", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, typography: {
      ...ts.tokens.typography, weights: { regular: NaN as unknown as number },
    } } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("typography leading non-number is caught", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, typography: {
      ...ts.tokens.typography, leading: { normal: NaN as unknown as number },
    } } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("typography families bucket: must be a record", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, typography: {
      ...ts.tokens.typography, families: "no" as unknown as object,
    } } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("extensions field non-object throws MALFORMED", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, extensions: "no" as unknown as object } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("padding value not an object", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "padding", value: "1rem" }],
    }]))).toContain("INVALID_PROPERTY_VALUE");
  });

  it("border value not an object", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "border", value: "1px solid red" }],
    }]))).toContain("INVALID_PROPERTY_VALUE");
  });

  it("text-decoration value not an object", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "text-decoration", value: "underline" }],
    }]))).toContain("INVALID_PROPERTY_VALUE");
  });

  it("property kind non-string", () => {
    expect(codesOf(docWithRules([{
      id: "r", selector: sel.type("p"), properties: [{ kind: 42, value: "x" }],
    }]))).toContain("MALFORMED");
  });

  it("rule with valid context that IS declared in document produces no warnings", () => {
    const doc = {
      ...emptyStyleDocument(),
      contexts: ["dark"],
      rules: [{ id: "r", selector: sel.type("p"), properties: [], context: "dark" }],
    };
    expect(validateStyleDocument(doc).warnings).toEqual([]);
  });

  it("StyleError with zero entries produces a sensible fallback message", () => {
    const e = new StyleError([]);
    expect(e.message).toMatch(/no entries/);
  });

  it("StyleRuleId constructor brands a string", () => {
    const id = styleRuleId("test");
    // type-level brand check is compile-time; runtime is just identity
    expect(id).toBe("test");
  });

  it("cyclic selector graph is bounded by depth guard, not stack overflow", () => {
    // Hand-rolled cycle: not.inner = not (self-referential)
    const cyclic: { kind: "not"; inner?: unknown } = { kind: "not" };
    cyclic.inner = cyclic;
    const codes = codesOf(docWithRules([{
      id: "r", selector: cyclic, properties: [],
    }]));
    expect(codes).toContain("MALFORMED");
    // Specifically, the message names the depth guard.
    const errors = (() => {
      try { validateStyleDocument(docWithRules([{ id: "r", selector: cyclic, properties: [] }])); return []; }
      catch (e) { return (e as StyleError).errors; }
    })();
    expect(errors.some((e) => /exceeds 1000 levels/.test(e.message))).toBe(true);
  });

  it("deeply nested but legal selector below the guard validates", () => {
    // Build `not(not(not(...500 deep)))` — well below the 1000 limit.
    let s: unknown = sel.type("p");
    for (let i = 0; i < 500; i++) s = { kind: "not", inner: s };
    expect(codesOf(docWithRules([{ id: "r", selector: s, properties: [] }])))
      .toEqual([]);
  });
});
