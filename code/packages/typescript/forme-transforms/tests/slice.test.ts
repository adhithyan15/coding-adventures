/**
 * slice.test.ts — take, drop, slice, chunk.
 */

import { describe, it, expect } from "vitest";
import { take, drop, slice, chunk } from "../src/index.js";

describe("take", () => {
  it("returns the first N items", () => {
    expect(take([1, 2, 3, 4, 5], 3)).toEqual([1, 2, 3]);
  });

  it("N larger than input → copy of input", () => {
    expect(take([1, 2], 100)).toEqual([1, 2]);
  });

  it("N === 0 → empty", () => {
    expect(take([1, 2, 3], 0)).toEqual([]);
  });

  it("negative N is clamped to 0 → empty", () => {
    expect(take([1, 2, 3], -5)).toEqual([]);
  });

  it("NaN → empty", () => {
    expect(take([1, 2, 3], NaN)).toEqual([]);
  });

  it("Infinity → empty (Number.isFinite gate)", () => {
    expect(take([1, 2, 3], Infinity)).toEqual([]);
  });

  it("fractional N is floored", () => {
    expect(take([1, 2, 3, 4], 2.7)).toEqual([1, 2]);
  });

  it("does not mutate input", () => {
    const input = [1, 2, 3];
    take(input, 2);
    expect(input).toEqual([1, 2, 3]);
  });
});

describe("drop", () => {
  it("skips the first N items", () => {
    expect(drop([1, 2, 3, 4, 5], 2)).toEqual([3, 4, 5]);
  });

  it("N === 0 → copy of input", () => {
    expect(drop([1, 2, 3], 0)).toEqual([1, 2, 3]);
  });

  it("negative N → copy of input", () => {
    expect(drop([1, 2, 3], -5)).toEqual([1, 2, 3]);
  });

  it("N larger than input → empty", () => {
    expect(drop([1, 2], 100)).toEqual([]);
  });

  it("NaN → copy of input", () => {
    expect(drop([1, 2], NaN)).toEqual([1, 2]);
  });

  it("does not mutate input", () => {
    const input = [1, 2, 3];
    drop(input, 1);
    expect(input).toEqual([1, 2, 3]);
  });

  it("returns a fresh array (not the input itself) even when N=0", () => {
    const input = [1, 2, 3];
    const out = drop(input, 0);
    expect(out).not.toBe(input);
  });
});

describe("slice", () => {
  it("returns items[start..end)", () => {
    expect(slice([1, 2, 3, 4, 5], 1, 4)).toEqual([2, 3, 4]);
  });

  it("defaults: start=0, end=length → copy of input", () => {
    expect(slice([1, 2, 3])).toEqual([1, 2, 3]);
  });

  it("end omitted → to end of array", () => {
    expect(slice([1, 2, 3, 4, 5], 2)).toEqual([3, 4, 5]);
  });

  it("negative start clamps to 0 (no from-end behaviour)", () => {
    expect(slice([1, 2, 3], -1)).toEqual([1, 2, 3]);
  });

  it("end < start collapses to empty", () => {
    expect(slice([1, 2, 3, 4, 5], 3, 1)).toEqual([]);
  });

  it("end beyond length clamps to length", () => {
    expect(slice([1, 2, 3], 1, 100)).toEqual([2, 3]);
  });

  it("does not mutate input", () => {
    const input = [1, 2, 3];
    slice(input, 0, 2);
    expect(input).toEqual([1, 2, 3]);
  });
});

describe("chunk", () => {
  it("splits into batches of N", () => {
    expect(chunk([1, 2, 3, 4, 5, 6], 2)).toEqual([[1, 2], [3, 4], [5, 6]]);
  });

  it("final batch is smaller when not a multiple", () => {
    expect(chunk([1, 2, 3, 4, 5], 2)).toEqual([[1, 2], [3, 4], [5]]);
  });

  it("empty input → empty output", () => {
    expect(chunk([], 5)).toEqual([]);
  });

  it("size > input length → single batch of the whole input", () => {
    expect(chunk([1, 2], 100)).toEqual([[1, 2]]);
  });

  it("throws RangeError on size 0", () => {
    expect(() => chunk([1, 2, 3], 0)).toThrow(RangeError);
  });

  it("throws RangeError on negative size", () => {
    expect(() => chunk([1, 2, 3], -1)).toThrow(RangeError);
  });

  it("throws RangeError on fractional size", () => {
    expect(() => chunk([1, 2, 3], 2.5)).toThrow(RangeError);
  });

  it("throws RangeError on NaN size", () => {
    expect(() => chunk([1, 2, 3], NaN)).toThrow(RangeError);
  });

  it("throws RangeError on Infinity size", () => {
    expect(() => chunk([1, 2, 3], Infinity)).toThrow(RangeError);
  });

  it("does not mutate input", () => {
    const input = [1, 2, 3, 4];
    chunk(input, 2);
    expect(input).toEqual([1, 2, 3, 4]);
  });
});
