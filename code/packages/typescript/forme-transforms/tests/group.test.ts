/**
 * group.test.ts — partition, groupBy, unique.
 */

import { describe, it, expect } from "vitest";
import { partition, groupBy, unique } from "../src/index.js";

describe("partition", () => {
  it("splits into { yes, no }", () => {
    const { yes, no } = partition([1, 2, 3, 4, 5], (n) => n % 2 === 0);
    expect(yes).toEqual([2, 4]);
    expect(no).toEqual([1, 3, 5]);
  });

  it("preserves input order in both halves", () => {
    const { yes, no } = partition([5, 4, 3, 2, 1], (n) => n > 2);
    expect(yes).toEqual([5, 4, 3]);
    expect(no).toEqual([2, 1]);
  });

  it("all-true → empty no", () => {
    const { yes, no } = partition([1, 2, 3], () => true);
    expect(yes).toEqual([1, 2, 3]);
    expect(no).toEqual([]);
  });

  it("all-false → empty yes", () => {
    const { yes, no } = partition([1, 2, 3], () => false);
    expect(yes).toEqual([]);
    expect(no).toEqual([1, 2, 3]);
  });

  it("empty input → empty both", () => {
    const { yes, no } = partition([], () => true);
    expect(yes).toEqual([]);
    expect(no).toEqual([]);
  });

  it("passes index to predicate", () => {
    const { yes } = partition(["a", "b", "c", "d"], (_, i) => i % 2 === 0);
    expect(yes).toEqual(["a", "c"]);
  });

  it("does not mutate input", () => {
    const input = [1, 2, 3];
    partition(input, (n) => n > 1);
    expect(input).toEqual([1, 2, 3]);
  });
});

describe("groupBy", () => {
  it("buckets items by key", () => {
    const out = groupBy(["apple", "ant", "bear", "boa"], (s) => s[0]);
    expect(out.get("a")).toEqual(["apple", "ant"]);
    expect(out.get("b")).toEqual(["bear", "boa"]);
  });

  it("returns a Map (not a plain object) — protection against __proto__ keys", () => {
    const out = groupBy(["x"], () => "__proto__");
    expect(out).toBeInstanceOf(Map);
    expect(out.get("__proto__")).toEqual(["x"]);
    // The Object prototype must not have been polluted.
    expect(({} as Record<string, unknown>).x).toBeUndefined();
  });

  it("Map iteration order = first-seen-bucket order", () => {
    const out = groupBy(
      [{ k: "b" }, { k: "a" }, { k: "b" }, { k: "c" }],
      (o) => o.k,
    );
    expect([...out.keys()]).toEqual(["b", "a", "c"]);
  });

  it("within-bucket order = input order", () => {
    const out = groupBy([1, 4, 2, 5, 3, 6], (n) => n % 2 === 0 ? "even" : "odd");
    expect(out.get("odd")).toEqual([1, 5, 3]);
    expect(out.get("even")).toEqual([4, 2, 6]);
  });

  it("supports numeric keys", () => {
    const out = groupBy([1.1, 2.2, 1.5, 2.9], (n) => Math.floor(n));
    expect(out.get(1)).toEqual([1.1, 1.5]);
    expect(out.get(2)).toEqual([2.2, 2.9]);
  });

  it("empty input → empty Map", () => {
    const out = groupBy([], () => "x");
    expect(out.size).toBe(0);
  });

  it("does not mutate input", () => {
    const input = [1, 2, 3];
    groupBy(input, (n) => n);
    expect(input).toEqual([1, 2, 3]);
  });
});

describe("unique — identity dedupe", () => {
  it("removes duplicate primitives, first occurrence wins", () => {
    expect(unique([1, 2, 1, 3, 2, 4])).toEqual([1, 2, 3, 4]);
  });

  it("works on strings", () => {
    expect(unique(["js", "ts", "js", "rs"])).toEqual(["js", "ts", "rs"]);
  });

  it("empty input → empty output", () => {
    expect(unique([])).toEqual([]);
  });

  it("all-unique → copy of input", () => {
    expect(unique([1, 2, 3])).toEqual([1, 2, 3]);
  });

  it("does not mutate input", () => {
    const input = [1, 2, 1, 3];
    unique(input);
    expect(input).toEqual([1, 2, 1, 3]);
  });

  it("preserves first reference for object identity", () => {
    const obj = { x: 1 };
    const out = unique([obj, { x: 1 }, obj]);
    expect(out.length).toBe(2);
    expect(out[0]).toBe(obj);
  });
});

describe("unique — keyFn dedupe", () => {
  it("dedupes by extracted key, first occurrence wins", () => {
    const posts = [
      { slug: "hello", title: "Hello v1" },
      { slug: "hello", title: "Hello v2" },
      { slug: "world", title: "World" },
    ];
    const out = unique(posts, (p) => p.slug);
    expect(out).toEqual([
      { slug: "hello", title: "Hello v1" },
      { slug: "world", title: "World" },
    ]);
  });

  it("keyFn can return non-string keys", () => {
    const out = unique(
      [{ a: 1, b: 1 }, { a: 2, b: 1 }, { a: 3, b: 2 }],
      (o) => o.b,
    );
    expect(out.length).toBe(2);
  });

  it("empty input → empty output", () => {
    expect(unique([], (n: number) => n)).toEqual([]);
  });

  it("__proto__ key doesn't pollute Object.prototype", () => {
    const out = unique([1, 2, 3], () => "__proto__");
    expect(out).toEqual([1]);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });
});
