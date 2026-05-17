/**
 * selectors.test.ts — `Selector` union, frozen kind list, `sel.*` helpers.
 */

import { describe, it, expect } from "vitest";
import { SELECTOR_KINDS, sel } from "../src/index.js";

describe("SELECTOR_KINDS", () => {
  it("matches the kind discriminants of every Selector variant", () => {
    expect(SELECTOR_KINDS).toEqual([
      "node-type", "node-type-level", "custom-kind",
      "tag", "id", "role",
      "nth",
      "child-of", "descendant-of", "adjacent",
      "and", "or", "not",
    ]);
  });

  it("is frozen", () => {
    expect(() => (SELECTOR_KINDS as unknown as string[]).push("zz")).toThrow();
  });
});

describe("sel.* helpers", () => {
  it("type() builds a node-type selector", () => {
    expect(sel.type("paragraph")).toEqual({ kind: "node-type", type: "paragraph" });
  });

  it("heading() lifts heading-level styling", () => {
    expect(sel.heading(2)).toEqual({ kind: "node-type-level", type: "heading", level: 2 });
  });

  it("custom() targets a plugin-registered content kind", () => {
    expect(sel.custom("callout")).toEqual({ kind: "custom-kind", customKind: "callout" });
  });

  it("tag(), id(), role() are bare identity selectors", () => {
    expect(sel.tag("warning")).toEqual({ kind: "tag", tag: "warning" });
    expect(sel.id("intro")).toEqual({ kind: "id", id: "intro" });
    expect(sel.role("byline")).toEqual({ kind: "role", role: "byline" });
  });

  it("nth() composes an index selector around an inner one", () => {
    const inner = sel.type("paragraph");
    expect(sel.nth(inner, 0)).toEqual({ kind: "nth", of: inner, n: 0 });
    expect(sel.nth(inner, { a: 2, b: 1 })).toEqual({ kind: "nth", of: inner, n: { a: 2, b: 1 } });
  });

  it("structural relation selectors carry both halves", () => {
    const parent = sel.type("list");
    const child  = sel.type("list_item");
    expect(sel.childOf(parent, child)).toEqual({ kind: "child-of", parent, child });
    expect(sel.descendantOf(parent, child)).toEqual({ kind: "descendant-of", ancestor: parent, descendant: child });
    expect(sel.adjacent(parent, child)).toEqual({ kind: "adjacent", previous: parent, following: child });
  });

  it("and() / or() / not() compose multiple selectors", () => {
    const a = sel.type("paragraph");
    const b = sel.tag("intro");
    expect(sel.and(a, b)).toEqual({ kind: "and", all: [a, b] });
    expect(sel.or(a, b)).toEqual({ kind: "or", any: [a, b] });
    expect(sel.not(a)).toEqual({ kind: "not", inner: a });
  });
});
