/**
 * pipe.test.ts — left-to-right composition over arrays.
 */

import { describe, it, expect } from "vitest";
import { pipe, filter, map, sortBy, take } from "../src/index.js";

describe("pipe — basic composition", () => {
  it("no steps → returns input unchanged", () => {
    const input = [1, 2, 3];
    const out = pipe(input);
    expect(out).toEqual([1, 2, 3]);
  });

  it("one step", () => {
    const out = pipe([1, 2, 3], (xs) => map(xs, (n) => n * 2));
    expect(out).toEqual([2, 4, 6]);
  });

  it("multiple steps run left-to-right", () => {
    const out = pipe([5, 3, 1, 4, 2],
      (xs) => filter(xs, (n) => n > 1),
      (xs) => sortBy(xs, (n) => n),
      (xs) => take(xs, 2),
    );
    expect(out).toEqual([2, 3]);
  });

  it("each step's output becomes the next step's input", () => {
    const seen: number[][] = [];
    pipe([1, 2, 3],
      (xs) => { seen.push([...xs]); return map(xs, (n) => n + 10); },
      (xs) => { seen.push([...xs]); return map(xs, (n) => n + 100); },
      (xs) => { seen.push([...xs]); return xs; },
    );
    expect(seen).toEqual([
      [1, 2, 3],
      [11, 12, 13],
      [111, 112, 113],
    ]);
  });

  it("type-changes between steps (T → U → V)", () => {
    const out = pipe(["a", "bb", "ccc"],
      (xs) => map(xs, (s) => s.length),
      (xs) => map(xs, (n) => `len=${n}`),
    );
    expect(out).toEqual(["len=1", "len=2", "len=3"]);
  });
});

describe("pipe — real-world Forme pipeline shapes", () => {
  interface Post { id: string; pubDate: string | null; draft: boolean; }

  const POSTS: Post[] = [
    { id: "a", pubDate: "2026-01-01", draft: false },
    { id: "b", pubDate: "2026-02-01", draft: true },
    { id: "c", pubDate: "2025-12-01", draft: false },
    { id: "d", pubDate: null,         draft: false },
    { id: "e", pubDate: "2026-03-01", draft: false },
  ];

  it("'recent published posts' shape", () => {
    const recent = pipe(POSTS,
      (xs) => filter(xs, (p) => !p.draft),
      (xs) => sortBy(xs, (p) => p.pubDate, "desc"),
      (xs) => take(xs, 2),
    );
    expect((recent as Post[]).map((p) => p.id)).toEqual(["e", "a"]);
  });

  it("'extract IDs of all non-draft posts' shape (map at the end)", () => {
    const ids = pipe(POSTS,
      (xs) => filter(xs, (p) => !p.draft),
      (xs) => map(xs, (p) => p.id),
    );
    expect(ids).toEqual(["a", "c", "d", "e"]);
  });

  it("does not mutate the input array", () => {
    const before = [...POSTS];
    pipe(POSTS,
      (xs) => filter(xs, (p) => !p.draft),
      (xs) => sortBy(xs, (p) => p.id),
    );
    expect(POSTS).toEqual(before);
  });
});

describe("pipe — edge cases", () => {
  it("empty input survives through all steps", () => {
    const out = pipe([] as number[],
      (xs) => map(xs, (n) => n * 2),
      (xs) => filter(xs, (n) => n > 0),
    );
    expect(out).toEqual([]);
  });
});
