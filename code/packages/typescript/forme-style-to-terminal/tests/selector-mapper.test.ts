/**
 * selector-mapper.test.ts — selectorDescription (informational comments).
 */

import { describe, it, expect } from "vitest";
import { sel } from "@coding-adventures/forme-style-ir";
import { selectorDescription } from "../src/index.js";

describe("selectorDescription — simple kinds", () => {
  it("node-type", () => {
    expect(selectorDescription(sel.type("paragraph"))).toBe("node-type:paragraph");
  });
  it("node-type-level", () => {
    expect(selectorDescription({ kind: "node-type-level", level: 1 })).toBe("heading-level:1");
  });
  it("custom-kind", () => {
    expect(selectorDescription({ kind: "custom-kind", customKind: "Callout" })).toBe("custom-kind:Callout");
  });
  it("tag", () => {
    expect(selectorDescription({ kind: "tag", tag: "warning" })).toBe("tag:warning");
  });
  it("id", () => {
    expect(selectorDescription({ kind: "id", id: "main" })).toBe("id:main");
  });
  it("role", () => {
    expect(selectorDescription({ kind: "role", role: "note" })).toBe("role:note");
  });
});

describe("selectorDescription — composition", () => {
  const p = sel.type("p");
  const h1 = { kind: "node-type-level" as const, level: 1 };

  it("nth with numeric index", () => {
    expect(selectorDescription({ kind: "nth", n: 0, of: p })).toBe("nth(0, node-type:p)");
  });
  it("nth with an+b formula", () => {
    expect(selectorDescription({ kind: "nth", n: { a: 2, b: 1 }, of: p })).toBe("nth(2n+1, node-type:p)");
  });
  it("nth with negative b", () => {
    expect(selectorDescription({ kind: "nth", n: { a: 3, b: -2 }, of: p })).toBe("nth(3n-2, node-type:p)");
  });
  it("nth with fromEnd flag", () => {
    expect(selectorDescription({ kind: "nth", n: { a: 1, b: 0, fromEnd: true }, of: p }))
      .toBe("nth(1n+0 from-end, node-type:p)");
  });
  it("child-of", () => {
    expect(selectorDescription({ kind: "child-of", parent: p, child: h1 }))
      .toBe("child-of(node-type:p, heading-level:1)");
  });
  it("descendant-of", () => {
    expect(selectorDescription({ kind: "descendant-of", ancestor: p, descendant: h1 }))
      .toBe("descendant-of(node-type:p, heading-level:1)");
  });
  it("adjacent", () => {
    expect(selectorDescription({ kind: "adjacent", previous: p, following: h1 }))
      .toBe("adjacent(node-type:p, heading-level:1)");
  });
  it("and", () => {
    expect(selectorDescription({ kind: "and", all: [p, h1] })).toBe("and(node-type:p, heading-level:1)");
  });
  it("or", () => {
    expect(selectorDescription({ kind: "or", any: [p, h1] })).toBe("or(node-type:p, heading-level:1)");
  });
  it("not", () => {
    expect(selectorDescription({ kind: "not", inner: p })).toBe("not(node-type:p)");
  });
});

describe("selectorDescription — depth cap (FM04 §9.6)", () => {
  it("a deeply nested selector returns a truncation marker rather than overflowing", () => {
    // Build a `not(not(not(...)))` chain 200 levels deep — well past
    // MAX_DESC_DEPTH (64).  Should NOT throw.
    let s: Parameters<typeof selectorDescription>[0] = sel.type("paragraph");
    for (let i = 0; i < 200; i++) {
      s = { kind: "not", inner: s };
    }
    const out = selectorDescription(s);
    expect(out).toContain("…(truncated)");
  });
});

describe("selectorDescription — defensive sanitisation", () => {
  it("strips ANSI-unsafe bytes from selector targets", () => {
    expect(selectorDescription(sel.type("evil\x1bname"))).toBe("node-type:evilname");
  });

  it("strips control chars from custom-kind / tag / id / role", () => {
    expect(selectorDescription({ kind: "custom-kind", customKind: "x\x00y" })).toBe("custom-kind:xy");
    expect(selectorDescription({ kind: "tag", tag: "x\x9by" })).toBe("tag:xy");
    expect(selectorDescription({ kind: "id", id: "x\x7fy" })).toBe("id:xy");
    expect(selectorDescription({ kind: "role", role: "x\x1by" })).toBe("role:xy");
  });
});
