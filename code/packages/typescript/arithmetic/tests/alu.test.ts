/**
 * Tests for the Arithmetic Logic Unit (ALU).
 *
 * Tests cover all six operations (ADD, SUB, AND, OR, XOR, NOT),
 * all four condition flags (zero, carry, negative, overflow),
 * and input validation.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { ALU, ALUOp } from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

/** Convert an integer to an LSB-first bit array of the given width. */
function intToBits(n: number, width: number): Bit[] {
  return Array.from({ length: width }, (_, i) => ((n >> i) & 1) as Bit);
}

/** Convert an LSB-first bit array back to an integer. */
function bitsToInt(bits: Bit[]): number {
  return bits.reduce((acc, bit, i) => acc + (bit << i), 0);
}

describe("ALU", () => {
  let alu: ALU;

  beforeEach(() => {
    alu = new ALU(8);
  });

  // -------------------------------------------------------------------------
  // ADD
  // -------------------------------------------------------------------------

  describe("ADD", () => {
    it("1 + 2 = 3 (the target program: x = 1 + 2)", () => {
      const result = alu.execute(
        ALUOp.ADD,
        intToBits(1, 8),
        intToBits(2, 8)
      );
      expect(bitsToInt(result.value)).toBe(3);
      expect(result.zero).toBe(false);
      expect(result.carry).toBe(false);
    });

    it("0 + 0 = 0 with zero flag", () => {
      const result = alu.execute(
        ALUOp.ADD,
        intToBits(0, 8),
        intToBits(0, 8)
      );
      expect(bitsToInt(result.value)).toBe(0);
      expect(result.zero).toBe(true);
    });

    it("255 + 1 overflows with carry flag", () => {
      const result = alu.execute(
        ALUOp.ADD,
        intToBits(255, 8),
        intToBits(1, 8)
      );
      expect(bitsToInt(result.value)).toBe(0);
      expect(result.carry).toBe(true);
      expect(result.zero).toBe(true);
    });
  });

  // -------------------------------------------------------------------------
  // SUB
  // -------------------------------------------------------------------------

  describe("SUB", () => {
    it("5 - 3 = 2", () => {
      const result = alu.execute(
        ALUOp.SUB,
        intToBits(5, 8),
        intToBits(3, 8)
      );
      expect(bitsToInt(result.value)).toBe(2);
      expect(result.zero).toBe(false);
    });

    it("3 - 3 = 0 with zero flag", () => {
      const result = alu.execute(
        ALUOp.SUB,
        intToBits(3, 8),
        intToBits(3, 8)
      );
      expect(bitsToInt(result.value)).toBe(0);
      expect(result.zero).toBe(true);
    });
  });

  // -------------------------------------------------------------------------
  // Bitwise operations
  // -------------------------------------------------------------------------

  describe("bitwise operations", () => {
    it("AND: 0xCC & 0xAA = 0x88", () => {
      // 0b11001100 AND 0b10101010 = 0b10001000
      const result = alu.execute(
        ALUOp.AND,
        intToBits(0xcc, 8),
        intToBits(0xaa, 8)
      );
      expect(bitsToInt(result.value)).toBe(0x88);
    });

    it("OR: 0xCC | 0xAA = 0xEE", () => {
      // 0b11001100 OR 0b10101010 = 0b11101110
      const result = alu.execute(
        ALUOp.OR,
        intToBits(0xcc, 8),
        intToBits(0xaa, 8)
      );
      expect(bitsToInt(result.value)).toBe(0xee);
    });

    it("XOR: 0xCC ^ 0xAA = 0x66", () => {
      // 0b11001100 XOR 0b10101010 = 0b01100110
      const result = alu.execute(
        ALUOp.XOR,
        intToBits(0xcc, 8),
        intToBits(0xaa, 8)
      );
      expect(bitsToInt(result.value)).toBe(0x66);
    });

    it("NOT: ~0x00 = 0xFF", () => {
      // NOT 0b00000000 = 0b11111111
      const result = alu.execute(ALUOp.NOT, intToBits(0, 8), []);
      expect(bitsToInt(result.value)).toBe(255);
    });
  });

  // -------------------------------------------------------------------------
  // Flags
  // -------------------------------------------------------------------------

  describe("flags", () => {
    it("zero flag on AND producing all zeros", () => {
      const result = alu.execute(
        ALUOp.AND,
        intToBits(0xf0, 8),
        intToBits(0x0f, 8)
      );
      expect(result.zero).toBe(true);
    });

    it("negative flag when MSB is set", () => {
      // MSB set = negative in two's complement
      const result = alu.execute(
        ALUOp.ADD,
        intToBits(128, 8),
        intToBits(0, 8)
      );
      expect(result.negative).toBe(true);
    });

    it("signed overflow: 127 + 1 = 128 overflows in signed 8-bit", () => {
      // 127 + 1 = 128, but in signed 8-bit, 127 + 1 = -128 (overflow)
      const result = alu.execute(
        ALUOp.ADD,
        intToBits(127, 8),
        intToBits(1, 8)
      );
      expect(result.overflow).toBe(true);
    });
  });

  // -------------------------------------------------------------------------
  // Validation
  // -------------------------------------------------------------------------

  describe("validation", () => {
    it("throws on wrong bit width for a", () => {
      expect(() =>
        alu.execute(ALUOp.ADD, [0, 1] as Bit[], [0, 1] as Bit[])
      ).toThrow("8 bits");
    });

    it("throws on bit_width < 1", () => {
      expect(() => new ALU(0)).toThrow("at least 1");
    });
  });
});
