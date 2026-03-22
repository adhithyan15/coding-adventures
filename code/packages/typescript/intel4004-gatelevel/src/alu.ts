/**
 * 4-bit ALU -- the arithmetic heart of the Intel 4004.
 *
 * === How the real 4004's ALU worked ===
 *
 * The Intel 4004 had a 4-bit ALU that could add, subtract, and perform
 * logical operations on 4-bit values. It used a ripple-carry adder built
 * from full adders, which were themselves built from AND, OR, and XOR gates.
 *
 * This module wraps the arithmetic package's ALU(bitWidth=4) to provide
 * the exact operations the 4004 needs. Every addition and subtraction
 * physically routes through the gate chain:
 *
 *     XOR -> AND -> OR -> fullAdder -> rippleCarryAdder -> ALU
 *
 * That's real hardware simulation -- not behavioral shortcuts.
 *
 * === Subtraction via complement-add ===
 *
 * The 4004 doesn't have a dedicated subtractor. Instead, it uses the
 * ones' complement method:
 *
 *     A - B = A + NOT(B) + borrow_in
 *
 * where borrow_in = 0 if carry_flag else 1 (inverted carry semantics).
 * The ALU's SUB operation does this internally using NOT gates to
 * complement B, then feeding through the same adder.
 */

import { ALU, ALUOp } from "@coding-adventures/arithmetic";
import { type Bit } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";

/**
 * 4-bit ALU for the Intel 4004 gate-level simulator.
 *
 * All operations route through real logic gates via the arithmetic
 * package's ALU class. No behavioral shortcuts.
 *
 * The ALU provides:
 *   - add(a, b, carryIn)       => [result, carryOut]
 *   - subtract(a, b, borrowIn) => [result, carryOut]
 *   - complement(a)            => result (4-bit NOT)
 *   - increment(a)             => [result, carryOut]
 *   - decrement(a)             => [result, borrowOut]
 */
export class GateALU {
  private _alu: ALU;

  constructor() {
    /** Create a 4-bit ALU using real logic gates. */
    this._alu = new ALU(4);
  }

  /**
   * Add two 4-bit values with carry.
   *
   * Routes through: XOR -> AND -> OR -> fullAdder x 4 -> rippleCarry
   *
   * @param a - First operand (0-15).
   * @param b - Second operand (0-15).
   * @param carryIn - Carry from previous operation (0 or 1).
   * @returns [result, carryOut] where result is 4-bit (0-15).
   */
  add(a: number, b: number, carryIn: number = 0): [number, boolean] {
    const aBits = intToBits(a, 4);
    const bBits = intToBits(b, 4);

    if (carryIn) {
      // Add carry_in by first adding a+b, then adding 1
      // This simulates the carry input to the LSB full adder
      const result1 = this._alu.execute(ALUOp.ADD, aBits, bBits);
      const oneBits = intToBits(1, 4);
      const result2 = this._alu.execute(ALUOp.ADD, result1.value, oneBits);
      // Carry is set if either addition overflowed
      const carry = result1.carry || result2.carry;
      return [bitsToInt(result2.value), carry];
    } else {
      const result = this._alu.execute(ALUOp.ADD, aBits, bBits);
      return [bitsToInt(result.value), result.carry];
    }
  }

  /**
   * Subtract using complement-add: A + NOT(B) + borrowIn.
   *
   * The 4004's carry flag semantics for subtraction:
   *   carry=true  => no borrow (result >= 0)
   *   carry=false => borrow occurred
   *
   * @param a - Minuend (0-15).
   * @param b - Subtrahend (0-15).
   * @param borrowIn - 1 if no previous borrow, 0 if borrow.
   * @returns [result, carryOut] where carryOut=true means no borrow.
   */
  subtract(a: number, b: number, borrowIn: number = 0): [number, boolean] {
    // Complement b using NOT gates
    const bBits = intToBits(b, 4);
    const bComp = this._alu.execute(ALUOp.NOT, bBits, bBits);
    // A + NOT(B) + borrowIn
    return this.add(a, bitsToInt(bComp.value), borrowIn);
  }

  /**
   * 4-bit NOT: invert all bits using NOT gates.
   *
   * @param a - Value to complement (0-15).
   * @returns Complemented value (0-15).
   */
  complement(a: number): number {
    const aBits = intToBits(a, 4);
    const result = this._alu.execute(ALUOp.NOT, aBits, aBits);
    return bitsToInt(result.value);
  }

  /** Increment by 1 using the adder. Returns [result, carry]. */
  increment(a: number): [number, boolean] {
    return this.add(a, 1, 0);
  }

  /**
   * Decrement by 1 using complement-add.
   *
   * A - 1 = A + NOT(1) + 1 = A + 14 + 1 = A + 15.
   * carry=true if A > 0 (no borrow), false if A == 0.
   */
  decrement(a: number): [number, boolean] {
    return this.subtract(a, 1, 1);
  }

  /** 4-bit AND using AND gates. */
  bitwiseAnd(a: number, b: number): number {
    const aBits = intToBits(a, 4);
    const bBits = intToBits(b, 4);
    const result = this._alu.execute(ALUOp.AND, aBits, bBits);
    return bitsToInt(result.value);
  }

  /** 4-bit OR using OR gates. */
  bitwiseOr(a: number, b: number): number {
    const aBits = intToBits(a, 4);
    const bBits = intToBits(b, 4);
    const result = this._alu.execute(ALUOp.OR, aBits, bBits);
    return bitsToInt(result.value);
  }

  /**
   * Estimated gate count for a 4-bit ALU.
   *
   * Each full adder: 5 gates (2 XOR + 2 AND + 1 OR).
   * 4-bit ripple carry: 4 x 5 = 20 gates.
   * SUB complement: 4 NOT gates.
   * Control muxing: ~8 gates.
   * Total: ~32 gates.
   */
  get gateCount(): number {
    return 32;
  }
}
