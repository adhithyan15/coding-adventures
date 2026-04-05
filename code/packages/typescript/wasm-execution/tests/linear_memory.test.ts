/**
 * linear_memory.test.ts --- Tests for WASM linear memory.
 */

import { describe, it, expect } from "vitest";
import { LinearMemory } from "../src/linear_memory.js";
import { TrapError } from "../src/host_interface.js";

// ===========================================================================
// Construction
// ===========================================================================

describe("LinearMemory construction", () => {
  it("should create memory with the given number of pages", () => {
    const mem = new LinearMemory(1);
    expect(mem.size()).toBe(1);
    expect(mem.byteLength()).toBe(65536);
  });

  it("should create memory with zero pages", () => {
    const mem = new LinearMemory(0);
    expect(mem.size()).toBe(0);
    expect(mem.byteLength()).toBe(0);
  });

  it("should create memory with multiple pages", () => {
    const mem = new LinearMemory(3);
    expect(mem.size()).toBe(3);
    expect(mem.byteLength()).toBe(3 * 65536);
  });

  it("should have PAGE_SIZE constant of 65536", () => {
    expect(LinearMemory.PAGE_SIZE).toBe(65536);
  });
});

// ===========================================================================
// i32 load/store
// ===========================================================================

describe("i32 load/store", () => {
  it("should store and load an i32 value", () => {
    const mem = new LinearMemory(1);
    mem.storeI32(0, 42);
    expect(mem.loadI32(0)).toBe(42);
  });

  it("should store negative i32 values", () => {
    const mem = new LinearMemory(1);
    mem.storeI32(0, -1);
    expect(mem.loadI32(0)).toBe(-1);
  });

  it("should use little-endian byte order", () => {
    const mem = new LinearMemory(1);
    mem.storeI32(0, 0x01020304);
    // In little-endian: bytes are [04, 03, 02, 01]
    expect(mem.loadI32_8u(0)).toBe(0x04);
    expect(mem.loadI32_8u(1)).toBe(0x03);
    expect(mem.loadI32_8u(2)).toBe(0x02);
    expect(mem.loadI32_8u(3)).toBe(0x01);
  });

  it("should store at non-zero offsets", () => {
    const mem = new LinearMemory(1);
    mem.storeI32(100, 999);
    expect(mem.loadI32(100)).toBe(999);
    expect(mem.loadI32(0)).toBe(0); // other locations untouched
  });

  it("should trap on out-of-bounds load", () => {
    const mem = new LinearMemory(1);
    // Last valid 4-byte read is at offset 65532
    expect(() => mem.loadI32(65533)).toThrow(TrapError);
    expect(() => mem.loadI32(65536)).toThrow(TrapError);
    expect(() => mem.loadI32(-1)).toThrow(TrapError);
  });

  it("should trap on out-of-bounds store", () => {
    const mem = new LinearMemory(1);
    expect(() => mem.storeI32(65533, 42)).toThrow(TrapError);
    expect(() => mem.storeI32(-1, 42)).toThrow(TrapError);
  });
});

// ===========================================================================
// i64 load/store
// ===========================================================================

describe("i64 load/store", () => {
  it("should store and load an i64 value", () => {
    const mem = new LinearMemory(1);
    mem.storeI64(0, 9223372036854775807n); // i64 max
    expect(mem.loadI64(0)).toBe(9223372036854775807n);
  });

  it("should handle negative i64 values", () => {
    const mem = new LinearMemory(1);
    mem.storeI64(0, -1n);
    expect(mem.loadI64(0)).toBe(-1n);
  });

  it("should trap on out-of-bounds", () => {
    const mem = new LinearMemory(1);
    expect(() => mem.loadI64(65529)).toThrow(TrapError);
    expect(() => mem.storeI64(65529, 0n)).toThrow(TrapError);
  });
});

// ===========================================================================
// f32 load/store
// ===========================================================================

describe("f32 load/store", () => {
  it("should store and load an f32 value", () => {
    const mem = new LinearMemory(1);
    mem.storeF32(0, 3.14);
    expect(mem.loadF32(0)).toBeCloseTo(3.14, 2);
  });

  it("should handle special float values", () => {
    const mem = new LinearMemory(1);
    mem.storeF32(0, Infinity);
    expect(mem.loadF32(0)).toBe(Infinity);
    mem.storeF32(0, -Infinity);
    expect(mem.loadF32(0)).toBe(-Infinity);
    mem.storeF32(0, NaN);
    expect(mem.loadF32(0)).toBeNaN();
  });

  it("should trap on out-of-bounds", () => {
    const mem = new LinearMemory(1);
    expect(() => mem.loadF32(65533)).toThrow(TrapError);
  });
});

// ===========================================================================
// f64 load/store
// ===========================================================================

describe("f64 load/store", () => {
  it("should store and load an f64 value", () => {
    const mem = new LinearMemory(1);
    mem.storeF64(0, 3.141592653589793);
    expect(mem.loadF64(0)).toBe(3.141592653589793);
  });

  it("should trap on out-of-bounds", () => {
    const mem = new LinearMemory(1);
    expect(() => mem.loadF64(65529)).toThrow(TrapError);
    expect(() => mem.storeF64(65529, 0)).toThrow(TrapError);
  });
});

// ===========================================================================
// Narrow loads: i32 from 8-bit and 16-bit
// ===========================================================================

describe("narrow i32 loads", () => {
  it("loadI32_8s should sign-extend from 8 bits", () => {
    const mem = new LinearMemory(1);
    mem.storeI32_8(0, 0xff); // -1 as i8
    expect(mem.loadI32_8s(0)).toBe(-1);
    mem.storeI32_8(0, 127);
    expect(mem.loadI32_8s(0)).toBe(127);
  });

  it("loadI32_8u should zero-extend from 8 bits", () => {
    const mem = new LinearMemory(1);
    mem.storeI32_8(0, 0xff);
    expect(mem.loadI32_8u(0)).toBe(255);
  });

  it("loadI32_16s should sign-extend from 16 bits", () => {
    const mem = new LinearMemory(1);
    mem.storeI32_16(0, 0xffff); // -1 as i16
    expect(mem.loadI32_16s(0)).toBe(-1);
    mem.storeI32_16(0, 32767);
    expect(mem.loadI32_16s(0)).toBe(32767);
  });

  it("loadI32_16u should zero-extend from 16 bits", () => {
    const mem = new LinearMemory(1);
    mem.storeI32_16(0, 0xffff);
    expect(mem.loadI32_16u(0)).toBe(65535);
  });
});

// ===========================================================================
// Narrow loads: i64 from 8-bit, 16-bit, 32-bit
// ===========================================================================

describe("narrow i64 loads", () => {
  it("loadI64_8s should sign-extend to bigint", () => {
    const mem = new LinearMemory(1);
    mem.storeI32_8(0, 0xff);
    expect(mem.loadI64_8s(0)).toBe(-1n);
  });

  it("loadI64_8u should zero-extend to bigint", () => {
    const mem = new LinearMemory(1);
    mem.storeI32_8(0, 0xff);
    expect(mem.loadI64_8u(0)).toBe(255n);
  });

  it("loadI64_16s should sign-extend to bigint", () => {
    const mem = new LinearMemory(1);
    mem.storeI32_16(0, -1);
    expect(mem.loadI64_16s(0)).toBe(-1n);
  });

  it("loadI64_16u should zero-extend to bigint", () => {
    const mem = new LinearMemory(1);
    mem.storeI32_16(0, 0xffff);
    expect(mem.loadI64_16u(0)).toBe(65535n);
  });

  it("loadI64_32s should sign-extend to bigint", () => {
    const mem = new LinearMemory(1);
    mem.storeI32(0, -1);
    expect(mem.loadI64_32s(0)).toBe(-1n);
  });

  it("loadI64_32u should zero-extend to bigint", () => {
    const mem = new LinearMemory(1);
    mem.storeI32(0, -1); // 0xFFFFFFFF
    expect(mem.loadI64_32u(0)).toBe(4294967295n);
  });
});

// ===========================================================================
// Narrow stores from i64
// ===========================================================================

describe("narrow i64 stores", () => {
  it("storeI64_8 should store the low byte", () => {
    const mem = new LinearMemory(1);
    mem.storeI64_8(0, 0x1234n);
    expect(mem.loadI32_8u(0)).toBe(0x34);
  });

  it("storeI64_16 should store the low 2 bytes", () => {
    const mem = new LinearMemory(1);
    mem.storeI64_16(0, 0x12345678n);
    expect(mem.loadI32_16u(0)).toBe(0x5678);
  });

  it("storeI64_32 should store the low 4 bytes", () => {
    const mem = new LinearMemory(1);
    mem.storeI64_32(0, 0x123456789ABCDEF0n);
    expect(mem.loadI64_32u(0)).toBe(0x9ABCDEF0n);
  });
});

// ===========================================================================
// Memory growth
// ===========================================================================

describe("grow", () => {
  it("should return old page count on success", () => {
    const mem = new LinearMemory(1);
    const oldSize = mem.grow(2);
    expect(oldSize).toBe(1);
    expect(mem.size()).toBe(3);
    expect(mem.byteLength()).toBe(3 * 65536);
  });

  it("should preserve existing data after growth", () => {
    const mem = new LinearMemory(1);
    mem.storeI32(0, 42);
    mem.grow(1);
    expect(mem.loadI32(0)).toBe(42);
  });

  it("should zero-initialize new pages", () => {
    const mem = new LinearMemory(1);
    mem.grow(1);
    // New page starts at offset 65536
    expect(mem.loadI32(65536)).toBe(0);
  });

  it("should return -1 when exceeding max pages", () => {
    const mem = new LinearMemory(1, 2);
    expect(mem.grow(1)).toBe(1); // OK: 1 + 1 = 2 <= max of 2
    expect(mem.grow(1)).toBe(-1); // Fail: 2 + 1 = 3 > max of 2
    expect(mem.size()).toBe(2); // unchanged
  });

  it("should allow growth to exact max", () => {
    const mem = new LinearMemory(1, 3);
    expect(mem.grow(2)).toBe(1);
    expect(mem.size()).toBe(3);
  });

  it("should handle grow(0) as a no-op returning current size", () => {
    const mem = new LinearMemory(2);
    expect(mem.grow(0)).toBe(2);
    expect(mem.size()).toBe(2);
  });
});

// ===========================================================================
// writeBytes
// ===========================================================================

describe("writeBytes", () => {
  it("should write raw bytes to memory", () => {
    const mem = new LinearMemory(1);
    const data = new Uint8Array([0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello"
    mem.writeBytes(100, data);
    expect(mem.loadI32_8u(100)).toBe(0x48); // 'H'
    expect(mem.loadI32_8u(104)).toBe(0x6f); // 'o'
  });

  it("should trap if data extends beyond memory", () => {
    const mem = new LinearMemory(1);
    const data = new Uint8Array(10);
    expect(() => mem.writeBytes(65530, data)).toThrow(TrapError);
  });

  it("should handle empty data", () => {
    const mem = new LinearMemory(1);
    // Should not throw
    mem.writeBytes(0, new Uint8Array(0));
  });
});

// ===========================================================================
// Zero-page memory
// ===========================================================================

describe("zero-page memory", () => {
  it("should trap on any access to zero-page memory", () => {
    const mem = new LinearMemory(0);
    expect(() => mem.loadI32(0)).toThrow(TrapError);
    expect(() => mem.storeI32(0, 42)).toThrow(TrapError);
  });
});
