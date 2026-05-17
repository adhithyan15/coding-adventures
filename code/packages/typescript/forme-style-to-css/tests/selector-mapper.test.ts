/**
 * selector-mapper.test.ts — every selector form maps to its
 * documented CSS form per FM04 §9.2.
 */

import { describe, it, expect } from "vitest";
import { selectorToCss } from "../src/index.js";
import { sel } from "@coding-adventures/forme-style-ir";

describe("selectorToCss — identity selectors", () => {
  it("node-type → element name", () => {
    expect(selectorToCss(sel.type("paragraph"))).toBe("paragraph");
    expect(selectorToCss(sel.type("blockquote"))).toBe("blockquote");
  });

  it("node-type-level → h1..h6", () => {
    for (const lv of [1, 2, 3, 4, 5, 6] as const) {
      expect(selectorToCss(sel.heading(lv))).toBe(`h${lv}`);
    }
  });

  it("custom-kind → [data-kind=...]", () => {
    expect(selectorToCss(sel.custom("callout"))).toBe(`[data-kind="callout"]`);
  });

  it("tag → [data-tag~=...] (whitespace-set match)", () => {
    expect(selectorToCss(sel.tag("warning"))).toBe(`[data-tag~="warning"]`);
  });

  it("id → #name", () => {
    expect(selectorToCss(sel.id("intro"))).toBe(`#intro`);
  });

  it("role → [role=...]", () => {
    expect(selectorToCss(sel.role("byline"))).toBe(`[role="byline"]`);
  });
});

describe("selectorToCss — nth", () => {
  it("0-based literal → 1-based CSS :nth-child", () => {
    expect(selectorToCss(sel.nth(sel.type("p"), 0))).toBe("p:nth-child(1)");
    expect(selectorToCss(sel.nth(sel.type("p"), 2))).toBe("p:nth-child(3)");
  });

  it("formula → :nth-child(an+b)", () => {
    expect(selectorToCss(sel.nth(sel.type("li"), { a: 2, b: 1 })))
      .toBe("li:nth-child(2n+1)");
  });

  it("formula with negative b", () => {
    expect(selectorToCss(sel.nth(sel.type("li"), { a: 3, b: -1 })))
      .toBe("li:nth-child(3n-1)");
  });

  it("fromEnd → :nth-last-child", () => {
    expect(selectorToCss(sel.nth(sel.type("li"), { a: 1, b: 0, fromEnd: true })))
      .toBe("li:nth-last-child(1n+0)");
  });
});

describe("selectorToCss — structural relations", () => {
  it("child-of → '<parent> > <child>'", () => {
    expect(selectorToCss(sel.childOf(sel.type("ul"), sel.type("li"))))
      .toBe("ul > li");
  });

  it("descendant-of → '<a> <b>'", () => {
    expect(selectorToCss(sel.descendantOf(sel.type("blockquote"), sel.type("paragraph"))))
      .toBe("blockquote paragraph");
  });

  it("adjacent → '<a> + <b>'", () => {
    expect(selectorToCss(sel.adjacent(sel.heading(2), sel.type("paragraph"))))
      .toBe("h2 + paragraph");
  });
});

describe("selectorToCss — composition", () => {
  it("and concatenates (no whitespace)", () => {
    expect(selectorToCss(sel.and(sel.type("paragraph"), sel.tag("intro"))))
      .toBe(`paragraph[data-tag~="intro"]`);
  });

  it("or comma-separates", () => {
    expect(selectorToCss(sel.or(sel.type("paragraph"), sel.heading(1))))
      .toBe("paragraph, h1");
  });

  it("not wraps in :not()", () => {
    expect(selectorToCss(sel.not(sel.id("excluded"))))
      .toBe(":not(#excluded)");
  });

  it("not(or(...)) becomes :not(a, b)", () => {
    expect(selectorToCss(sel.not(sel.or(sel.id("a"), sel.id("b")))))
      .toBe(":not(#a, #b)");
  });

  it("and over an or expands cartesian product", () => {
    // (p OR h1) AND .intro  →  p.intro, h1.intro
    expect(selectorToCss(sel.and(sel.or(sel.type("p"), sel.heading(1)), sel.tag("intro"))))
      .toBe(`p[data-tag~="intro"], h1[data-tag~="intro"]`);
  });

  it("child-of over an or expands per path", () => {
    expect(selectorToCss(sel.childOf(sel.or(sel.type("ul"), sel.type("ol")), sel.type("li"))))
      .toBe("ul > li, ol > li");
  });
});

describe("selectorToCss — escaping", () => {
  it("attribute value: escapes embedded double quote", () => {
    expect(selectorToCss(sel.custom(`box"quote`)))
      .toBe(`[data-kind="box\\"quote"]`);
  });

  it("attribute value: escapes embedded backslash", () => {
    expect(selectorToCss(sel.custom(`box\\quote`)))
      .toBe(`[data-kind="box\\\\quote"]`);
  });

  it("ident: keeps safe chars unchanged", () => {
    expect(selectorToCss(sel.type("my-element-2"))).toBe("my-element-2");
  });
});
