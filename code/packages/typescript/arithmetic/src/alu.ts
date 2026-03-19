/**
 * Arithmetic Logic Unit (ALU) — the computational heart of a CPU.
 *
 * Takes two N-bit inputs and an operation code, produces an N-bit result
 * plus status flags. Built from adders and logic gates.
 *
 * # The CPU's Calculator
 *
 * An ALU is the part of a CPU that actually executes commands. You give it
 * two numbers (A and B) and a control signal (the operation). It routes
 * those numbers through various circuits (like our rippleCarryAdder) and
 * outputs the result alongside helpful "flags" that let the CPU make decisions
 * based on the result (like "Jump if Zero").
 */

import { AND, OR, XOR, NOT, type Bit } from "@coding-adventures/logic-gates";
import { rippleCarryAdder } from "./adders.js";

/**
 * ALU operation codes.
 *
 * These are the "instructions" the ALU understands. In a real CPU, the
 * control unit decodes a machine instruction and sends the appropriate
 * opcode to the ALU.
 */
export enum ALUOp {
  /** A + B using the ripple carry adder */
  ADD = "add",
  /** A - B using two's complement (A + NOT(B) + 1) */
  SUB = "sub",
  /** Bitwise AND — useful for masking bits */
  AND = "and",
  /** Bitwise OR — useful for setting bits */
  OR = "or",
  /** Bitwise XOR — useful for toggling bits */
  XOR = "xor",
  /** Bitwise NOT — flip all bits (unary, only uses A) */
  NOT = "not",
}

/**
 * Result of an ALU operation.
 *
 * Beyond the computed value, the ALU also produces "condition flags" —
 * single-bit signals that describe properties of the result. The CPU
 * uses these flags to make branching decisions (e.g., "jump if zero",
 * "jump if negative").
 */
export interface ALUResult {
  /** Result bits (LSB first) */
  value: Bit[];
  /** Is result all zeros? (Useful for branching: "if x == 0") */
  zero: boolean;
  /** Did the unsigned addition overflow out of the top bit? */
  carry: boolean;
  /** Is MSB 1? (In two's complement, 1 = negative, 0 = positive) */
  negative: boolean;
  /** Did signed arithmetic wrap around incorrectly? */
  overflow: boolean;
}

/**
 * Apply a 2-input gate bitwise across two bit arrays.
 *
 * This is how the ALU performs operations like AND, OR, XOR on multi-bit
 * numbers: it simply applies the same single-bit gate to each pair of
 * corresponding bits in parallel.
 */
function bitwiseOp(
  a: Bit[],
  b: Bit[],
  op: (x: Bit, y: Bit) => Bit
): Bit[] {
  return a.map((_, i) => op(a[i], b[i]));
}

/**
 * Negate a number in two's complement: NOT(bits) + 1.
 *
 * # Two's Complement Magic
 *
 * How do computing systems represent negative numbers? They use a trick called
 * Two's Complement. To turn `x` into `-x`:
 *   1. Flip every bit (NOT operation).
 *   2. Add 1.
 *
 * Why this works: A number `x` plus its bitwise inverse `NOT(x)` is always
 * a number with all 1s (e.g., 1111). If you add 1 to `1111`, it rolls over
 * to `0000` (disregarding the carry out). So:
 *     x + NOT(x) = 1111
 *     x + NOT(x) + 1 = 0000
 * Therefore:
 *     NOT(x) + 1 = -x
 *
 * The beauty of this is that the ALU can use the EXACT same adder circuit for
 * both positive and negative math. No special subtraction hardware is needed!
 */
function twosComplementNegate(bits: Bit[]): [Bit[], Bit] {
  const inverted: Bit[] = bits.map((b) => NOT(b));
  const one: Bit[] = [1 as Bit, ...new Array<Bit>(bits.length - 1).fill(0 as Bit)];
  return rippleCarryAdder(inverted, one);
}

/**
 * N-bit Arithmetic Logic Unit.
 *
 * The ALU is initialized with a fixed bit width (like 8-bit, 16-bit, or
 * 32-bit). All inputs and outputs must match this width. This mirrors how
 * real CPUs have a fixed "word size" that determines how wide their data
 * buses are.
 */
export class ALU {
  readonly bitWidth: number;

  constructor(bitWidth: number = 8) {
    if (bitWidth < 1) {
      throw new Error("bit_width must be at least 1");
    }
    this.bitWidth = bitWidth;
  }

  /**
   * Execute an ALU operation on two N-bit inputs.
   *
   * This is the main entry point. It routes the A and B buses into the
   * appropriate circuit based on the op code, and then computes the
   * condition flags corresponding to the output.
   *
   * @param op - The operation to perform.
   * @param a - First operand as bits (LSB first), length must equal bitWidth.
   * @param b - Second operand as bits (LSB first), length must equal bitWidth.
   *            Ignored for NOT operation.
   * @returns ALUResult with value, zero, carry, negative, and overflow flags.
   */
  execute(op: ALUOp, a: Bit[], b: Bit[]): ALUResult {
    if (a.length !== this.bitWidth) {
      throw new Error(
        `a must have ${this.bitWidth} bits, got ${a.length}`
      );
    }
    // The NOT instruction only uses the A bus, so B can be empty.
    if (op !== ALUOp.NOT && b.length !== this.bitWidth) {
      throw new Error(
        `b must have ${this.bitWidth} bits, got ${b.length}`
      );
    }

    let value: Bit[];
    let carry = false;

    // 1. Calculate the result based on the requested operation.
    switch (op) {
      case ALUOp.ADD: {
        const [result, carryBit] = rippleCarryAdder(a, b);
        value = result;
        carry = carryBit === 1;
        break;
      }

      case ALUOp.SUB: {
        // A - B is mathematically equivalent to A + (-B).
        // We use Two's Complement to negate B, and add them!
        const [negB] = twosComplementNegate(b);
        const [result, carryBit] = rippleCarryAdder(a, negB);
        value = result;
        carry = carryBit === 1;
        break;
      }

      case ALUOp.AND:
        value = bitwiseOp(a, b, AND);
        break;

      case ALUOp.OR:
        value = bitwiseOp(a, b, OR);
        break;

      case ALUOp.XOR:
        value = bitwiseOp(a, b, XOR);
        break;

      case ALUOp.NOT:
        value = a.map((bit) => NOT(bit));
        break;

      default:
        throw new Error(`Unknown operation: ${op}`);
    }

    // 2. Calculate the condition flags.

    // Zero flag is true if every single bit is 0.
    const zero = value.every((bit) => bit === 0);

    // Negative flag simply checks the Most Significant Bit (MSB).
    // In two's complement, an MSB of 1 signifies a negative number.
    const negative = value.length > 0 && value[value.length - 1] === 1;

    // Overflow flag indicates when the sign of the result is mathematically
    // impossible, implying we "ran out of bits" to represent the magnitude.
    // E.g., Adding two large positive numbers shouldn't give a negative sum.
    let overflow = false;
    if (op === ALUOp.ADD || op === ALUOp.SUB) {
      const aSign = a[a.length - 1];
      // For subtraction, we are adding NOT(B) + 1, so the effective
      // sign of the second operand in the inner addition is inverted.
      const bSign =
        op === ALUOp.ADD ? b[b.length - 1] : NOT(b[b.length - 1]);
      const resultSign = value[value.length - 1];

      // If both operands had the same sign, but the result has a different sign,
      // an overflow corruption occurred.
      if (aSign === bSign && resultSign !== aSign) {
        overflow = true;
      }
    }

    return {
      value,
      zero,
      carry,
      negative,
      overflow,
    };
  }
}
