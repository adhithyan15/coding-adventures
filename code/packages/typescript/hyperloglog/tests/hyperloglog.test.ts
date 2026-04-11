import { describe, expect, it } from "vitest";
import { HyperLogLog } from "../src/index.js";

describe("HyperLogLog", () => {
  it("starts empty", () => {
    const hll = new HyperLogLog();
    expect(hll.count()).toBe(0);
    expect(hll.len()).toBe(0);
  });

  it("ignores duplicates and grows for unique values", () => {
    const hll = new HyperLogLog();
    for (let i = 0; i < 1_000; i += 1) {
      hll.add("same");
    }
    expect(hll.count()).toBeLessThan(10);

    const spread = new HyperLogLog();
    for (let i = 0; i < 1_000; i += 1) {
      spread.add(`item-${i}`);
    }
    expect(spread.count()).toBeGreaterThan(800);
    expect(spread.count()).toBeLessThan(1_200);
  });

  it("merges sketches with the same precision", () => {
    const left = new HyperLogLog(10);
    const right = new HyperLogLog(10);
    for (let i = 0; i < 200; i += 1) {
      left.add(`left-${i}`);
      right.add(`right-${i}`);
    }

    const merged = left.merge(right);
    expect(merged.count()).toBeGreaterThanOrEqual(left.count());
    expect(merged.count()).toBeGreaterThanOrEqual(right.count());
  });

  it("rejects precision mismatches", () => {
    const left = new HyperLogLog(10);
    const right = new HyperLogLog(14);
    expect(left.tryMerge(right)).toBeNull();
    expect(() => left.merge(right)).toThrow("precision mismatch");
  });

  it("exposes helper math", () => {
    expect(HyperLogLog.memoryBytes(14)).toBe(12_288);
    expect(HyperLogLog.optimalPrecision(0.01)).toBe(14);
    expect(HyperLogLog.errorRateForPrecision(14)).toBeGreaterThan(0.008);
  });
});
