/**
 * Tests for nproc -- print the number of processing units available.
 *
 * We test the exported `getProcessorCount` function, which returns
 * the number of CPU cores minus an optional `ignore` count, clamped
 * to a minimum of 1.
 *
 * Note: ESM namespaces cannot be spied on with vi.spyOn for native
 * modules like `os`, so we test the function's behavior using the
 * real CPU count and exercise the clamping/ignore logic directly.
 */

import { describe, it, expect } from "vitest";
import * as os from "node:os";
import { getProcessorCount } from "../src/nproc.js";

// Get the real CPU count once for comparison in tests.
const REAL_CPU_COUNT = os.cpus().length;

describe("getProcessorCount", () => {
  // -------------------------------------------------------------------------
  // Default behavior (no ignore)
  // -------------------------------------------------------------------------

  it("should return a positive integer", () => {
    const count = getProcessorCount();
    expect(count).toBeGreaterThan(0);
    expect(Number.isInteger(count)).toBe(true);
  });

  it("should match os.cpus().length when ignore is 0", () => {
    const result = getProcessorCount(0);
    expect(result).toBe(REAL_CPU_COUNT);
  });

  it("should match os.cpus().length when ignore is not specified", () => {
    const result = getProcessorCount();
    expect(result).toBe(REAL_CPU_COUNT);
  });

  // -------------------------------------------------------------------------
  // --ignore flag
  // -------------------------------------------------------------------------

  it("should subtract ignore count from total", () => {
    if (REAL_CPU_COUNT > 1) {
      const result = getProcessorCount(1);
      expect(result).toBe(REAL_CPU_COUNT - 1);
    }
  });

  it("should subtract 2 from total when ignore is 2", () => {
    if (REAL_CPU_COUNT > 2) {
      const result = getProcessorCount(2);
      expect(result).toBe(REAL_CPU_COUNT - 2);
    }
  });

  it("should clamp to minimum of 1 when ignore exceeds total", () => {
    const result = getProcessorCount(99999);
    expect(result).toBe(1);
  });

  it("should clamp to minimum of 1 when ignore equals total", () => {
    const result = getProcessorCount(REAL_CPU_COUNT);
    expect(result).toBe(1);
  });

  it("should clamp to minimum of 1 when ignore is total minus 1 plus 1", () => {
    // ignore = total results in max(1, 0) = 1
    const result = getProcessorCount(REAL_CPU_COUNT);
    expect(result).toBe(1);
  });

  it("should return total when ignore is 0", () => {
    const result = getProcessorCount(0);
    expect(result).toBe(REAL_CPU_COUNT);
  });

  // -------------------------------------------------------------------------
  // Clamping behavior
  // -------------------------------------------------------------------------

  it("should never return less than 1", () => {
    // Test with a range of ignore values, including extreme ones.
    for (const ignore of [0, 1, 10, 100, 1000, 999999]) {
      const result = getProcessorCount(ignore);
      expect(result).toBeGreaterThanOrEqual(1);
    }
  });

  it("should return at least 1 even with negative CPU count scenario", () => {
    // Even if someone passes a huge ignore value, the result is always >= 1.
    const result = getProcessorCount(Number.MAX_SAFE_INTEGER);
    expect(result).toBe(1);
  });

  // -------------------------------------------------------------------------
  // Type verification
  // -------------------------------------------------------------------------

  it("should always return an integer", () => {
    expect(Number.isInteger(getProcessorCount())).toBe(true);
    expect(Number.isInteger(getProcessorCount(0))).toBe(true);
    expect(Number.isInteger(getProcessorCount(1))).toBe(true);
  });

  it("should return a number type", () => {
    expect(typeof getProcessorCount()).toBe("number");
  });
});
