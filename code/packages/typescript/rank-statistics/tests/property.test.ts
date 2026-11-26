import { describe, expect, it } from "vitest";
import { median, medianInterval } from "../src/index.js";

class SeededValues {
  private state: number;
  constructor(seed: number) { this.state = seed >>> 0; }
  next(max: number): number {
    this.state ^= this.state << 13;
    this.state ^= this.state >>> 17;
    this.state ^= this.state << 5;
    return (this.state >>> 0) % max;
  }
}

describe("generated rank-statistics properties", () => {
  it("median is permutation invariant and lies in the observed range", () => {
    const rng = new SeededValues(0x4d454449);
    for (let sample = 0; sample < 2048; sample += 1) {
      const length = rng.next(40) + 1;
      const values = Array.from({ length }, () => rng.next(200001) - 100000);
      const reverse = values.slice().reverse();
      const value = median(values);
      expect(median(reverse)).toBe(value);
      expect(value).toBeGreaterThanOrEqual(Math.min(...values));
      expect(value).toBeLessThanOrEqual(Math.max(...values));
    }
  });

  it("interval endpoints are observed values and ordered", () => {
    const rng = new SeededValues(0x494e5445);
    for (let sample = 0; sample < 2048; sample += 1) {
      const length = rng.next(24) + 1;
      const values = Array.from({ length }, () => rng.next(65) - 32);
      const confidence = rng.next(99) + 1;
      const interval = medianInterval(values, confidence);
      if (interval !== null) {
        expect(interval.low).toBeLessThanOrEqual(interval.high);
        expect(values).toContain(interval.low);
        expect(values).toContain(interval.high);
      }
    }
  });
});
