import { describe, expect, it } from "vitest";
import { BloomFilter } from "../src/index.js";

describe("BloomFilter", () => {
  it("starts empty and reports statistics", () => {
    const filter = new BloomFilter();
    expect(filter.bitsSet).toBe(0);
    expect(filter.fillRatio).toBe(0);
    expect(filter.estimatedFalsePositiveRate).toBe(0);
    expect(filter.isOverCapacity()).toBe(false);
    expect(filter.contains("anything")).toBe(false);
  });

  it("has no false negatives for inserted values", () => {
    const filter = new BloomFilter({ expectedItems: 1_000, falsePositiveRate: 0.01 });
    for (let i = 0; i < 250; i += 1) {
      filter.add(`item-${i}`);
    }
    for (let i = 0; i < 250; i += 1) {
      expect(filter.contains(`item-${i}`)).toBe(true);
    }
    expect(filter.bitsSet).toBeGreaterThan(0);
  });

  it("supports explicit parameters", () => {
    const filter = BloomFilter.fromParams(10_000, 7);
    expect(filter.bitCount).toBe(10_000);
    expect(filter.hashCount).toBe(7);
    expect(filter.sizeBytes()).toBe(1_250);
    filter.add("hello");
    expect(filter.contains("hello")).toBe(true);
    expect(filter.isOverCapacity()).toBe(false);
  });

  it("keeps duplicate adds stable for bit counts", () => {
    const filter = new BloomFilter({ expectedItems: 100 });
    filter.add("dup");
    const afterFirst = filter.bitsSet;
    filter.add("dup");
    expect(filter.bitsSet).toBe(afterFirst);
  });

  it("computes sizing helpers", () => {
    const m = BloomFilter.optimalM(1_000_000, 0.01);
    const k = BloomFilter.optimalK(m, 1_000_000);
    expect(m).toBeGreaterThan(9_000_000);
    expect(k).toBe(7);
    expect(BloomFilter.capacityForMemory(1_000_000, 0.01)).toBeGreaterThan(0);
  });

  it("detects over-capacity filters", () => {
    const filter = new BloomFilter({ expectedItems: 3, falsePositiveRate: 0.01 });
    filter.add("a");
    filter.add("b");
    filter.add("c");
    expect(filter.isOverCapacity()).toBe(false);
    filter.add("d");
    expect(filter.isOverCapacity()).toBe(true);
    expect(filter.estimatedFalsePositiveRate).toBeGreaterThan(0);
  });

  it("handles varied element types and rendering", () => {
    const filter = new BloomFilter({ expectedItems: 100 });
    for (const value of [42, 3.14, true, null, { a: 1 }, ["x"], "cafe\u0301"]) {
      filter.add(value);
      expect(filter.contains(value)).toBe(true);
    }
    expect(String(filter)).toContain("BloomFilter");
  });

  it("rejects invalid parameters", () => {
    expect(() => new BloomFilter({ expectedItems: 0 })).toThrow(RangeError);
    expect(() => new BloomFilter({ falsePositiveRate: 0 })).toThrow(RangeError);
    expect(() => new BloomFilter({ falsePositiveRate: 1 })).toThrow(RangeError);
    expect(() => BloomFilter.fromParams(0, 1)).toThrow(RangeError);
    expect(() => BloomFilter.fromParams(1, 0)).toThrow(RangeError);
  });
});
