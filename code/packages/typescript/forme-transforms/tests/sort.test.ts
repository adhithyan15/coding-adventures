/**
 * sort.test.ts — stable sortBy / sortBy2 with null-last and id tiebreaker.
 */

import { describe, it, expect } from "vitest";
import { sortBy, sortBy2 } from "../src/index.js";

interface Post { id: string; title: string; pubDate: string | null; }

const POSTS: Post[] = [
  { id: "a", title: "Alpha",  pubDate: "2026-01-15" },
  { id: "b", title: "Beta",   pubDate: "2026-02-20" },
  { id: "c", title: "Carrot", pubDate: null },
  { id: "d", title: "Delta",  pubDate: "2025-12-01" },
];

describe("sortBy — direction handling", () => {
  it("asc (default): ascending by extracted key", () => {
    const out = sortBy(POSTS, (p) => p.title);
    expect(out.map((p) => p.id)).toEqual(["a", "b", "c", "d"]);
  });

  it("desc: reverses non-null ordering", () => {
    const out = sortBy(POSTS, (p) => p.title, "desc");
    expect(out.map((p) => p.id)).toEqual(["d", "c", "b", "a"]);
  });
});

describe("sortBy — null/undefined go last", () => {
  it("null pubDate sorts to end on asc", () => {
    const out = sortBy(POSTS, (p) => p.pubDate, "asc");
    expect(out[out.length - 1]!.id).toBe("c");
  });

  it("null pubDate sorts to end on desc too (not flipped)", () => {
    const out = sortBy(POSTS, (p) => p.pubDate, "desc");
    expect(out[out.length - 1]!.id).toBe("c");
  });

  it("two nulls tie and stay in input order", () => {
    const both: Post[] = [
      { id: "x", title: "X", pubDate: null },
      { id: "y", title: "Y", pubDate: null },
    ];
    const out = sortBy(both, (p) => p.pubDate);
    expect(out.map((p) => p.id)).toEqual(["x", "y"]);
  });

  it("undefined treated same as null", () => {
    const input = [{ id: "a", k: 5 }, { id: "b", k: undefined }, { id: "c", k: 3 }];
    const out = sortBy(input, (p) => p.k);
    expect(out.map((p) => p.id)).toEqual(["c", "a", "b"]);
  });
});

describe("sortBy — stability", () => {
  it("ties broken by input index (stable)", () => {
    const input = [
      { id: "a", k: 1 },
      { id: "b", k: 1 },
      { id: "c", k: 1 },
    ];
    const out = sortBy(input, (p) => p.k);
    expect(out.map((p) => p.id)).toEqual(["a", "b", "c"]);
  });

  it("does not mutate input", () => {
    const input = [...POSTS];
    sortBy(input, (p) => p.pubDate);
    expect(input).toEqual(POSTS);
  });

  it("keyFn invoked once per item (memoised internally)", () => {
    let calls = 0;
    sortBy([1, 2, 3, 4, 5], (n) => { calls++; return n; });
    expect(calls).toBe(5);
  });
});

describe("sortBy — numeric and bigint keys", () => {
  it("numeric ascending", () => {
    expect(sortBy([3, 1, 4, 1, 5], (n) => n)).toEqual([1, 1, 3, 4, 5]);
  });

  it("numeric descending", () => {
    expect(sortBy([3, 1, 4, 1, 5], (n) => n, "desc")).toEqual([5, 4, 3, 1, 1]);
  });

  it("NaN keys treated as ties (kept in input order)", () => {
    const input = [{ id: "a", k: NaN }, { id: "b", k: NaN }];
    const out = sortBy(input, (p) => p.k);
    expect(out.map((p) => p.id)).toEqual(["a", "b"]);
  });
});

describe("sortBy2 — primary then secondary key", () => {
  it("primary tie → secondary decides", () => {
    const input = [
      { id: "a", pri: 1, sec: 30 },
      { id: "b", pri: 1, sec: 10 },
      { id: "c", pri: 2, sec: 20 },
    ];
    const out = sortBy2(input, (p) => p.pri, (p) => p.sec);
    expect(out.map((p) => p.id)).toEqual(["b", "a", "c"]);
  });

  it("primary alone is enough when no ties", () => {
    const input = [
      { id: "a", pri: 3, sec: 0 },
      { id: "b", pri: 1, sec: 0 },
      { id: "c", pri: 2, sec: 0 },
    ];
    const out = sortBy2(input, (p) => p.pri, (p) => p.sec);
    expect(out.map((p) => p.id)).toEqual(["b", "c", "a"]);
  });

  it("primary desc + secondary asc (FM00 archive idiom)", () => {
    const input = [
      { id: "a", pubDate: "2026-01-01", title: "Z" },
      { id: "b", pubDate: "2026-01-01", title: "A" },
      { id: "c", pubDate: "2026-02-01", title: "M" },
    ];
    const out = sortBy2(
      input,
      (p) => p.pubDate,
      (p) => p.title,
      "desc",
      "asc",
    );
    expect(out.map((p) => p.id)).toEqual(["c", "b", "a"]);
  });

  it("null primary still goes last; secondary then breaks ties among nulls", () => {
    const input = [
      { id: "a", pri: null, sec: 2 },
      { id: "b", pri: null, sec: 1 },
      { id: "c", pri: 5,    sec: 9 },
    ];
    const out = sortBy2(input, (p) => p.pri, (p) => p.sec);
    expect(out.map((p) => p.id)).toEqual(["c", "b", "a"]);
  });

  it("both keys tie → input order preserved (index tiebreaker)", () => {
    const input = [
      { id: "a", pri: 1, sec: 1 },
      { id: "b", pri: 1, sec: 1 },
      { id: "c", pri: 1, sec: 1 },
    ];
    const out = sortBy2(input, (p) => p.pri, (p) => p.sec);
    expect(out.map((p) => p.id)).toEqual(["a", "b", "c"]);
  });

  it("does not mutate input", () => {
    const input = [...POSTS];
    sortBy2(input, (p) => p.pubDate, (p) => p.id);
    expect(input).toEqual(POSTS);
  });
});
