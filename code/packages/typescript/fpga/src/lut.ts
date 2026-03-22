/**
 * Look-Up Table (LUT) -- the atom of programmable logic.
 *
 * === What is a LUT? ===
 *
 * A Look-Up Table is the fundamental building block of every FPGA. The key
 * insight behind programmable logic is deceptively simple:
 *
 *     **A truth table IS a program.**
 *
 * Any boolean function of K inputs can be described by a truth table with
 * 2^K entries. A K-input LUT stores that truth table in SRAM and uses a
 * MUX tree to select the correct output for any combination of inputs.
 *
 * This means a single LUT can implement ANY boolean function of K variables:
 * AND, OR, XOR, majority vote, parity -- anything. To "reprogram" the LUT,
 * you just load a different truth table into the SRAM.
 *
 * === How it works ===
 *
 * A 4-input LUT (K=4) has:
 * - 16 SRAM cells (2^4 = 16 truth table entries)
 * - A 16-to-1 MUX tree (built from 2:1 MUXes)
 * - 4 input signals that act as MUX select lines
 *
 * Example -- configuring a LUT as a 2-input AND gate (using only I0, I1):
 *
 *     Inputs -> Truth Table Entry -> Output
 *     I3 I2 I1 I0
 *      0  0  0  0  -> SRAM[0]  = 0
 *      0  0  0  1  -> SRAM[1]  = 0
 *      0  0  1  0  -> SRAM[2]  = 0
 *      0  0  1  1  -> SRAM[3]  = 1  <- only case where I0 AND I1 = 1
 *      0  1  0  0  -> SRAM[4]  = 0
 *      ...           (all others = 0 since we only care about I0, I1)
 *
 * The truth table index is computed as:
 *     index = I0 + 2*I1 + 4*I2 + 8*I3  (binary number with I0 as LSB)
 *
 * Then the MUX tree selects SRAM[index] as the output.
 */

import { type Bit, validateBit, muxN } from "@coding-adventures/logic-gates";
import { SRAMCell } from "@coding-adventures/block-ram";

/**
 * K-input Look-Up Table -- the atom of programmable logic.
 *
 * A LUT stores a truth table in SRAM cells and uses a MUX tree to
 * select the output based on input signals. It can implement ANY
 * boolean function of K variables.
 *
 * @example
 * // 2-input AND gate in a 4-input LUT:
 * const andTable = Array(16).fill(0) as Bit[];
 * andTable[3] = 1;  // I0=1, I1=1 -> index = 1 + 2 = 3
 * const lut = new LUT(4, andTable);
 * lut.evaluate([0, 0, 0, 0])  // 0
 * lut.evaluate([1, 1, 0, 0])  // 1 (I0=1, I1=1)
 */
export class LUT {
  private readonly _k: number;
  private readonly _size: number;
  private readonly _sram: SRAMCell[];

  /**
   * @param k - Number of inputs (2 to 6, default 4)
   * @param truthTable - Initial truth table (2^k entries, each 0 or 1).
   *                     If undefined, all entries default to 0.
   */
  constructor(k: number = 4, truthTable?: Bit[]) {
    if (typeof k !== "number" || !Number.isInteger(k)) {
      throw new TypeError(`k must be an integer, got ${typeof k}`);
    }
    if (k < 2 || k > 6) {
      throw new RangeError(`k must be between 2 and 6, got ${k}`);
    }

    this._k = k;
    this._size = 1 << k; // 2^k
    this._sram = Array.from({ length: this._size }, () => new SRAMCell());

    if (truthTable !== undefined) {
      this.configure(truthTable);
    }
  }

  /**
   * Load a new truth table (reprogram the LUT).
   *
   * @param truthTable - Array of 2^k bits (each 0 or 1).
   * @throws RangeError if length doesn't match 2^k or entries aren't 0/1.
   * @throws TypeError if truthTable is not an array.
   */
  configure(truthTable: Bit[]): void {
    if (!Array.isArray(truthTable)) {
      throw new TypeError("truthTable must be an array of bits");
    }
    if (truthTable.length !== this._size) {
      throw new RangeError(
        `truthTable length ${truthTable.length} does not match 2^k = ${this._size}`,
      );
    }

    for (let i = 0; i < truthTable.length; i++) {
      validateBit(truthTable[i], `truthTable[${i}]`);
    }

    // Program each SRAM cell
    for (let i = 0; i < truthTable.length; i++) {
      this._sram[i].write(1, truthTable[i]);
    }
  }

  /**
   * Compute the LUT output for the given inputs.
   *
   * Uses a MUX tree (via muxN) to select the correct truth table
   * entry based on the input signals.
   *
   * @param inputs - Array of k input bits (each 0 or 1).
   *                 inputs[0] = I0 (LSB of truth table index)
   *                 inputs[k-1] = I_{k-1} (MSB of truth table index)
   * @returns The truth table output (0 or 1).
   */
  evaluate(inputs: Bit[]): Bit {
    if (!Array.isArray(inputs)) {
      throw new TypeError("inputs must be an array of bits");
    }
    if (inputs.length !== this._k) {
      throw new RangeError(
        `inputs length ${inputs.length} does not match k = ${this._k}`,
      );
    }

    for (let i = 0; i < inputs.length; i++) {
      validateBit(inputs[i], `inputs[${i}]`);
    }

    // Read all SRAM cells to form the MUX data inputs
    const data: Bit[] = [];
    for (const cell of this._sram) {
      const val = cell.read(1);
      data.push(val as Bit);
    }

    // Use MUX tree to select the output
    return muxN(data, inputs);
  }

  /** Number of inputs. */
  get k(): number {
    return this._k;
  }

  /** Current truth table (copy). */
  get truthTable(): Bit[] {
    return this._sram.map((cell) => cell.read(1) as Bit);
  }
}
