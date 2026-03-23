/**
 * Tests for adder circuits.
 *
 * These tests verify every level of the adder hierarchy:
 * - Half adder: exhaustive truth table (4 cases)
 * - Full adder: exhaustive truth table (8 cases)
 * - Ripple carry adder: various bit widths and edge cases
 */

import { describe, it, expect } from "vitest";
import { halfAdder, fullAdder, rippleCarryAdder } from "../src/index.js";
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
// Half Adder
// ---------------------------------------------------------------------------

describe("halfAdder", () => {
  it("0 + 0 = 0, carry 0", () => {
    expect(halfAdder(0, 0)).toEqual([0, 0]);
  });

  it("0 + 1 = 1, carry 0", () => {
    expect(halfAdder(0, 1)).toEqual([1, 0]);
  });

  it("1 + 0 = 1, carry 0", () => {
    expect(halfAdder(1, 0)).toEqual([1, 0]);
  });

  it("1 + 1 = 0, carry 1 (binary 10)", () => {
    expect(halfAdder(1, 1)).toEqual([0, 1]);
  });
});

// ---------------------------------------------------------------------------
// Full Adder
// ---------------------------------------------------------------------------

describe("fullAdder", () => {
  it("0 + 0 + 0 = 0, carry 0", () => {
    expect(fullAdder(0, 0, 0)).toEqual([0, 0]);
  });

  it("0 + 0 + 1 = 1, carry 0", () => {
    expect(fullAdder(0, 0, 1)).toEqual([1, 0]);
  });

  it("0 + 1 + 0 = 1, carry 0", () => {
    expect(fullAdder(0, 1, 0)).toEqual([1, 0]);
  });

  it("0 + 1 + 1 = 0, carry 1", () => {
    expect(fullAdder(0, 1, 1)).toEqual([0, 1]);
  });

  it("1 + 0 + 0 = 1, carry 0", () => {
    expect(fullAdder(1, 0, 0)).toEqual([1, 0]);
  });

  it("1 + 0 + 1 = 0, carry 1", () => {
    expect(fullAdder(1, 0, 1)).toEqual([0, 1]);
  });

  it("1 + 1 + 0 = 0, carry 1", () => {
    expect(fullAdder(1, 1, 0)).toEqual([0, 1]);
  });

  it("1 + 1 + 1 = 1, carry 1", () => {
    expect(fullAdder(1, 1, 1)).toEqual([1, 1]);
  });
});

// ---------------------------------------------------------------------------
// Ripple Carry Adder
// ---------------------------------------------------------------------------

describe("rippleCarryAdder", () => {
  it("0 + 0 = 0", () => {
    const a: Bit[] = [0, 0, 0, 0];
    const b: Bit[] = [0, 0, 0, 0];
    const [result, carry] = rippleCarryAdder(a, b);
    expect(bitsToInt(result)).toBe(0);
    expect(carry).toBe(0);
  });

  it("1 + 2 = 3 (the target program: x = 1 + 2)", () => {
    const a = intToBits(1, 4); // [1, 0, 0, 0]
    const b = intToBits(2, 4); // [0, 1, 0, 0]
    const [result, carry] = rippleCarryAdder(a, b);
    expect(bitsToInt(result)).toBe(3);
    expect(carry).toBe(0);
  });

  it("5 + 3 = 8", () => {
    const a = intToBits(5, 4);
    const b = intToBits(3, 4);
    const [result, carry] = rippleCarryAdder(a, b);
    expect(bitsToInt(result)).toBe(8);
    expect(carry).toBe(0);
  });

  it("15 + 1 overflows in 4 bits", () => {
    /**
     * 4-bit overflow: 15 + 1 = 16, which doesn't fit in 4 bits.
     * The result wraps around to 0 with carry = 1.
     */
    const a = intToBits(15, 4); // [1, 1, 1, 1]
    const b = intToBits(1, 4); // [1, 0, 0, 0]
    const [result, carry] = rippleCarryAdder(a, b);
    expect(bitsToInt(result)).toBe(0); // wraps around
    expect(carry).toBe(1);
  });

  it("1 + 1 + carry_in = 3", () => {
    const a = intToBits(1, 4);
    const b = intToBits(1, 4);
    const [result, carry] = rippleCarryAdder(a, b, 1);
    expect(bitsToInt(result)).toBe(3); // 1 + 1 + carry = 3
    expect(carry).toBe(0);
  });

  it("8-bit addition: 100 + 155 = 255", () => {
    const a = intToBits(100, 8);
    const b = intToBits(155, 8);
    const [result, carry] = rippleCarryAdder(a, b);
    expect(bitsToInt(result)).toBe(255);
    expect(carry).toBe(0);
  });

  it("throws on mismatched lengths", () => {
    expect(() => rippleCarryAdder([0, 1] as Bit[], [0, 1, 0] as Bit[])).toThrow(
      "same length"
    );
  });

  it("throws on empty bit lists", () => {
    expect(() => rippleCarryAdder([], [])).toThrow("must not be empty");
  });
});
