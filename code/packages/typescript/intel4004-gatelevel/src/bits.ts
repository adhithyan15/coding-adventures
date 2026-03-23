/**
 * Bit conversion helpers -- the bridge between integers and gate-level bits.
 *
 * === Why this module exists ===
 *
 * The gate-level simulator operates on individual bits (arrays of 0s and 1s),
 * because that's what real hardware does. But the outside world (test programs,
 * the behavioral simulator) works with integers. This module converts between
 * the two representations.
 *
 * === Bit ordering: LSB first ===
 *
 * All bit arrays use LSB-first ordering, matching the logic-gates and arithmetic
 * packages. Index 0 is the least significant bit.
 *
 *     intToBits(5, 4)  =>  [1, 0, 1, 0]
 *     //                    bit0=1(x1) + bit1=0(x2) + bit2=1(x4) + bit3=0(x8) = 5
 *
 * This convention is used throughout the computing stack because it maps
 * naturally to how adders chain: bit 0 feeds the first full adder, bit 1
 * feeds the second, and so on.
 */

import { type Bit } from "@coding-adventures/logic-gates";

/**
 * Convert an integer to an array of bits (LSB first).
 *
 * @param value - Non-negative integer to convert.
 * @param width - Number of bits in the output array.
 * @returns Array of 0s and 1s, length = width, LSB at index 0.
 *
 * @example
 * intToBits(5, 4)      // => [1, 0, 1, 0]
 * intToBits(0, 4)      // => [0, 0, 0, 0]
 * intToBits(15, 4)     // => [1, 1, 1, 1]
 * intToBits(0xABC, 12) // => [0, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1]
 */
export function intToBits(value: number, width: number): Bit[] {
  // Mask to width to handle negative or oversized values
  const masked = value & ((1 << width) - 1);
  const bits: Bit[] = [];
  for (let i = 0; i < width; i++) {
    bits.push(((masked >> i) & 1) as Bit);
  }
  return bits;
}

/**
 * Convert an array of bits (LSB first) to an integer.
 *
 * @param bits - Array of 0s and 1s, LSB at index 0.
 * @returns Non-negative integer.
 *
 * @example
 * bitsToInt([1, 0, 1, 0]) // => 5
 * bitsToInt([0, 0, 0, 0]) // => 0
 * bitsToInt([1, 1, 1, 1]) // => 15
 */
export function bitsToInt(bits: Bit[]): number {
  let result = 0;
  for (let i = 0; i < bits.length; i++) {
    result |= bits[i] << i;
  }
  return result;
}
