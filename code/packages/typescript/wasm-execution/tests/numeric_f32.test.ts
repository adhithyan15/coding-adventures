/**
 * Tests for f32 numeric instruction handlers.
 */

import { describe, it, expect } from "vitest";
import { makeVm, runUnary, runBinary, runInstructions, f32, i32 } from "./helpers.js";
import { registerNumericF32 } from "../src/instructions/numeric_f32.js";

describe("numeric_f32", () => {
  const vm = makeVm(registerNumericF32);

  describe("f32.const (0x43)", () => {
    it("pushes an f32 constant", () => {
      /* Use a value exactly representable in f32 (1.5 = 1 + 0.5, both
         powers of 2, so no precision loss). */
      runInstructions(vm, [{ opcode: 0x43, operand: 1.5 }]);
      const result = vm.peekTyped();
      expect(result.type).toBe(0x7d);
      expect(result.value).toBe(1.5);
    });
  });

  describe("f32.add (0x92)", () => {
    it("adds two f32 values", () => {
      const result = runBinary(vm, 0x92, f32(1.5), f32(2.5));
      expect(result.value).toBe(Math.fround(4.0));
    });
  });

  describe("f32.mul (0x94)", () => {
    it("multiplies with f32 precision", () => {
      const result = runBinary(vm, 0x94, f32(3.0), f32(2.0));
      expect(result.value).toBe(Math.fround(6.0));
    });
  });

  describe("f32.nearest (0x90) --- banker's rounding", () => {
    it("rounds 0.5 to 0 (round to even)", () => {
      expect(runUnary(vm, 0x90, f32(0.5)).value).toBe(0);
    });

    it("rounds 1.5 to 2 (round to even)", () => {
      expect(runUnary(vm, 0x90, f32(1.5)).value).toBe(2);
    });

    it("rounds 2.5 to 2 (round to even)", () => {
      expect(runUnary(vm, 0x90, f32(2.5)).value).toBe(2);
    });

    it("rounds 3.5 to 4 (round to even)", () => {
      expect(runUnary(vm, 0x90, f32(3.5)).value).toBe(4);
    });

    it("rounds 1.4 to 1 (standard round-down)", () => {
      expect(runUnary(vm, 0x90, f32(1.4)).value).toBe(Math.fround(1));
    });

    it("rounds 1.6 to 2 (standard round-up)", () => {
      expect(runUnary(vm, 0x90, f32(1.6)).value).toBe(Math.fround(2));
    });

    it("preserves NaN", () => {
      expect(runUnary(vm, 0x90, f32(NaN)).value).toBeNaN();
    });

    it("preserves infinity", () => {
      expect(runUnary(vm, 0x90, f32(Infinity)).value).toBe(Infinity);
    });
  });

  describe("f32.min (0x96)", () => {
    it("returns the smaller value", () => {
      expect(runBinary(vm, 0x96, f32(3.0), f32(5.0)).value).toBe(Math.fround(3.0));
    });

    it("returns NaN if either operand is NaN", () => {
      expect(runBinary(vm, 0x96, f32(NaN), f32(5.0)).value).toBeNaN();
      expect(runBinary(vm, 0x96, f32(3.0), f32(NaN)).value).toBeNaN();
    });

    it("returns -0 when comparing +0 and -0", () => {
      const result = runBinary(vm, 0x96, f32(0), f32(-0));
      expect(Object.is(result.value, -0)).toBe(true);
    });
  });

  describe("f32.max (0x97)", () => {
    it("returns the larger value", () => {
      expect(runBinary(vm, 0x97, f32(3.0), f32(5.0)).value).toBe(Math.fround(5.0));
    });

    it("returns NaN if either operand is NaN", () => {
      expect(runBinary(vm, 0x97, f32(NaN), f32(5.0)).value).toBeNaN();
    });

    it("returns +0 when comparing +0 and -0", () => {
      const result = runBinary(vm, 0x97, f32(-0), f32(0));
      expect(Object.is(result.value, 0)).toBe(true);
      expect(Object.is(result.value, -0)).toBe(false);
    });
  });

  describe("f32 comparisons return i32", () => {
    it("f32.eq returns i32", () => {
      const result = runBinary(vm, 0x5b, f32(1.0), f32(1.0));
      expect(result.value).toBe(1);
      expect(result.type).toBe(0x7f);
    });

    it("NaN != NaN", () => {
      expect(runBinary(vm, 0x5b, f32(NaN), f32(NaN)).value).toBe(0);
    });
  });

  describe("f32.sub (0x93)", () => {
    it("subtracts correctly", () => {
      expect(runBinary(vm, 0x93, f32(10.5), f32(3.5)).value).toBe(Math.fround(7.0));
    });
  });

  describe("f32.div (0x95)", () => {
    it("divides correctly", () => {
      expect(runBinary(vm, 0x95, f32(10.0), f32(4.0)).value).toBe(Math.fround(2.5));
    });

    it("returns Infinity on division by zero", () => {
      expect(runBinary(vm, 0x95, f32(1.0), f32(0.0)).value).toBe(Infinity);
    });
  });

  describe("f32.abs (0x8B)", () => {
    it("returns absolute value", () => {
      expect(runUnary(vm, 0x8b, f32(-5.0)).value).toBe(Math.fround(5.0));
      expect(runUnary(vm, 0x8b, f32(5.0)).value).toBe(Math.fround(5.0));
    });
  });

  describe("f32.neg (0x8C)", () => {
    it("negates values", () => {
      expect(runUnary(vm, 0x8c, f32(3.0)).value).toBe(Math.fround(-3.0));
      expect(runUnary(vm, 0x8c, f32(-3.0)).value).toBe(Math.fround(3.0));
    });
  });

  describe("f32.ceil (0x8D)", () => {
    it("rounds up", () => {
      expect(runUnary(vm, 0x8d, f32(1.1)).value).toBe(Math.fround(2));
    });
  });

  describe("f32.floor (0x8E)", () => {
    it("rounds down", () => {
      expect(runUnary(vm, 0x8e, f32(1.9)).value).toBe(Math.fround(1));
    });
  });

  describe("f32.trunc (0x8F)", () => {
    it("truncates toward zero", () => {
      expect(runUnary(vm, 0x8f, f32(1.9)).value).toBe(Math.fround(1));
      expect(runUnary(vm, 0x8f, f32(-1.9)).value).toBe(Math.fround(-1));
    });
  });

  describe("f32.sqrt (0x91)", () => {
    it("computes square root", () => {
      expect(runUnary(vm, 0x91, f32(4.0)).value).toBe(Math.fround(2.0));
    });
  });

  describe("f32.copysign (0x98)", () => {
    it("copies sign from second operand", () => {
      expect(runBinary(vm, 0x98, f32(5.0), f32(-1.0)).value).toBe(Math.fround(-5.0));
      expect(runBinary(vm, 0x98, f32(-5.0), f32(1.0)).value).toBe(Math.fround(5.0));
    });
  });

  describe("f32.ne (0x5C)", () => {
    it("returns 1 when not equal", () => {
      expect(runBinary(vm, 0x5c, f32(1.0), f32(2.0)).value).toBe(1);
    });

    it("NaN != NaN is true", () => {
      expect(runBinary(vm, 0x5c, f32(NaN), f32(NaN)).value).toBe(1);
    });
  });

  describe("f32.lt (0x5D)", () => {
    it("returns 1 when a < b", () => {
      expect(runBinary(vm, 0x5d, f32(1.0), f32(2.0)).value).toBe(1);
      expect(runBinary(vm, 0x5d, f32(2.0), f32(1.0)).value).toBe(0);
    });
  });

  describe("f32.gt (0x5E)", () => {
    it("returns 1 when a > b", () => {
      expect(runBinary(vm, 0x5e, f32(2.0), f32(1.0)).value).toBe(1);
      expect(runBinary(vm, 0x5e, f32(1.0), f32(2.0)).value).toBe(0);
    });
  });

  describe("f32.le (0x5F)", () => {
    it("returns 1 when a <= b", () => {
      expect(runBinary(vm, 0x5f, f32(1.0), f32(2.0)).value).toBe(1);
      expect(runBinary(vm, 0x5f, f32(1.0), f32(1.0)).value).toBe(1);
    });
  });

  describe("f32.ge (0x60)", () => {
    it("returns 1 when a >= b", () => {
      expect(runBinary(vm, 0x60, f32(2.0), f32(1.0)).value).toBe(1);
      expect(runBinary(vm, 0x60, f32(1.0), f32(1.0)).value).toBe(1);
    });
  });
});
