/**
 * Tests for i32 numeric instruction handlers.
 */

import { describe, it, expect } from "vitest";
import { makeVm, runUnary, runBinary, runInstructions, i32, makeContext } from "./helpers.js";
import { registerNumericI32 } from "../src/instructions/numeric_i32.js";
import { TrapError } from "../src/host_interface.js";

describe("numeric_i32", () => {
  const vm = makeVm(registerNumericI32);

  describe("i32.const (0x41)", () => {
    it("pushes an i32 constant", () => {
      runInstructions(vm, [{ opcode: 0x41, operand: 42 }]);
      const result = vm.peekTyped();
      expect(result.value).toBe(42);
      expect(result.type).toBe(0x7f);
    });

    it("pushes zero", () => {
      runInstructions(vm, [{ opcode: 0x41, operand: 0 }]);
      expect(vm.peekTyped().value).toBe(0);
    });

    it("pushes negative", () => {
      runInstructions(vm, [{ opcode: 0x41, operand: -1 }]);
      expect(vm.peekTyped().value).toBe(-1);
    });
  });

  describe("i32.eqz (0x45)", () => {
    it("returns 1 for zero", () => {
      const result = runUnary(vm, 0x45, i32(0));
      expect(result.value).toBe(1);
    });

    it("returns 0 for non-zero", () => {
      expect(runUnary(vm, 0x45, i32(42)).value).toBe(0);
      expect(runUnary(vm, 0x45, i32(-1)).value).toBe(0);
    });
  });

  describe("i32.add (0x6A)", () => {
    it("adds two positive numbers", () => {
      expect(runBinary(vm, 0x6a, i32(10), i32(20)).value).toBe(30);
    });

    it("wraps on overflow", () => {
      /* INT32_MAX + 1 should wrap to INT32_MIN */
      expect(runBinary(vm, 0x6a, i32(2147483647), i32(1)).value).toBe(-2147483648);
    });
  });

  describe("i32.sub (0x6B)", () => {
    it("subtracts correctly", () => {
      expect(runBinary(vm, 0x6b, i32(30), i32(10)).value).toBe(20);
    });

    it("wraps on underflow", () => {
      expect(runBinary(vm, 0x6b, i32(-2147483648), i32(1)).value).toBe(2147483647);
    });
  });

  describe("i32.mul (0x6C)", () => {
    it("multiplies correctly", () => {
      expect(runBinary(vm, 0x6c, i32(6), i32(7)).value).toBe(42);
    });

    it("handles large values with Math.imul", () => {
      /* This would give wrong results without Math.imul. */
      const result = runBinary(vm, 0x6c, i32(0x7fffffff), i32(0x7fffffff));
      expect(result.value).toBe(Math.imul(0x7fffffff, 0x7fffffff));
    });
  });

  describe("i32.div_s (0x6D)", () => {
    it("divides correctly", () => {
      expect(runBinary(vm, 0x6d, i32(10), i32(3)).value).toBe(3);
    });

    it("truncates toward zero for negative", () => {
      expect(runBinary(vm, 0x6d, i32(-7), i32(2)).value).toBe(-3);
    });

    it("traps on division by zero", () => {
      expect(() => runBinary(vm, 0x6d, i32(1), i32(0))).toThrow(TrapError);
    });

    it("traps on INT32_MIN / -1 overflow", () => {
      expect(() => runBinary(vm, 0x6d, i32(-2147483648), i32(-1))).toThrow(TrapError);
    });
  });

  describe("i32.lt_s (0x48)", () => {
    it("returns 1 when a < b (signed)", () => {
      expect(runBinary(vm, 0x48, i32(-1), i32(0)).value).toBe(1);
    });

    it("returns 0 when a >= b", () => {
      expect(runBinary(vm, 0x48, i32(5), i32(5)).value).toBe(0);
      expect(runBinary(vm, 0x48, i32(10), i32(5)).value).toBe(0);
    });
  });

  describe("i32.lt_u (0x49)", () => {
    it("treats -1 as MAX_UINT32", () => {
      /* -1 unsigned = 0xFFFFFFFF = 4294967295, which is > 0 */
      expect(runBinary(vm, 0x49, i32(-1), i32(0)).value).toBe(0);
      expect(runBinary(vm, 0x49, i32(0), i32(-1)).value).toBe(1);
    });
  });

  describe("i32.shl (0x74)", () => {
    it("shifts left", () => {
      expect(runBinary(vm, 0x74, i32(1), i32(4)).value).toBe(16);
    });

    it("masks shift amount to 5 bits (mod 32)", () => {
      /* Shifting by 32 is same as shifting by 0 */
      expect(runBinary(vm, 0x74, i32(1), i32(32)).value).toBe(1);
    });
  });

  describe("i32.rotr (0x78)", () => {
    it("rotates right", () => {
      /* 0x80000001 rotated right by 1 = 0xC0000000 */
      const result = runBinary(vm, 0x78, i32(0x80000001 | 0), i32(1));
      expect(result.value).toBe(0xc0000000 | 0);
    });
  });

  describe("i32.clz (0x67)", () => {
    it("counts leading zeros", () => {
      expect(runUnary(vm, 0x67, i32(1)).value).toBe(31);
      expect(runUnary(vm, 0x67, i32(0)).value).toBe(32);
      expect(runUnary(vm, 0x67, i32(-1)).value).toBe(0);
    });
  });

  describe("i32.ctz (0x68)", () => {
    it("counts trailing zeros", () => {
      expect(runUnary(vm, 0x68, i32(0)).value).toBe(32);
      expect(runUnary(vm, 0x68, i32(1)).value).toBe(0);
      expect(runUnary(vm, 0x68, i32(8)).value).toBe(3);  /* 0b1000 */
    });
  });

  describe("i32.popcnt (0x69)", () => {
    it("counts set bits", () => {
      expect(runUnary(vm, 0x69, i32(0)).value).toBe(0);
      expect(runUnary(vm, 0x69, i32(-1)).value).toBe(32);  /* all bits set */
      expect(runUnary(vm, 0x69, i32(0b10110011)).value).toBe(5);
    });
  });

  describe("i32.div_u (0x6E)", () => {
    it("divides unsigned", () => {
      expect(runBinary(vm, 0x6e, i32(10), i32(3)).value).toBe(3);
    });

    it("treats -1 as MAX_UINT32", () => {
      /* -1 unsigned = 4294967295 / 2 = 2147483647 */
      expect(runBinary(vm, 0x6e, i32(-1), i32(2)).value).toBe(2147483647);
    });

    it("traps on division by zero", () => {
      expect(() => runBinary(vm, 0x6e, i32(1), i32(0))).toThrow(TrapError);
    });
  });

  describe("i32.rem_s (0x6F)", () => {
    it("computes signed remainder", () => {
      expect(runBinary(vm, 0x6f, i32(7), i32(3)).value).toBe(1);
    });

    it("handles negative dividend", () => {
      expect(runBinary(vm, 0x6f, i32(-7), i32(3)).value).toBe(-1);
    });

    it("traps on division by zero", () => {
      expect(() => runBinary(vm, 0x6f, i32(7), i32(0))).toThrow(TrapError);
    });
  });

  describe("i32.rem_u (0x70)", () => {
    it("computes unsigned remainder", () => {
      expect(runBinary(vm, 0x70, i32(7), i32(3)).value).toBe(1);
    });

    it("traps on division by zero", () => {
      expect(() => runBinary(vm, 0x70, i32(7), i32(0))).toThrow(TrapError);
    });
  });

  describe("i32.and (0x71)", () => {
    it("bitwise AND", () => {
      expect(runBinary(vm, 0x71, i32(0xFF), i32(0x0F)).value).toBe(0x0F);
    });
  });

  describe("i32.or (0x72)", () => {
    it("bitwise OR", () => {
      expect(runBinary(vm, 0x72, i32(0xF0), i32(0x0F)).value).toBe(0xFF);
    });
  });

  describe("i32.xor (0x73)", () => {
    it("bitwise XOR", () => {
      expect(runBinary(vm, 0x73, i32(0xFF), i32(0x0F)).value).toBe(0xF0);
    });
  });

  describe("i32.shr_s (0x75)", () => {
    it("arithmetic shift right", () => {
      expect(runBinary(vm, 0x75, i32(-8), i32(2)).value).toBe(-2);
    });
  });

  describe("i32.shr_u (0x76)", () => {
    it("logical shift right", () => {
      expect(runBinary(vm, 0x76, i32(-1), i32(24)).value).toBe(255);
    });
  });

  describe("i32.rotl (0x77)", () => {
    it("rotates left", () => {
      expect(runBinary(vm, 0x77, i32(1), i32(1)).value).toBe(2);
    });
  });

  describe("i32.eq (0x46)", () => {
    it("returns 1 when equal", () => {
      expect(runBinary(vm, 0x46, i32(42), i32(42)).value).toBe(1);
    });

    it("returns 0 when not equal", () => {
      expect(runBinary(vm, 0x46, i32(1), i32(2)).value).toBe(0);
    });
  });

  describe("i32.ne (0x47)", () => {
    it("returns 1 when not equal", () => {
      expect(runBinary(vm, 0x47, i32(1), i32(2)).value).toBe(1);
    });

    it("returns 0 when equal", () => {
      expect(runBinary(vm, 0x47, i32(42), i32(42)).value).toBe(0);
    });
  });

  describe("i32.gt_s (0x4A)", () => {
    it("returns 1 when a > b signed", () => {
      expect(runBinary(vm, 0x4a, i32(5), i32(3)).value).toBe(1);
    });

    it("returns 0 otherwise", () => {
      expect(runBinary(vm, 0x4a, i32(3), i32(5)).value).toBe(0);
    });
  });

  describe("i32.gt_u (0x4B)", () => {
    it("treats values as unsigned", () => {
      expect(runBinary(vm, 0x4b, i32(-1), i32(0)).value).toBe(1);
    });
  });

  describe("i32.le_s (0x4C)", () => {
    it("returns 1 when a <= b signed", () => {
      expect(runBinary(vm, 0x4c, i32(3), i32(5)).value).toBe(1);
      expect(runBinary(vm, 0x4c, i32(5), i32(5)).value).toBe(1);
    });
  });

  describe("i32.le_u (0x4D)", () => {
    it("returns 1 when a <= b unsigned", () => {
      expect(runBinary(vm, 0x4d, i32(0), i32(-1)).value).toBe(1);
    });
  });

  describe("i32.ge_s (0x4E)", () => {
    it("returns 1 when a >= b signed", () => {
      expect(runBinary(vm, 0x4e, i32(5), i32(3)).value).toBe(1);
      expect(runBinary(vm, 0x4e, i32(5), i32(5)).value).toBe(1);
    });
  });

  describe("i32.ge_u (0x4F)", () => {
    it("returns 1 when a >= b unsigned", () => {
      expect(runBinary(vm, 0x4f, i32(-1), i32(0)).value).toBe(1);
    });
  });
});
