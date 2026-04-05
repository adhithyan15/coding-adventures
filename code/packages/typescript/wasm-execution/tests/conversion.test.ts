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
});
