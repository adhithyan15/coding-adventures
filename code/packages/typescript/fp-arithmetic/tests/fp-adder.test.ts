/**
 * Tests for fp-adder.ts -- floating-point addition, subtraction, and comparison.
 */
import { describe, it, expect } from "vitest";
import { FP32, type FloatBits } from "../src/formats.js";
import {
  shiftRight, shiftLeft, findLeadingOne,
  subtractUnsigned, addBitsMsb,
  fpAdd, fpSub, fpNeg, fpAbs, fpCompare,
} from "../src/fp-adder.js";
import {
  floatToBits, bitsToFloat, bitsMsbToInt,
  isInf, isNaN as fpIsNaN, isZero,
} from "../src/ieee754.js";

function addAndCheck(aVal: number, bVal: number): void {
  const a = floatToBits(aVal, FP32);
  const b = floatToBits(bVal, FP32);
  const result = fpAdd(a, b);
  const resultFloat = bitsToFloat(result);
  const expected = aVal + bVal;

  if (Number.isNaN(expected)) {
    expect(Number.isNaN(resultFloat)).toBe(true);
  } else if (!Number.isFinite(expected)) {
    expect(!Number.isFinite(resultFloat)).toBe(true);
    expect(Math.sign(resultFloat)).toBe(Math.sign(expected));
  } else if (expected === 0.0) {
    expect(resultFloat).toBe(0.0);
  } else {
    const relErr = Math.abs(resultFloat - expected) / Math.max(Math.abs(expected), 1e-45);
    expect(relErr).toBeLessThan(1e-6);
  }
}

// ---------------------------------------------------------------------------
// Helper tests
// ---------------------------------------------------------------------------

describe("shiftRight", () => {
  it("by zero", () => expect(shiftRight([1, 0, 1, 1], 0)).toEqual([1, 0, 1, 1]));
  it("by one", () => expect(shiftRight([1, 0, 1, 1], 1)).toEqual([0, 1, 0, 1]));
  it("by two", () => expect(shiftRight([1, 0, 1, 1], 2)).toEqual([0, 0, 1, 0]));
  it("exceeds width", () => expect(shiftRight([1, 0, 1, 1], 5)).toEqual([0, 0, 0, 0]));
  it("equals width", () => expect(shiftRight([1, 0, 1, 1], 4)).toEqual([0, 0, 0, 0]));
  it("negative", () => expect(shiftRight([1, 0, 1], -1)).toEqual([1, 0, 1]));
});

describe("shiftLeft", () => {
  it("by zero", () => expect(shiftLeft([1, 0, 1, 1], 0)).toEqual([1, 0, 1, 1]));
  it("by one", () => expect(shiftLeft([1, 0, 1, 1], 1)).toEqual([0, 1, 1, 0]));
  it("by two", () => expect(shiftLeft([1, 0, 1, 1], 2)).toEqual([1, 1, 0, 0]));
  it("exceeds width", () => expect(shiftLeft([1, 0, 1, 1], 5)).toEqual([0, 0, 0, 0]));
  it("equals width", () => expect(shiftLeft([1, 0, 1, 1], 4)).toEqual([0, 0, 0, 0]));
  it("negative", () => expect(shiftLeft([1, 0, 1], -1)).toEqual([1, 0, 1]));
});

describe("findLeadingOne", () => {
  it("first bit", () => expect(findLeadingOne([1, 0, 0, 0])).toBe(0));
  it("middle bit", () => expect(findLeadingOne([0, 0, 1, 0, 1])).toBe(2));
  it("last bit", () => expect(findLeadingOne([0, 0, 0, 1])).toBe(3));
  it("all zeros", () => expect(findLeadingOne([0, 0, 0, 0, 0])).toBe(-1));
  it("all ones", () => expect(findLeadingOne([1, 1, 1, 1])).toBe(0));
  it("single one", () => expect(findLeadingOne([1])).toBe(0));
  it("single zero", () => expect(findLeadingOne([0])).toBe(-1));
});

describe("subtractUnsigned", () => {
  it("5 - 3 = 2", () => {
    const [result, borrow] = subtractUnsigned([0, 1, 0, 1], [0, 0, 1, 1]);
    expect(bitsMsbToInt(result)).toBe(2);
    expect(borrow).toBe(0);
  });
  it("3 - 5 has borrow", () => {
    const [, borrow] = subtractUnsigned([0, 0, 1, 1], [0, 1, 0, 1]);
    expect(borrow).toBe(1);
  });
  it("equal values", () => {
    const [result, borrow] = subtractUnsigned([0, 1, 0, 1], [0, 1, 0, 1]);
    expect(bitsMsbToInt(result)).toBe(0);
    expect(borrow).toBe(0);
  });
});

describe("addBitsMsb", () => {
  it("3 + 5 = 8", () => {
    const [result, carry] = addBitsMsb([0, 0, 1, 1], [0, 1, 0, 1]);
    expect(bitsMsbToInt(result)).toBe(8);
    expect(carry).toBe(0);
  });
  it("overflow", () => {
    const [, carry] = addBitsMsb([1, 1, 1, 1], [0, 0, 0, 1]);
    expect(carry).toBe(1);
  });
  it("with carry in", () => {
    const [result, carry] = addBitsMsb([0, 0, 0, 0], [0, 0, 0, 0], 1);
    expect(bitsMsbToInt(result)).toBe(1);
    expect(carry).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Basic FP32 addition
// ---------------------------------------------------------------------------

describe("fpAdd basic", () => {
  it("1 + 1", () => addAndCheck(1.0, 1.0));
  it("1 + 2", () => addAndCheck(1.0, 2.0));
  it("0.5 + 0.5", () => addAndCheck(0.5, 0.5));
  it("pi + e", () => addAndCheck(3.14, 2.71));
  it("large + small", () => addAndCheck(1000.0, 0.001));
  it("same value", () => addAndCheck(42.0, 42.0));
  it("neg + neg", () => addAndCheck(-3.0, -4.0));
  it("pos + neg same magnitude", () => addAndCheck(5.0, -5.0));
  it("pos + neg different", () => addAndCheck(5.0, -3.0));
  it("neg + pos", () => addAndCheck(-3.0, 5.0));
  it("small subtraction (catastrophic cancellation)", () => {
    const a = floatToBits(1.0000001, FP32);
    const b = floatToBits(-1.0, FP32);
    const result = fpAdd(a, b);
    const val = bitsToFloat(result);
    expect(Math.abs(val)).toBeLessThan(1e-6);
    expect(Math.abs(val)).toBeGreaterThan(1e-9);
  });
  it("very large exponent diff", () => addAndCheck(1e20, 1e-20));
  it("quarter values", () => addAndCheck(0.25, 0.75));
  it("negative result", () => addAndCheck(1.0, -3.0));
});

// ---------------------------------------------------------------------------
// Special values
// ---------------------------------------------------------------------------

describe("fpAdd special values", () => {
  it("NaN + number = NaN", () => expect(fpIsNaN(fpAdd(floatToBits(NaN, FP32), floatToBits(1.0, FP32)))).toBe(true));
  it("number + NaN = NaN", () => expect(fpIsNaN(fpAdd(floatToBits(1.0, FP32), floatToBits(NaN, FP32)))).toBe(true));
  it("NaN + NaN = NaN", () => expect(fpIsNaN(fpAdd(floatToBits(NaN, FP32), floatToBits(NaN, FP32)))).toBe(true));

  it("Inf + Inf = Inf", () => {
    const a = floatToBits(Infinity, FP32);
    const result = fpAdd(a, a);
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(0);
  });
  it("Inf + (-Inf) = NaN", () => expect(fpIsNaN(fpAdd(floatToBits(Infinity, FP32), floatToBits(-Infinity, FP32)))).toBe(true));
  it("(-Inf) + Inf = NaN", () => expect(fpIsNaN(fpAdd(floatToBits(-Infinity, FP32), floatToBits(Infinity, FP32)))).toBe(true));

  it("Inf + number = Inf", () => {
    const result = fpAdd(floatToBits(Infinity, FP32), floatToBits(42.0, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(0);
  });

  it("number + Inf = Inf", () => {
    expect(isInf(fpAdd(floatToBits(42.0, FP32), floatToBits(Infinity, FP32)))).toBe(true);
  });

  it("(-Inf) + (-Inf) = -Inf", () => {
    const a = floatToBits(-Infinity, FP32);
    const result = fpAdd(a, a);
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });

  it("0 + 0 = 0", () => {
    const a = floatToBits(0.0, FP32);
    const result = fpAdd(a, a);
    expect(isZero(result)).toBe(true);
    expect(result.sign).toBe(0);
  });

  it("-0 + -0 = -0", () => {
    const a = floatToBits(-0.0, FP32);
    const result = fpAdd(a, a);
    expect(isZero(result)).toBe(true);
    expect(result.sign).toBe(1);
  });

  it("+0 + (-0) = +0", () => {
    const result = fpAdd(floatToBits(0.0, FP32), floatToBits(-0.0, FP32));
    expect(isZero(result)).toBe(true);
    expect(result.sign).toBe(0);
  });

  it("0 + number = number", () => {
    const result = fpAdd(floatToBits(0.0, FP32), floatToBits(3.14, FP32));
    expect(Math.abs(bitsToFloat(result) - 3.14)).toBeLessThan(0.001);
  });

  it("NaN + Inf = NaN", () => expect(fpIsNaN(fpAdd(floatToBits(NaN, FP32), floatToBits(Infinity, FP32)))).toBe(true));
  it("NaN + 0 = NaN", () => expect(fpIsNaN(fpAdd(floatToBits(NaN, FP32), floatToBits(0.0, FP32)))).toBe(true));
});

// ---------------------------------------------------------------------------
// Overflow and underflow
// ---------------------------------------------------------------------------

describe("fpAdd overflow/underflow", () => {
  it("overflow to +Inf", () => {
    const result = fpAdd(floatToBits(3.0e38, FP32), floatToBits(3.0e38, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(0);
  });

  it("overflow to -Inf", () => {
    const result = fpAdd(floatToBits(-3.0e38, FP32), floatToBits(-3.0e38, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });

  it("denormal + denormal", () => {
    const d1: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [1, ...new Array(22).fill(0)], fmt: FP32 };
    const d2: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [0, 1, ...new Array(21).fill(0)], fmt: FP32 };
    const result = fpAdd(d1, d2);
    const resultVal = bitsToFloat(result);
    expect(Math.abs(resultVal)).toBeGreaterThan(0);
  });

  it("subtraction to zero", () => {
    const result = fpAdd(floatToBits(42.0, FP32), floatToBits(-42.0, FP32));
    expect(isZero(result)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Parametrized tests
// ---------------------------------------------------------------------------

describe("fpAdd many values", () => {
  const cases: [number, number][] = [
    [1.0, 1.0], [1.0, -1.0], [0.1, 0.2], [100.0, 0.01],
    [-7.5, 3.25], [1e10, 1e-10], [0.5, 0.25], [1.5, 2.5],
    [-1.0, -2.0], [3.14, 2.71], [0.125, 0.0625], [-0.5, 0.5],
    [256.0, 256.0], [0.001, -0.0005],
  ];
  for (const [a, b] of cases) {
    it(`${a} + ${b}`, () => addAndCheck(a, b));
  }
});

// ---------------------------------------------------------------------------
// Subtraction
// ---------------------------------------------------------------------------

describe("fpSub", () => {
  it("3 - 1 = 2", () => {
    expect(Math.abs(bitsToFloat(fpSub(floatToBits(3.0, FP32), floatToBits(1.0, FP32))) - 2.0)).toBeLessThan(1e-6);
  });
  it("1 - 3 = -2", () => {
    expect(Math.abs(bitsToFloat(fpSub(floatToBits(1.0, FP32), floatToBits(3.0, FP32))) - (-2.0))).toBeLessThan(1e-6);
  });
  it("same value = 0", () => {
    const a = floatToBits(42.0, FP32);
    expect(isZero(fpSub(a, a))).toBe(true);
  });
  it("1 - Inf = -Inf", () => {
    const result = fpSub(floatToBits(1.0, FP32), floatToBits(Infinity, FP32));
    expect(isInf(result)).toBe(true);
    expect(result.sign).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Negation
// ---------------------------------------------------------------------------

describe("fpNeg", () => {
  it("negate positive", () => {
    const result = fpNeg(floatToBits(3.14, FP32));
    expect(result.sign).toBe(1);
  });
  it("negate negative", () => expect(fpNeg(floatToBits(-2.5, FP32)).sign).toBe(0));
  it("negate zero", () => expect(fpNeg(floatToBits(0.0, FP32)).sign).toBe(1));
  it("negate -0", () => expect(fpNeg(floatToBits(-0.0, FP32)).sign).toBe(0));
  it("double negate", () => {
    const a = floatToBits(1.0, FP32);
    expect(fpNeg(fpNeg(a)).sign).toBe(a.sign);
  });
  it("negate inf", () => {
    const result = fpNeg(floatToBits(Infinity, FP32));
    expect(result.sign).toBe(1);
    expect(isInf(result)).toBe(true);
  });
  it("negate nan", () => {
    const result = fpNeg(floatToBits(NaN, FP32));
    expect(result.sign).toBe(1);
    expect(fpIsNaN(result)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Absolute value
// ---------------------------------------------------------------------------

describe("fpAbs", () => {
  it("abs positive", () => expect(fpAbs(floatToBits(3.14, FP32)).sign).toBe(0));
  it("abs negative", () => expect(fpAbs(floatToBits(-3.14, FP32)).sign).toBe(0));
  it("abs -0", () => expect(fpAbs(floatToBits(-0.0, FP32)).sign).toBe(0));
  it("abs nan", () => {
    const result = fpAbs(floatToBits(NaN, FP32));
    expect(result.sign).toBe(0);
    expect(fpIsNaN(result)).toBe(true);
  });
  it("abs -inf", () => {
    const result = fpAbs(floatToBits(-Infinity, FP32));
    expect(result.sign).toBe(0);
    expect(isInf(result)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

describe("fpCompare", () => {
  it("equal", () => expect(fpCompare(floatToBits(1.0, FP32), floatToBits(1.0, FP32))).toBe(0));
  it("less than", () => expect(fpCompare(floatToBits(1.0, FP32), floatToBits(2.0, FP32))).toBe(-1));
  it("greater than", () => expect(fpCompare(floatToBits(2.0, FP32), floatToBits(1.0, FP32))).toBe(1));
  it("neg < pos", () => expect(fpCompare(floatToBits(-1.0, FP32), floatToBits(1.0, FP32))).toBe(-1));
  it("pos > neg", () => expect(fpCompare(floatToBits(1.0, FP32), floatToBits(-1.0, FP32))).toBe(1));
  it("neg compare", () => expect(fpCompare(floatToBits(-3.0, FP32), floatToBits(-1.0, FP32))).toBe(-1));
  it("neg compare reversed", () => expect(fpCompare(floatToBits(-1.0, FP32), floatToBits(-3.0, FP32))).toBe(1));
  it("zeros equal", () => expect(fpCompare(floatToBits(0.0, FP32), floatToBits(-0.0, FP32))).toBe(0));
  it("nan unordered", () => expect(fpCompare(floatToBits(NaN, FP32), floatToBits(1.0, FP32))).toBe(0));
  it("nan vs nan", () => expect(fpCompare(floatToBits(NaN, FP32), floatToBits(NaN, FP32))).toBe(0));
  it("inf compare", () => expect(fpCompare(floatToBits(Infinity, FP32), floatToBits(1e38, FP32))).toBe(1));
  it("-inf compare", () => expect(fpCompare(floatToBits(-Infinity, FP32), floatToBits(-1e38, FP32))).toBe(-1));
  it("same exp different mant", () => expect(fpCompare(floatToBits(1.5, FP32), floatToBits(1.25, FP32))).toBe(1));
  it("zero vs positive", () => expect(fpCompare(floatToBits(0.0, FP32), floatToBits(1.0, FP32))).toBe(-1));
  it("zero vs negative", () => expect(fpCompare(floatToBits(0.0, FP32), floatToBits(-1.0, FP32))).toBe(1));
  it("negative same exp", () => expect(fpCompare(floatToBits(-1.5, FP32), floatToBits(-1.25, FP32))).toBe(-1));
  it("-0 vs positive", () => expect(fpCompare(floatToBits(-0.0, FP32), floatToBits(1.0, FP32))).toBe(-1));
  it("-0 vs negative", () => expect(fpCompare(floatToBits(-0.0, FP32), floatToBits(-1.0, FP32))).toBe(1));
  it("neg same mant different exp", () => expect(fpCompare(floatToBits(-10.0, FP32), floatToBits(-1.0, FP32))).toBe(-1));
  it("neg same mant different exp reversed", () => expect(fpCompare(floatToBits(-1.0, FP32), floatToBits(-10.0, FP32))).toBe(1));
});
