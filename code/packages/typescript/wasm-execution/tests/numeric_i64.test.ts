/**
 * Tests for i64 numeric instruction handlers.
 */

import { describe, it, expect } from "vitest";
import { makeVm, runUnary, runBinary, runInstructions, i64, i32 } from "./helpers.js";
import { registerNumericI64 } from "../src/instructions/numeric_i64.js";
import { TrapError } from "../src/host_interface.js";

describe("numeric_i64", () => {
  const vm = makeVm(registerNumericI64);

  describe("i64.const (0x42)", () => {
    it("pushes an i64 constant", () => {
      runInstructions(vm, [{ opcode: 0x42, operand: 42 }]);
      const result = vm.peekTyped();
      expect(result.value).toBe(42n);
      expect(result.type).toBe(0x7e);
    });
  });

  describe("i64.add (0x7C)", () => {
    it("adds two values", () => {
      expect(runBinary(vm, 0x7c, i64(10n), i64(20n)).value).toBe(30n);
    });

    it("wraps on overflow", () => {
      const max = 9223372036854775807n;
      const result = runBinary(vm, 0x7c, i64(max), i64(1n));
      expect(result.value).toBe(-9223372036854775808n);
    });
  });

  describe("i64.mul (0x7E)", () => {
    it("multiplies correctly", () => {
      expect(runBinary(vm, 0x7e, i64(6n), i64(7n)).value).toBe(42n);
    });
  });

  describe("i64.div_s (0x7F)", () => {
    it("divides correctly", () => {
      expect(runBinary(vm, 0x7f, i64(10n), i64(3n)).value).toBe(3n);
    });

    it("traps on division by zero", () => {
      expect(() => runBinary(vm, 0x7f, i64(1n), i64(0n))).toThrow(TrapError);
    });

    it("traps on MIN_I64 / -1 overflow", () => {
      expect(() =>
        runBinary(vm, 0x7f, i64(-9223372036854775808n), i64(-1n))
      ).toThrow(TrapError);
    });
  });

  describe("i64.clz (0x79)", () => {
    it("counts leading zeros", () => {
      expect(runUnary(vm, 0x79, i64(0n)).value).toBe(64n);
      expect(runUnary(vm, 0x79, i64(1n)).value).toBe(63n);
    });
  });

  describe("i64 comparisons return i32", () => {
    it("i64.eqz (0x50) returns i32", () => {
      const result = runUnary(vm, 0x50, i64(0n));
      expect(result.value).toBe(1);
      expect(result.type).toBe(0x7f); /* i32! */
    });

    it("i64.eq (0x51) returns i32", () => {
      const result = runBinary(vm, 0x51, i64(42n), i64(42n));
      expect(result.value).toBe(1);
      expect(result.type).toBe(0x7f);
    });

    it("i64.lt_s (0x53) handles signed comparison", () => {
      expect(runBinary(vm, 0x53, i64(-1n), i64(0n)).value).toBe(1);
    });

    it("i64.lt_u (0x54) handles unsigned comparison", () => {
      /* -1n unsigned is MAX_UINT64, so it should be greater than 0 */
      expect(runBinary(vm, 0x54, i64(-1n), i64(0n)).value).toBe(0);
    });
  });
});
