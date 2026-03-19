/**
 * Tests for fma.ts -- fused multiply-add and format conversion.
 */
import { describe, it, expect } from "vitest";
import { FP32, FP16, BF16, type FloatBits } from "../src/formats.js";
import { fpFma, fpConvert } from "../src/fma.js";
import { floatToBits, bitsToFloat, isInf, isNaN as fpIsNaN, isZero } from "../src/ieee754.js";

function fmaAndCheck(aVal: number, bVal: number, cVal: number): void {
  const a = floatToBits(aVal, FP32);
  const b = floatToBits(bVal, FP32);
  const c = floatToBits(cVal, FP32);
  const result = fpFma(a, b, c);
  const resultFloat = bitsToFloat(result);
  const expected = aVal * bVal + cVal;

  if (Number.isNaN(expected)) {
    expect(Number.isNaN(resultFloat)).toBe(true);
  } else if (!Number.isFinite(expected)) {
    expect(!Number.isFinite(resultFloat)).toBe(true);
  } else if (expected === 0.0) {
    expect(Math.abs(resultFloat)).toBeLessThan(1e-6);
  } else {
    const relErr = Math.abs(resultFloat - expected) / Math.max(Math.abs(expected), 1e-45);
    expect(relErr).toBeLessThan(1e-5);
  }
}

describe("fpFma basic", () => {
  it("simple fma", () => fmaAndCheck(1.5, 2.0, 0.25));
  it("multiply only", () => fmaAndCheck(3.0, 4.0, 0.0));
  it("add only", () => fmaAndCheck(1.0, 3.0, 4.0));
  it("negative addend", () => fmaAndCheck(2.0, 3.0, -1.0));
  it("all negative", () => fmaAndCheck(-2.0, -3.0, -1.0));
  it("cancellation", () => fmaAndCheck(2.0, 3.0, -6.0));
  it("pi * e + 1", () => fmaAndCheck(3.14, 2.71, 1.0));
  it("small values", () => fmaAndCheck(0.1, 0.2, 0.3));
  it("large product small addend", () => fmaAndCheck(100.0, 100.0, 0.001));
  it("small product large addend", () => fmaAndCheck(0.001, 0.001, 100.0));
  it("negative product positive addend", () => fmaAndCheck(-2.0, 3.0, 10.0));
  it("positive product negative addend", () => fmaAndCheck(2.0, 3.0, -10.0));
});

describe("fpFma special values", () => {
  it("NaN a", () => expect(fpIsNaN(fpFma(floatToBits(NaN, FP32), floatToBits(1.0, FP32), floatToBits(1.0, FP32)))).toBe(true));
  it("NaN b", () => expect(fpIsNaN(fpFma(floatToBits(1.0, FP32), floatToBits(NaN, FP32), floatToBits(1.0, FP32)))).toBe(true));
  it("NaN c", () => expect(fpIsNaN(fpFma(floatToBits(1.0, FP32), floatToBits(1.0, FP32), floatToBits(NaN, FP32)))).toBe(true));
  it("NaN all", () => {
    const nan = floatToBits(NaN, FP32);
    expect(fpIsNaN(fpFma(nan, nan, nan))).toBe(true);
  });

  it("Inf * 0 = NaN", () => {
    expect(fpIsNaN(fpFma(floatToBits(Infinity, FP32), floatToBits(0.0, FP32), floatToBits(1.0, FP32)))).toBe(true);
  });
  it("0 * Inf = NaN", () => {
    expect(fpIsNaN(fpFma(floatToBits(0.0, FP32), floatToBits(Infinity, FP32), floatToBits(1.0, FP32)))).toBe(true);
  });

  it("Inf * finite + finite = Inf", () => {
    const result = fpFma(floatToBits(Infinity, FP32), floatToBits(2.0, FP32), floatToBits(1.0, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(0);
  });

  it("-Inf * pos + finite = -Inf", () => {
    const result = fpFma(floatToBits(-Infinity, FP32), floatToBits(2.0, FP32), floatToBits(1.0, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });

  it("Inf * 1 + (-Inf) = NaN", () => {
    expect(fpIsNaN(fpFma(floatToBits(Infinity, FP32), floatToBits(1.0, FP32), floatToBits(-Infinity, FP32)))).toBe(true);
  });

  it("Inf * 1 + Inf = Inf", () => {
    const result = fpFma(floatToBits(Infinity, FP32), floatToBits(1.0, FP32), floatToBits(Infinity, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(0);
  });

  it("0 * 0 + number = number", () => {
    const result = fpFma(floatToBits(0.0, FP32), floatToBits(0.0, FP32), floatToBits(5.0, FP32));
    expect(Math.abs(bitsToFloat(result) - 5.0)).toBeLessThan(1e-6);
  });

  it("0 * number + 0 = 0", () => {
    const result = fpFma(floatToBits(0.0, FP32), floatToBits(5.0, FP32), floatToBits(0.0, FP32));
    expect(isZero(result)).toBe(true);
  });

  it("0 * (-1) + (-0) = -0", () => {
    const result = fpFma(floatToBits(0.0, FP32), floatToBits(-1.0, FP32), floatToBits(-0.0, FP32));
    expect(isZero(result)).toBe(true);
    expect(result.sign).toBe(1);
  });

  it("0 * 1 + 0 = +0", () => {
    const result = fpFma(floatToBits(0.0, FP32), floatToBits(1.0, FP32), floatToBits(0.0, FP32));
    expect(isZero(result)).toBe(true);
    expect(result.sign).toBe(0);
  });

  it("finite * finite + Inf = Inf", () => {
    const result = fpFma(floatToBits(2.0, FP32), floatToBits(3.0, FP32), floatToBits(Infinity, FP32));
    expect(isInf(result)).toBe(true);
  });

  it("finite * finite + -Inf = -Inf", () => {
    const result = fpFma(floatToBits(2.0, FP32), floatToBits(3.0, FP32), floatToBits(-Infinity, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });

  it("neg product sign", () => {
    expect(bitsToFloat(fpFma(floatToBits(-1.0, FP32), floatToBits(2.0, FP32), floatToBits(0.0, FP32)))).toBe(-2.0);
  });

  it("0 * number returns c", () => {
    const result = fpFma(floatToBits(0.0, FP32), floatToBits(42.0, FP32), floatToBits(7.0, FP32));
    expect(Math.abs(bitsToFloat(result) - 7.0)).toBeLessThan(1e-6);
  });
});

describe("fpFma overflow/underflow", () => {
  it("overflow to Inf", () => {
    expect(isInf(fpFma(floatToBits(1e30, FP32), floatToBits(1e30, FP32), floatToBits(0.0, FP32)))).toBe(true);
  });
  it("cancellation to zero", () => fmaAndCheck(2.0, 3.0, -6.0));
  it("denormal addend", () => {
    const denorm: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [1, ...new Array(22).fill(0)], fmt: FP32 };
    const result = fpFma(floatToBits(1.0, FP32), floatToBits(1.0, FP32), denorm);
    expect(Math.abs(bitsToFloat(result) - 1.0)).toBeLessThan(1e-6);
  });
});

describe("fpFma many values", () => {
  const cases: [number, number, number][] = [
    [1.0, 1.0, 1.0], [2.0, 3.0, 4.0], [0.5, 0.5, 0.5],
    [-1.0, 2.0, 3.0], [10.0, 10.0, -100.0], [3.14, 2.71, 1.41],
    [0.1, 0.2, 0.3], [100.0, 0.01, 1.0], [0.25, 4.0, 0.0],
    [-0.5, -0.5, 0.25], [1.0, -1.0, 1.0], [8.0, 0.125, 0.0],
  ];
  for (const [a, b, c] of cases) {
    it(`FMA(${a}, ${b}, ${c})`, () => fmaAndCheck(a, b, c));
  }
});

// ---------------------------------------------------------------------------
// Format conversion
// ---------------------------------------------------------------------------

describe("fpConvert", () => {
  it("same format noop", () => {
    const bits = floatToBits(3.14, FP32);
    expect(bitsToFloat(fpConvert(bits, FP32))).toBe(bitsToFloat(bits));
  });

  it("FP32 to FP16", () => {
    const result = fpConvert(floatToBits(1.0, FP32), FP16);
    expect(result.fmt).toBe(FP16);
    expect(bitsToFloat(result)).toBe(1.0);
  });

  it("FP32 to BF16", () => {
    const result = fpConvert(floatToBits(1.0, FP32), BF16);
    expect(result.fmt).toBe(BF16);
    expect(bitsToFloat(result)).toBe(1.0);
  });

  it("FP16 to FP32", () => {
    const result = fpConvert(floatToBits(2.0, FP16), FP32);
    expect(result.fmt).toBe(FP32);
    expect(bitsToFloat(result)).toBe(2.0);
  });

  it("BF16 to FP32", () => {
    const result = fpConvert(floatToBits(0.5, BF16), FP32);
    expect(result.fmt).toBe(FP32);
    expect(bitsToFloat(result)).toBe(0.5);
  });

  it("FP32 to FP16 precision loss", () => {
    const fp16 = fpConvert(floatToBits(3.14, FP32), FP16);
    const back = fpConvert(fp16, FP32);
    expect(Math.abs(bitsToFloat(back) - 3.14)).toBeLessThan(0.01);
  });

  it("FP32 to BF16 precision loss", () => {
    const bf16 = fpConvert(floatToBits(3.14, FP32), BF16);
    expect(Math.abs(bitsToFloat(bf16) - 3.14)).toBeLessThan(0.05);
  });

  it("convert NaN to FP16", () => expect(fpIsNaN(fpConvert(floatToBits(NaN, FP32), FP16))).toBe(true));
  it("convert NaN to BF16", () => expect(fpIsNaN(fpConvert(floatToBits(NaN, FP32), BF16))).toBe(true));
  it("convert Inf to FP16", () => expect(isInf(fpConvert(floatToBits(Infinity, FP32), FP16))).toBe(true));
  it("convert -Inf to FP16", () => {
    const result = fpConvert(floatToBits(-Infinity, FP32), FP16);
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });
  it("convert Inf to BF16", () => expect(isInf(fpConvert(floatToBits(Infinity, FP32), BF16))).toBe(true));
  it("convert zero to BF16", () => expect(isZero(fpConvert(floatToBits(0.0, FP32), BF16))).toBe(true));
  it("convert -0 to FP16", () => {
    const result = fpConvert(floatToBits(-0.0, FP32), FP16);
    expect(isZero(result)).toBe(true);
    expect(result.sign).toBe(1);
  });
  it("convert overflow to Inf", () => expect(isInf(fpConvert(floatToBits(100000.0, FP32), FP16))).toBe(true));

  it("FP16 to BF16", () => {
    const result = fpConvert(floatToBits(1.0, FP16), BF16);
    expect(result.fmt).toBe(BF16);
    expect(bitsToFloat(result)).toBe(1.0);
  });

  it("BF16 to FP16", () => {
    const result = fpConvert(floatToBits(1.0, BF16), FP16);
    expect(result.fmt).toBe(FP16);
    expect(bitsToFloat(result)).toBe(1.0);
  });

  it("convert negative", () => {
    const result = fpConvert(floatToBits(-2.5, FP32), FP16);
    expect(result.sign).toBe(1);
    expect(Math.abs(bitsToFloat(result) - (-2.5))).toBeLessThan(0.01);
  });

  it("FP16 exact roundtrip", () => {
    for (const val of [0.0, 1.0, -1.0, 0.5, 2.0, 4.0, 0.25]) {
      const fp16 = fpConvert(floatToBits(val, FP32), FP16);
      const back = fpConvert(fp16, FP32);
      expect(bitsToFloat(back)).toBe(val);
    }
  });

  it("BF16 exact roundtrip", () => {
    for (const val of [0.0, 1.0, -1.0, 0.5, 2.0, 128.0]) {
      const bf16 = fpConvert(floatToBits(val, FP32), BF16);
      const back = fpConvert(bf16, FP32);
      expect(bitsToFloat(back)).toBe(val);
    }
  });
});
