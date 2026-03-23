/**
 * Tests for the shift-and-add multiplier.
 *
 * Verifies:
 * - Correct products for various inputs (0×0, 1×1, 5×3, max×max)
 * - Per-step trace data (partial products, running totals)
 * - Edge cases: multiply by zero, multiply by one, powers of two
 * - Error handling: mismatched lengths, empty arrays
 * - Double-width output: products that need more bits than inputs
 */

import { describe, it, expect } from "vitest";
import { shiftAndAddMultiplier } from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

/** Convert an integer to an LSB-first bit array of the given width. */
function intToBits(n: number, width: number): Bit[] {
  return Array.from({ length: width }, (_, i) => ((n >> i) & 1) as Bit);
}

/** Convert an LSB-first bit array back to an integer. */
function bitsToInt(bits: Bit[]): number {
  return bits.reduce((acc, bit, i) => acc + (bit << i), 0);
}

// ---------------------------------------------------------------------------
// Basic multiplication
// ---------------------------------------------------------------------------

describe("shiftAndAddMultiplier", () => {
  it("0 × 0 = 0", () => {
    const result = shiftAndAddMultiplier(intToBits(0, 4), intToBits(0, 4));
    expect(bitsToInt(result.product)).toBe(0);
  });

  it("1 × 1 = 1", () => {
    const result = shiftAndAddMultiplier(intToBits(1, 4), intToBits(1, 4));
    expect(bitsToInt(result.product)).toBe(1);
  });

  it("5 × 3 = 15", () => {
    const result = shiftAndAddMultiplier(intToBits(5, 4), intToBits(3, 4));
    expect(bitsToInt(result.product)).toBe(15);
  });

  it("7 × 7 = 49", () => {
    const result = shiftAndAddMultiplier(intToBits(7, 4), intToBits(7, 4));
    expect(bitsToInt(result.product)).toBe(49);
  });

  it("15 × 15 = 225 (max 4-bit × max 4-bit)", () => {
    const result = shiftAndAddMultiplier(intToBits(15, 4), intToBits(15, 4));
    expect(bitsToInt(result.product)).toBe(225);
  });

  it("6 × 9 = 54", () => {
    const result = shiftAndAddMultiplier(intToBits(6, 4), intToBits(9, 4));
    expect(bitsToInt(result.product)).toBe(54);
  });

  // ---------------------------------------------------------------------------
  // Edge cases: multiply by zero
  // ---------------------------------------------------------------------------

  it("any × 0 = 0", () => {
    const result = shiftAndAddMultiplier(intToBits(13, 4), intToBits(0, 4));
    expect(bitsToInt(result.product)).toBe(0);
  });

  it("0 × any = 0", () => {
    const result = shiftAndAddMultiplier(intToBits(0, 4), intToBits(11, 4));
    expect(bitsToInt(result.product)).toBe(0);
  });

  // ---------------------------------------------------------------------------
  // Edge cases: multiply by one
  // ---------------------------------------------------------------------------

  it("any × 1 = any", () => {
    const result = shiftAndAddMultiplier(intToBits(13, 4), intToBits(1, 4));
    expect(bitsToInt(result.product)).toBe(13);
  });

  it("1 × any = any", () => {
    const result = shiftAndAddMultiplier(intToBits(1, 4), intToBits(9, 4));
    expect(bitsToInt(result.product)).toBe(9);
  });

  // ---------------------------------------------------------------------------
  // Powers of two (shift behavior)
  // ---------------------------------------------------------------------------

  it("3 × 2 = 6 (multiply by 2 = shift left by 1)", () => {
    const result = shiftAndAddMultiplier(intToBits(3, 4), intToBits(2, 4));
    expect(bitsToInt(result.product)).toBe(6);
  });

  it("5 × 4 = 20 (multiply by 4 = shift left by 2)", () => {
    const result = shiftAndAddMultiplier(intToBits(5, 4), intToBits(4, 4));
    expect(bitsToInt(result.product)).toBe(20);
  });

  it("3 × 8 = 24 (multiply by 8 = shift left by 3)", () => {
    const result = shiftAndAddMultiplier(intToBits(3, 4), intToBits(8, 4));
    expect(bitsToInt(result.product)).toBe(24);
  });

  // ---------------------------------------------------------------------------
  // Different bit widths
  // ---------------------------------------------------------------------------

  it("works with 2-bit inputs", () => {
    // 3 × 3 = 9
    const result = shiftAndAddMultiplier(intToBits(3, 2), intToBits(3, 2));
    expect(bitsToInt(result.product)).toBe(9);
  });

  it("works with 8-bit inputs", () => {
    // 200 × 100 = 20000
    const result = shiftAndAddMultiplier(intToBits(200, 8), intToBits(100, 8));
    expect(bitsToInt(result.product)).toBe(20000);
  });

  // ---------------------------------------------------------------------------
  // Product width is double the input width
  // ---------------------------------------------------------------------------

  it("product has 2N bits", () => {
    const result = shiftAndAddMultiplier(intToBits(5, 4), intToBits(3, 4));
    expect(result.product.length).toBe(8);
  });

  it("product needs full double width for max inputs", () => {
    // 15 × 15 = 225 = 11100001 in binary (needs 8 bits)
    const result = shiftAndAddMultiplier(intToBits(15, 4), intToBits(15, 4));
    expect(result.product.length).toBe(8);
    expect(bitsToInt(result.product)).toBe(225);
  });

  // ---------------------------------------------------------------------------
  // Trace data: steps
  // ---------------------------------------------------------------------------

  it("produces one step per multiplier bit", () => {
    const result = shiftAndAddMultiplier(intToBits(5, 4), intToBits(3, 4));
    expect(result.steps.length).toBe(4);
    expect(result.steps[0].bitIndex).toBe(0);
    expect(result.steps[1].bitIndex).toBe(1);
    expect(result.steps[2].bitIndex).toBe(2);
    expect(result.steps[3].bitIndex).toBe(3);
  });

  it("step records correct multiplier bits", () => {
    // B = 3 = 0011 (LSB first: [1, 1, 0, 0])
    const result = shiftAndAddMultiplier(intToBits(5, 4), intToBits(3, 4));
    expect(result.steps[0].multiplierBit).toBe(1); // bit 0
    expect(result.steps[1].multiplierBit).toBe(1); // bit 1
    expect(result.steps[2].multiplierBit).toBe(0); // bit 2
    expect(result.steps[3].multiplierBit).toBe(0); // bit 3
  });

  it("step partial products are zero when multiplier bit is 0", () => {
    // B = 1 = 0001 (LSB first: [1, 0, 0, 0])
    const result = shiftAndAddMultiplier(intToBits(5, 4), intToBits(1, 4));
    // Only step 0 has a non-zero partial product
    expect(bitsToInt(result.steps[0].partialProduct)).toBe(5);
    expect(bitsToInt(result.steps[1].partialProduct)).toBe(0);
    expect(bitsToInt(result.steps[2].partialProduct)).toBe(0);
    expect(bitsToInt(result.steps[3].partialProduct)).toBe(0);
  });

  it("step partial products show shifted multiplicand when bit is 1", () => {
    // A = 5 (0101), B = 3 (0011)
    const result = shiftAndAddMultiplier(intToBits(5, 4), intToBits(3, 4));
    // Step 0: bit 0 = 1, partial = 5 << 0 = 5
    expect(bitsToInt(result.steps[0].partialProduct)).toBe(5);
    // Step 1: bit 1 = 1, partial = 5 << 1 = 10
    expect(bitsToInt(result.steps[1].partialProduct)).toBe(10);
  });

  it("running total accumulates correctly", () => {
    // A = 5, B = 3: 5×3 = 15
    const result = shiftAndAddMultiplier(intToBits(5, 4), intToBits(3, 4));
    // After step 0: running total = 5 (added 5 << 0)
    expect(bitsToInt(result.steps[0].runningTotal)).toBe(5);
    // After step 1: running total = 5 + 10 = 15 (added 5 << 1)
    expect(bitsToInt(result.steps[1].runningTotal)).toBe(15);
    // After step 2: running total stays 15 (bit 2 = 0, skip)
    expect(bitsToInt(result.steps[2].runningTotal)).toBe(15);
    // After step 3: running total stays 15 (bit 3 = 0, skip)
    expect(bitsToInt(result.steps[3].runningTotal)).toBe(15);
  });

  it("preserves original inputs in result", () => {
    const a = intToBits(5, 4);
    const b = intToBits(3, 4);
    const result = shiftAndAddMultiplier(a, b);
    expect(result.a).toEqual(a);
    expect(result.b).toEqual(b);
  });

  // ---------------------------------------------------------------------------
  // Commutativity: a × b = b × a
  // ---------------------------------------------------------------------------

  it("multiplication is commutative", () => {
    const r1 = shiftAndAddMultiplier(intToBits(7, 4), intToBits(3, 4));
    const r2 = shiftAndAddMultiplier(intToBits(3, 4), intToBits(7, 4));
    expect(bitsToInt(r1.product)).toBe(bitsToInt(r2.product));
    expect(bitsToInt(r1.product)).toBe(21);
  });

  // ---------------------------------------------------------------------------
  // Error handling
  // ---------------------------------------------------------------------------

  it("throws on mismatched lengths", () => {
    expect(() =>
      shiftAndAddMultiplier(intToBits(1, 4), intToBits(1, 8))
    ).toThrow("same length");
  });

  it("throws on empty arrays", () => {
    expect(() => shiftAndAddMultiplier([], [])).toThrow("must not be empty");
  });

  // ---------------------------------------------------------------------------
  // 1-bit multiplication (degenerate case)
  // ---------------------------------------------------------------------------

  it("1-bit: 0 × 0 = 0", () => {
    const result = shiftAndAddMultiplier([0 as Bit], [0 as Bit]);
    expect(bitsToInt(result.product)).toBe(0);
  });

  it("1-bit: 1 × 1 = 1", () => {
    const result = shiftAndAddMultiplier([1 as Bit], [1 as Bit]);
    expect(bitsToInt(result.product)).toBe(1);
  });
});
