/**
 * Bit conversion helpers — the bridge between integers and gate-level bits.
 *
 * === Why this module exists ===
 *
 * The gate-level simulator operates on individual bits (arrays of 0s and 1s),
 * because that's what real hardware does. Transistors switch between two voltage
 * levels (logic HIGH and logic LOW), and we model that directly as 0 and 1.
 *
 * The outside world works with integers. This module provides:
 * - `intToBits`: integer → bit array (for feeding values into gates)
 * - `bitsToInt`: bit array → integer (for reading results from gates)
 * - `computeParity`: parity of a bit array (for the 8008 Parity flag)
 *
 * === Bit ordering: LSB first ===
 *
 * All bit arrays use LSB-first ordering, matching the logic-gates and arithmetic
 * packages' convention. Index 0 is the least significant bit (value 1 = 2^0).
 *
 *     intToBits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
 *                        ^                     ^
 *                        bit0 (value=1)        bit7 (value=128)
 *
 * This maps naturally to how ripple-carry adders chain: bit 0 feeds the first
 * full adder, bit 1 feeds the second, and so on.
 *
 * === Parity ===
 *
 * The Intel 8008 Parity flag (P) is set when the result has an EVEN number of
 * 1-bits. It is computed via XOR reduction followed by NOT:
 *
 *     parity_xor = XOR(bit0, XOR(bit1, XOR(bit2, ...)))
 *     P = NOT(parity_xor)
 *
 * XOR of N bits is 0 when N has even parity (even count of 1s), so NOT flips
 * this to 1 (P=1 = even parity). This matches the 8008 convention exactly.
 *
 * The parity tree for 8 bits requires 7 XOR gates:
 *
 *     parity = XOR(XOR(XOR(b0,b1), XOR(b2,b3)), XOR(XOR(b4,b5), XOR(b6,b7)))
 */

import { NOT, xorN, type Bit } from "@coding-adventures/logic-gates";

/**
 * Convert an integer to an array of bits (LSB first).
 *
 * Values larger than `width` bits are silently truncated by masking.
 * Negative values are masked to their two's complement representation.
 *
 * @param value - Non-negative integer to convert.
 * @param width - Number of bits in the output array (must be ≥ 1).
 * @returns Array of 0s and 1s with length = `width`, LSB at index 0.
 *
 * @example
 * intToBits(5, 8)       // => [1, 0, 1, 0, 0, 0, 0, 0]   (5 = 2^0 + 2^2)
 * intToBits(0, 4)       // => [0, 0, 0, 0]
 * intToBits(255, 8)     // => [1, 1, 1, 1, 1, 1, 1, 1]
 * intToBits(0x3FFF, 14) // => [1,1,1,1,1,1,1,1,1,1,1,1,1,1] (14-bit max)
 */
export function intToBits(value: number, width: number): Bit[] {
  // Mask to `width` bits. The formula (1 << width) - 1 gives a mask with
  // the lowest `width` bits all set. For width=8: 0xFF, width=14: 0x3FFF.
  const masked = value & ((1 << width) - 1);
  const bits: Bit[] = [];
  for (let i = 0; i < width; i++) {
    // Shift right by i and mask to 1 bit (LSB first).
    bits.push(((masked >> i) & 1) as Bit);
  }
  return bits;
}

/**
 * Convert an array of bits (LSB first) to an integer.
 *
 * Each bit at index i contributes 2^i to the result. This reverses `intToBits`.
 *
 * @param bits - Array of 0s and 1s, LSB at index 0.
 * @returns Non-negative integer (sum of bit[i] × 2^i).
 *
 * @example
 * bitsToInt([1, 0, 1, 0, 0, 0, 0, 0]) // => 5  (bit0=1 + bit2=1 = 1+4 = 5)
 * bitsToInt([0, 0, 0, 0])              // => 0
 * bitsToInt([1, 1, 1, 1])              // => 15 (2^0+2^1+2^2+2^3)
 */
export function bitsToInt(bits: Bit[]): number {
  let result = 0;
  for (let i = 0; i < bits.length; i++) {
    // OR in the bit shifted to its position. Using |= is safe for 14-bit values.
    result |= bits[i] << i;
  }
  return result;
}

/**
 * Compute parity of a bit array using XOR reduction (gate-level).
 *
 * Returns 1 when the bit array has an EVEN number of 1-bits (even parity).
 * Returns 0 when the bit array has an ODD number of 1-bits (odd parity).
 *
 * This matches the Intel 8008's Parity flag convention: P=1 = even parity.
 *
 * === Implementation ===
 *
 * The XOR gate is the parity element of Boolean algebra:
 *   XOR(a, b) = 1 iff exactly one of a, b is 1 (odd parity of 2 bits)
 *
 * Chaining XOR gates (the xorN function from logic-gates) produces a
 * multi-bit parity checker. If the XOR of all bits is 1, there are an
 * odd number of 1s. NOT inverts this to the 8008 convention.
 *
 * For 8 bits, this can be optimized as a balanced tree:
 *
 *   Level 0 (leaf pairs):
 *     x01 = XOR(b0, b1)    x23 = XOR(b2, b3)
 *     x45 = XOR(b4, b5)    x67 = XOR(b6, b7)
 *   Level 1:
 *     x0123 = XOR(x01, x23)    x4567 = XOR(x45, x67)
 *   Level 2 (root):
 *     xall  = XOR(x0123, x4567)
 *   Invert:
 *     P = NOT(xall)   [1 = even parity]
 *
 * This balanced tree has 7 XOR gates + 1 NOT gate = 8 gates for 8-bit parity.
 * The linear xorN chain has the same gate count but longer critical path.
 *
 * @param bits - Array of 0s and 1s (typically 8 bits for ALU result parity).
 * @returns 1 if even parity (even number of 1s), 0 if odd parity.
 *
 * @example
 * computeParity([0,0,0,0,0,0,0,0]) // => 1  (0 ones = even)
 * computeParity([1,0,0,0,0,0,0,0]) // => 0  (1 one = odd)
 * computeParity([1,1,0,0,0,0,0,0]) // => 1  (2 ones = even)
 * computeParity([1,1,1,0,0,0,0,0]) // => 0  (3 ones = odd)
 */
export function computeParity(bits: Bit[]): Bit {
  if (bits.length === 0) return 1 as Bit;
  if (bits.length === 1) return NOT(bits[0]);
  // xorN reduces all bits via chained XOR gates (from logic-gates package).
  // Result is 1 if odd number of 1s, 0 if even number of 1s.
  // NOT inverts to 8008 convention (1 = even parity).
  return NOT(xorN(...bits)) as Bit;
}
