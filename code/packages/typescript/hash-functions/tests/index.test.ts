import { describe, expect, it } from "vitest";

import {
  avalancheScore,
  distributionTest,
  djb2,
  fnv1a32,
  fnv1a64,
  murmur3_32,
  polynomialRolling,
} from "../src/index";

describe("FNV-1a", () => {
  it("matches 32-bit vectors", () => {
    expect(fnv1a32("")).toBe(2166136261);
    expect(fnv1a32("a")).toBe(3826002220);
    expect(fnv1a32("abc")).toBe(440920331);
    expect(fnv1a32("hello")).toBe(1335831723);
    expect(fnv1a32("foobar")).toBe(3214735720);
  });

  it("matches 64-bit vectors", () => {
    expect(fnv1a64("")).toBe(14695981039346656037n);
    expect(fnv1a64("a")).toBe(12638187200555641996n);
    expect(fnv1a64("abc")).toBe(16654208175385433931n);
    expect(fnv1a64("hello")).toBe(11831194018420276491n);
  });

  it("treats strings as UTF-8 bytes", () => {
    expect(fnv1a32("cafe")).toBe(fnv1a32(new TextEncoder().encode("cafe")));
  });
});

describe("DJB2", () => {
  it("matches known vectors", () => {
    expect(djb2("")).toBe(5381n);
    expect(djb2("a")).toBe(177670n);
    expect(djb2("abc")).toBe(193485963n);
    expect(djb2("hello")).toBe(210714636441n);
  });
});

describe("polynomialRolling", () => {
  it("matches manual computations", () => {
    expect(polynomialRolling("")).toBe(0n);
    expect(polynomialRolling("a")).toBe(97n);
    expect(polynomialRolling("ab")).toBe(3105n);
    expect(polynomialRolling("abc")).toBe(96354n);
  });

  it("honors custom parameters", () => {
    expect(polynomialRolling("hello", 37n)).not.toBe(polynomialRolling("hello"));
    expect(polynomialRolling("hello world", 31n, 100n)).toBeLessThan(100n);
    expect(() => polynomialRolling("x", 31n, 0n)).toThrow(RangeError);
  });
});

describe("murmur3_32", () => {
  it("matches source-of-truth vectors", () => {
    expect(murmur3_32("", 0)).toBe(0);
    expect(murmur3_32("", 1)).toBe(0x514e28b7);
    expect(murmur3_32("a", 0)).toBe(0x3c2569b2);
    expect(murmur3_32("abc", 0)).toBe(0xb3dd93fa);
  });

  it("covers all tail paths and seed variation", () => {
    expect(murmur3_32("abcd")).not.toBe(murmur3_32("abce"));
    expect(murmur3_32("abcde")).toBeGreaterThanOrEqual(0);
    expect(murmur3_32("abcdef")).toBeGreaterThanOrEqual(0);
    expect(murmur3_32("abcdefg")).toBeGreaterThanOrEqual(0);
    expect(murmur3_32("hello", 0)).not.toBe(murmur3_32("hello", 1));
  });
});

describe("analysis helpers", () => {
  it("computes bounded avalanche scores", () => {
    const score = avalancheScore(fnv1a32, 32, 8);
    expect(score).toBeGreaterThanOrEqual(0);
    expect(score).toBeLessThanOrEqual(1);
  });

  it("computes exact chi-squared values", () => {
    const chi2 = distributionTest(() => 0, ["a", "b", "c", "d"], 4);
    expect(chi2).toBe(12);
  });

  it("rejects invalid analysis inputs", () => {
    expect(() => avalancheScore(fnv1a32, 0, 1)).toThrow(RangeError);
    expect(() => avalancheScore(fnv1a32, 32, 0)).toThrow(RangeError);
    expect(() => distributionTest(fnv1a32, [], 10)).toThrow(RangeError);
    expect(() => distributionTest(fnv1a32, ["x"], 0)).toThrow(RangeError);
  });
});
