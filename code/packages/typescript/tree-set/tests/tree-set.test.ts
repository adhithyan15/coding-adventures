import { describe, expect, it } from "vitest";
import { TreeSet, fromValues } from "../src/index.js";

describe("TreeSet", () => {
  it("supports insertion, lookup, and sorted iteration", () => {
    const set = new TreeSet([5, 1, 3, 3, 9]);
    set.add(7);

    expect(set.toSortedArray()).toEqual([1, 3, 5, 7, 9]);
    expect(set.size()).toBe(5);
    expect(set.length).toBe(5);
    expect(set.contains(7)).toBe(true);
    expect(set.has(2)).toBe(false);
  });

  it("supports rank and selection helpers", () => {
    const set = fromValues([10, 20, 30, 40]);

    expect(set.rank(5)).toBe(0);
    expect(set.rank(25)).toBe(2);
    expect(set.byRank(0)).toBe(10);
    expect(set.byRank(3)).toBe(40);
    expect(set.kthSmallest(3)).toBe(30);
    expect(set.predecessor(30)).toBe(20);
    expect(set.successor(30)).toBe(40);
  });

  it("supports range queries", () => {
    const set = fromValues([1, 3, 5, 7, 9]);

    expect(set.range(3, 7)).toEqual([3, 5, 7]);
    expect(set.range(3, 7, false)).toEqual([5]);
  });

  it("supports set algebra", () => {
    const left = fromValues([1, 2, 3, 5]);
    const right = fromValues([3, 4, 5, 6]);

    expect(left.union(right).toSortedArray()).toEqual([1, 2, 3, 4, 5, 6]);
    expect(left.intersection(right).toSortedArray()).toEqual([3, 5]);
    expect(left.difference(right).toSortedArray()).toEqual([1, 2]);
    expect(left.symmetricDifference(right).toSortedArray()).toEqual([1, 2, 4, 6]);
    expect(left.isSubset(left.union(right))).toBe(true);
    expect(left.isSuperset(left.intersection(right))).toBe(true);
    expect(left.isDisjoint(fromValues([8, 9]))).toBe(true);
    expect(left.equals(fromValues([1, 2, 3, 5]))).toBe(true);
  });

  it("supports custom comparators", () => {
    const byLength = new TreeSet<string>([], (left, right) => {
      if (left.length < right.length) return -1;
      if (left.length > right.length) return 1;
      return left.localeCompare(right);
    });
    byLength.add("banana");
    byLength.add("fig");
    byLength.add("apple");

    expect(byLength.toSortedArray()).toEqual(["fig", "apple", "banana"]);
  });
});

