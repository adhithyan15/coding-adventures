/**
 * Integer multiplication using shift-and-add — built from adders and AND gates.
 *
 * # Everything Reduces to Addition
 *
 * Binary multiplication works exactly like the long multiplication you learned
 * in school, but simpler because each digit is only 0 or 1. For each bit of
 * the multiplier:
 *   - If the bit is 1, add the multiplicand (shifted left by the bit position)
 *   - If the bit is 0, add nothing (skip)
 *
 * That's it. Multiplication is just repeated, conditional addition.
 *
 * # Worked Example: 5 × 3 = 15
 *
 * ```
 *       0101  (A = 5, the multiplicand)
 *     × 0011  (B = 3, the multiplier)
 *     ------
 *       0101  (bit 0 of B is 1 → add A << 0)
 *      0101   (bit 1 of B is 1 → add A << 1)
 *     0000    (bit 2 of B is 0 → skip)
 *    0000     (bit 3 of B is 0 → skip)
 *    --------
 *    0001111  (= 15 ✓)
 * ```
 *
 * # Hardware Implementation
 *
 * In a real chip, each "multiply by one bit" is an AND gate array: the
 * multiplier bit is ANDed with every bit of the multiplicand. If the
 * multiplier bit is 0, the AND gates output all zeros (no contribution).
 * If it's 1, the AND gates pass the multiplicand through unchanged.
 *
 * Each partial product is then added to the running total using a
 * ripple-carry adder. A 4-bit multiplier needs at most 3 additions
 * (the first partial product IS the initial running total).
 *
 * # Why Double-Width Output?
 *
 * The product of two N-bit numbers can be up to 2N bits wide:
 *   - 4-bit max: 15 × 15 = 225, which needs 8 bits (11100001)
 *   - 8-bit max: 255 × 255 = 65025, which needs 16 bits
 *
 * This is why CPU multiply instructions often produce a double-width result,
 * and why "overflow" is a concern in fixed-width arithmetic.
 */

import { AND, type Bit } from "@coding-adventures/logic-gates";
import { rippleCarryAdder } from "./adders.js";

/**
 * One step of the shift-and-add multiplication algorithm.
 *
 * Each step examines one bit of the multiplier and either adds the
 * shifted multiplicand to the running total (if the bit is 1) or
 * skips (if the bit is 0). The trace captures both the partial
 * product and the accumulated running total for visualization.
 */
export interface MultiplierStep {
  /** Which bit of the multiplier we're examining (0 = LSB). */
  bitIndex: number;

  /** The value of this multiplier bit (0 = skip, 1 = add). */
  multiplierBit: Bit;

  /**
   * The partial product for this step: multiplicand shifted left by bitIndex
   * positions (if multiplierBit = 1) or all zeros (if multiplierBit = 0).
   * Width is 2N to accommodate the full product.
   */
  partialProduct: Bit[];

  /**
   * The accumulated sum after adding this step's partial product.
   * This is what "long multiplication on paper" looks like after each row.
   */
  runningTotal: Bit[];

  /** Carry out from the addition (usually 0 for intermediate steps). */
  carryOut: Bit;
}

/**
 * Complete result of a traced shift-and-add multiplication.
 *
 * Contains the final product plus step-by-step trace data showing
 * how each partial product was generated and accumulated.
 */
export interface MultiplierResult {
  /** The multiplicand (first input, unchanged). */
  a: Bit[];

  /** The multiplier (second input, unchanged). */
  b: Bit[];

  /** The final product as a 2N-bit array (LSB first). */
  product: Bit[];

  /**
   * Per-bit trace: one step for each bit of the multiplier.
   * steps[0] is for the LSB, steps[N-1] is for the MSB.
   */
  steps: MultiplierStep[];
}

/**
 * Multiply two N-bit unsigned integers using the shift-and-add algorithm.
 *
 * Returns a traced result with step-by-step partial products for
 * visualization. The algorithm processes the multiplier from LSB to MSB,
 * adding shifted copies of the multiplicand to a running total.
 *
 * Both inputs must have the same length (N bits, LSB first). The output
 * product is 2N bits wide (LSB first) to hold the full result without
 * overflow.
 *
 * @param a - Multiplicand as bits (LSB first)
 * @param b - Multiplier as bits (LSB first)
 * @returns MultiplierResult with product and per-step trace
 *
 * @example
 * ```ts
 * // 5 × 3 = 15
 * const a: Bit[] = [1, 0, 1, 0];  // 5 (LSB first)
 * const b: Bit[] = [1, 1, 0, 0];  // 3 (LSB first)
 * const result = shiftAndAddMultiplier(a, b);
 * // result.product = [1, 1, 1, 1, 0, 0, 0, 0]  → 15 (LSB first)
 * // result.steps has 4 entries showing each partial product
 * ```
 */
export function shiftAndAddMultiplier(a: Bit[], b: Bit[]): MultiplierResult {
  if (a.length !== b.length) {
    throw new Error(
      `a and b must have the same length, got ${a.length} and ${b.length}`
    );
  }
  if (a.length === 0) {
    throw new Error("bit lists must not be empty");
  }

  const n = a.length;
  const doubleWidth = 2 * n;

  // The running total starts at zero (2N bits wide).
  let runningTotal: Bit[] = new Array<Bit>(doubleWidth).fill(0 as Bit);
  const steps: MultiplierStep[] = [];

  for (let i = 0; i < n; i++) {
    const multiplierBit = b[i];

    // Generate the partial product for this bit position.
    //
    // In hardware, this is an array of AND gates: each bit of the
    // multiplicand is ANDed with the current multiplier bit.
    //   - If multiplierBit = 1: partial product = a (the multiplicand passes through)
    //   - If multiplierBit = 0: partial product = all zeros (AND gates block everything)
    //
    // The partial product is then shifted left by `i` positions (the bit index).
    // In hardware, "shifting" is just wiring — no gates needed, just connect
    // the wires to offset positions.
    const partialProduct: Bit[] = new Array<Bit>(doubleWidth).fill(0 as Bit);
    for (let j = 0; j < n; j++) {
      // AND(a[j], multiplierBit): gate the multiplicand bit
      // Place at position j + i: this is the "shift left by i"
      partialProduct[j + i] = AND(a[j], multiplierBit);
    }

    // Add the partial product to the running total using a ripple-carry adder.
    // This is the "add" part of "shift-and-add".
    const [newTotal, carryOut] = rippleCarryAdder(runningTotal, partialProduct);

    runningTotal = newTotal;

    steps.push({
      bitIndex: i,
      multiplierBit,
      partialProduct,
      runningTotal: [...runningTotal],
      carryOut,
    });
  }

  return {
    a: [...a],
    b: [...b],
    product: runningTotal,
    steps,
  };
}
