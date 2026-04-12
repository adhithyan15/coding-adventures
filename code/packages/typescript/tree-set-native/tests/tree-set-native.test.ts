import { describe, expect, it } from "vitest";
import { TreeSet, fromValues } from "../index.js";

describe("TreeSet native addon", () => {
  it("supports insertion and sorted iteration", () => {
    const set = new TreeSet([5, 1, 3, 3, 9]);
    set.add(7);

    expect(set.toSortedArray()).toEqual([1, 3, 5, 7, 9]);
    expect(set.size()).toBe(5);
    expect(set.contains(7)).toBe(true);
  });

  it("supports rank and range helpers", () => {
    const set = new TreeSet([10, 20, 30, 40]);

    expect(set.rank(25)).toBe(2);
    expect(set.byRank(0)).toBe(10);
    expect(set.kthSmallest(3)).toBe(30);
    expect(set.range(15, 35)).toEqual([20, 30]);
  });

  it("supports set algebra", () => {
    const left = new TreeSet([1, 2, 3, 5]);
    const right = new TreeSet([3, 4, 5, 6]);

    expect(left.union(right).toSortedArray()).toEqual([1, 2, 3, 4, 5, 6]);
    expect(left.intersection(right).toSortedArray()).toEqual([3, 5]);
    expect(left.difference(right).toSortedArray()).toEqual([1, 2]);
    expect(left.symmetricDifference(right).toSortedArray()).toEqual([1, 2, 4, 6]);
    expect(left.isSubset(left.union(right))).toBe(true);
    expect(left.isDisjoint(new TreeSet([8, 9]))).toBe(true);
  });

  it("supports comparison, iteration, and string helpers", () => {
    const set = new TreeSet([4, 2, 8]);

    expect(set.first()).toBe(2);
    expect(set.last()).toBe(8);
    expect(set.isSuperset(new TreeSet([2, 4]))).toBe(true);
    expect(set.equals(new TreeSet([2, 4, 8]))).toBe(true);
    expect([...set]).toEqual([2, 4, 8]);
    expect(set.toString()).toContain("TreeSet");
  });

  it("exposes the convenience constructor helper", () => {
    expect(fromValues([3, 1, 2]).toSortedArray()).toEqual([1, 2, 3]);
  });
});
