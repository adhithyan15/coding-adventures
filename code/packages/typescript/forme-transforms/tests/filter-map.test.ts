/**
 * filter-map.test.ts — filter, map, flatMap pure-transform behaviour.
 */

import { describe, it, expect } from "vitest";
import { filter, map, flatMap } from "../src/index.js";

describe("filter", () => {
  it("keeps items where predicate returns true", () => {
    expect(filter([1, 2, 3, 4], (n) => n % 2 === 0)).toEqual([2, 4]);
  });

  it("passes index as the second arg", () => {
    expect(filter(["a", "b", "c"], (_v, i) => i > 0)).toEqual(["b", "c"]);
  });

  it("does not mutate the input array", () => {
    const input = [1, 2, 3];
    filter(input, (n) => n > 1);
    expect(input).toEqual([1, 2, 3]);
  });

  it("returns a fresh array each call", () => {
    const input = [1, 2, 3];
    const a = filter(input, () => true);
    const b = filter(input, () => true);
    expect(a).not.toBe(b);
    expect(a).not.toBe(input);
  });

  it("empty input → empty output", () => {
    expect(filter([], () => true)).toEqual([]);
  });

  it("everything-rejected → empty output", () => {
    expect(filter([1, 2, 3], () => false)).toEqual([]);
  });
});

describe("map", () => {
  it("applies mapper to each item", () => {
    expect(map([1, 2, 3], (n) => n * 2)).toEqual([2, 4, 6]);
  });

  it("passes index as the second arg", () => {
    expect(map(["a", "b", "c"], (v, i) => `${i}:${v}`)).toEqual(["0:a", "1:b", "2:c"]);
  });

  it("preserves input length exactly", () => {
    expect(map([10, 20, 30, 40], (n) => n + 1).length).toBe(4);
  });

  it("does not mutate the input array", () => {
    const input = [{ x: 1 }, { x: 2 }];
    map(input, (o) => o.x);
    expect(input).toEqual([{ x: 1 }, { x: 2 }]);
  });

  it("empty input → empty output", () => {
    expect(map([], (n: number) => n * 2)).toEqual([]);
  });

  it("type-changes input → output (T → U)", () => {
    const out: string[] = map([1, 2, 3], (n) => `n=${n}`);
    expect(out).toEqual(["n=1", "n=2", "n=3"]);
  });
});

describe("flatMap", () => {
  it("flattens one level", () => {
    expect(flatMap([1, 2, 3], (n) => [n, n * 10])).toEqual([1, 10, 2, 20, 3, 30]);
  });

  it("empty inner arrays are dropped", () => {
    expect(flatMap([1, 2, 3], (n) => (n === 2 ? [] : [n]))).toEqual([1, 3]);
  });

  it("passes index as the second arg", () => {
    expect(flatMap(["a", "b"], (v, i) => [`${i}-${v}`])).toEqual(["0-a", "1-b"]);
  });

  it("empty input → empty output", () => {
    expect(flatMap([], () => [1])).toEqual([]);
  });

  it("does not mutate the input array", () => {
    const input = [1, 2];
    flatMap(input, (n) => [n, n]);
    expect(input).toEqual([1, 2]);
  });

  it("does NOT flatten nested arrays deeper than one level", () => {
    const out = flatMap([1, 2], (n) => [[n, n + 1]] as unknown as number[]);
    // First-level flatten produces [[1,2],[2,3]] not [1,2,2,3].
    expect(out.length).toBe(2);
  });
});
