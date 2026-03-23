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
 *
 * === Native trace emission (V2) ===
 *
 * The ALU now captures per-adder intermediate state during every arithmetic
 * operation via `rippleCarryAdderTraced()`. After any add/subtract/increment/
 * decrement, the `lastTrace` property holds the full ALU trace — inputs,
 * per-adder snapshots, result, and carry. This eliminates the need for
 * consumers (like the Busicom calculator app) to replay the adder chain.
 */

import { ALU, ALUOp, rippleCarryAdderTraced } from "@coding-adventures/arithmetic";
import type { FullAdderSnapshot } from "@coding-adventures/arithmetic";
import { type Bit } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";

/**
 * Complete trace of a 4-bit ALU operation.
 *
 * Captures every intermediate value so visualizations can show
 * exactly how the result was computed — from input bits through
 * the carry chain to the final output.
 */
export interface ALUTrace {
  /** Which ALU operation was performed. */
  operation: "add" | "sub" | "inc" | "dec" | "complement" | "and" | "or" | "daa";

  /** 4-bit input A (LSB first). */
  inputA: Bit[];

  /** 4-bit input B (LSB first). For SUB, this is the complemented value. */
  inputB: Bit[];

  /** Carry/borrow input to the adder chain. */
  carryIn: Bit;

  /**
   * Per-bit full adder snapshots, from bit 0 (LSB) to bit 3 (MSB).
   * Empty for non-adder operations (complement, and, or).
   */
  adders: FullAdderSnapshot[];

  /** 4-bit result (LSB first). */
  result: Bit[];

  /** Final carry out from the MSB adder. */
  carryOut: Bit;
}

/**
 * 4-bit ALU for the Intel 4004 gate-level simulator.
 *
 * All operations route through real logic gates via the arithmetic
 * package's ALU class. No behavioral shortcuts.
 *
 * After any operation, `lastTrace` holds the full trace data.
 */
export class GateALU {
  private _alu: ALU;

  /** Trace from the most recent operation. Cleared before each new op. */
  private _lastTrace: ALUTrace | undefined;

  constructor() {
    /** Create a 4-bit ALU using real logic gates. */
    this._alu = new ALU(4);
    this._lastTrace = undefined;
  }

  /** The most recent ALU operation trace. */
  get lastTrace(): ALUTrace | undefined {
    return this._lastTrace;
  }

  /** Clear the last trace. Called at the start of each CPU step(). */
  clearTrace(): void {
    this._lastTrace = undefined;
  }

  /**
   * Add two 4-bit values with carry.
   *
   * Routes through: XOR -> AND -> OR -> fullAdder x 4 -> rippleCarry
   */
  add(a: number, b: number, carryIn: number = 0): [number, boolean] {
    const aBits = intToBits(a, 4);
    const bBits = intToBits(b, 4);

    if (carryIn) {
      const result1 = this._alu.execute(ALUOp.ADD, aBits, bBits);
      const oneBits = intToBits(1, 4);
      const result2 = this._alu.execute(ALUOp.ADD, result1.value, oneBits);
      const carry = result1.carry || result2.carry;

      // Capture trace using the traced adder
      const traced = rippleCarryAdderTraced(aBits, bBits, 1 as Bit);
      this._lastTrace = {
        operation: "add",
        inputA: aBits,
        inputB: bBits,
        carryIn: 1 as Bit,
        adders: traced.adders,
        result: traced.sum,
        carryOut: traced.carryOut,
      };

      return [bitsToInt(result2.value), carry];
    } else {
      const result = this._alu.execute(ALUOp.ADD, aBits, bBits);

      const traced = rippleCarryAdderTraced(aBits, bBits, 0 as Bit);
      this._lastTrace = {
        operation: "add",
        inputA: aBits,
        inputB: bBits,
        carryIn: 0 as Bit,
        adders: traced.adders,
        result: traced.sum,
        carryOut: traced.carryOut,
      };

      return [bitsToInt(result.value), result.carry];
    }
  }

  /**
   * Subtract using complement-add: A + NOT(B) + borrowIn.
   */
  subtract(a: number, b: number, borrowIn: number = 0): [number, boolean] {
    const bBits = intToBits(b, 4);
    const bComp = this._alu.execute(ALUOp.NOT, bBits, bBits);
    const bCompBits = bComp.value as Bit[];
    const [result, carry] = this.add(a, bitsToInt(bCompBits), borrowIn);

    // Fix the trace to show SUB operation with complemented B
    if (this._lastTrace) {
      this._lastTrace.operation = "sub";
      this._lastTrace.inputB = bCompBits;
    }

    return [result, carry];
  }

  /**
   * 4-bit NOT: invert all bits using NOT gates.
   */
  complement(a: number): number {
    const aBits = intToBits(a, 4);
    const result = this._alu.execute(ALUOp.NOT, aBits, aBits);
    const resultBits = result.value as Bit[];

    this._lastTrace = {
      operation: "complement",
      inputA: aBits,
      inputB: [0, 0, 0, 0],
      carryIn: 0,
      adders: [],
      result: resultBits,
      carryOut: 0,
    };

    return bitsToInt(resultBits);
  }

  /** Increment by 1 using the adder. Returns [result, carry]. */
  increment(a: number): [number, boolean] {
    const result = this.add(a, 1, 0);
    if (this._lastTrace) {
      this._lastTrace.operation = "inc";
    }
    return result;
  }

  /**
   * Decrement by 1 using complement-add.
   */
  decrement(a: number): [number, boolean] {
    const result = this.subtract(a, 1, 1);
    if (this._lastTrace) {
      this._lastTrace.operation = "dec";
    }
    return result;
  }

  /** 4-bit AND using AND gates. */
  bitwiseAnd(a: number, b: number): number {
    const aBits = intToBits(a, 4);
    const bBits = intToBits(b, 4);
    const result = this._alu.execute(ALUOp.AND, aBits, bBits);
    this._lastTrace = undefined;
    return bitsToInt(result.value);
  }

  /** 4-bit OR using OR gates. */
  bitwiseOr(a: number, b: number): number {
    const aBits = intToBits(a, 4);
    const bBits = intToBits(b, 4);
    const result = this._alu.execute(ALUOp.OR, aBits, bBits);
    this._lastTrace = undefined;
    return bitsToInt(result.value);
  }

  /**
   * Estimated gate count for a 4-bit ALU.
   */
  get gateCount(): number {
    return 32;
  }
}
