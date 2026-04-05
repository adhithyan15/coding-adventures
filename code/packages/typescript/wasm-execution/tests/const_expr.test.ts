/**
 * const_expr.test.ts --- Tests for WASM constant expression evaluator.
 */

import { describe, it, expect } from "vitest";
import { ValueType } from "@coding-adventures/wasm-types";
import { evaluateConstExpr } from "../src/const_expr.js";
import { i32, i64, f32, f64 } from "../src/values.js";
import { TrapError } from "../src/host_interface.js";

// ===========================================================================
// Helper: encode bytes for test expressions
// ===========================================================================

/**
 * Encode a signed 32-bit value as LEB128 bytes.
 * Used to build test constant expressions.
 */
function signedLEB128(value: number): number[] {
  const bytes: number[] = [];
  let v = value;
  let more = true;
  while (more) {
    let byte = v & 0x7f;
    v >>= 7;
    // Check if we need more bytes: for positive values, all remaining
    // bits must be 0 and sign bit of current byte must be 0; for
    // negative values, all remaining bits must be 1 and sign bit must be 1.
    if ((v === 0 && (byte & 0x40) === 0) || (v === -1 && (byte & 0x40) !== 0)) {
      more = false;
    } else {
      byte |= 0x80;
    }
    bytes.push(byte);
  }
  return bytes;
}

/**
 * Encode an unsigned 32-bit value as LEB128 bytes.
 */
function unsignedLEB128(value: number): number[] {
  const bytes: number[] = [];
  let v = value;
  do {
    let byte = v & 0x7f;
    v >>>= 7;
    if (v !== 0) byte |= 0x80;
    bytes.push(byte);
  } while (v !== 0);
  return bytes;
}

/**
 * Encode a float32 as 4 little-endian bytes.
 */
function float32Bytes(value: number): number[] {
  const buf = new ArrayBuffer(4);
  new DataView(buf).setFloat32(0, value, true);
  return [...new Uint8Array(buf)];
}

/**
 * Encode a float64 as 8 little-endian bytes.
 */
function float64Bytes(value: number): number[] {
  const buf = new ArrayBuffer(8);
  new DataView(buf).setFloat64(0, value, true);
  return [...new Uint8Array(buf)];
}

/**
 * Encode a signed 64-bit BigInt as LEB128 bytes.
 */
function signedLEB128_64(value: bigint): number[] {
  const bytes: number[] = [];
  let v = value;
  let more = true;
  while (more) {
    let byte = Number(v & 0x7fn);
    v >>= 7n;
    if (
      (v === 0n && (byte & 0x40) === 0) ||
      (v === -1n && (byte & 0x40) !== 0)
    ) {
      more = false;
    } else {
      byte |= 0x80;
    }
    bytes.push(byte);
  }
  return bytes;
}

// ===========================================================================
// i32.const
// ===========================================================================

describe("evaluateConstExpr: i32.const", () => {
  it("should evaluate a simple i32.const", () => {
    // (i32.const 42) = [0x41, LEB128(42), 0x0B]
    const expr = new Uint8Array([0x41, ...signedLEB128(42), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.I32);
    expect(result.value).toBe(42);
  });

  it("should handle i32.const with zero", () => {
    const expr = new Uint8Array([0x41, ...signedLEB128(0), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.I32);
    expect(result.value).toBe(0);
  });

  it("should handle negative i32.const", () => {
    const expr = new Uint8Array([0x41, ...signedLEB128(-1), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.I32);
    expect(result.value).toBe(-1);
  });

  it("should handle large positive i32.const", () => {
    const expr = new Uint8Array([0x41, ...signedLEB128(2147483647), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.I32);
    expect(result.value).toBe(2147483647);
  });

  it("should handle minimum i32 value", () => {
    const expr = new Uint8Array([
      0x41,
      ...signedLEB128(-2147483648),
      0x0b,
    ]);
    const result = evaluateConstExpr(expr, []);
    expect(result.value).toBe(-2147483648);
  });
});

// ===========================================================================
// i64.const
// ===========================================================================

describe("evaluateConstExpr: i64.const", () => {
  it("should evaluate a simple i64.const", () => {
    const expr = new Uint8Array([0x42, ...signedLEB128_64(100n), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.I64);
    expect(result.value).toBe(100n);
  });

  it("should handle negative i64.const", () => {
    const expr = new Uint8Array([0x42, ...signedLEB128_64(-1n), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.I64);
    expect(result.value).toBe(-1n);
  });

  it("should handle large i64 values", () => {
    const big = 9223372036854775807n; // i64 max
    const expr = new Uint8Array([0x42, ...signedLEB128_64(big), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.I64);
    expect(result.value).toBe(big);
  });

  it("should handle i64 zero", () => {
    const expr = new Uint8Array([0x42, ...signedLEB128_64(0n), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.value).toBe(0n);
  });
});

// ===========================================================================
// f32.const
// ===========================================================================

describe("evaluateConstExpr: f32.const", () => {
  it("should evaluate an f32.const", () => {
    const expr = new Uint8Array([0x43, ...float32Bytes(3.14), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.F32);
    expect(result.value).toBeCloseTo(3.14, 2);
  });

  it("should handle f32 zero", () => {
    const expr = new Uint8Array([0x43, ...float32Bytes(0), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.value).toBe(0);
  });

  it("should handle f32 negative", () => {
    const expr = new Uint8Array([0x43, ...float32Bytes(-1.5), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.value).toBe(Math.fround(-1.5));
  });

  it("should handle f32 infinity", () => {
    const expr = new Uint8Array([0x43, ...float32Bytes(Infinity), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.value).toBe(Infinity);
  });
});

// ===========================================================================
// f64.const
// ===========================================================================

describe("evaluateConstExpr: f64.const", () => {
  it("should evaluate an f64.const", () => {
    const expr = new Uint8Array([0x44, ...float64Bytes(3.141592653589793), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.type).toBe(ValueType.F64);
    expect(result.value).toBe(3.141592653589793);
  });

  it("should handle f64 zero", () => {
    const expr = new Uint8Array([0x44, ...float64Bytes(0), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.value).toBe(0);
  });

  it("should handle f64 negative", () => {
    const expr = new Uint8Array([0x44, ...float64Bytes(-2.718), 0x0b]);
    const result = evaluateConstExpr(expr, []);
    expect(result.value).toBe(-2.718);
  });
});

// ===========================================================================
// global.get
// ===========================================================================

describe("evaluateConstExpr: global.get", () => {
  it("should read a global value", () => {
    const globals = [i32(100), i64(200n), f32(3.0), f64(4.0)];
    // global.get 0
    const expr = new Uint8Array([0x23, ...unsignedLEB128(0), 0x0b]);
    const result = evaluateConstExpr(expr, globals);
    expect(result.type).toBe(ValueType.I32);
    expect(result.value).toBe(100);
  });

  it("should read globals at various indices", () => {
    const globals = [i32(10), i64(20n), f64(30.0)];

    const expr1 = new Uint8Array([0x23, ...unsignedLEB128(1), 0x0b]);
    expect(evaluateConstExpr(expr1, globals).value).toBe(20n);

    const expr2 = new Uint8Array([0x23, ...unsignedLEB128(2), 0x0b]);
    expect(evaluateConstExpr(expr2, globals).value).toBe(30.0);
  });

  it("should trap on out-of-bounds global index", () => {
    const globals = [i32(42)];
    const expr = new Uint8Array([0x23, ...unsignedLEB128(5), 0x0b]);
    expect(() => evaluateConstExpr(expr, globals)).toThrow(TrapError);
  });
});

// ===========================================================================
// Error cases
// ===========================================================================

describe("evaluateConstExpr: errors", () => {
  it("should trap on illegal opcode", () => {
    // 0x6A = i32.add, not allowed in const expr
    const expr = new Uint8Array([0x6a, 0x0b]);
    expect(() => evaluateConstExpr(expr, [])).toThrow(TrapError);
    expect(() => evaluateConstExpr(expr, [])).toThrow(/Illegal opcode/);
  });

  it("should trap on empty expression (no value produced)", () => {
    const expr = new Uint8Array([0x0b]);
    expect(() => evaluateConstExpr(expr, [])).toThrow(TrapError);
  });

  it("should trap on missing end opcode", () => {
    const expr = new Uint8Array([0x41, 0x2a]); // i32.const 42, no end
    expect(() => evaluateConstExpr(expr, [])).toThrow(TrapError);
    expect(() => evaluateConstExpr(expr, [])).toThrow(/missing end/);
  });

  it("should trap on f32.const with insufficient bytes", () => {
    const expr = new Uint8Array([0x43, 0x00, 0x00]); // only 2 of 4 bytes
    expect(() => evaluateConstExpr(expr, [])).toThrow(TrapError);
  });

  it("should trap on f64.const with insufficient bytes", () => {
    const expr = new Uint8Array([0x44, 0x00, 0x00, 0x00, 0x00]); // only 4 of 8
    expect(() => evaluateConstExpr(expr, [])).toThrow(TrapError);
  });
});
