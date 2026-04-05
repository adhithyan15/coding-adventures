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
});
