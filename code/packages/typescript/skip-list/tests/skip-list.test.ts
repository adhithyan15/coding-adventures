import { describe, expect, it } from "vitest";
import { SkipList } from "../src/index.js";

describe("SkipList", () => {
  it("inserts, searches, and deletes in sorted order", () => {
    const list = new SkipList<number, string>();
    list.insert(20, "b");
    list.insert(10, "a");
    list.insert(30, "c");

    expect(list.toList()).toEqual([10, 20, 30]);
    expect(list.search(20)).toBe("b");
    expect(list.delete(20)).toBe(true);
    expect(list.search(20)).toBeUndefined();
    expect(list.toList()).toEqual([10, 30]);
  });

  it("computes rank and by-rank consistently", () => {
    const list = new SkipList<number, string>();
    for (const key of [50, 10, 30, 20]) {
      list.insert(key, String(key));
    }

    expect(list.rank(10)).toBe(0);
    expect(list.rank(20)).toBe(1);
    expect(list.byRank(2)).toBe(30);
    expect(list.byRank(10)).toBeUndefined();
  });

  it("returns bounded ranges", () => {
    const list = new SkipList<number, string>();
    for (const key of [10, 20, 30, 40, 50]) {
      list.insert(key, String(key));
    }

    expect(list.range(15, 45, true)).toEqual([
      [20, "20"],
      [30, "30"],
      [40, "40"],
    ]);
    expect(list.range(10, 40, false)).toEqual([
      [20, "20"],
      [30, "30"],
    ]);
  });

  it("supports custom comparators for composite keys", () => {
    const comparator = (
      left: readonly [number, string],
      right: readonly [number, string],
    ) => {
      if (left[0] !== right[0]) {
        return left[0] - right[0];
      }
      return left[1].localeCompare(right[1]);
    };

    const list = new SkipList<readonly [number, string], string>(comparator);
    list.insert([10, "b"], "b");
    list.insert([10, "a"], "a");
    list.insert([5, "z"], "z");

    expect(list.toList()).toEqual([
      [5, "z"],
      [10, "a"],
      [10, "b"],
    ]);
  });
});
