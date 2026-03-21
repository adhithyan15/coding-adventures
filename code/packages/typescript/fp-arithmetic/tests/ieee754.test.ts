/**
 * Tests for ieee754.ts -- encoding, decoding, and special value detection.
 */
import { describe, it, expect } from "vitest";
import { FP32, FP16, BF16, type FloatBits, type FloatFormat } from "../src/formats.js";
import {
  intToBitsMsb,
  bitsMsbToInt,
  floatToBits,
  bitsToFloat,
  isNaN as fpIsNaN,
  isInf,
  isZero,
  isDenormalized,
  allOnes,
  allZeros,
} from "../src/ieee754.js";

// ---------------------------------------------------------------------------
// Tests for internal helpers
// ---------------------------------------------------------------------------

describe("intToBitsMsb / bitsMsbToInt", () => {
  it("zero", () => {
    expect(intToBitsMsb(0, 8)).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
  });

  it("one", () => {
    expect(intToBitsMsb(1, 8)).toEqual([0, 0, 0, 0, 0, 0, 0, 1]);
  });

  it("max byte", () => {
    expect(intToBitsMsb(255, 8)).toEqual([1, 1, 1, 1, 1, 1, 1, 1]);
  });

  it("five", () => {
    expect(intToBitsMsb(5, 8)).toEqual([0, 0, 0, 0, 0, 1, 0, 1]);
  });

  it("width 1", () => {
    expect(intToBitsMsb(0, 1)).toEqual([0]);
    expect(intToBitsMsb(1, 1)).toEqual([1]);
  });

  it("width 16 roundtrip", () => {
    const bits = intToBitsMsb(0xabcd, 16);
    expect(bits.length).toBe(16);
    expect(bitsMsbToInt(bits)).toBe(0xabcd);
  });

  it("roundtrip 8-bit", () => {
    for (const val of [0, 1, 42, 127, 255]) {
      expect(bitsMsbToInt(intToBitsMsb(val, 8))).toBe(val);
    }
  });

  it("roundtrip wide", () => {
    for (const val of [0, 1, 1023, 65535, (1 << 23) - 1]) {
      expect(bitsMsbToInt(intToBitsMsb(val, 23))).toBe(val);
    }
  });

  it("bitsMsbToInt examples", () => {
    expect(bitsMsbToInt([1, 0, 1])).toBe(5);
    expect(bitsMsbToInt([0])).toBe(0);
    expect(bitsMsbToInt([1])).toBe(1);
  });

  it("bitsMsbToInt empty", () => {
    expect(bitsMsbToInt([])).toBe(0);
  });
});

describe("allOnes / allZeros", () => {
  it("allOnes true", () => expect(allOnes([1, 1, 1, 1])).toBe(true));
  it("allOnes false", () => expect(allOnes([1, 0, 1, 1])).toBe(false));
  it("allOnes single", () => {
    expect(allOnes([1])).toBe(true);
    expect(allOnes([0])).toBe(false);
  });
  it("allZeros true", () => expect(allZeros([0, 0, 0, 0])).toBe(true));
  it("allZeros false", () => expect(allZeros([0, 0, 1, 0])).toBe(false));
  it("allZeros single", () => {
    expect(allZeros([0])).toBe(true);
    expect(allZeros([1])).toBe(false);
  });
  it("allOnes 8-bit", () => {
    expect(allOnes(new Array(8).fill(1))).toBe(true);
    expect(allOnes([1, 1, 1, 1, 1, 1, 1, 0])).toBe(false);
  });
  it("allZeros 8-bit", () => {
    expect(allZeros(new Array(8).fill(0))).toBe(true);
    expect(allZeros([0, 0, 0, 0, 0, 0, 0, 1])).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Tests for floatToBits -- FP32
// ---------------------------------------------------------------------------

function verifyAgainstDataView(value: number): void {
  const bits = floatToBits(value, FP32);
  const intVal = ((bits.sign << 31) | (bitsMsbToInt(bits.exponent) << 23) | bitsMsbToInt(bits.mantissa)) >>> 0;
  const buf = new ArrayBuffer(4);
  const view = new DataView(buf);
  view.setFloat32(0, value);
  const expected = view.getUint32(0);
  expect(intVal).toBe(expected);
}

describe("floatToBits FP32", () => {
  it("positive one", () => {
    verifyAgainstDataView(1.0);
    const bits = floatToBits(1.0, FP32);
    expect(bits.sign).toBe(0);
    expect(bits.exponent).toEqual([0, 1, 1, 1, 1, 1, 1, 1]);
    expect(bits.mantissa).toEqual(new Array(23).fill(0));
  });

  it("negative one", () => {
    verifyAgainstDataView(-1.0);
    expect(floatToBits(-1.0, FP32).sign).toBe(1);
  });

  it("two", () => {
    verifyAgainstDataView(2.0);
    expect(floatToBits(2.0, FP32).exponent).toEqual([1, 0, 0, 0, 0, 0, 0, 0]);
  });

  it("pi", () => verifyAgainstDataView(3.14));
  it("zero", () => {
    verifyAgainstDataView(0.0);
    const bits = floatToBits(0.0, FP32);
    expect(bits.sign).toBe(0);
    expect(bits.exponent).toEqual(new Array(8).fill(0));
    expect(bits.mantissa).toEqual(new Array(23).fill(0));
  });

  it("negative zero", () => {
    const bits = floatToBits(-0.0, FP32);
    expect(bits.sign).toBe(1);
    expect(bits.exponent).toEqual(new Array(8).fill(0));
  });

  it("half", () => verifyAgainstDataView(0.5));

  it("many values", () => {
    for (const v of [0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.14, 100.0, 1000.0, -0.1, -42.0]) {
      verifyAgainstDataView(v);
    }
  });

  it("default format is FP32", () => {
    expect(floatToBits(1.0).fmt).toBe(FP32);
  });
});

// ---------------------------------------------------------------------------
// Tests for floatToBits -- special values
// ---------------------------------------------------------------------------

describe("floatToBits special values", () => {
  it("NaN FP32", () => {
    const bits = floatToBits(NaN, FP32);
    expect(bits.exponent).toEqual(new Array(8).fill(1));
    expect(bits.mantissa[0]).toBe(1);
  });

  it("+Inf FP32", () => {
    const bits = floatToBits(Infinity, FP32);
    expect(bits.sign).toBe(0);
    expect(bits.exponent).toEqual(new Array(8).fill(1));
    expect(bits.mantissa).toEqual(new Array(23).fill(0));
  });

  it("-Inf FP32", () => {
    const bits = floatToBits(-Infinity, FP32);
    expect(bits.sign).toBe(1);
    expect(bits.exponent).toEqual(new Array(8).fill(1));
  });

  it("NaN FP16", () => {
    const bits = floatToBits(NaN, FP16);
    expect(bits.exponent).toEqual(new Array(5).fill(1));
    expect(bits.mantissa[0]).toBe(1);
  });

  it("NaN BF16", () => {
    const bits = floatToBits(NaN, BF16);
    expect(bits.exponent).toEqual(new Array(8).fill(1));
    expect(bits.mantissa[0]).toBe(1);
  });

  it("+Inf FP16", () => {
    const bits = floatToBits(Infinity, FP16);
    expect(bits.sign).toBe(0);
    expect(bits.exponent).toEqual(new Array(5).fill(1));
    expect(bits.mantissa).toEqual(new Array(10).fill(0));
  });

  it("-Inf FP16", () => {
    expect(floatToBits(-Infinity, FP16).sign).toBe(1);
  });

  it("+Inf BF16", () => {
    const bits = floatToBits(Infinity, BF16);
    expect(bits.sign).toBe(0);
    expect(bits.exponent).toEqual(new Array(8).fill(1));
    expect(bits.mantissa).toEqual(new Array(7).fill(0));
  });

  it("-Inf BF16", () => {
    expect(floatToBits(-Infinity, BF16).sign).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Tests for floatToBits -- FP16
// ---------------------------------------------------------------------------

describe("floatToBits FP16", () => {
  it("one", () => {
    const bits = floatToBits(1.0, FP16);
    expect(bits.sign).toBe(0);
    expect(bits.exponent).toEqual([0, 1, 1, 1, 1]);
    expect(bits.mantissa).toEqual(new Array(10).fill(0));
  });

  it("negative one", () => expect(floatToBits(-1.0, FP16).sign).toBe(1));

  it("zero", () => {
    const bits = floatToBits(0.0, FP16);
    expect(bits.exponent).toEqual(new Array(5).fill(0));
    expect(bits.mantissa).toEqual(new Array(10).fill(0));
  });

  it("negative zero", () => {
    const bits = floatToBits(-0.0, FP16);
    expect(bits.sign).toBe(1);
  });

  it("overflow to inf", () => {
    expect(isInf(floatToBits(100000.0, FP16))).toBe(true);
  });

  it("two", () => {
    expect(floatToBits(2.0, FP16).exponent).toEqual([1, 0, 0, 0, 0]);
  });

  it("half", () => {
    const bits = floatToBits(0.5, FP16);
    expect(bits.exponent).toEqual([0, 1, 1, 1, 0]);
    expect(bits.mantissa).toEqual(new Array(10).fill(0));
  });

  it("roundtrip simple", () => {
    for (const val of [1.0, -1.0, 2.0, 0.5, 0.25]) {
      expect(bitsToFloat(floatToBits(val, FP16))).toBe(val);
    }
  });

  it("fp16 max normal", () => {
    expect(bitsToFloat(floatToBits(65504.0, FP16))).toBe(65504.0);
  });

  it("fp16 underflow to denormal", () => {
    const bits = floatToBits(1e-7, FP16);
    expect(bitsMsbToInt(bits.exponent)).toBe(0);
  });

  it("fp16 underflow to zero", () => {
    expect(isZero(floatToBits(1e-20, FP16))).toBe(true);
  });

  it("fp16 rounding", () => {
    const result = bitsToFloat(floatToBits(3.14, FP16));
    expect(Math.abs(result - 3.14)).toBeLessThan(0.01);
  });
});

// ---------------------------------------------------------------------------
// Tests for floatToBits -- BF16
// ---------------------------------------------------------------------------

describe("floatToBits BF16", () => {
  it("one", () => {
    const bits = floatToBits(1.0, BF16);
    expect(bits.sign).toBe(0);
    expect(bits.exponent).toEqual([0, 1, 1, 1, 1, 1, 1, 1]);
    expect(bits.mantissa).toEqual(new Array(7).fill(0));
  });

  it("zero", () => {
    const bits = floatToBits(0.0, BF16);
    expect(bits.exponent).toEqual(new Array(8).fill(0));
    expect(bits.mantissa).toEqual(new Array(7).fill(0));
  });

  it("negative zero", () => expect(floatToBits(-0.0, BF16).sign).toBe(1));

  it("two roundtrip", () => expect(bitsToFloat(floatToBits(2.0, BF16))).toBe(2.0));

  it("bf16 large value", () => {
    const result = bitsToFloat(floatToBits(1e30, BF16));
    expect(Math.abs(result - 1e30) / 1e30).toBeLessThan(0.01);
  });

  it("bf16 rounding overflow", () => {
    const result = bitsToFloat(floatToBits(1.9921875, BF16));
    expect(Math.abs(result - 2.0)).toBeLessThan(0.02);
  });

  it("bf16 overflow to inf", () => {
    expect(isInf(floatToBits(Infinity, BF16))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Tests for bitsToFloat
// ---------------------------------------------------------------------------

describe("bitsToFloat", () => {
  it("roundtrip FP32", () => {
    for (const val of [0.0, 1.0, -1.0, 0.5, 2.0, 3.14, -42.0, 100.0]) {
      const bits = floatToBits(val, FP32);
      const result = bitsToFloat(bits);
      if (val === 0.0) {
        expect(result).toBe(0.0);
      } else {
        expect(Math.abs(result - val)).toBeLessThan(Math.abs(val) * 1e-6);
      }
    }
  });

  it("NaN roundtrip", () => {
    expect(Number.isNaN(bitsToFloat(floatToBits(NaN, FP32)))).toBe(true);
  });

  it("Inf roundtrip", () => {
    expect(bitsToFloat(floatToBits(Infinity, FP32))).toBe(Infinity);
  });

  it("-Inf roundtrip", () => {
    expect(bitsToFloat(floatToBits(-Infinity, FP32))).toBe(-Infinity);
  });

  it("negative zero roundtrip", () => {
    const result = bitsToFloat(floatToBits(-0.0, FP32));
    expect(result).toBe(-0);
    expect(1 / result).toBe(-Infinity); // distinguishes -0 from +0
  });

  it("FP16 roundtrip", () => {
    for (const val of [0.0, 1.0, -1.0, 0.5, 2.0, -0.25]) {
      expect(bitsToFloat(floatToBits(val, FP16))).toBe(val);
    }
  });

  it("BF16 roundtrip", () => {
    for (const val of [0.0, 1.0, -1.0, 0.5, 2.0]) {
      expect(bitsToFloat(floatToBits(val, BF16))).toBe(val);
    }
  });

  it("FP16 NaN roundtrip", () => {
    expect(Number.isNaN(bitsToFloat(floatToBits(NaN, FP16)))).toBe(true);
  });

  it("FP16 Inf roundtrip", () => {
    expect(bitsToFloat(floatToBits(Infinity, FP16))).toBe(Infinity);
  });

  it("FP16 -Inf roundtrip", () => {
    expect(bitsToFloat(floatToBits(-Infinity, FP16))).toBe(-Infinity);
  });

  it("BF16 NaN roundtrip", () => {
    expect(Number.isNaN(bitsToFloat(floatToBits(NaN, BF16)))).toBe(true);
  });

  it("BF16 Inf roundtrip", () => {
    expect(bitsToFloat(floatToBits(Infinity, BF16))).toBe(Infinity);
  });

  it("BF16 -Inf roundtrip", () => {
    expect(bitsToFloat(floatToBits(-Infinity, BF16))).toBe(-Infinity);
  });

  it("FP16 negative zero", () => {
    const result = bitsToFloat(floatToBits(-0.0, FP16));
    expect(result).toBe(-0);
    expect(1 / result).toBe(-Infinity);
  });

  it("BF16 negative zero", () => {
    const result = bitsToFloat(floatToBits(-0.0, BF16));
    expect(result).toBe(-0);
    expect(1 / result).toBe(-Infinity);
  });

  it("FP16 denormal decode", () => {
    const tiny: FloatBits = {
      sign: 0,
      exponent: new Array(5).fill(0),
      mantissa: [...new Array(9).fill(0), 1],
      fmt: FP16,
    };
    const val = bitsToFloat(tiny);
    expect(val).toBeGreaterThan(0);
    expect(val).toBeLessThan(1e-6);
  });

  it("BF16 denormal decode", () => {
    const tiny: FloatBits = {
      sign: 0,
      exponent: new Array(8).fill(0),
      mantissa: [...new Array(6).fill(0), 1],
      fmt: BF16,
    };
    expect(bitsToFloat(tiny)).toBeGreaterThan(0);
  });

  it("FP16 normal decode", () => {
    const bits: FloatBits = {
      sign: 0,
      exponent: [0, 1, 1, 1, 1],
      mantissa: [1, ...new Array(9).fill(0)],
      fmt: FP16,
    };
    expect(bitsToFloat(bits)).toBe(1.5);
  });

  it("BF16 normal decode", () => {
    const bits: FloatBits = {
      sign: 0,
      exponent: [0, 1, 1, 1, 1, 1, 1, 1],
      mantissa: [1, ...new Array(6).fill(0)],
      fmt: BF16,
    };
    expect(bitsToFloat(bits)).toBe(1.5);
  });

  it("negative FP16 value", () => {
    const bits: FloatBits = {
      sign: 1,
      exponent: [0, 1, 1, 1, 1],
      mantissa: [1, ...new Array(9).fill(0)],
      fmt: FP16,
    };
    expect(bitsToFloat(bits)).toBe(-1.5);
  });
});

// ---------------------------------------------------------------------------
// Tests for special value detection
// ---------------------------------------------------------------------------

describe("special value detection", () => {
  it("isNaN true", () => expect(fpIsNaN(floatToBits(NaN, FP32))).toBe(true));
  it("isNaN false for inf", () => expect(fpIsNaN(floatToBits(Infinity, FP32))).toBe(false));
  it("isNaN false for number", () => expect(fpIsNaN(floatToBits(1.0, FP32))).toBe(false));
  it("isNaN false for zero", () => expect(fpIsNaN(floatToBits(0.0, FP32))).toBe(false));

  it("isInf positive", () => expect(isInf(floatToBits(Infinity, FP32))).toBe(true));
  it("isInf negative", () => expect(isInf(floatToBits(-Infinity, FP32))).toBe(true));
  it("isInf false for nan", () => expect(isInf(floatToBits(NaN, FP32))).toBe(false));
  it("isInf false for number", () => expect(isInf(floatToBits(1.0, FP32))).toBe(false));
  it("isInf false for zero", () => expect(isInf(floatToBits(0.0, FP32))).toBe(false));

  it("isZero positive", () => expect(isZero(floatToBits(0.0, FP32))).toBe(true));
  it("isZero negative", () => expect(isZero(floatToBits(-0.0, FP32))).toBe(true));
  it("isZero false for number", () => expect(isZero(floatToBits(1.0, FP32))).toBe(false));

  it("isDenormalized", () => {
    const tiny: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [...new Array(22).fill(0), 1], fmt: FP32 };
    expect(isDenormalized(tiny)).toBe(true);
  });
  it("isDenormalized false for normal", () => expect(isDenormalized(floatToBits(1.0, FP32))).toBe(false));
  it("isDenormalized false for zero", () => expect(isDenormalized(floatToBits(0.0, FP32))).toBe(false));
  it("isDenormalized false for inf", () => expect(isDenormalized(floatToBits(Infinity, FP32))).toBe(false));

  it("special values FP16", () => {
    expect(fpIsNaN(floatToBits(NaN, FP16))).toBe(true);
    expect(isInf(floatToBits(Infinity, FP16))).toBe(true);
    expect(isZero(floatToBits(0.0, FP16))).toBe(true);
  });

  it("special values BF16", () => {
    expect(fpIsNaN(floatToBits(NaN, BF16))).toBe(true);
    expect(isInf(floatToBits(Infinity, BF16))).toBe(true);
    expect(isZero(floatToBits(0.0, BF16))).toBe(true);
  });

  it("FP16 denormalized", () => {
    const tiny: FloatBits = { sign: 0, exponent: new Array(5).fill(0), mantissa: [...new Array(9).fill(0), 1], fmt: FP16 };
    expect(isDenormalized(tiny)).toBe(true);
  });

  it("BF16 denormalized", () => {
    const tiny: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [...new Array(6).fill(0), 1], fmt: BF16 };
    expect(isDenormalized(tiny)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Tests for denormal encoding/decoding
// ---------------------------------------------------------------------------

describe("denormal encoding", () => {
  it("smallest FP32 denormal", () => {
    const tiny: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [...new Array(22).fill(0), 1], fmt: FP32 };
    const val = bitsToFloat(tiny);
    expect(val).toBeGreaterThan(0);
    expect(val).toBeLessThan(1e-44);
  });

  it("largest FP32 denormal", () => {
    const large: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: new Array(23).fill(1), fmt: FP32 };
    const val = bitsToFloat(large);
    expect(val).toBeGreaterThan(0);
    expect(val).toBeLessThan(1.18e-38);
  });

  it("denormal roundtrip", () => {
    const denorm: FloatBits = { sign: 0, exponent: new Array(8).fill(0), mantissa: [1, ...new Array(22).fill(0)], fmt: FP32 };
    const val = bitsToFloat(denorm);
    expect(val).toBeGreaterThan(0);
    const bits2 = floatToBits(val, FP32);
    expect(bitsMsbToInt(bits2.exponent)).toBe(0);
    expect(bits2.mantissa[0]).toBe(1);
  });

  it("negative denormal", () => {
    const denorm: FloatBits = { sign: 1, exponent: new Array(8).fill(0), mantissa: [1, ...new Array(22).fill(0)], fmt: FP32 };
    expect(bitsToFloat(denorm)).toBeLessThan(0);
  });
});

// ---------------------------------------------------------------------------
// FP16/BF16 edge cases
// ---------------------------------------------------------------------------

describe("FP16/BF16 edge cases", () => {
  it("fp16 just below overflow", () => {
    const bits = floatToBits(65504.0, FP16);
    expect(isInf(bits)).toBe(false);
    expect(bitsToFloat(bits)).toBe(65504.0);
  });

  it("fp16 just above overflow", () => {
    expect(isInf(floatToBits(65536.0, FP16))).toBe(true);
  });

  it("bf16 roundtrip various", () => {
    for (const val of [0.0, 1.0, -1.0, 0.5, 2.0, 128.0, -256.0]) {
      expect(bitsToFloat(floatToBits(val, BF16))).toBe(val);
    }
  });

  it("fp16 mantissa wider than fp32 path", () => {
    const wideFmt: FloatFormat = { name: "wide", totalBits: 40, exponentBits: 8, mantissaBits: 31, bias: 127 };
    const bits = floatToBits(1.0, wideFmt);
    expect(bits.fmt).toBe(wideFmt);
  });
});
