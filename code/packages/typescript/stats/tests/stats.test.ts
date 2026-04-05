import { describe, it, expect } from "vitest";
import {
  mean,
  median,
  mode,
  variance,
  standardDeviation,
  min,
  max,
  range,
  frequencyCount,
  frequencyDistribution,
  chiSquared,
  chiSquaredText,
  indexOfCoincidence,
  entropy,
  ENGLISH_FREQUENCIES,
} from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────
// Descriptive Statistics
// ─────────────────────────────────────────────────────────────────────

describe("mean", () => {
  it("computes the arithmetic mean (parity test vector)", () => {
    expect(mean([1, 2, 3, 4, 5])).toBeCloseTo(3.0, 10);
  });

  it("computes mean of the worked example", () => {
    expect(mean([2, 4, 4, 4, 5, 5, 7, 9])).toBeCloseTo(5.0, 10);
  });

  it("returns the value for a single-element array", () => {
    expect(mean([42])).toBe(42);
  });

  it("throws on empty array", () => {
    expect(() => mean([])).toThrow("empty");
  });
});

describe("median", () => {
  it("returns middle value for odd-length array", () => {
    expect(median([1, 3, 5])).toBe(3);
  });

  it("averages two middle values for even-length array", () => {
    expect(median([2, 4, 4, 4, 5, 5, 7, 9])).toBeCloseTo(4.5, 10);
  });

  it("handles unsorted input", () => {
    expect(median([5, 1, 3])).toBe(3);
  });

  it("returns the value for single element", () => {
    expect(median([7])).toBe(7);
  });

  it("throws on empty array", () => {
    expect(() => median([])).toThrow("empty");
  });
});

describe("mode", () => {
  it("finds the most frequent value", () => {
    expect(mode([2, 4, 4, 4, 5, 5, 7, 9])).toBe(4);
  });

  it("returns first occurrence on tie", () => {
    // 1 and 2 both appear twice; 1 comes first
    expect(mode([1, 2, 1, 2, 3])).toBe(1);
  });

  it("handles single element", () => {
    expect(mode([99])).toBe(99);
  });

  it("throws on empty array", () => {
    expect(() => mode([])).toThrow("empty");
  });
});

describe("variance", () => {
  const values = [2, 4, 4, 4, 5, 5, 7, 9];

  it("computes sample variance (parity test vector)", () => {
    expect(variance(values)).toBeCloseTo(4.571428571428571, 10);
  });

  it("computes population variance (parity test vector)", () => {
    expect(variance(values, true)).toBeCloseTo(4.0, 10);
  });

  it("throws on empty array", () => {
    expect(() => variance([])).toThrow("empty");
  });

  it("throws on single-element sample variance", () => {
    expect(() => variance([5])).toThrow("at least 2");
  });

  it("allows single-element population variance", () => {
    expect(variance([5], true)).toBe(0);
  });
});

describe("standardDeviation", () => {
  it("is the square root of sample variance", () => {
    const values = [2, 4, 4, 4, 5, 5, 7, 9];
    expect(standardDeviation(values)).toBeCloseTo(Math.sqrt(4.571428571428571), 10);
  });

  it("is the square root of population variance", () => {
    const values = [2, 4, 4, 4, 5, 5, 7, 9];
    expect(standardDeviation(values, true)).toBeCloseTo(2.0, 10);
  });
});

describe("min", () => {
  it("finds the minimum value", () => {
    expect(min([2, 4, 4, 4, 5, 5, 7, 9])).toBe(2);
  });

  it("handles negative values", () => {
    expect(min([-3, -1, 0, 5])).toBe(-3);
  });

  it("handles single element", () => {
    expect(min([42])).toBe(42);
  });

  it("throws on empty array", () => {
    expect(() => min([])).toThrow("empty");
  });
});

describe("max", () => {
  it("finds the maximum value", () => {
    expect(max([2, 4, 4, 4, 5, 5, 7, 9])).toBe(9);
  });

  it("handles negative values", () => {
    expect(max([-3, -1, 0, 5])).toBe(5);
  });

  it("handles single element", () => {
    expect(max([42])).toBe(42);
  });

  it("throws on empty array", () => {
    expect(() => max([])).toThrow("empty");
  });
});

describe("range", () => {
  it("computes max - min (worked example)", () => {
    expect(range([2, 4, 4, 4, 5, 5, 7, 9])).toBeCloseTo(7.0, 10);
  });

  it("returns 0 for identical values", () => {
    expect(range([5, 5, 5])).toBe(0);
  });

  it("throws on empty array", () => {
    expect(() => range([])).toThrow("empty");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Frequency Analysis
// ─────────────────────────────────────────────────────────────────────

describe("frequencyCount", () => {
  it("counts letters case-insensitively", () => {
    const counts = frequencyCount("Hello!");
    expect(counts.get("H")).toBe(1);
    expect(counts.get("E")).toBe(1);
    expect(counts.get("L")).toBe(2);
    expect(counts.get("O")).toBe(1);
  });

  it("ignores non-alphabetic characters", () => {
    const counts = frequencyCount("123!@#");
    expect(counts.size).toBe(0);
  });

  it("handles empty string", () => {
    const counts = frequencyCount("");
    expect(counts.size).toBe(0);
  });
});

describe("frequencyDistribution", () => {
  it("converts counts to proportions", () => {
    const dist = frequencyDistribution("AABB");
    expect(dist.get("A")).toBeCloseTo(0.5, 10);
    expect(dist.get("B")).toBeCloseTo(0.5, 10);
  });

  it("proportions sum to 1.0", () => {
    const dist = frequencyDistribution("HELLO WORLD");
    let sum = 0;
    for (const p of dist.values()) {
      sum += p;
    }
    expect(sum).toBeCloseTo(1.0, 10);
  });

  it("handles empty string", () => {
    const dist = frequencyDistribution("");
    expect(dist.size).toBe(0);
  });
});

describe("chiSquared", () => {
  it("computes chi-squared (parity test vector)", () => {
    expect(chiSquared([10, 20, 30], [20, 20, 20])).toBeCloseTo(10.0, 10);
  });

  it("returns 0 when observed equals expected", () => {
    expect(chiSquared([20, 20, 20], [20, 20, 20])).toBeCloseTo(0.0, 10);
  });

  it("throws on mismatched lengths", () => {
    expect(() => chiSquared([1, 2], [1])).toThrow("same length");
  });

  it("throws on empty arrays", () => {
    expect(() => chiSquared([], [])).toThrow("empty");
  });

  it("throws when expected contains zero", () => {
    expect(() => chiSquared([1], [0])).toThrow("zero");
  });
});

describe("chiSquaredText", () => {
  it("computes chi-squared of text against frequencies", () => {
    // A simple text where we know the distribution.
    const result = chiSquaredText("AAAA", { A: 1.0 });
    expect(result).toBeCloseTo(0.0, 10);
  });

  it("returns 0 for empty text", () => {
    expect(chiSquaredText("", ENGLISH_FREQUENCIES)).toBe(0);
  });

  it("returns a positive value for non-English text", () => {
    const result = chiSquaredText("ZZZZZZZZZZ", ENGLISH_FREQUENCIES);
    expect(result).toBeGreaterThan(0);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Cryptanalysis Helpers
// ─────────────────────────────────────────────────────────────────────

describe("indexOfCoincidence", () => {
  it("computes IC (parity test vector: AABB)", () => {
    // A=2, B=2, N=4
    // IC = (2*1 + 2*1) / (4*3) = 4/12 = 0.333...
    expect(indexOfCoincidence("AABB")).toBeCloseTo(1 / 3, 10);
  });

  it("returns 1.0 for repeated single letter", () => {
    // All same letter: IC = n*(n-1) / (n*(n-1)) = 1.0
    expect(indexOfCoincidence("AAAA")).toBeCloseTo(1.0, 10);
  });

  it("returns 0 for text shorter than 2 chars", () => {
    expect(indexOfCoincidence("A")).toBe(0);
    expect(indexOfCoincidence("")).toBe(0);
  });

  it("approaches 1/26 for uniformly distributed text", () => {
    // Build a string with each letter appearing exactly once.
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const ic = indexOfCoincidence(alphabet);
    // IC = 26 * (1*0) / (26*25) = 0/650 = 0
    // With each letter once, n_i=1, so n_i*(n_i-1) = 0 for all.
    expect(ic).toBe(0);
  });
});

describe("entropy", () => {
  it("computes maximum entropy for uniform distribution", () => {
    // 26 letters each appearing once: entropy = log2(26) ~ 4.700
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    expect(entropy(alphabet)).toBeCloseTo(Math.log2(26), 2);
  });

  it("returns 0 for single letter repeated", () => {
    expect(entropy("AAAA")).toBeCloseTo(0.0, 10);
  });

  it("returns 1.0 for two equally frequent letters", () => {
    // H = -2 * (0.5 * log2(0.5)) = -2 * (-0.5) = 1.0
    expect(entropy("AABB")).toBeCloseTo(1.0, 10);
  });

  it("returns 0 for empty string", () => {
    expect(entropy("")).toBe(0);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────

describe("ENGLISH_FREQUENCIES", () => {
  it("has 26 entries", () => {
    expect(Object.keys(ENGLISH_FREQUENCIES).length).toBe(26);
  });

  it("frequencies sum to approximately 1.0", () => {
    const sum = Object.values(ENGLISH_FREQUENCIES).reduce((a, b) => a + b, 0);
    expect(sum).toBeCloseTo(1.0, 2);
  });

  it("E is the most common letter", () => {
    expect(ENGLISH_FREQUENCIES["E"]).toBeGreaterThan(
      ENGLISH_FREQUENCIES["T"]
    );
  });

  it("Z is the least common letter", () => {
    for (const [letter, freq] of Object.entries(ENGLISH_FREQUENCIES)) {
      if (letter !== "Z") {
        expect(freq).toBeGreaterThanOrEqual(ENGLISH_FREQUENCIES["Z"]);
      }
    }
  });
});
