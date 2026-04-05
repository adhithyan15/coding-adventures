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

  describe("i64.sub (0x7D)", () => {
    it("subtracts correctly", () => {
      expect(runBinary(vm, 0x7d, i64(30n), i64(10n)).value).toBe(20n);
    });

    it("wraps on underflow", () => {
      const min = -9223372036854775808n;
      expect(runBinary(vm, 0x7d, i64(min), i64(1n)).value).toBe(9223372036854775807n);
    });
  });

  describe("i64.and (0x83)", () => {
    it("bitwise AND", () => {
      expect(runBinary(vm, 0x83, i64(0xFFn), i64(0x0Fn)).value).toBe(0x0Fn);
    });

    it("AND with zero gives zero", () => {
      expect(runBinary(vm, 0x83, i64(12345n), i64(0n)).value).toBe(0n);
    });
  });

  describe("i64.or (0x84)", () => {
    it("bitwise OR", () => {
      expect(runBinary(vm, 0x84, i64(0xF0n), i64(0x0Fn)).value).toBe(0xFFn);
    });
  });

  describe("i64.xor (0x85)", () => {
    it("bitwise XOR", () => {
      expect(runBinary(vm, 0x85, i64(0xFFn), i64(0x0Fn)).value).toBe(0xF0n);
    });

    it("XOR with self gives zero", () => {
      expect(runBinary(vm, 0x85, i64(42n), i64(42n)).value).toBe(0n);
    });
  });

  describe("i64.shl (0x86)", () => {
    it("shifts left", () => {
      expect(runBinary(vm, 0x86, i64(1n), i64(4n)).value).toBe(16n);
    });

    it("masks shift amount to 6 bits (mod 64)", () => {
      /* Shifting by 64 is same as shifting by 0 */
      expect(runBinary(vm, 0x86, i64(1n), i64(64n)).value).toBe(1n);
    });
  });

  describe("i64.shr_s (0x87)", () => {
    it("arithmetic shift right (preserves sign)", () => {
      expect(runBinary(vm, 0x87, i64(-8n), i64(2n)).value).toBe(-2n);
    });

    it("shifts positive values", () => {
      expect(runBinary(vm, 0x87, i64(16n), i64(2n)).value).toBe(4n);
    });
  });

  describe("i64.shr_u (0x88)", () => {
    it("logical shift right (zero-fills)", () => {
      /* -1n in unsigned 64-bit is all 1s. Shifting right by 63 gives 1. */
      expect(runBinary(vm, 0x88, i64(-1n), i64(63n)).value).toBe(1n);
    });
  });

  describe("i64.rem_s (0x81)", () => {
    it("computes signed remainder", () => {
      expect(runBinary(vm, 0x81, i64(7n), i64(3n)).value).toBe(1n);
    });

    it("handles negative dividend", () => {
      expect(runBinary(vm, 0x81, i64(-7n), i64(3n)).value).toBe(-1n);
    });

    it("traps on division by zero", () => {
      expect(() => runBinary(vm, 0x81, i64(7n), i64(0n))).toThrow(TrapError);
    });
  });

  describe("i64.rem_u (0x82)", () => {
    it("computes unsigned remainder", () => {
      expect(runBinary(vm, 0x82, i64(7n), i64(3n)).value).toBe(1n);
    });

    it("traps on division by zero", () => {
      expect(() => runBinary(vm, 0x82, i64(7n), i64(0n))).toThrow(TrapError);
    });
  });

  describe("i64.ctz (0x7A)", () => {
    it("counts trailing zeros", () => {
      expect(runUnary(vm, 0x7a, i64(0n)).value).toBe(64n);
      expect(runUnary(vm, 0x7a, i64(1n)).value).toBe(0n);
      expect(runUnary(vm, 0x7a, i64(8n)).value).toBe(3n);
    });
  });

  describe("i64.popcnt (0x7B)", () => {
    it("counts set bits", () => {
      expect(runUnary(vm, 0x7b, i64(0n)).value).toBe(0n);
      expect(runUnary(vm, 0x7b, i64(-1n)).value).toBe(64n);
      expect(runUnary(vm, 0x7b, i64(0b10110011n)).value).toBe(5n);
    });
  });

  describe("i64.ne (0x52)", () => {
    it("returns 1 when values differ", () => {
      expect(runBinary(vm, 0x52, i64(1n), i64(2n)).value).toBe(1);
    });

    it("returns 0 when values are equal", () => {
      expect(runBinary(vm, 0x52, i64(42n), i64(42n)).value).toBe(0);
    });
  });

  describe("i64.gt_s (0x55)", () => {
    it("returns 1 when a > b (signed)", () => {
      expect(runBinary(vm, 0x55, i64(5n), i64(3n)).value).toBe(1);
    });

    it("returns 0 when a <= b", () => {
      expect(runBinary(vm, 0x55, i64(3n), i64(5n)).value).toBe(0);
      expect(runBinary(vm, 0x55, i64(5n), i64(5n)).value).toBe(0);
    });
  });

  describe("i64.ge_u (0x5A)", () => {
    it("returns 1 when a >= b (unsigned)", () => {
      expect(runBinary(vm, 0x5a, i64(5n), i64(5n)).value).toBe(1);
      expect(runBinary(vm, 0x5a, i64(-1n), i64(0n)).value).toBe(1); // -1n unsigned is max
    });

    it("returns 0 when a < b (unsigned)", () => {
      expect(runBinary(vm, 0x5a, i64(0n), i64(-1n)).value).toBe(0);
    });
  });

  describe("i64.eqz additional cases (0x50)", () => {
    it("returns 0 for nonzero", () => {
      expect(runUnary(vm, 0x50, i64(42n)).value).toBe(0);
      expect(runUnary(vm, 0x50, i64(-1n)).value).toBe(0);
    });
  });
});
