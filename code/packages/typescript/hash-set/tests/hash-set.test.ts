import { describe, expect, it } from "vitest";
import { HashSet } from "../src/index.js";

describe("HashSet", () => {
  it("tracks membership and clones immutably", () => {
    const base = HashSet.fromList(["alpha", "beta"]);
    const next = base.add("gamma");

    expect(base.has("gamma")).toBe(false);
    expect(next.has("gamma")).toBe(true);
    expect(base.size).toBe(2);
    expect(next.size).toBe(3);
  });

  it("removes entries without mutating the original", () => {
    const base = HashSet.fromList(["alpha", "beta"]);
    const next = base.remove("alpha");

    expect(base.has("alpha")).toBe(true);
    expect(next.has("alpha")).toBe(false);
  });

  it("supports set algebra", () => {
    const left = HashSet.fromList(["alpha", "beta", "gamma"]);
    const right = HashSet.fromList(["beta", "delta"]);

    expect(left.union(right).toList().sort()).toEqual([
      "alpha",
      "beta",
      "delta",
      "gamma",
    ]);
    expect(left.intersection(right).toList()).toEqual(["beta"]);
    expect(left.difference(right).toList().sort()).toEqual(["alpha", "gamma"]);
    expect(left.symmetricDifference(right).toList().sort()).toEqual([
      "alpha",
      "delta",
      "gamma",
    ]);
  });

  it("supports relation helpers", () => {
    const left = HashSet.fromList(["alpha", "beta"]);
    const right = HashSet.fromList(["alpha", "beta", "gamma"]);
    const disjoint = HashSet.fromList(["delta"]);

    expect(left.isSubset(right)).toBe(true);
    expect(right.isSuperset(left)).toBe(true);
    expect(left.isDisjoint(disjoint)).toBe(true);
    expect(left.equals(HashSet.fromList(["beta", "alpha"]))).toBe(true);
  });
});
