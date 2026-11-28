import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { median, medianInterval } from "../src/index.js";

function signCountOracle(n: number, confidence: number): number {
  let rank = 0;
  for (let k = 1; k <= n; k += 1) {
    let count = 0;
    for (let mask = 0; mask < 2 ** n; mask += 1) {
      let below = 0;
      for (let bit = 0; bit < n; bit += 1) if ((mask & (1 << bit)) !== 0) below += 1;
      if (below <= k - 1) count += 1;
    }
    if (2 * count * 100 <= (100 - confidence) * 2 ** n) rank = k;
  }
  return rank;
}

describe("rank statistics", () => {
  it("computes odd and even medians without mutating", () => {
    const values = [9, 1, 5, 3];
    expect(median(values)).toBe(4);
    expect(values).toEqual([9, 1, 5, 3]);
    expect(median([Number.MAX_VALUE, Number.MAX_VALUE])).toBe(Number.MAX_VALUE);
    expect(median([-Number.MAX_VALUE, Number.MAX_VALUE])).toBe(0);
    expect(median([Number.MIN_VALUE, Number.MIN_VALUE])).toBe(Number.MIN_VALUE);
    expect(median([Number.MIN_VALUE, 2 * Number.MIN_VALUE])).toBe(2 * Number.MIN_VALUE);
  });

  it("selects exact interval ranks and retains ties", () => {
    expect(medianInterval([1, 2, 3, 4, 5], 95)).toBeNull();
    expect(medianInterval([0.8, 0.8, 1.2, 1.2, 1.2, 0.8], 50)).toEqual({ low: 0.8, high: 1.2 });
    for (let n = 1; n <= 10; n += 1) {
      const values = Array.from({ length: n }, (_, i) => i % 3);
      for (let confidence = 1; confidence <= 99; confidence += 1) {
        const want = signCountOracle(n, confidence);
        const got = medianInterval(values, confidence);
        expect(got).toEqual(want === 0 ? null : { low: values.slice().sort((a, b) => a - b)[want - 1], high: values.slice().sort((a, b) => a - b)[n - want] });
      }
    }
  });

  it("rejects invalid samples and confidence values", () => {
    for (const values of [[], [NaN], [Infinity], [-Infinity]]) {
      expect(() => median(values)).toThrow();
      expect(() => medianInterval(values, 95)).toThrow();
    }
    expect(() => medianInterval([1], 0)).toThrow(RangeError);
    expect(() => medianInterval([1], 100)).toThrow(RangeError);
    expect(() => medianInterval([1], 95.5)).toThrow(RangeError);
  });

  it("replays the shared portable fixture and verifies the package copy", () => {
    const packaged = readFileSync(new URL("../testdata/microbenchmark-pairs-v1.json", import.meta.url), "utf8");
    const shared = readFileSync(new URL("../../../../specs/fixtures/microbenchmark-pairs-v1.json", import.meta.url), "utf8");
    expect(packaged).toBe(shared);
    const fixture = JSON.parse(packaged) as { confidence_percent: number; cases: Array<{ baseline_ns: number; candidate_ns: number[]; ratio: number; interval: [number, number] | null }> };
    for (const example of fixture.cases) {
      const ratios = example.candidate_ns.map((candidate) => candidate / example.baseline_ns);
      const interval = medianInterval(ratios, fixture.confidence_percent);
      expect(median(ratios)).toBe(example.ratio);
      expect(interval === null ? null : [interval.low, interval.high]).toEqual(example.interval);
    }
  });
});
