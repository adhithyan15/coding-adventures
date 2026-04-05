/**
 * Tests for f64 numeric instruction handlers.
 */

import { describe, it, expect } from "vitest";
import { makeVm, runUnary, runBinary, runInstructions, f64 } from "./helpers.js";
import { registerNumericF64 } from "../src/instructions/numeric_f64.js";

describe("numeric_f64", () => {
  const vm = makeVm(registerNumericF64);

  describe("f64.const (0x44)", () => {
    it("pushes an f64 constant", () => {
      runInstructions(vm, [{ opcode: 0x44, operand: Math.PI }]);
      const result = vm.peekTyped();
      expect(result.type).toBe(0x7c);
      expect(result.value).toBe(Math.PI);
    });
  });

  describe("f64.add (0xA0)", () => {
    it("adds two f64 values", () => {
      expect(runBinary(vm, 0xa0, f64(1.5), f64(2.5)).value).toBe(4.0);
    });
  });

  describe("f64.nearest (0x9E) --- banker's rounding", () => {
    it("rounds 0.5 to 0 (even)", () => {
      expect(runUnary(vm, 0x9e, f64(0.5)).value).toBe(0);
    });

    it("rounds 1.5 to 2 (even)", () => {
      expect(runUnary(vm, 0x9e, f64(1.5)).value).toBe(2);
    });

    it("rounds 2.5 to 2 (even)", () => {
      expect(runUnary(vm, 0x9e, f64(2.5)).value).toBe(2);
    });

    it("rounds 1.7 to 2", () => {
      expect(runUnary(vm, 0x9e, f64(1.7)).value).toBe(2);
    });
  });

  describe("f64.sub (0xA1)", () => {
    it("subtracts two f64 values", () => {
      expect(runBinary(vm, 0xa1, f64(10.5), f64(3.5)).value).toBe(7.0);
    });
  });

  describe("f64.mul (0xA2)", () => {
    it("multiplies two f64 values", () => {
      expect(runBinary(vm, 0xa2, f64(3.0), f64(4.0)).value).toBe(12.0);
    });

    it("handles multiplication by zero", () => {
      expect(runBinary(vm, 0xa2, f64(42.0), f64(0.0)).value).toBe(0.0);
    });
  });

  describe("f64.div (0xA3)", () => {
    it("divides two f64 values", () => {
      expect(runBinary(vm, 0xa3, f64(10.0), f64(4.0)).value).toBe(2.5);
    });

    it("returns Infinity on division by zero", () => {
      expect(runBinary(vm, 0xa3, f64(1.0), f64(0.0)).value).toBe(Infinity);
    });

    it("returns NaN for 0/0", () => {
      expect(runBinary(vm, 0xa3, f64(0.0), f64(0.0)).value).toBe(NaN);
    });
  });

  describe("f64.abs (0x99)", () => {
    it("returns absolute value", () => {
      expect(runUnary(vm, 0x99, f64(-5.5)).value).toBe(5.5);
      expect(runUnary(vm, 0x99, f64(5.5)).value).toBe(5.5);
    });
  });

  describe("f64.neg (0x9A)", () => {
    it("negates a positive value", () => {
      expect(runUnary(vm, 0x9a, f64(3.14)).value).toBe(-3.14);
    });

    it("negates a negative value", () => {
      expect(runUnary(vm, 0x9a, f64(-3.14)).value).toBe(3.14);
    });

    it("negates zero to negative zero", () => {
      expect(Object.is(runUnary(vm, 0x9a, f64(0.0)).value, -0)).toBe(true);
    });
  });

  describe("f64.ceil (0x9B)", () => {
    it("rounds up", () => {
      expect(runUnary(vm, 0x9b, f64(1.1)).value).toBe(2);
      expect(runUnary(vm, 0x9b, f64(-1.1)).value).toBe(-1);
    });
  });

  describe("f64.floor (0x9C)", () => {
    it("rounds down", () => {
      expect(runUnary(vm, 0x9c, f64(1.9)).value).toBe(1);
      expect(runUnary(vm, 0x9c, f64(-1.1)).value).toBe(-2);
    });
  });

  describe("f64.trunc (0x9D)", () => {
    it("truncates toward zero", () => {
      expect(runUnary(vm, 0x9d, f64(1.9)).value).toBe(1);
      expect(runUnary(vm, 0x9d, f64(-1.9)).value).toBe(-1);
    });
  });

  describe("f64.sqrt (0x9F)", () => {
    it("computes square root", () => {
      expect(runUnary(vm, 0x9f, f64(4.0)).value).toBe(2.0);
      expect(runUnary(vm, 0x9f, f64(2.0)).value).toBeCloseTo(Math.SQRT2, 10);
    });

    it("returns NaN for negative input", () => {
      expect(runUnary(vm, 0x9f, f64(-1.0)).value).toBeNaN();
    });
  });

  describe("f64.min (0xA4)", () => {
    it("returns the smaller value", () => {
      expect(runBinary(vm, 0xa4, f64(3.0), f64(5.0)).value).toBe(3.0);
    });

    it("returns NaN if either operand is NaN", () => {
      expect(runBinary(vm, 0xa4, f64(NaN), f64(5.0)).value).toBeNaN();
      expect(runBinary(vm, 0xa4, f64(5.0), f64(NaN)).value).toBeNaN();
    });

    it("handles negative zero: min(-0, +0) = -0", () => {
      const result = runBinary(vm, 0xa4, f64(-0.0), f64(0.0));
      expect(Object.is(result.value, -0)).toBe(true);
    });
  });

  describe("f64.max (0xA5)", () => {
    it("returns the larger value", () => {
      expect(runBinary(vm, 0xa5, f64(3.0), f64(5.0)).value).toBe(5.0);
    });

    it("returns NaN if either operand is NaN", () => {
      expect(runBinary(vm, 0xa5, f64(NaN), f64(5.0)).value).toBeNaN();
    });

    it("handles negative zero: max(-0, +0) = +0", () => {
      const result = runBinary(vm, 0xa5, f64(-0.0), f64(0.0));
      expect(Object.is(result.value, 0)).toBe(true);
      expect(Object.is(result.value, -0)).toBe(false);
    });
  });

  describe("f64.copysign (0xA6)", () => {
    it("copies sign from second operand", () => {
      expect(runBinary(vm, 0xa6, f64(5.0), f64(-1.0)).value).toBe(-5.0);
      expect(runBinary(vm, 0xa6, f64(-5.0), f64(1.0)).value).toBe(5.0);
    });

    it("preserves magnitude", () => {
      expect(runBinary(vm, 0xa6, f64(3.14), f64(-0.0)).value).toBe(-3.14);
    });
  });

  describe("f64 comparisons (return i32)", () => {
    it("f64.eq (0x61)", () => {
      expect(runBinary(vm, 0x61, f64(1.0), f64(1.0)).value).toBe(1);
      expect(runBinary(vm, 0x61, f64(1.0), f64(2.0)).value).toBe(0);
      // NaN is not equal to itself
      expect(runBinary(vm, 0x61, f64(NaN), f64(NaN)).value).toBe(0);
    });

    it("f64.ne (0x62)", () => {
      expect(runBinary(vm, 0x62, f64(1.0), f64(2.0)).value).toBe(1);
      expect(runBinary(vm, 0x62, f64(1.0), f64(1.0)).value).toBe(0);
      // NaN != NaN is true
      expect(runBinary(vm, 0x62, f64(NaN), f64(NaN)).value).toBe(1);
    });

    it("f64.lt (0x63)", () => {
      expect(runBinary(vm, 0x63, f64(1.0), f64(2.0)).value).toBe(1);
      expect(runBinary(vm, 0x63, f64(2.0), f64(1.0)).value).toBe(0);
    });

    it("f64.gt (0x64)", () => {
      expect(runBinary(vm, 0x64, f64(2.0), f64(1.0)).value).toBe(1);
      expect(runBinary(vm, 0x64, f64(1.0), f64(2.0)).value).toBe(0);
    });

    it("f64.le (0x65)", () => {
      expect(runBinary(vm, 0x65, f64(1.0), f64(2.0)).value).toBe(1);
      expect(runBinary(vm, 0x65, f64(1.0), f64(1.0)).value).toBe(1);
      expect(runBinary(vm, 0x65, f64(2.0), f64(1.0)).value).toBe(0);
    });

    it("f64.ge (0x66)", () => {
      expect(runBinary(vm, 0x66, f64(2.0), f64(1.0)).value).toBe(1);
      expect(runBinary(vm, 0x66, f64(1.0), f64(1.0)).value).toBe(1);
      expect(runBinary(vm, 0x66, f64(1.0), f64(2.0)).value).toBe(0);
    });
  });
});
