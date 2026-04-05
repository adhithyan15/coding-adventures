/**
 * Tests for type conversion instruction handlers.
 */

import { describe, it, expect } from "vitest";
import { makeVm, runUnary, i32, i64, f32, f64 } from "./helpers.js";
import { registerConversion } from "../src/instructions/conversion.js";
import { TrapError } from "../src/host_interface.js";

describe("conversion", () => {
  const vm = makeVm(registerConversion);

  describe("i32.wrap_i64 (0xA7)", () => {
    it("truncates to low 32 bits", () => {
      const result = runUnary(vm, 0xa7, i64(0x100000042n));
      expect(result.value).toBe(0x42);
      expect(result.type).toBe(0x7f); /* i32 */
    });

    it("preserves sign for small negative values", () => {
      expect(runUnary(vm, 0xa7, i64(-1n)).value).toBe(-1);
    });
  });

  describe("i64.extend_i32_s (0xAC)", () => {
    it("sign-extends positive", () => {
      const result = runUnary(vm, 0xac, i32(42));
      expect(result.value).toBe(42n);
      expect(result.type).toBe(0x7e); /* i64 */
    });

    it("sign-extends negative", () => {
      expect(runUnary(vm, 0xac, i32(-1)).value).toBe(-1n);
    });
  });

  describe("i64.extend_i32_u (0xAD)", () => {
    it("zero-extends", () => {
      expect(runUnary(vm, 0xad, i32(-1)).value).toBe(4294967295n);
    });
  });

  describe("i32.trunc_f64_s (0xAA)", () => {
    it("truncates toward zero", () => {
      expect(runUnary(vm, 0xaa, f64(3.7)).value).toBe(3);
      expect(runUnary(vm, 0xaa, f64(-3.7)).value).toBe(-3);
    });

    it("traps on NaN", () => {
      expect(() => runUnary(vm, 0xaa, f64(NaN))).toThrow(TrapError);
    });

    it("traps on out-of-range", () => {
      expect(() => runUnary(vm, 0xaa, f64(3e10))).toThrow(TrapError);
    });
  });

  describe("i32.trunc_f32_s (0xA8)", () => {
    it("truncates toward zero", () => {
      expect(runUnary(vm, 0xa8, f32(2.9)).value).toBe(2);
    });

    it("traps on NaN", () => {
      expect(() => runUnary(vm, 0xa8, f32(NaN))).toThrow(TrapError);
    });
  });

  describe("reinterpret instructions", () => {
    it("i32.reinterpret_f32 (0xBC)", () => {
      /* 1.0f has bit pattern 0x3F800000 = 1065353216 */
      const result = runUnary(vm, 0xbc, f32(1.0));
      expect(result.type).toBe(0x7f); /* i32 */
      expect(result.value).toBe(1065353216);
    });

    it("f32.reinterpret_i32 (0xBE)", () => {
      const result = runUnary(vm, 0xbe, i32(1065353216));
      expect(result.type).toBe(0x7d); /* f32 */
      expect(result.value).toBe(1.0);
    });

    it("round-trip: f32 -> i32 -> f32", () => {
      const original = Math.fround(3.14);
      const asInt = runUnary(vm, 0xbc, f32(original));
      const backToFloat = runUnary(vm, 0xbe, i32(asInt.value as number));
      expect(backToFloat.value).toBe(original);
    });

    it("i64.reinterpret_f64 (0xBD)", () => {
      const result = runUnary(vm, 0xbd, f64(1.0));
      expect(result.type).toBe(0x7e); /* i64 */
      /* 1.0 as f64 has bit pattern 0x3FF0000000000000 */
      expect(result.value).toBe(4607182418800017408n);
    });

    it("f64.reinterpret_i64 (0xBF)", () => {
      const result = runUnary(vm, 0xbf, i64(4607182418800017408n));
      expect(result.type).toBe(0x7c); /* f64 */
      expect(result.value).toBe(1.0);
    });
  });

  describe("f32.convert_i32_s (0xB2)", () => {
    it("converts signed i32 to f32", () => {
      const result = runUnary(vm, 0xb2, i32(42));
      expect(result.type).toBe(0x7d); /* f32 */
      expect(result.value).toBe(Math.fround(42));
    });

    it("converts negative i32", () => {
      expect(runUnary(vm, 0xb2, i32(-100)).value).toBe(Math.fround(-100));
    });
  });

  describe("f32.convert_i32_u (0xB3)", () => {
    it("converts unsigned i32 to f32", () => {
      /* -1 as i32 is 0xFFFFFFFF = 4294967295 unsigned */
      const result = runUnary(vm, 0xb3, i32(-1));
      expect(result.type).toBe(0x7d);
      expect(result.value).toBe(Math.fround(4294967295));
    });
  });

  describe("f64.convert_i32_s (0xB7)", () => {
    it("converts signed i32 to f64", () => {
      const result = runUnary(vm, 0xb7, i32(-42));
      expect(result.type).toBe(0x7c); /* f64 */
      expect(result.value).toBe(-42);
    });
  });

  describe("f64.convert_i64_s (0xB9)", () => {
    it("converts signed i64 to f64", () => {
      const result = runUnary(vm, 0xb9, i64(1000000n));
      expect(result.type).toBe(0x7c);
      expect(result.value).toBe(1000000);
    });

    it("may lose precision for very large i64", () => {
      /* 2^53 + 1 cannot be represented exactly as f64 */
      const result = runUnary(vm, 0xb9, i64(9007199254740993n));
      expect(result.type).toBe(0x7c);
      expect(typeof result.value).toBe("number");
    });
  });

  describe("i64.trunc_f32_s (0xAE)", () => {
    it("truncates f32 toward zero to i64", () => {
      const result = runUnary(vm, 0xae, f32(3.7));
      expect(result.type).toBe(0x7e); /* i64 */
      expect(result.value).toBe(3n);
    });

    it("traps on NaN", () => {
      expect(() => runUnary(vm, 0xae, f32(NaN))).toThrow(TrapError);
    });

    it("traps on Infinity", () => {
      expect(() => runUnary(vm, 0xae, f32(Infinity))).toThrow(TrapError);
    });
  });

  describe("i64.trunc_f64_s (0xB0)", () => {
    it("truncates f64 toward zero to i64", () => {
      expect(runUnary(vm, 0xb0, f64(-7.9)).value).toBe(-7n);
    });

    it("traps on NaN", () => {
      expect(() => runUnary(vm, 0xb0, f64(NaN))).toThrow(TrapError);
    });
  });

  describe("f32.demote_f64 (0xB6)", () => {
    it("narrows f64 to f32 precision", () => {
      const result = runUnary(vm, 0xb6, f64(3.14));
      expect(result.type).toBe(0x7d); /* f32 */
      expect(result.value).toBe(Math.fround(3.14));
    });

    it("handles values outside f32 range", () => {
      const result = runUnary(vm, 0xb6, f64(1e40));
      expect(result.value).toBe(Infinity);
    });
  });

  describe("f64.promote_f32 (0xBB)", () => {
    it("widens f32 to f64", () => {
      const result = runUnary(vm, 0xbb, f32(1.5));
      expect(result.type).toBe(0x7c); /* f64 */
      expect(result.value).toBe(1.5);
    });

    it("preserves the f32-rounded value", () => {
      /* Math.fround(1.1) gives a slightly different value */
      const f32val = Math.fround(1.1);
      const result = runUnary(vm, 0xbb, f32(1.1));
      expect(result.value).toBe(f32val);
    });
  });

  describe("i32.trunc_f32_u (0xA9)", () => {
    it("truncates positive f32 to unsigned i32", () => {
      expect(runUnary(vm, 0xa9, f32(3.7)).value).toBe(3);
    });

    it("traps on negative values", () => {
      expect(() => runUnary(vm, 0xa9, f32(-1.0))).toThrow(TrapError);
    });
  });

  describe("i32.trunc_f64_u (0xAB)", () => {
    it("truncates positive f64 to unsigned i32", () => {
      expect(runUnary(vm, 0xab, f64(3.7)).value).toBe(3);
    });

    it("traps on values exceeding uint32 range", () => {
      expect(() => runUnary(vm, 0xab, f64(5e9))).toThrow(TrapError);
    });
  });

  describe("i64.trunc_f32_u (0xAF)", () => {
    it("truncates positive f32 to unsigned i64", () => {
      const result = runUnary(vm, 0xaf, f32(42.5));
      expect(result.type).toBe(0x7e);
      expect(result.value).toBe(42n);
    });

    it("traps on negative", () => {
      expect(() => runUnary(vm, 0xaf, f32(-1.0))).toThrow(TrapError);
    });
  });

  describe("i64.trunc_f64_u (0xB1)", () => {
    it("truncates positive f64 to unsigned i64", () => {
      expect(runUnary(vm, 0xb1, f64(100.9)).value).toBe(100n);
    });

    it("traps on negative", () => {
      expect(() => runUnary(vm, 0xb1, f64(-1.0))).toThrow(TrapError);
    });
  });

  describe("f32.convert_i64_s (0xB4)", () => {
    it("converts signed i64 to f32", () => {
      const result = runUnary(vm, 0xb4, i64(42n));
      expect(result.type).toBe(0x7d);
      expect(result.value).toBe(Math.fround(42));
    });
  });

  describe("f32.convert_i64_u (0xB5)", () => {
    it("converts unsigned i64 to f32", () => {
      const result = runUnary(vm, 0xb5, i64(42n));
      expect(result.type).toBe(0x7d);
      expect(result.value).toBe(Math.fround(42));
    });
  });

  describe("f64.convert_i32_u (0xB8)", () => {
    it("converts unsigned i32 to f64", () => {
      /* -1 as i32 = 4294967295 unsigned */
      const result = runUnary(vm, 0xb8, i32(-1));
      expect(result.type).toBe(0x7c);
      expect(result.value).toBe(4294967295);
    });
  });

  describe("f64.convert_i64_u (0xBA)", () => {
    it("converts unsigned i64 to f64", () => {
      const result = runUnary(vm, 0xba, i64(1000n));
      expect(result.type).toBe(0x7c);
      expect(result.value).toBe(1000);
    });
  });

  describe("reinterpret round-trips (additional)", () => {
    it("i32.reinterpret_f32 preserves NaN bits", () => {
      const result = runUnary(vm, 0xbc, f32(NaN));
      expect(result.type).toBe(0x7f);
      /* Standard NaN bit pattern for f32: 0x7FC00000 = 2143289344 */
      /* Just verify it's a valid NaN pattern (sign=0, exp=all 1s, frac != 0) */
      const bits = (result.value as number) >>> 0; // unsigned
      expect(bits & 0x7F800000).toBe(0x7F800000); // exponent all 1s
      expect(bits & 0x007FFFFF).not.toBe(0);       // fraction non-zero (NaN)
    });

    it("f32.reinterpret_i32 with zero bits gives 0.0", () => {
      const result = runUnary(vm, 0xbe, i32(0));
      expect(result.value).toBe(0.0);
    });

    it("i64.reinterpret_f64 round-trip with negative value", () => {
      const asInt = runUnary(vm, 0xbd, f64(-1.0));
      const backToFloat = runUnary(vm, 0xbf, i64(asInt.value as bigint));
      expect(backToFloat.value).toBe(-1.0);
    });

    it("f64.reinterpret_i64 with zero bits gives 0.0", () => {
      const result = runUnary(vm, 0xbf, i64(0n));
      expect(result.value).toBe(0.0);
    });
  });
});
