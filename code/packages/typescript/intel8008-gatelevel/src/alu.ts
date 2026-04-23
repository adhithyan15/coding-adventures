/**
 * 8-bit ALU for the Intel 8008 gate-level simulator.
 *
 * === How the real 8008's ALU worked ===
 *
 * The Intel 8008 had an 8-bit ALU — twice the width of the 4004's 4-bit ALU.
 * This doubling required 8 full-adders in the ripple-carry chain instead of 4,
 * giving 8 gate delays from LSB to MSB (vs 4 for the 4004).
 *
 * The operations supported:
 *   - ADD/ADC: ripple_carry_adder(A, B, cin)
 *   - SUB/SBB: ripple_carry_adder(A, NOT(B), cin) — two's complement subtract
 *   - AND:     8 AND gates in parallel
 *   - OR:      8 OR gates in parallel
 *   - XOR:     8 XOR gates in parallel
 *   - CMP:     same as SUB but discards result
 *   - INC:     ripple_carry_adder(A, 0, cin=1)
 *   - DEC:     A + 0xFF (equivalent to A - 1 mod 256)
 *   - Rotates: bit shifts via wiring (no arithmetic gates needed)
 *
 * === 8-bit ripple-carry chain ===
 *
 * The ripple-carry adder for 8 bits chains 8 full adders. Each full adder
 * is built from XOR, AND, OR gates. The carry "ripples" from bit 0 to bit 7:
 *
 *   FA(a0, b0, cin) → sum0, carry0
 *   FA(a1, b1, carry0) → sum1, carry1
 *   ...
 *   FA(a7, b7, carry6) → sum7, carry_out
 *
 * Carry propagation delay: 8 × gate delays (vs 4 for 4004).
 * Gates per 8-bit addition: 8 × 5 = 40 gates (AND×2, OR×1, XOR×2 per FA).
 *
 * === Subtraction via complement ===
 *
 * The 8008 doesn't have a dedicated subtractor. Subtraction is done via
 * two's complement: A - B = A + (~B) + 1. In hardware:
 *   1. Each bit of B is inverted through 8 NOT gates
 *   2. cin is set to 1 (the "+1")
 *   3. The same ripple-carry adder is used
 *
 * For SBB (subtract with borrow): cin = NOT(carry_flag) for borrow semantics.
 *
 * === 8008 borrow/carry convention ===
 *
 * The 8008 uses inverted carry for subtraction: CY=1 after SUB means a borrow
 * occurred (unsigned A < B). This is opposite to some architectures where CY=1
 * means NO borrow. The gate implementation reflects this convention.
 */

import { AND, OR, XOR, NOT, type Bit } from "@coding-adventures/logic-gates";
import { ALU, ALUOp } from "@coding-adventures/arithmetic";
import { intToBits, bitsToInt, computeParity } from "./bits.js";

/**
 * Flags computed by an 8008 ALU operation.
 *
 * All four flags are computed from the 8-bit result.
 * These are the logical outputs of the flag computation circuit.
 */
export interface GateFlags {
  /** CY — carry out from the MSB adder (or borrow for subtract). */
  carry: Bit;
  /** Z  — NOR of all 8 result bits (result === 0). */
  zero: Bit;
  /** S  — bit[7] of the result (sign bit). */
  sign: Bit;
  /** P  — NOT(XOR of all 8 result bits) (even parity = 1). */
  parity: Bit;
}

/**
 * 8-bit ALU for the Intel 8008 gate-level simulator.
 *
 * All arithmetic routes through the arithmetic package's ALU class,
 * which internally uses the ripple-carry adder chain (full adders made
 * of AND/OR/XOR gates). Bitwise operations use direct gate application.
 *
 * No behavioral shortcuts — every bit passes through gate functions.
 */
export class GateALU8 {
  /** The 8-bit ALU from the arithmetic package (gate-based ripple-carry). */
  private readonly _alu: ALU;

  constructor() {
    // Create an 8-bit ALU. Internally this uses ripple_carry_adder,
    // which chains 8 full_adders, each made from AND/XOR/OR gates.
    this._alu = new ALU(8);
  }

  /**
   * Compute flags from an 8-bit result and carry.
   *
   * === Flag circuit ===
   *
   * zero:   NOR gate tree on all 8 bits
   *         zero = NOR(b7, b6, b5, b4, b3, b2, b1, b0) = 1 iff all bits are 0
   *         Implementation: OR all bits, then NOT.
   *
   * sign:   direct wire from bit[7] of result
   *         sign = result_bits[7]
   *
   * carry:  from adder carry-out (provided as parameter)
   *
   * parity: XOR tree of 8 bits, then NOT
   *         parity = NOT(XOR(b0 XOR b1 XOR b2 XOR b3 XOR b4 XOR b5 XOR b6 XOR b7))
   *
   * @param resultBits - 8-bit result (LSB first).
   * @param carryBit   - Carry out from the adder (1 or 0).
   */
  computeFlags(resultBits: Bit[], carryBit: Bit): GateFlags {
    // Zero: OR all bits (is any bit 1?), then NOT (all bits 0?)
    // In a real circuit this would be an 8-input NOR gate, but we build
    // it from smaller gates (2-input OR chain + NOT) as our library provides.
    let anyOne: Bit = resultBits[0];
    for (let i = 1; i < 8; i++) {
      anyOne = OR(anyOne, resultBits[i]);
    }
    const zero = NOT(anyOne) as Bit;

    // Sign: bit 7 (MSB) is a direct wire — no gates needed.
    const sign = resultBits[7];

    // Parity: XOR reduction tree + NOT (from bits.ts, which uses xorN).
    const parity = computeParity(resultBits);

    return { carry: carryBit, zero, sign, parity };
  }

  /**
   * Add two 8-bit values with carry-in.
   *
   * Routes through: ALU(8) → ripple_carry_adder → 8 × full_adder → gates.
   *
   * @param a - 8-bit integer value A.
   * @param b - 8-bit integer value B.
   * @param cin - Carry-in (0 or 1).
   * @returns [result, carryOut] as integers.
   */
  add(a: number, b: number, cin: Bit = 0): [number, Bit] {
    const aBits = intToBits(a, 8);
    const bBits = intToBits(b, 8);

    if (cin === 1) {
      // Add A + B, then add 1 for the carry-in.
      // This is how the real hardware handles it — the cin feeds the
      // carry-in of the first full adder directly.
      const step1 = this._alu.execute(ALUOp.ADD, aBits, bBits);
      const oneBits = intToBits(1, 8);
      const step2 = this._alu.execute(ALUOp.ADD, step1.value, oneBits);
      const carry = (step1.carry || step2.carry) ? 1 as Bit : 0 as Bit;
      return [bitsToInt(step2.value as Bit[]), carry];
    } else {
      const result = this._alu.execute(ALUOp.ADD, aBits, bBits);
      return [bitsToInt(result.value as Bit[]), result.carry ? 1 as Bit : 0 as Bit];
    }
  }

  /**
   * Subtract B from A (with borrow-in) using two's complement via gates.
   *
   * === Two's complement subtraction ===
   *
   * A - B = A + (~B) + 1  (two's complement identity)
   *
   * In hardware:
   *   1. Invert all bits of B through 8 NOT gates: notB = [NOT(b0)...NOT(b7)]
   *   2. Feed notB and cin=1 (for the "+1") into the ripple-carry adder
   *
   * For SBB (subtract with borrow), the borrow-in replaces the +1:
   *   A - B - borrow = A + (~B) + (1 - borrow) = A + (~B) + NOT(borrow)
   *
   * Carry convention: 8008 sets CY=1 when a borrow occurred (A < B in unsigned).
   * This is the INVERSE of the carry-out from the adder (complement addition).
   *
   * @param a - 8-bit value A.
   * @param b - 8-bit value B.
   * @param borrowIn - Borrow from previous operation (0 or 1).
   * @returns [result, borrowOut] — borrowOut=1 means A < B (borrow occurred).
   */
  subtract(a: number, b: number, borrowIn: Bit = 0): [number, Bit] {
    // Step 1: Invert all bits of B using 8 NOT gates (two's complement first step)
    const bBits = intToBits(b, 8);
    const notBBits = bBits.map((bit) => NOT(bit)) as Bit[];
    const notB = bitsToInt(notBBits);

    // Step 2: cin = NOT(borrowIn) for borrow semantics
    // When borrowIn=0: cin=1 (standard SUB: A + ~B + 1 = A - B)
    // When borrowIn=1: cin=0 (SBB: A + ~B + 0 = A - B - 1)
    const cin = NOT(borrowIn) as Bit;

    const [result, carryOut] = this.add(a, notB, cin);

    // Step 3: Invert carry to get borrow flag (8008 convention)
    // carryOut=1 from complement addition means NO borrow (A >= B)
    // carryOut=0 means borrow DID occur (A < B)
    const borrow = NOT(carryOut) as Bit;
    return [result, borrow];
  }

  /**
   * Bitwise AND of two 8-bit values via 8 AND gates.
   *
   * Each bit pair is fed into an independent AND gate:
   *   result[i] = AND(a_bits[i], b_bits[i])
   *
   * AND always clears carry (CY=0) per 8008 spec.
   *
   * @returns result (integer).
   */
  bitwiseAnd(a: number, b: number): number {
    const aBits = intToBits(a, 8);
    const bBits = intToBits(b, 8);
    // 8 parallel AND gates — one per bit position.
    const resultBits = aBits.map((ab, i) => AND(ab, bBits[i])) as Bit[];
    return bitsToInt(resultBits);
  }

  /**
   * Bitwise OR of two 8-bit values via 8 OR gates.
   *
   * OR always clears carry per 8008 spec.
   */
  bitwiseOr(a: number, b: number): number {
    const aBits = intToBits(a, 8);
    const bBits = intToBits(b, 8);
    const resultBits = aBits.map((ab, i) => OR(ab, bBits[i])) as Bit[];
    return bitsToInt(resultBits);
  }

  /**
   * Bitwise XOR of two 8-bit values via 8 XOR gates.
   *
   * XOR always clears carry per 8008 spec.
   */
  bitwiseXor(a: number, b: number): number {
    const aBits = intToBits(a, 8);
    const bBits = intToBits(b, 8);
    const resultBits = aBits.map((ab, i) => XOR(ab, bBits[i])) as Bit[];
    return bitsToInt(resultBits);
  }

  /**
   * Increment A by 1 via ripple_carry_adder(A, 0, cin=1).
   *
   * Returns [result, carry]. Note: INR in 8008 does NOT update CY —
   * the carry from this adder is discarded by the control unit.
   */
  increment(a: number): [number, Bit] {
    const aBits = intToBits(a, 8);
    const zeroBits = intToBits(0, 8);
    const result = this._alu.execute(ALUOp.ADD, aBits, zeroBits);
    // Add 1 by treating it as ADD(a, 0) + 1 carry-in.
    // Simpler: just add 1 directly.
    const oneBits = intToBits(1, 8);
    const result2 = this._alu.execute(ALUOp.ADD, aBits, oneBits);
    return [
      bitsToInt(result2.value as Bit[]),
      result2.carry ? 1 as Bit : 0 as Bit,
    ];
  }

  /**
   * Decrement A by 1 via A + 0xFF (adding -1 in two's complement).
   *
   * Returns [result, borrow]. Note: DCR in 8008 does NOT update CY.
   *
   * === Why A + 0xFF = A - 1 ===
   *
   * 0xFF is the two's complement representation of -1 for an 8-bit number.
   * Adding -1 gives the same result as subtracting 1.
   *
   *   5 + 0xFF = 5 + 255 = 260 = 0x104 → low 8 bits = 0x04 = 4 (with carry)
   *   0x00 + 0xFF = 255 = 0xFF → low 8 bits = 0xFF = 255 (no carry, so borrow)
   */
  decrement(a: number): [number, Bit] {
    const aBits = intToBits(a, 8);
    const ffBits = intToBits(0xFF, 8);
    const result = this._alu.execute(ALUOp.ADD, aBits, ffBits);
    // Borrow occurred when A=0 (result=0xFF with carry=0).
    // NOT(carry) gives borrow flag.
    const carry = result.carry ? 1 as Bit : 0 as Bit;
    return [bitsToInt(result.value as Bit[]), NOT(carry) as Bit];
  }

  /**
   * Rotate A left circular: CY ← A[7]; A[0] ← A[7].
   *
   * In hardware, this is pure wiring — no arithmetic gates:
   *   new_A = [A[7], A[6], A[5], A[4], A[3], A[2], A[1], A[0]] shifted left
   *   i.e., new_A[0] = old_A[7], new_A[i] = old_A[i-1]
   *
   * @returns [rotated_A, carry] where carry = bit[7] of original A.
   */
  rotateLeftCircular(a: number): [number, Bit] {
    const bits = intToBits(a, 8);
    const bit7 = bits[7];  // MSB — becomes new bit0 and carry
    // Rotate: shift all bits up by 1 position, wrap MSB to bit0.
    const rotated: Bit[] = [bit7, ...bits.slice(0, 7)] as Bit[];
    return [bitsToInt(rotated), bit7];
  }

  /**
   * Rotate A right circular: CY ← A[0]; A[7] ← A[0].
   *
   * Pure wiring: new_A[7] = old_A[0], new_A[i] = old_A[i+1].
   *
   * @returns [rotated_A, carry] where carry = bit[0] of original A.
   */
  rotateRightCircular(a: number): [number, Bit] {
    const bits = intToBits(a, 8);
    const bit0 = bits[0];  // LSB — becomes new bit7 and carry
    const rotated: Bit[] = [...bits.slice(1), bit0] as Bit[];
    return [bitsToInt(rotated), bit0];
  }

  /**
   * Rotate A left through carry (9-bit rotation).
   *
   * The 9-bit register [CY | A7..A0] rotates left by 1:
   *   new_CY ← A[7]; new_A[0] ← old_CY
   *
   * @param a - Accumulator value.
   * @param cy - Current carry flag.
   * @returns [rotated_A, new_carry].
   */
  rotateLeftCarry(a: number, cy: Bit): [number, Bit] {
    const bits = intToBits(a, 8);
    const bit7 = bits[7];  // Will become new carry
    // Shift left: bit[i] ← old_bit[i-1], bit[0] ← old_CY
    const rotated: Bit[] = [cy, ...bits.slice(0, 7)] as Bit[];
    return [bitsToInt(rotated), bit7];
  }

  /**
   * Rotate A right through carry (9-bit rotation).
   *
   * The 9-bit register [A7..A0 | CY] rotates right by 1:
   *   new_CY ← A[0]; new_A[7] ← old_CY
   *
   * @param a - Accumulator value.
   * @param cy - Current carry flag.
   * @returns [rotated_A, new_carry].
   */
  rotateRightCarry(a: number, cy: Bit): [number, Bit] {
    const bits = intToBits(a, 8);
    const bit0 = bits[0];  // Will become new carry
    const rotated: Bit[] = [...bits.slice(1), cy] as Bit[];
    return [bitsToInt(rotated), bit0];
  }

  /**
   * Compute flag register bits from an 8-bit value and carry.
   *
   * This wraps computeFlags with integer input for convenience.
   *
   * @param result - 8-bit result integer.
   * @param carry  - Carry/borrow bit.
   */
  flagsFromResult(result: number, carry: Bit): GateFlags {
    const resultBits = intToBits(result, 8);
    return this.computeFlags(resultBits, carry);
  }
}
