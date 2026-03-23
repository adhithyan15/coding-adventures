import { describe, it, expect } from "vitest";
import { Bitset, BitsetError } from "../src/index.js";

// =====================================================================
// Bitset Test Suite
// =====================================================================
//
// This test suite covers every public method of the Bitset class with
// edge cases, boundary conditions, and property-based checks. Tests
// are organized by feature area, mirroring the implementation structure.

// ---------------------------------------------------------------------------
// Constructor: new Bitset(size)
// ---------------------------------------------------------------------------

describe("constructor", () => {
  it("creates an empty bitset with size 0", () => {
    const bs = new Bitset(0);
    expect(bs.size).toBe(0);
    expect(bs.capacity).toBe(0);
    expect(bs.popcount()).toBe(0);
  });

  it("creates a bitset with the given size and all zeros", () => {
    const bs = new Bitset(100);
    expect(bs.size).toBe(100);
    expect(bs.capacity).toBe(128); // ceil(100/32) * 32 = 4 words * 32
    expect(bs.popcount()).toBe(0);
  });

  it("rounds capacity up to next multiple of 32", () => {
    const bs = new Bitset(1);
    expect(bs.size).toBe(1);
    expect(bs.capacity).toBe(32);

    const bs2 = new Bitset(32);
    expect(bs2.capacity).toBe(32);

    const bs3 = new Bitset(33);
    expect(bs3.capacity).toBe(64);

    const bs4 = new Bitset(64);
    expect(bs4.capacity).toBe(64);

    const bs5 = new Bitset(65);
    expect(bs5.capacity).toBe(96);
  });

  it("handles exact word boundary sizes", () => {
    const bs = new Bitset(32);
    expect(bs.size).toBe(32);
    expect(bs.capacity).toBe(32);
  });
});

// ---------------------------------------------------------------------------
// Static factory: fromInteger
// ---------------------------------------------------------------------------

describe("fromInteger", () => {
  it("creates empty bitset from 0", () => {
    const bs = Bitset.fromInteger(0);
    expect(bs.size).toBe(0);
    expect(bs.popcount()).toBe(0);
  });

  it("creates bitset from 1 (single bit)", () => {
    const bs = Bitset.fromInteger(1);
    expect(bs.size).toBe(1);
    expect(bs.test(0)).toBe(true);
    expect(bs.popcount()).toBe(1);
  });

  it("creates bitset from 5 (binary 101)", () => {
    const bs = Bitset.fromInteger(5);
    expect(bs.size).toBe(3);
    expect(bs.test(0)).toBe(true);
    expect(bs.test(1)).toBe(false);
    expect(bs.test(2)).toBe(true);
    expect(bs.popcount()).toBe(2);
  });

  it("creates bitset from 255 (8 bits all set)", () => {
    const bs = Bitset.fromInteger(255);
    expect(bs.size).toBe(8);
    expect(bs.popcount()).toBe(8);
    for (let i = 0; i < 8; i++) {
      expect(bs.test(i)).toBe(true);
    }
  });

  it("handles values larger than 32 bits", () => {
    // 2^32 = 4294967296. This value has bit 32 set.
    const bs = Bitset.fromInteger(4294967296);
    expect(bs.size).toBe(33);
    expect(bs.test(32)).toBe(true);
    expect(bs.test(0)).toBe(false);
  });

  it("handles power of two", () => {
    const bs = Bitset.fromInteger(1024); // 2^10
    expect(bs.size).toBe(11);
    expect(bs.test(10)).toBe(true);
    expect(bs.popcount()).toBe(1);
  });

  it("throws on negative value", () => {
    expect(() => Bitset.fromInteger(-1)).toThrow(BitsetError);
  });

  it("throws on non-integer", () => {
    expect(() => Bitset.fromInteger(3.14)).toThrow(BitsetError);
  });

  it("roundtrips with toInteger for small values", () => {
    for (const val of [0, 1, 2, 7, 42, 255, 1023, 65535]) {
      const bs = Bitset.fromInteger(val);
      expect(bs.toInteger()).toBe(val);
    }
  });
});

// ---------------------------------------------------------------------------
// Static factory: fromBinaryStr
// ---------------------------------------------------------------------------

describe("fromBinaryStr", () => {
  it("creates empty bitset from empty string", () => {
    const bs = Bitset.fromBinaryStr("");
    expect(bs.size).toBe(0);
    expect(bs.popcount()).toBe(0);
  });

  it("creates bitset from '0'", () => {
    const bs = Bitset.fromBinaryStr("0");
    expect(bs.size).toBe(1);
    expect(bs.test(0)).toBe(false);
  });

  it("creates bitset from '1'", () => {
    const bs = Bitset.fromBinaryStr("1");
    expect(bs.size).toBe(1);
    expect(bs.test(0)).toBe(true);
  });

  it("creates bitset from '1010' (MSB on left)", () => {
    const bs = Bitset.fromBinaryStr("1010");
    expect(bs.size).toBe(4);
    expect(bs.test(0)).toBe(false); // rightmost char = '0'
    expect(bs.test(1)).toBe(true); // '1'
    expect(bs.test(2)).toBe(false); // '0'
    expect(bs.test(3)).toBe(true); // leftmost char = '1'
  });

  it("produces same result as fromInteger for equivalent values", () => {
    const fromStr = Bitset.fromBinaryStr("1010");
    const fromInt = Bitset.fromInteger(10); // 10 = 0b1010
    expect(fromStr.equals(fromInt)).toBe(true);
  });

  it("handles long strings spanning multiple words", () => {
    // 40-character string -> 40 bits -> 2 words
    const s = "1" + "0".repeat(39);
    const bs = Bitset.fromBinaryStr(s);
    expect(bs.size).toBe(40);
    expect(bs.test(39)).toBe(true); // the leftmost '1'
    expect(bs.popcount()).toBe(1);
  });

  it("throws on invalid characters", () => {
    expect(() => Bitset.fromBinaryStr("102")).toThrow(BitsetError);
    expect(() => Bitset.fromBinaryStr("abc")).toThrow(BitsetError);
    expect(() => Bitset.fromBinaryStr("0 1")).toThrow(BitsetError);
  });

  it("roundtrips with toBinaryStr", () => {
    for (const s of ["", "0", "1", "101", "1010", "11111111"]) {
      const bs = Bitset.fromBinaryStr(s);
      expect(bs.toBinaryStr()).toBe(s);
    }
  });
});

// ---------------------------------------------------------------------------
// Single-bit operations: set, clear, test, toggle
// ---------------------------------------------------------------------------

describe("set", () => {
  it("sets a bit within range", () => {
    const bs = new Bitset(10);
    bs.set(5);
    expect(bs.test(5)).toBe(true);
    expect(bs.popcount()).toBe(1);
  });

  it("is idempotent (setting already-set bit is no-op)", () => {
    const bs = new Bitset(10);
    bs.set(5);
    bs.set(5);
    expect(bs.popcount()).toBe(1);
  });

  it("auto-grows when setting beyond len", () => {
    const bs = new Bitset(10);
    bs.set(100);
    expect(bs.size).toBe(101);
    expect(bs.test(100)).toBe(true);
  });

  it("auto-grows from empty bitset", () => {
    const bs = new Bitset(0);
    bs.set(0);
    expect(bs.size).toBe(1);
    expect(bs.test(0)).toBe(true);
  });

  it("preserves existing bits when growing", () => {
    const bs = new Bitset(10);
    bs.set(3);
    bs.set(7);
    bs.set(100); // trigger growth
    expect(bs.test(3)).toBe(true);
    expect(bs.test(7)).toBe(true);
    expect(bs.test(100)).toBe(true);
    expect(bs.popcount()).toBe(3);
  });

  it("sets bits at word boundaries", () => {
    const bs = new Bitset(100);
    bs.set(0);
    bs.set(31);
    bs.set(32);
    bs.set(63);
    bs.set(64);
    expect(bs.test(0)).toBe(true);
    expect(bs.test(31)).toBe(true);
    expect(bs.test(32)).toBe(true);
    expect(bs.test(63)).toBe(true);
    expect(bs.test(64)).toBe(true);
  });
});

describe("clear", () => {
  it("clears a set bit", () => {
    const bs = new Bitset(10);
    bs.set(5);
    expect(bs.test(5)).toBe(true);
    bs.clear(5);
    expect(bs.test(5)).toBe(false);
  });

  it("is no-op for already-clear bit", () => {
    const bs = new Bitset(10);
    bs.clear(5);
    expect(bs.test(5)).toBe(false);
  });

  it("is no-op for index beyond len (does not grow)", () => {
    const bs = new Bitset(10);
    bs.clear(999);
    expect(bs.size).toBe(10);
  });

  it("preserves other bits when clearing", () => {
    const bs = new Bitset(10);
    bs.set(3);
    bs.set(5);
    bs.set(7);
    bs.clear(5);
    expect(bs.test(3)).toBe(true);
    expect(bs.test(5)).toBe(false);
    expect(bs.test(7)).toBe(true);
  });
});

describe("test", () => {
  it("returns false for unset bit", () => {
    const bs = new Bitset(10);
    expect(bs.test(5)).toBe(false);
  });

  it("returns true for set bit", () => {
    const bs = new Bitset(10);
    bs.set(5);
    expect(bs.test(5)).toBe(true);
  });

  it("returns false for index beyond len (does not grow)", () => {
    const bs = new Bitset(10);
    expect(bs.test(999)).toBe(false);
    expect(bs.size).toBe(10);
  });

  it("returns false for empty bitset", () => {
    const bs = new Bitset(0);
    expect(bs.test(0)).toBe(false);
  });
});

describe("toggle", () => {
  it("flips 0 to 1", () => {
    const bs = new Bitset(10);
    bs.toggle(5);
    expect(bs.test(5)).toBe(true);
  });

  it("flips 1 to 0", () => {
    const bs = new Bitset(10);
    bs.set(5);
    bs.toggle(5);
    expect(bs.test(5)).toBe(false);
  });

  it("double toggle is identity", () => {
    const bs = new Bitset(10);
    bs.set(5);
    bs.toggle(5);
    bs.toggle(5);
    expect(bs.test(5)).toBe(true);
  });

  it("auto-grows when toggling beyond len", () => {
    const bs = new Bitset(10);
    bs.toggle(100);
    expect(bs.size).toBe(101);
    expect(bs.test(100)).toBe(true);
  });

  it("auto-grows from empty bitset", () => {
    const bs = new Bitset(0);
    bs.toggle(0);
    expect(bs.size).toBe(1);
    expect(bs.test(0)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Bulk bitwise operations
// ---------------------------------------------------------------------------

describe("and", () => {
  it("AND of identical bitsets is itself", () => {
    const a = Bitset.fromInteger(0b1010);
    const b = Bitset.fromInteger(0b1010);
    const c = a.and(b);
    expect(c.toInteger()).toBe(0b1010);
  });

  it("AND of disjoint bitsets is zero", () => {
    const a = Bitset.fromInteger(0b1100);
    const b = Bitset.fromInteger(0b0011);
    const c = a.and(b);
    expect(c.toInteger()).toBe(0);
  });

  it("AND with zero is zero", () => {
    const a = Bitset.fromInteger(0b1111);
    const b = new Bitset(4);
    const c = a.and(b);
    expect(c.popcount()).toBe(0);
  });

  it("handles different lengths", () => {
    const a = Bitset.fromInteger(0b1100); // len=4
    const b = Bitset.fromInteger(0b1010); // len=4
    const c = a.and(b);
    expect(c.toInteger()).toBe(0b1000); // only bit 3
    expect(c.size).toBe(4);
  });

  it("result len is max of operand lens", () => {
    const a = new Bitset(100);
    const b = new Bitset(200);
    a.set(50);
    b.set(50);
    const c = a.and(b);
    expect(c.size).toBe(200);
    expect(c.test(50)).toBe(true);
  });

  it("does not modify operands", () => {
    const a = Bitset.fromInteger(0b1100);
    const b = Bitset.fromInteger(0b1010);
    a.and(b);
    expect(a.toInteger()).toBe(0b1100);
    expect(b.toInteger()).toBe(0b1010);
  });
});

describe("or", () => {
  it("OR of disjoint bitsets is union", () => {
    const a = Bitset.fromInteger(0b1100);
    const b = Bitset.fromInteger(0b0011);
    const c = a.or(b);
    expect(c.toInteger()).toBe(0b1111);
  });

  it("OR with zero is identity", () => {
    const a = Bitset.fromInteger(0b1010);
    const b = new Bitset(4);
    const c = a.or(b);
    expect(c.toInteger()).toBe(0b1010);
  });

  it("OR of identical bitsets is itself", () => {
    const a = Bitset.fromInteger(42);
    const c = a.or(a);
    expect(c.toInteger()).toBe(42);
  });

  it("handles different lengths", () => {
    const a = Bitset.fromInteger(0b1100); // len=4
    const b = Bitset.fromInteger(0b1010); // len=4
    const c = a.or(b);
    expect(c.toInteger()).toBe(0b1110);
  });

  it("does not modify operands", () => {
    const a = Bitset.fromInteger(0b1100);
    const b = Bitset.fromInteger(0b0011);
    a.or(b);
    expect(a.toInteger()).toBe(0b1100);
    expect(b.toInteger()).toBe(0b0011);
  });
});

describe("xor", () => {
  it("XOR of identical bitsets is zero", () => {
    const a = Bitset.fromInteger(0b1010);
    const c = a.xor(a);
    expect(c.popcount()).toBe(0);
  });

  it("XOR of disjoint bitsets is union", () => {
    const a = Bitset.fromInteger(0b1100);
    const b = Bitset.fromInteger(0b0011);
    const c = a.xor(b);
    expect(c.toInteger()).toBe(0b1111);
  });

  it("XOR finds symmetric difference", () => {
    const a = Bitset.fromInteger(0b1100);
    const b = Bitset.fromInteger(0b1010);
    const c = a.xor(b);
    expect(c.toInteger()).toBe(0b0110); // bits 1 and 2
  });

  it("does not modify operands", () => {
    const a = Bitset.fromInteger(0b1100);
    const b = Bitset.fromInteger(0b1010);
    a.xor(b);
    expect(a.toInteger()).toBe(0b1100);
    expect(b.toInteger()).toBe(0b1010);
  });
});

describe("not", () => {
  it("flips all bits within len", () => {
    const a = Bitset.fromInteger(0b1010); // len=4
    const b = a.not();
    expect(b.toInteger()).toBe(0b0101);
    expect(b.size).toBe(4);
  });

  it("double NOT is identity", () => {
    const a = Bitset.fromInteger(42);
    const b = a.not().not();
    expect(b.equals(a)).toBe(true);
  });

  it("NOT of all-zero bitset is all-one", () => {
    const a = new Bitset(8);
    const b = a.not();
    expect(b.popcount()).toBe(8);
    expect(b.toInteger()).toBe(0xff);
  });

  it("NOT of all-one bitset is all-zero", () => {
    const a = Bitset.fromBinaryStr("11111111");
    const b = a.not();
    expect(b.popcount()).toBe(0);
  });

  it("preserves len and cleans trailing bits", () => {
    // len=5, capacity=32. NOT should only flip bits 0-4.
    const a = Bitset.fromBinaryStr("10101"); // bits 0,2,4 set
    const b = a.not();
    expect(b.size).toBe(5);
    expect(b.test(0)).toBe(false);
    expect(b.test(1)).toBe(true);
    expect(b.test(2)).toBe(false);
    expect(b.test(3)).toBe(true);
    expect(b.test(4)).toBe(false);
    expect(b.popcount()).toBe(2);
  });

  it("does not modify operand", () => {
    const a = Bitset.fromInteger(0b1010);
    a.not();
    expect(a.toInteger()).toBe(0b1010);
  });

  it("NOT of empty bitset is empty", () => {
    const a = new Bitset(0);
    const b = a.not();
    expect(b.size).toBe(0);
    expect(b.popcount()).toBe(0);
  });
});

describe("andNot", () => {
  it("set difference: elements in A but not in B", () => {
    const a = Bitset.fromInteger(0b1110); // bits 1,2,3
    const b = Bitset.fromInteger(0b1010); // bits 1,3
    const c = a.andNot(b);
    expect(c.toInteger()).toBe(0b0100); // only bit 2
  });

  it("andNot with itself is zero", () => {
    const a = Bitset.fromInteger(0b1111);
    const c = a.andNot(a);
    expect(c.popcount()).toBe(0);
  });

  it("andNot with zero is identity", () => {
    const a = Bitset.fromInteger(0b1010);
    const b = new Bitset(4);
    const c = a.andNot(b);
    expect(c.toInteger()).toBe(0b1010);
  });

  it("does not modify operands", () => {
    const a = Bitset.fromInteger(0b1110);
    const b = Bitset.fromInteger(0b1010);
    a.andNot(b);
    expect(a.toInteger()).toBe(0b1110);
    expect(b.toInteger()).toBe(0b1010);
  });
});

// ---------------------------------------------------------------------------
// Counting and query operations
// ---------------------------------------------------------------------------

describe("popcount", () => {
  it("returns 0 for empty bitset", () => {
    expect(new Bitset(0).popcount()).toBe(0);
  });

  it("returns 0 for all-zero bitset", () => {
    expect(new Bitset(100).popcount()).toBe(0);
  });

  it("counts set bits correctly", () => {
    const bs = Bitset.fromInteger(0b10110); // bits 1,2,4
    expect(bs.popcount()).toBe(3);
  });

  it("counts across multiple words", () => {
    const bs = new Bitset(100);
    bs.set(0);
    bs.set(31);
    bs.set(32);
    bs.set(63);
    bs.set(64);
    bs.set(99);
    expect(bs.popcount()).toBe(6);
  });

  it("counts all bits set", () => {
    const bs = Bitset.fromBinaryStr("11111111");
    expect(bs.popcount()).toBe(8);
  });
});

describe("size (len)", () => {
  it("returns 0 for empty bitset", () => {
    expect(new Bitset(0).size).toBe(0);
  });

  it("returns constructor size", () => {
    expect(new Bitset(100).size).toBe(100);
  });

  it("updates after auto-growth", () => {
    const bs = new Bitset(10);
    bs.set(200);
    expect(bs.size).toBe(201);
  });
});

describe("capacity", () => {
  it("is always a multiple of 32", () => {
    for (const n of [0, 1, 10, 31, 32, 33, 64, 65, 100]) {
      const bs = new Bitset(n);
      expect(bs.capacity % 32).toBe(0);
    }
  });

  it("is always >= size", () => {
    for (const n of [0, 1, 10, 31, 32, 33, 64, 100]) {
      const bs = new Bitset(n);
      expect(bs.capacity).toBeGreaterThanOrEqual(bs.size);
    }
  });
});

describe("any", () => {
  it("returns false for empty bitset", () => {
    expect(new Bitset(0).any()).toBe(false);
  });

  it("returns false for all-zero bitset", () => {
    expect(new Bitset(100).any()).toBe(false);
  });

  it("returns true when at least one bit is set", () => {
    const bs = new Bitset(100);
    bs.set(50);
    expect(bs.any()).toBe(true);
  });

  it("returns false after clearing all set bits", () => {
    const bs = new Bitset(10);
    bs.set(5);
    bs.clear(5);
    expect(bs.any()).toBe(false);
  });
});

describe("all", () => {
  it("returns true for empty bitset (vacuous truth)", () => {
    expect(new Bitset(0).all()).toBe(true);
  });

  it("returns false for all-zero non-empty bitset", () => {
    expect(new Bitset(10).all()).toBe(false);
  });

  it("returns true when all bits are set", () => {
    const bs = Bitset.fromBinaryStr("1111");
    expect(bs.all()).toBe(true);
  });

  it("returns false when one bit is missing", () => {
    const bs = Bitset.fromBinaryStr("1110");
    expect(bs.all()).toBe(false);
  });

  it("works across word boundaries", () => {
    const bs = new Bitset(40);
    for (let i = 0; i < 40; i++) {
      bs.set(i);
    }
    expect(bs.all()).toBe(true);

    bs.clear(33);
    expect(bs.all()).toBe(false);
  });

  it("handles exact word boundary (32 bits)", () => {
    const bs = new Bitset(32);
    for (let i = 0; i < 32; i++) {
      bs.set(i);
    }
    expect(bs.all()).toBe(true);
  });
});

describe("none", () => {
  it("returns true for empty bitset", () => {
    expect(new Bitset(0).none()).toBe(true);
  });

  it("returns true for all-zero bitset", () => {
    expect(new Bitset(100).none()).toBe(true);
  });

  it("returns false when a bit is set", () => {
    const bs = new Bitset(100);
    bs.set(50);
    expect(bs.none()).toBe(false);
  });
});

describe("isEmpty", () => {
  it("returns true for size 0", () => {
    expect(new Bitset(0).isEmpty()).toBe(true);
  });

  it("returns false for non-zero size", () => {
    expect(new Bitset(1).isEmpty()).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Iteration
// ---------------------------------------------------------------------------

describe("iterSetBits", () => {
  it("yields nothing for empty bitset", () => {
    const bs = new Bitset(0);
    expect([...bs.iterSetBits()]).toEqual([]);
  });

  it("yields nothing for all-zero bitset", () => {
    const bs = new Bitset(100);
    expect([...bs.iterSetBits()]).toEqual([]);
  });

  it("yields set bits in ascending order", () => {
    const bs = Bitset.fromInteger(0b10100101);
    expect([...bs.iterSetBits()]).toEqual([0, 2, 5, 7]);
  });

  it("yields single set bit", () => {
    const bs = new Bitset(100);
    bs.set(42);
    expect([...bs.iterSetBits()]).toEqual([42]);
  });

  it("works across multiple words", () => {
    const bs = new Bitset(100);
    bs.set(0);
    bs.set(31);
    bs.set(32);
    bs.set(63);
    bs.set(64);
    bs.set(99);
    expect([...bs.iterSetBits()]).toEqual([0, 31, 32, 63, 64, 99]);
  });

  it("skips zero words efficiently", () => {
    const bs = new Bitset(200);
    bs.set(150); // only one bit set, far into the array
    const result = [...bs.iterSetBits()];
    expect(result).toEqual([150]);
  });

  it("handles all bits set in a word", () => {
    const bs = new Bitset(32);
    for (let i = 0; i < 32; i++) {
      bs.set(i);
    }
    const result = [...bs.iterSetBits()];
    expect(result).toEqual(Array.from({ length: 32 }, (_, i) => i));
  });
});

// ---------------------------------------------------------------------------
// Conversion operations
// ---------------------------------------------------------------------------

describe("toInteger", () => {
  it("returns 0 for empty bitset", () => {
    expect(new Bitset(0).toInteger()).toBe(0);
  });

  it("returns 0 for all-zero bitset", () => {
    expect(new Bitset(100).toInteger()).toBe(0);
  });

  it("converts small values correctly", () => {
    expect(Bitset.fromInteger(42).toInteger()).toBe(42);
    expect(Bitset.fromInteger(255).toInteger()).toBe(255);
    expect(Bitset.fromInteger(1).toInteger()).toBe(1);
  });

  it("handles value at word boundary", () => {
    // 2^32 - 1 = 4294967295 (max u32)
    const bs = Bitset.fromInteger(4294967295);
    expect(bs.toInteger()).toBe(4294967295);
  });

  it("handles values spanning two words (up to 2^53-1)", () => {
    const val = 4294967296; // 2^32
    const bs = Bitset.fromInteger(val);
    expect(bs.toInteger()).toBe(val);
  });

  it("throws for values exceeding Number.MAX_SAFE_INTEGER", () => {
    // Create a bitset with a bit set beyond position 52
    const bs = new Bitset(100);
    bs.set(53);
    expect(() => bs.toInteger()).toThrow(BitsetError);
  });
});

describe("toBinaryStr", () => {
  it("returns empty string for empty bitset", () => {
    expect(new Bitset(0).toBinaryStr()).toBe("");
  });

  it("converts correctly", () => {
    expect(Bitset.fromInteger(5).toBinaryStr()).toBe("101");
    expect(Bitset.fromInteger(10).toBinaryStr()).toBe("1010");
  });

  it("preserves leading zeros based on len", () => {
    const bs = Bitset.fromBinaryStr("0010");
    expect(bs.toBinaryStr()).toBe("0010");
    expect(bs.size).toBe(4);
  });

  it("all zeros", () => {
    const bs = new Bitset(4);
    expect(bs.toBinaryStr()).toBe("0000");
  });

  it("all ones", () => {
    const bs = Bitset.fromBinaryStr("1111");
    expect(bs.toBinaryStr()).toBe("1111");
  });
});

describe("toString", () => {
  it("formats as Bitset(...)", () => {
    expect(Bitset.fromInteger(5).toString()).toBe("Bitset(101)");
  });

  it("formats empty bitset", () => {
    expect(new Bitset(0).toString()).toBe("Bitset()");
  });
});

// ---------------------------------------------------------------------------
// Equality
// ---------------------------------------------------------------------------

describe("equals", () => {
  it("equal bitsets are equal", () => {
    const a = Bitset.fromInteger(42);
    const b = Bitset.fromInteger(42);
    expect(a.equals(b)).toBe(true);
  });

  it("different values are not equal", () => {
    const a = Bitset.fromInteger(42);
    const b = Bitset.fromInteger(43);
    expect(a.equals(b)).toBe(false);
  });

  it("different lengths are not equal even with same bits", () => {
    const a = Bitset.fromBinaryStr("101");
    const b = Bitset.fromBinaryStr("0101");
    expect(a.equals(b)).toBe(false);
  });

  it("empty bitsets are equal", () => {
    const a = new Bitset(0);
    const b = new Bitset(0);
    expect(a.equals(b)).toBe(true);
  });

  it("same value with different capacities are equal", () => {
    const a = Bitset.fromInteger(5);
    const b = Bitset.fromBinaryStr("101");
    expect(a.equals(b)).toBe(true);
  });

  it("reflexive: a.equals(a) is true", () => {
    const a = Bitset.fromInteger(42);
    expect(a.equals(a)).toBe(true);
  });

  it("symmetric: a.equals(b) === b.equals(a)", () => {
    const a = Bitset.fromInteger(42);
    const b = Bitset.fromInteger(42);
    expect(a.equals(b)).toBe(b.equals(a));
  });
});

// ---------------------------------------------------------------------------
// Auto-growth
// ---------------------------------------------------------------------------

describe("auto-growth", () => {
  it("doubles capacity on growth", () => {
    const bs = new Bitset(10);
    expect(bs.capacity).toBe(32); // 1 word

    bs.set(50);
    // 50 >= 32, so double: 32 -> 64. Now 50 < 64, stop.
    expect(bs.capacity).toBe(64);
    expect(bs.size).toBe(51);
  });

  it("grows multiple doublings", () => {
    const bs = new Bitset(10);
    expect(bs.capacity).toBe(32);

    bs.set(500);
    // 32 -> 64 -> 128 -> 256 -> 512. 500 < 512, stop.
    expect(bs.capacity).toBe(512);
    expect(bs.size).toBe(501);
  });

  it("growth from empty", () => {
    const bs = new Bitset(0);
    expect(bs.capacity).toBe(0);

    bs.set(5);
    // Start at max(0, 32) = 32. 5 < 32, stop.
    expect(bs.capacity).toBe(32);
    expect(bs.size).toBe(6);
  });

  it("preserves data during growth", () => {
    const bs = new Bitset(32);
    for (let i = 0; i < 32; i++) {
      bs.set(i);
    }
    expect(bs.popcount()).toBe(32);

    bs.set(100); // trigger growth
    expect(bs.popcount()).toBe(33);
    for (let i = 0; i < 32; i++) {
      expect(bs.test(i)).toBe(true);
    }
    expect(bs.test(100)).toBe(true);
  });

  it("set within capacity but beyond len updates len", () => {
    const bs = new Bitset(10);
    // capacity is 32, so bit 20 is within capacity but beyond len
    bs.set(20);
    expect(bs.size).toBe(21);
    expect(bs.capacity).toBe(32); // no growth needed
  });
});

// ---------------------------------------------------------------------------
// Clean-trailing-bits invariant
// ---------------------------------------------------------------------------

describe("clean-trailing-bits invariant", () => {
  it("NOT does not leak bits beyond len", () => {
    const a = new Bitset(5); // len=5, capacity=32
    a.set(0);
    a.set(2);
    a.set(4);
    const b = a.not();
    expect(b.popcount()).toBe(2); // bits 1 and 3
    expect(b.size).toBe(5);
    // Bit 5 and beyond must be 0
    expect(b.test(5)).toBe(false);
  });

  it("toggle maintains invariant", () => {
    const bs = new Bitset(5);
    bs.toggle(3);
    expect(bs.popcount()).toBe(1);
    bs.toggle(3);
    expect(bs.popcount()).toBe(0);
  });

  it("popcount is accurate after NOT on non-word-aligned len", () => {
    const bs = new Bitset(7); // 7 bits in a single 32-bit word
    bs.set(0);
    bs.set(1);
    bs.set(2);
    const flipped = bs.not();
    // Original: bits 0,1,2 set. NOT: bits 3,4,5,6 set.
    expect(flipped.popcount()).toBe(4);
  });
});

// ---------------------------------------------------------------------------
// Operations with different-length bitsets
// ---------------------------------------------------------------------------

describe("different-length operations", () => {
  it("OR with shorter bitset zero-extends correctly", () => {
    const a = new Bitset(100);
    a.set(50);
    a.set(90);
    const b = new Bitset(60);
    b.set(50);
    const c = a.or(b);
    expect(c.size).toBe(100);
    expect(c.test(50)).toBe(true);
    expect(c.test(90)).toBe(true);
    expect(c.popcount()).toBe(2);
  });

  it("AND with shorter bitset zeros high bits", () => {
    const a = new Bitset(100);
    a.set(50);
    a.set(90);
    const b = new Bitset(60);
    b.set(50);
    const c = a.and(b);
    expect(c.test(50)).toBe(true);
    expect(c.test(90)).toBe(false); // b has no word for bit 90
    expect(c.popcount()).toBe(1);
  });

  it("XOR with different lengths", () => {
    const a = Bitset.fromBinaryStr("11000");
    const b = Bitset.fromBinaryStr("10");
    const c = a.xor(b);
    expect(c.size).toBe(5);
    expect(c.toBinaryStr()).toBe("11010");
  });
});

// ---------------------------------------------------------------------------
// Edge cases and stress tests
// ---------------------------------------------------------------------------

describe("edge cases", () => {
  it("handles bitset with single bit (len=1)", () => {
    const bs = new Bitset(1);
    expect(bs.size).toBe(1);
    bs.set(0);
    expect(bs.test(0)).toBe(true);
    expect(bs.popcount()).toBe(1);
    expect(bs.all()).toBe(true);
    bs.clear(0);
    expect(bs.all()).toBe(false);
    expect(bs.none()).toBe(true);
  });

  it("handles large bitset (1000 bits)", () => {
    const bs = new Bitset(1000);
    // Set every 7th bit
    for (let i = 0; i < 1000; i += 7) {
      bs.set(i);
    }
    const setBits = [...bs.iterSetBits()];
    expect(setBits.length).toBe(Math.ceil(1000 / 7));
    expect(bs.popcount()).toBe(Math.ceil(1000 / 7));
  });

  it("fromBinaryStr and fromInteger agree", () => {
    // 42 in binary is "101010"
    const fromInt = Bitset.fromInteger(42);
    const fromStr = Bitset.fromBinaryStr("101010");
    expect(fromInt.equals(fromStr)).toBe(true);
  });

  it("all bitwise operations preserve the invariant for non-aligned len", () => {
    // len=13, not aligned to 32
    const a = new Bitset(13);
    a.set(0);
    a.set(5);
    a.set(12);
    const b = new Bitset(13);
    b.set(5);
    b.set(10);

    // AND
    const andResult = a.and(b);
    expect(andResult.popcount()).toBe(1); // bit 5
    expect(andResult.size).toBe(13);

    // OR
    const orResult = a.or(b);
    expect(orResult.popcount()).toBe(4); // bits 0,5,10,12
    expect(orResult.size).toBe(13);

    // XOR
    const xorResult = a.xor(b);
    expect(xorResult.popcount()).toBe(3); // bits 0,10,12
    expect(xorResult.size).toBe(13);

    // NOT
    const notResult = a.not();
    expect(notResult.popcount()).toBe(10);
    expect(notResult.size).toBe(13);

    // AND-NOT
    const andNotResult = a.andNot(b);
    expect(andNotResult.popcount()).toBe(2); // bits 0,12
  });

  it("fromInteger roundtrip for powers of 2", () => {
    for (let i = 0; i < 32; i++) {
      const val = 1 << i;
      const bs = Bitset.fromInteger(val >>> 0); // unsigned
      expect(bs.toInteger()).toBe(val >>> 0);
    }
  });

  it("multiple set/clear cycles maintain correctness", () => {
    const bs = new Bitset(100);
    for (let i = 0; i < 100; i++) {
      bs.set(i);
    }
    expect(bs.popcount()).toBe(100);
    expect(bs.all()).toBe(true);

    for (let i = 0; i < 100; i += 2) {
      bs.clear(i);
    }
    expect(bs.popcount()).toBe(50);

    const setBits = [...bs.iterSetBits()];
    expect(setBits).toEqual(
      Array.from({ length: 50 }, (_, i) => i * 2 + 1)
    );
  });
});

// ---------------------------------------------------------------------------
// BitsetError
// ---------------------------------------------------------------------------

describe("BitsetError", () => {
  it("is an instance of Error", () => {
    const err = new BitsetError("test");
    expect(err).toBeInstanceOf(Error);
    expect(err).toBeInstanceOf(BitsetError);
  });

  it("has correct name", () => {
    const err = new BitsetError("test");
    expect(err.name).toBe("BitsetError");
  });

  it("has correct message", () => {
    const err = new BitsetError("test message");
    expect(err.message).toBe("test message");
  });
});
