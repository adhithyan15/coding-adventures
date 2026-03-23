// bitset.test.ts -- Comprehensive tests for the native Bitset addon
// ==================================================================
//
// These tests verify that the Rust Bitset implementation is correctly
// exposed to JavaScript via the N-API node-bridge. Every public method
// is tested, including edge cases like empty bitsets, auto-growth,
// and binary operations between bitsets of different sizes.

import { describe, it, expect } from "vitest";
import { Bitset } from "../index.js";

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

describe("Bitset construction", () => {
  it("creates a zero-filled bitset with new Bitset(size)", () => {
    const bs = new Bitset(100);
    expect(bs.len()).toBe(100);
    expect(bs.popcount()).toBe(0);
    expect(bs.none()).toBe(true);
  });

  it("creates an empty bitset with new Bitset(0)", () => {
    const bs = new Bitset(0);
    expect(bs.len()).toBe(0);
    expect(bs.capacity()).toBe(0);
    expect(bs.isEmpty()).toBe(true);
  });

  it("creates a bitset from an integer", () => {
    // 42 in binary is 101010
    const bs = new Bitset(42, "integer");
    expect(bs.test(1)).toBe(true);
    expect(bs.test(3)).toBe(true);
    expect(bs.test(5)).toBe(true);
    expect(bs.test(0)).toBe(false);
    expect(bs.test(2)).toBe(false);
    expect(bs.test(4)).toBe(false);
    expect(bs.popcount()).toBe(3);
  });

  it("creates a bitset from integer 0", () => {
    const bs = new Bitset(0, "integer");
    expect(bs.len()).toBe(0);
    expect(bs.isEmpty()).toBe(true);
  });

  it("creates a bitset from a binary string", () => {
    const bs = new Bitset("1010", "binary");
    expect(bs.len()).toBe(4);
    expect(bs.test(1)).toBe(true);  // second char from right = '1'
    expect(bs.test(3)).toBe(true);  // leftmost char = '1'
    expect(bs.test(0)).toBe(false); // rightmost char = '0'
    expect(bs.test(2)).toBe(false);
  });

  it("creates a bitset from an empty binary string", () => {
    const bs = new Bitset("", "binary");
    expect(bs.len()).toBe(0);
    expect(bs.isEmpty()).toBe(true);
  });

  it("throws on invalid binary string", () => {
    expect(() => new Bitset("10201", "binary")).toThrow("invalid binary string");
  });

  it("throws on unknown mode", () => {
    // @ts-expect-error -- testing runtime error for invalid mode
    expect(() => new Bitset(5, "hex")).toThrow("unknown mode");
  });

  it("capacity is a multiple of 64", () => {
    const bs = new Bitset(100);
    expect(bs.capacity()).toBe(128); // ceil(100/64) * 64 = 128
  });

  it("capacity matches exactly for multiples of 64", () => {
    const bs = new Bitset(64);
    expect(bs.capacity()).toBe(64);
  });
});

// ---------------------------------------------------------------------------
// Single-bit operations
// ---------------------------------------------------------------------------

describe("single-bit operations", () => {
  it("set and test a bit", () => {
    const bs = new Bitset(10);
    bs.set(5);
    expect(bs.test(5)).toBe(true);
    expect(bs.test(4)).toBe(false);
  });

  it("set is idempotent", () => {
    const bs = new Bitset(10);
    bs.set(3);
    bs.set(3);
    expect(bs.popcount()).toBe(1);
  });

  it("clear removes a set bit", () => {
    const bs = new Bitset(10);
    bs.set(5);
    expect(bs.test(5)).toBe(true);
    bs.clear(5);
    expect(bs.test(5)).toBe(false);
  });

  it("clear is a no-op for unset bits", () => {
    const bs = new Bitset(10);
    bs.clear(5); // no-op, already 0
    expect(bs.test(5)).toBe(false);
  });

  it("clear is a no-op for out-of-range indices", () => {
    const bs = new Bitset(10);
    bs.clear(100); // out of range, no-op
    expect(bs.len()).toBe(10); // should not grow
  });

  it("test returns false for out-of-range indices", () => {
    const bs = new Bitset(10);
    expect(bs.test(100)).toBe(false);
  });

  it("toggle flips a bit from 0 to 1", () => {
    const bs = new Bitset(10);
    bs.toggle(5);
    expect(bs.test(5)).toBe(true);
  });

  it("toggle flips a bit from 1 to 0", () => {
    const bs = new Bitset(10);
    bs.set(5);
    bs.toggle(5);
    expect(bs.test(5)).toBe(false);
  });

  it("set auto-grows the bitset", () => {
    const bs = new Bitset(10);
    expect(bs.len()).toBe(10);
    bs.set(100);
    expect(bs.len()).toBe(101);
    expect(bs.test(100)).toBe(true);
  });

  it("toggle auto-grows the bitset", () => {
    const bs = new Bitset(10);
    bs.toggle(200);
    expect(bs.len()).toBe(201);
    expect(bs.test(200)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Bulk bitwise operations
// ---------------------------------------------------------------------------

describe("bulk bitwise operations", () => {
  it("AND computes intersection", () => {
    const a = new Bitset(8, "integer"); // 00001000
    a.set(0); a.set(1); a.set(2); a.set(3); // 00001111
    // Actually, let's use from_integer for clarity:
    const x = new Bitset(0b1100, "integer"); // bits 2,3
    const y = new Bitset(0b1010, "integer"); // bits 1,3
    const result = x.and(y);
    expect(result.toBinaryStr()).toBe("1000"); // only bit 3
    expect(result.popcount()).toBe(1);
  });

  it("OR computes union", () => {
    const x = new Bitset(0b1100, "integer"); // bits 2,3
    const y = new Bitset(0b1010, "integer"); // bits 1,3
    const result = x.or(y);
    expect(result.toBinaryStr()).toBe("1110"); // bits 1,2,3
    expect(result.popcount()).toBe(3);
  });

  it("XOR computes symmetric difference", () => {
    const x = new Bitset(0b1100, "integer"); // bits 2,3
    const y = new Bitset(0b1010, "integer"); // bits 1,3
    const result = x.xor(y);
    expect(result.toBinaryStr()).toBe("0110"); // bits 1,2 (len=4 preserved)
    expect(result.popcount()).toBe(2);
  });

  it("NOT flips all bits within len", () => {
    const bs = new Bitset(4);
    bs.set(0);
    bs.set(2);
    // bits: 0101 -> NOT -> 1010
    const result = bs.not();
    expect(result.test(0)).toBe(false);
    expect(result.test(1)).toBe(true);
    expect(result.test(2)).toBe(false);
    expect(result.test(3)).toBe(true);
    expect(result.len()).toBe(4);
  });

  it("andNot computes difference", () => {
    const x = new Bitset(0b1110, "integer"); // bits 1,2,3
    const y = new Bitset(0b1010, "integer"); // bits 1,3
    const result = x.andNot(y);
    // x AND (NOT y) = 1110 AND 0101 = 0100
    expect(result.test(2)).toBe(true);
    expect(result.popcount()).toBe(1);
  });

  it("binary operations with different-sized bitsets", () => {
    const small = new Bitset(0b11, "integer"); // 2 bits: 1,1
    const large = new Bitset(100);
    large.set(0);
    large.set(1);
    large.set(50);

    const result = small.and(large);
    expect(result.test(0)).toBe(true);
    expect(result.test(1)).toBe(true);
    expect(result.test(50)).toBe(false); // small doesn't have bit 50
  });

  it("NOT of empty bitset is empty", () => {
    const bs = new Bitset(0);
    const result = bs.not();
    expect(result.len()).toBe(0);
    expect(result.popcount()).toBe(0);
  });

  it("AND returns a new instance (does not mutate)", () => {
    const a = new Bitset(0b1111, "integer");
    const b = new Bitset(0b1010, "integer");
    const result = a.and(b);
    // Original should be unchanged.
    expect(a.popcount()).toBe(4);
    expect(b.popcount()).toBe(2);
    expect(result.popcount()).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// Query operations
// ---------------------------------------------------------------------------

describe("query operations", () => {
  it("popcount counts set bits", () => {
    const bs = new Bitset(0b10110, "integer");
    expect(bs.popcount()).toBe(3);
  });

  it("popcount of empty bitset is 0", () => {
    const bs = new Bitset(0);
    expect(bs.popcount()).toBe(0);
  });

  it("len returns logical length", () => {
    const bs = new Bitset(42);
    expect(bs.len()).toBe(42);
  });

  it("any returns true when bits are set", () => {
    const bs = new Bitset(100);
    expect(bs.any()).toBe(false);
    bs.set(50);
    expect(bs.any()).toBe(true);
  });

  it("all returns true when all bits are set", () => {
    const bs = new Bitset(4);
    expect(bs.all()).toBe(false);
    bs.set(0);
    bs.set(1);
    bs.set(2);
    bs.set(3);
    expect(bs.all()).toBe(true);
  });

  it("all returns true for empty bitset (vacuous truth)", () => {
    const bs = new Bitset(0);
    expect(bs.all()).toBe(true);
  });

  it("none returns true when no bits are set", () => {
    const bs = new Bitset(100);
    expect(bs.none()).toBe(true);
    bs.set(0);
    expect(bs.none()).toBe(false);
  });

  it("isEmpty returns true for len == 0", () => {
    const bs = new Bitset(0);
    expect(bs.isEmpty()).toBe(true);
    const bs2 = new Bitset(10);
    expect(bs2.isEmpty()).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Iteration and conversion
// ---------------------------------------------------------------------------

describe("iteration and conversion", () => {
  it("iterSetBits returns indices in ascending order", () => {
    const bs = new Bitset(0b10100101, "integer");
    const bits = bs.iterSetBits();
    expect(bits).toEqual([0, 2, 5, 7]);
  });

  it("iterSetBits returns empty array for zero bitset", () => {
    const bs = new Bitset(100);
    expect(bs.iterSetBits()).toEqual([]);
  });

  it("iterSetBits works across word boundaries", () => {
    const bs = new Bitset(200);
    bs.set(0);
    bs.set(63);
    bs.set(64);
    bs.set(127);
    bs.set(128);
    bs.set(199);
    const bits = bs.iterSetBits();
    expect(bits).toEqual([0, 63, 64, 127, 128, 199]);
  });

  it("toInteger returns the integer value", () => {
    const bs = new Bitset(42, "integer");
    expect(bs.toInteger()).toBe(42);
  });

  it("toInteger returns 0 for empty bitset", () => {
    const bs = new Bitset(0);
    expect(bs.toInteger()).toBe(0);
  });

  it("toInteger returns null for large bitsets", () => {
    const bs = new Bitset(200);
    bs.set(0);
    bs.set(100); // bit beyond 63 -- won't fit in a u64
    expect(bs.toInteger()).toBeNull();
  });

  it("toBinaryStr returns binary representation", () => {
    const bs = new Bitset(5, "integer"); // 101
    expect(bs.toBinaryStr()).toBe("101");
  });

  it("toBinaryStr returns empty string for empty bitset", () => {
    const bs = new Bitset(0);
    expect(bs.toBinaryStr()).toBe("");
  });

  it("toBinaryStr preserves leading zeros for fixed-size bitsets", () => {
    // new Bitset(8) then set bit 0 -> "00000001"
    const bs = new Bitset(8);
    bs.set(0);
    expect(bs.toBinaryStr()).toBe("00000001");
  });

  it("round-trips through toBinaryStr / fromBinaryStr", () => {
    const original = new Bitset(100);
    original.set(0);
    original.set(42);
    original.set(99);
    const str = original.toBinaryStr();
    const restored = new Bitset(str, "binary");
    expect(restored.len()).toBe(original.len());
    expect(restored.iterSetBits()).toEqual(original.iterSetBits());
  });

  it("round-trips through toInteger / fromInteger", () => {
    const original = new Bitset(42, "integer");
    const val = original.toInteger();
    expect(val).not.toBeNull();
    const restored = new Bitset(val!, "integer");
    expect(restored.toBinaryStr()).toBe(original.toBinaryStr());
  });
});

// ---------------------------------------------------------------------------
// Edge cases and stress tests
// ---------------------------------------------------------------------------

describe("edge cases", () => {
  it("handles bit 0 correctly", () => {
    const bs = new Bitset(1);
    bs.set(0);
    expect(bs.test(0)).toBe(true);
    expect(bs.popcount()).toBe(1);
  });

  it("handles bit 63 (last bit of first word)", () => {
    const bs = new Bitset(64);
    bs.set(63);
    expect(bs.test(63)).toBe(true);
    expect(bs.popcount()).toBe(1);
  });

  it("handles bit 64 (first bit of second word)", () => {
    const bs = new Bitset(65);
    bs.set(64);
    expect(bs.test(64)).toBe(true);
    expect(bs.popcount()).toBe(1);
  });

  it("handles large bitsets", () => {
    const bs = new Bitset(10000);
    for (let i = 0; i < 10000; i += 100) {
      bs.set(i);
    }
    expect(bs.popcount()).toBe(100);
    const bits = bs.iterSetBits();
    expect(bits.length).toBe(100);
    expect(bits[0]).toBe(0);
    expect(bits[99]).toBe(9900);
  });

  it("chained binary operations", () => {
    const a = new Bitset(0b1111, "integer");
    const b = new Bitset(0b1010, "integer");
    const c = new Bitset(0b1100, "integer");

    // (a AND b) OR c = 1010 OR 1100 = 1110
    const result = a.and(b).or(c);
    expect(result.toBinaryStr()).toBe("1110");
  });

  it("NOT is self-inverse", () => {
    const bs = new Bitset(100);
    bs.set(0);
    bs.set(42);
    bs.set(99);
    const double_not = bs.not().not();
    expect(double_not.iterSetBits()).toEqual(bs.iterSetBits());
    expect(double_not.len()).toBe(bs.len());
  });

  it("XOR with self produces all zeros", () => {
    const bs = new Bitset(0b11011011, "integer");
    const result = bs.xor(bs);
    expect(result.popcount()).toBe(0);
    expect(result.none()).toBe(true);
  });

  it("AND with self is identity", () => {
    const bs = new Bitset(0b11011011, "integer");
    const result = bs.and(bs);
    expect(result.toBinaryStr()).toBe(bs.toBinaryStr());
  });

  it("OR with self is identity", () => {
    const bs = new Bitset(0b11011011, "integer");
    const result = bs.or(bs);
    expect(result.toBinaryStr()).toBe(bs.toBinaryStr());
  });

  it("fromInteger with power of 2", () => {
    const bs = new Bitset(256, "integer"); // 2^8
    expect(bs.test(8)).toBe(true);
    expect(bs.popcount()).toBe(1);
    expect(bs.len()).toBe(9);
  });

  it("fromBinaryStr with all ones", () => {
    const bs = new Bitset("11111111", "binary");
    expect(bs.all()).toBe(true);
    expect(bs.popcount()).toBe(8);
  });

  it("fromBinaryStr with single bit", () => {
    const bs = new Bitset("1", "binary");
    expect(bs.len()).toBe(1);
    expect(bs.test(0)).toBe(true);
  });
});
