/**
 * Adder circuits built from logic gates.
 *
 * # Moving from Logic to Math
 *
 * In the logic-gates package, we saw how transistors combine to form gates
 * that perform basic Boolean operations (AND, OR, XOR). But how do we get a
 * computer to do actual math?
 *
 * This module answers that question. By creatively wiring together those
 * fundamental logic gates, we can build circuits that add binary numbers.
 * From a simple "Half Adder" that adds two individual bits, we build up to
 * a "Ripple Carry Adder" that adds multi-bit numbers — the same way you
 * add decimal numbers on paper, column by column, carrying as you go.
 *
 * Half adder: adds two bits.
 * Full adder: adds two bits + carry-in.
 * Ripple carry adder: chains full adders for N-bit addition.
 */

import { AND, OR, XOR, type Bit } from "@coding-adventures/logic-gates";

/**
 * Add two single bits.
 *
 * # Why "Half"?
 *
 * Adding two binary bits is simple, but we have to account for carrying over
 * to the next column — exactly like grade-school addition:
 *
 *       1
 *     + 1
 *     ---
 *      10  (which is 2 in binary)
 *
 * In the 1s column, the sum is 0, and we "carry" a 1 to the next column.
 * The Half Adder produces both these outputs: a Sum bit and a Carry bit.
 * It is called a "Half" adder because, while it can generate a carry, it
 * cannot ACCEPT a carry input from a previous column.
 *
 * Truth table:
 *
 *     A | B | Sum | Carry
 *     --|---|-----|------
 *     0 | 0 |  0  |   0
 *     0 | 1 |  1  |   0
 *     1 | 0 |  1  |   0
 *     1 | 1 |  0  |   1
 *
 * If you look closely at the truth table:
 * - Sum is exactly the XOR operation (1 only when inputs differ).
 * - Carry is exactly the AND operation (1 only when both inputs are 1).
 *
 * @param a - First bit (0 or 1)
 * @param b - Second bit (0 or 1)
 * @returns [sum, carry] — the sum bit and the carry bit
 */
export function halfAdder(a: Bit, b: Bit): [Bit, Bit] {
  const sumBit = XOR(a, b);
  const carry = AND(a, b);
  return [sumBit, carry];
}

/**
 * Add two bits plus a carry-in from a previous addition.
 *
 * # Handling the Ripple
 *
 * To add multi-bit numbers, every column beyond the first might receive a
 * carry from the column to its right. A Full Adder takes three inputs (A, B,
 * and CarryIn) and correctly produces a Sum and a CarryOut.
 *
 * Built from two half adders and an OR gate:
 *   1. Half-add A and B → partial_sum, partial_carry
 *   2. Half-add partial_sum and carry_in → sum, carry2
 *   3. carry_out = OR(partial_carry, carry2)
 *
 * If EITHER step generated a carry, our final CarryOut is 1.
 *
 * Truth table:
 *
 *     A | B | Cin | Sum | Cout
 *     --|---|-----|-----|-----
 *     0 | 0 |  0  |  0  |  0
 *     0 | 0 |  1  |  1  |  0
 *     0 | 1 |  0  |  1  |  0
 *     0 | 1 |  1  |  0  |  1
 *     1 | 0 |  0  |  1  |  0
 *     1 | 0 |  1  |  0  |  1
 *     1 | 1 |  0  |  0  |  1
 *     1 | 1 |  1  |  1  |  1
 *
 * @param a - First bit
 * @param b - Second bit
 * @param carryIn - Carry from previous column
 * @returns [sum, carryOut]
 */
export function fullAdder(a: Bit, b: Bit, carryIn: Bit): [Bit, Bit] {
  const [partialSum, partialCarry] = halfAdder(a, b);
  const [sumBit, carry2] = halfAdder(partialSum, carryIn);
  const carryOut = OR(partialCarry, carry2);
  return [sumBit, carryOut];
}

/**
 * Add two N-bit numbers using a chain of full adders.
 *
 * # The Ripple Effect
 *
 * Just like you add large numbers on paper starting from the rightmost digit
 * and moving left, the Ripple Carry Adder lines up a series of Full Adders.
 * The CarryOut of bit 0 is wired directly into the CarryIn of bit 1. The
 * CarryOut of bit 1 goes into bit 2, and so on.
 *
 * The worst-case performance is when adding something like 1111 + 0001. The
 * carry generated at the first bit must "ripple" all the way through every
 * single adder before the final sum is ready. In physical hardware, this
 * takes time, which is why modern CPUs use faster tricks like "Carry Lookahead".
 *
 * ```
 *      A3 B3      A2 B2      A1 B1      A0 B0
 *       |  |       |  |       |  |       |  |
 *     +----+     +----+     +----+     +----+
 *     | FA |<----| FA |<----| FA |<----| FA |<-- 0 (initial carry)
 *     +----+     +----+     +----+     +----+
 *        S3         S2         S1         S0
 * ```
 *
 * @param a - First number as array of bits, LSB first (index 0 = least significant).
 * @param b - Second number as array of bits, LSB first.
 * @param carryIn - Initial carry (default 0).
 * @returns [sumBits, carryOut] where sumBits is LSB first.
 *
 * @example
 * ```ts
 * // 5 + 3 = 8
 * const a: Bit[] = [1, 0, 1, 0];  // 5 in binary (LSB first: 1*1 + 0*2 + 1*4 + 0*8)
 * const b: Bit[] = [1, 1, 0, 0];  // 3 in binary (LSB first: 1*1 + 1*2 + 0*4 + 0*8)
 * const [result, carry] = rippleCarryAdder(a, b);
 * // result = [0, 0, 0, 1], carry = 0  → 8 in binary
 * ```
 */
export function rippleCarryAdder(
  a: Bit[],
  b: Bit[],
  carryIn: Bit = 0
): [Bit[], Bit] {
  if (a.length !== b.length) {
    throw new Error(
      `a and b must have the same length, got ${a.length} and ${b.length}`
    );
  }
  if (a.length === 0) {
    throw new Error("bit lists must not be empty");
  }

  const sumBits: Bit[] = [];
  let carry: Bit = carryIn;

  for (let i = 0; i < a.length; i++) {
    const [sumBit, newCarry] = fullAdder(a[i], b[i], carry);
    sumBits.push(sumBit);
    carry = newCarry;
  }

  return [sumBits, carry];
}
