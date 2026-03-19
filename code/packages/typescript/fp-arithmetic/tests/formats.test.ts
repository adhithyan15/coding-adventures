/**
 * Tests for formats.ts -- FloatFormat and FloatBits data structures.
 */
import { describe, it, expect } from "vitest";
import { FP32, FP16, BF16, type FloatBits, type FloatFormat } from "../src/formats.js";

describe("FloatFormat", () => {
  it("FP32 constants", () => {
    expect(FP32.name).toBe("fp32");
    expect(FP32.totalBits).toBe(32);
    expect(FP32.exponentBits).toBe(8);
    expect(FP32.mantissaBits).toBe(23);
    expect(FP32.bias).toBe(127);
  });

  it("FP16 constants", () => {
    expect(FP16.name).toBe("fp16");
    expect(FP16.totalBits).toBe(16);
    expect(FP16.exponentBits).toBe(5);
    expect(FP16.mantissaBits).toBe(10);
    expect(FP16.bias).toBe(15);
  });

  it("BF16 constants", () => {
    expect(BF16.name).toBe("bf16");
    expect(BF16.totalBits).toBe(16);
    expect(BF16.exponentBits).toBe(8);
    expect(BF16.mantissaBits).toBe(7);
    expect(BF16.bias).toBe(127);
  });

  it("FP32 bit counts add up", () => {
    expect(1 + FP32.exponentBits + FP32.mantissaBits).toBe(FP32.totalBits);
  });

  it("FP16 bit counts add up", () => {
    expect(1 + FP16.exponentBits + FP16.mantissaBits).toBe(FP16.totalBits);
  });

  it("BF16 bit counts add up", () => {
    expect(1 + BF16.exponentBits + BF16.mantissaBits).toBe(BF16.totalBits);
  });

  it("frozen (immutable)", () => {
    expect(() => { (FP32 as any).bias = 42; }).toThrow();
  });

  it("custom format", () => {
    const custom: FloatFormat = { name: "fp8", totalBits: 8, exponentBits: 4, mantissaBits: 3, bias: 7 };
    expect(custom.totalBits).toBe(8);
    expect(custom.name).toBe("fp8");
  });

  it("equality by value", () => {
    const fp32Copy: FloatFormat = { name: "fp32", totalBits: 32, exponentBits: 8, mantissaBits: 23, bias: 127 };
    expect(fp32Copy.name).toBe(FP32.name);
    expect(fp32Copy.bias).toBe(FP32.bias);
  });

  it("inequality", () => {
    expect(FP32.name).not.toBe(FP16.name);
    expect(FP16.name).not.toBe(BF16.name);
  });

  it("BF16 same exponent as FP32", () => {
    expect(BF16.exponentBits).toBe(FP32.exponentBits);
    expect(BF16.bias).toBe(FP32.bias);
  });
});

describe("FloatBits", () => {
  it("create positive one", () => {
    const bits: FloatBits = {
      sign: 0,
      exponent: [0, 1, 1, 1, 1, 1, 1, 1],
      mantissa: new Array(23).fill(0),
      fmt: FP32,
    };
    expect(bits.sign).toBe(0);
    expect(bits.exponent).toEqual([0, 1, 1, 1, 1, 1, 1, 1]);
    expect(bits.mantissa.length).toBe(23);
  });

  it("create negative one", () => {
    const bits: FloatBits = {
      sign: 1,
      exponent: [0, 1, 1, 1, 1, 1, 1, 1],
      mantissa: new Array(23).fill(0),
      fmt: FP32,
    };
    expect(bits.sign).toBe(1);
  });

  it("format reference", () => {
    const bits: FloatBits = {
      sign: 0, exponent: new Array(5).fill(0), mantissa: new Array(10).fill(0), fmt: FP16,
    };
    expect(bits.fmt).toBe(FP16);
  });

  it("BF16 FloatBits", () => {
    const bits: FloatBits = {
      sign: 0, exponent: new Array(8).fill(0), mantissa: new Array(7).fill(0), fmt: BF16,
    };
    expect(bits.fmt).toBe(BF16);
    expect(bits.exponent.length).toBe(8);
    expect(bits.mantissa.length).toBe(7);
  });

  it("create zero", () => {
    const bits: FloatBits = {
      sign: 0, exponent: new Array(8).fill(0), mantissa: new Array(23).fill(0), fmt: FP32,
    };
    expect(bits.exponent.every(b => b === 0)).toBe(true);
    expect(bits.mantissa.every(b => b === 0)).toBe(true);
  });

  it("create inf", () => {
    const bits: FloatBits = {
      sign: 0, exponent: new Array(8).fill(1), mantissa: new Array(23).fill(0), fmt: FP32,
    };
    expect(bits.exponent.every(b => b === 1)).toBe(true);
    expect(bits.mantissa.every(b => b === 0)).toBe(true);
  });

  it("create nan", () => {
    const bits: FloatBits = {
      sign: 0, exponent: new Array(8).fill(1), mantissa: [1, ...new Array(22).fill(0)], fmt: FP32,
    };
    expect(bits.mantissa[0]).toBe(1);
  });
});
