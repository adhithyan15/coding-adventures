/**
 * values.test.ts --- Tests for WASM typed value constructors and extractors.
 */

import { describe, it, expect } from "vitest";
import { ValueType } from "@coding-adventures/wasm-types";
import {
  i32,
  i64,
  f32,
  f64,
  defaultValue,
  asI32,
  asI64,
  asF32,
  asF64,
} from "../src/values.js";
import { TrapError } from "../src/host_interface.js";

// ===========================================================================
// i32 constructor
// ===========================================================================

describe("i32", () => {
  it("should create an i32 value with correct type tag", () => {
    const v = i32(42);
    expect(v.type).toBe(ValueType.I32);
    expect(v.value).toBe(42);
  });

  it("should wrap with | 0 (truncate to signed 32-bit)", () => {
    // 0xFFFFFFFF in unsigned is 4294967295, but as signed i32 it's -1
    expect(i32(0xffffffff).value).toBe(-1);
  });

  it("should truncate values beyond 32-bit range", () => {
    // 2^32 should wrap to 0
    expect(i32(0x100000000).value).toBe(0);
  });

  it("should truncate fractional parts", () => {
    expect(i32(3.7).value).toBe(3);
    expect(i32(-3.7).value).toBe(-3);
  });

  it("should convert NaN to 0", () => {
    expect(i32(NaN).value).toBe(0);
  });

  it("should handle negative values", () => {
    expect(i32(-1).value).toBe(-1);
    expect(i32(-2147483648).value).toBe(-2147483648); // i32 min
  });

  it("should handle zero", () => {
    expect(i32(0).value).toBe(0);
  });
});

// ===========================================================================
// i64 constructor
// ===========================================================================

describe("i64", () => {
  it("should create an i64 value with correct type tag", () => {
    const v = i64(42n);
    expect(v.type).toBe(ValueType.I64);
    expect(v.value).toBe(42n);
  });

  it("should clamp to signed 64-bit range via BigInt.asIntN", () => {
    // 2^63 should wrap to the minimum i64 value
    const maxPlus1 = 2n ** 63n;
    expect(i64(maxPlus1).value).toBe(-9223372036854775808n);
  });

  it("should handle 2^64 wrapping to 0", () => {
    expect(i64(2n ** 64n).value).toBe(0n);
  });

  it("should handle negative values", () => {
    expect(i64(-1n).value).toBe(-1n);
  });

  it("should handle zero", () => {
    expect(i64(0n).value).toBe(0n);
  });

  it("should handle max i64 value", () => {
    const maxI64 = 9223372036854775807n;
    expect(i64(maxI64).value).toBe(maxI64);
  });
});

// ===========================================================================
// f32 constructor
// ===========================================================================

describe("f32", () => {
  it("should create an f32 value with correct type tag", () => {
    const v = f32(3.14);
    expect(v.type).toBe(ValueType.F32);
  });

  it("should round to single precision via Math.fround", () => {
    // 1.1 cannot be represented exactly in f32
    const v = f32(1.1);
    expect(v.value).toBe(Math.fround(1.1));
    expect(v.value).not.toBe(1.1);
  });

  it("should preserve exact f32 values", () => {
    expect(f32(0).value).toBe(0);
    expect(f32(1).value).toBe(1);
    expect(f32(-1).value).toBe(-1);
  });

  it("should handle special values", () => {
    expect(f32(Infinity).value).toBe(Infinity);
    expect(f32(-Infinity).value).toBe(-Infinity);
    expect(f32(NaN).value).toBeNaN();
  });
});

// ===========================================================================
// f64 constructor
// ===========================================================================

describe("f64", () => {
  it("should create an f64 value with correct type tag", () => {
    const v = f64(3.14);
    expect(v.type).toBe(ValueType.F64);
    expect(v.value).toBe(3.14);
  });

  it("should preserve full double precision", () => {
    // This value differs between f32 and f64
    const v = f64(1.1);
    expect(v.value).toBe(1.1);
  });

  it("should handle special values", () => {
    expect(f64(Infinity).value).toBe(Infinity);
    expect(f64(-Infinity).value).toBe(-Infinity);
    expect(f64(NaN).value).toBeNaN();
  });
});

// ===========================================================================
// defaultValue
// ===========================================================================

describe("defaultValue", () => {
  it("should return i32(0) for ValueType.I32", () => {
    const v = defaultValue(ValueType.I32);
    expect(v.type).toBe(ValueType.I32);
    expect(v.value).toBe(0);
  });

  it("should return i64(0n) for ValueType.I64", () => {
    const v = defaultValue(ValueType.I64);
    expect(v.type).toBe(ValueType.I64);
    expect(v.value).toBe(0n);
  });

  it("should return f32(0) for ValueType.F32", () => {
    const v = defaultValue(ValueType.F32);
    expect(v.type).toBe(ValueType.F32);
    expect(v.value).toBe(0);
  });

  it("should return f64(0) for ValueType.F64", () => {
    const v = defaultValue(ValueType.F64);
    expect(v.type).toBe(ValueType.F64);
    expect(v.value).toBe(0);
  });

  it("should throw TrapError for unknown type", () => {
    expect(() => defaultValue(0x99)).toThrow(TrapError);
  });
});

// ===========================================================================
// Type extraction helpers
// ===========================================================================

describe("asI32", () => {
  it("should extract the value from an i32", () => {
    expect(asI32(i32(42))).toBe(42);
    expect(asI32(i32(-1))).toBe(-1);
  });

  it("should throw TrapError on type mismatch", () => {
    expect(() => asI32(i64(42n))).toThrow(TrapError);
    expect(() => asI32(f32(1.0))).toThrow(TrapError);
    expect(() => asI32(f64(1.0))).toThrow(TrapError);
  });

  it("should include type name in error message", () => {
    expect(() => asI32(f64(1.0))).toThrow(/expected i32/);
    expect(() => asI32(f64(1.0))).toThrow(/got f64/);
  });
});

describe("asI64", () => {
  it("should extract the value from an i64", () => {
    expect(asI64(i64(100n))).toBe(100n);
    expect(asI64(i64(-1n))).toBe(-1n);
  });

  it("should throw TrapError on type mismatch", () => {
    expect(() => asI64(i32(42))).toThrow(TrapError);
    expect(() => asI64(f32(1.0))).toThrow(TrapError);
    expect(() => asI64(f64(1.0))).toThrow(TrapError);
  });
});

describe("asF32", () => {
  it("should extract the value from an f32", () => {
    expect(asF32(f32(3.14))).toBe(Math.fround(3.14));
  });

  it("should throw TrapError on type mismatch", () => {
    expect(() => asF32(i32(42))).toThrow(TrapError);
    expect(() => asF32(i64(42n))).toThrow(TrapError);
    expect(() => asF32(f64(1.0))).toThrow(TrapError);
  });
});

describe("asF64", () => {
  it("should extract the value from an f64", () => {
    expect(asF64(f64(3.14))).toBe(3.14);
  });

  it("should throw TrapError on type mismatch", () => {
    expect(() => asF64(i32(42))).toThrow(TrapError);
    expect(() => asF64(i64(42n))).toThrow(TrapError);
    expect(() => asF64(f32(1.0))).toThrow(TrapError);
  });
});
