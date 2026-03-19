/**
 * Tests for fp-multiplier.ts -- floating-point multiplication.
 */
import { describe, it, expect } from "vitest";
import { FP32, type FloatBits } from "../src/formats.js";
import { fpMul } from "../src/fp-multiplier.js";
import { floatToBits, bitsToFloat, bitsMsbToInt, isInf, isNaN as fpIsNaN, isZero } from "../src/ieee754.js";

function mulAndCheck(aVal: number, bVal: number): void {
  const a = floatToBits(aVal, FP32);
  const b = floatToBits(bVal, FP32);
  const result = fpMul(a, b);
  const resultFloat = bitsToFloat(result);
  const expected = aVal * bVal;

  if (Number.isNaN(expected)) {
    expect(Number.isNaN(resultFloat)).toBe(true);
  } else if (!Number.isFinite(expected)) {
    expect(!Number.isFinite(resultFloat)).toBe(true);
  } else if (expected === 0.0) {
    expect(resultFloat).toBe(0.0);
  } else {
    const relErr = Math.abs(resultFloat - expected) / Math.max(Math.abs(expected), 1e-45);
    expect(relErr).toBeLessThan(1e-6);
  }
}

describe("fpMul basic", () => {
  it("1 * 1", () => mulAndCheck(1.0, 1.0));
  it("2 * 3", () => mulAndCheck(2.0, 3.0));
  it("0.5 * 0.5", () => mulAndCheck(0.5, 0.5));
  it("pi * e", () => mulAndCheck(3.14, 2.71));
  it("large * small", () => mulAndCheck(1000.0, 0.001));
  it("neg * pos", () => mulAndCheck(-3.0, 4.0));
  it("neg * neg", () => mulAndCheck(-3.0, -4.0));
  it("1 * value", () => mulAndCheck(1.0, 42.0));
  it("power of two", () => mulAndCheck(3.14, 8.0));
  it("quarter * quarter", () => mulAndCheck(0.25, 0.25));
  it("large integer", () => mulAndCheck(1000.0, 1000.0));
  it("fractional", () => mulAndCheck(0.1, 0.3));
});

describe("fpMul special values", () => {
  it("NaN * number = NaN", () => expect(fpIsNaN(fpMul(floatToBits(NaN, FP32), floatToBits(1.0, FP32)))).toBe(true));
  it("number * NaN = NaN", () => expect(fpIsNaN(fpMul(floatToBits(1.0, FP32), floatToBits(NaN, FP32)))).toBe(true));
  it("NaN * NaN = NaN", () => expect(fpIsNaN(fpMul(floatToBits(NaN, FP32), floatToBits(NaN, FP32)))).toBe(true));
  it("NaN * 0 = NaN", () => expect(fpIsNaN(fpMul(floatToBits(NaN, FP32), floatToBits(0.0, FP32)))).toBe(true));
  it("NaN * Inf = NaN", () => expect(fpIsNaN(fpMul(floatToBits(NaN, FP32), floatToBits(Infinity, FP32)))).toBe(true));

  it("Inf * number = Inf", () => {
    const result = fpMul(floatToBits(Infinity, FP32), floatToBits(2.0, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(0);
  });
  it("Inf * negative = -Inf", () => {
    const result = fpMul(floatToBits(Infinity, FP32), floatToBits(-2.0, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });
  it("-Inf * negative = +Inf", () => {
    const result = fpMul(floatToBits(-Infinity, FP32), floatToBits(-2.0, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(0);
  });
  it("Inf * 0 = NaN", () => expect(fpIsNaN(fpMul(floatToBits(Infinity, FP32), floatToBits(0.0, FP32)))).toBe(true));
  it("0 * Inf = NaN", () => expect(fpIsNaN(fpMul(floatToBits(0.0, FP32), floatToBits(Infinity, FP32)))).toBe(true));
  it("-Inf * 0 = NaN", () => expect(fpIsNaN(fpMul(floatToBits(-Infinity, FP32), floatToBits(0.0, FP32)))).toBe(true));
  it("0 * number = 0", () => expect(isZero(fpMul(floatToBits(0.0, FP32), floatToBits(42.0, FP32)))).toBe(true));
  it("number * 0 = 0", () => expect(isZero(fpMul(floatToBits(42.0, FP32), floatToBits(0.0, FP32)))).toBe(true));

  it("zero sign positive", () => {
    expect(fpMul(floatToBits(1.0, FP32), floatToBits(0.0, FP32)).sign).toBe(0);
  });
  it("zero sign negative", () => {
    expect(fpMul(floatToBits(1.0, FP32), floatToBits(-0.0, FP32)).sign).toBe(1);
  });
  it("-0 * -0 = +0", () => {
    const result = fpMul(floatToBits(-0.0, FP32), floatToBits(-0.0, FP32));
    expect(isZero(result)).toBe(true);
    expect(result.sign).toBe(0);
  });
  it("Inf * Inf = Inf", () => {
    const result = fpMul(floatToBits(Infinity, FP32), floatToBits(Infinity, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(0);
  });
  it("-Inf * Inf = -Inf", () => {
    const result = fpMul(floatToBits(-Infinity, FP32), floatToBits(Infinity, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });
});

describe("fpMul overflow/underflow", () => {
  it("overflow to Inf", () => {
    expect(isInf(fpMul(floatToBits(1e30, FP32), floatToBits(1e30, FP32)))).toBe(true);
  });
  it("overflow negative Inf", () => {
    const result = fpMul(floatToBits(-1e30, FP32), floatToBits(1e30, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });
  it("underflow to zero", () => {
    expect(isZero(fpMul(floatToBits(1e-30, FP32), floatToBits(1e-30, FP32)))).toBe(true);
  });
  it("underflow to denormal", () => {
    const result = fpMul(floatToBits(1e-20, FP32), floatToBits(1e-20, FP32));
    expect(bitsMsbToInt(result.exponent)).toBe(0);
  });
  it("denormal * normal", () => {
    const denorm: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [1, ...new Array(22).fill(0)], fmt: FP32 };
    const normal = floatToBits(2.0, FP32);
    const result = fpMul(denorm, normal);
    expect(bitsToFloat(result)).toBeGreaterThan(0);
  });
  it("denormal * denormal", () => {
    const d1: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [1, ...new Array(22).fill(0)], fmt: FP32 };
    const d2: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [0, 1, ...new Array(21).fill(0)], fmt: FP32 };
    const result = fpMul(d1, d2);
    expect(isZero(result) || bitsToFloat(result) >= 0).toBe(true);
  });
});

describe("fpMul many values", () => {
  const cases: [number, number][] = [
    [1.0, 1.0], [2.0, 3.0], [0.5, 0.5], [-1.0, 1.0], [-1.0, -1.0],
    [0.1, 10.0], [3.14, 2.71], [100.0, 100.0], [0.001, 1000.0],
    [1.5, 2.5], [0.125, 8.0], [-0.5, -0.5], [1e10, 1e-10], [2.0, 2.0],
  ];
  for (const [a, b] of cases) {
    it(`${a} * ${b}`, () => mulAndCheck(a, b));
  }
});

describe("fpMul sign handling", () => {
  it("pos * pos = pos", () => expect(fpMul(floatToBits(2.0, FP32), floatToBits(3.0, FP32)).sign).toBe(0));
  it("pos * neg = neg", () => expect(fpMul(floatToBits(2.0, FP32), floatToBits(-3.0, FP32)).sign).toBe(1));
  it("neg * pos = neg", () => expect(fpMul(floatToBits(-2.0, FP32), floatToBits(3.0, FP32)).sign).toBe(1));
  it("neg * neg = pos", () => expect(fpMul(floatToBits(-2.0, FP32), floatToBits(-3.0, FP32)).sign).toBe(0));
});
