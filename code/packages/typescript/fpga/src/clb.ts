/**
 * Configurable Logic Block (CLB) -- the core compute tile of an FPGA.
 *
 * === What is a CLB? ===
 *
 * A CLB is the primary logic resource in an FPGA. It's a tile on the FPGA
 * grid that contains multiple slices, each with LUTs, flip-flops, and carry
 * chains. CLBs are connected to each other through the routing fabric.
 *
 * === CLB Architecture ===
 *
 * Our CLB follows the Xilinx-style architecture with 2 slices:
 *
 *     +----------------------------------------------+
 *     |                     CLB                       |
 *     |                                               |
 *     |  +---------------------+                      |
 *     |  |       Slice 0       |                      |
 *     |  |  [LUT A] [LUT B]   |                      |
 *     |  |  [FF A]  [FF B]    |                      |
 *     |  |  [carry chain]      |                      |
 *     |  +---------+-----------+                      |
 *     |            | carry                            |
 *     |  +---------v-----------+                      |
 *     |  |       Slice 1       |                      |
 *     |  |  [LUT A] [LUT B]   |                      |
 *     |  |  [FF A]  [FF B]    |                      |
 *     |  |  [carry chain]      |                      |
 *     |  +---------------------+                      |
 *     |                                               |
 *     +----------------------------------------------+
 *
 * The carry chain flows from slice 0 -> slice 1, enabling fast multi-bit
 * arithmetic within a single CLB.
 */

import { type Bit } from "@coding-adventures/logic-gates";
import { Slice, type SliceOutput } from "./slice.js";

/**
 * Output from a CLB evaluation.
 */
export interface CLBOutput {
  /** Output from slice 0. */
  readonly slice0: SliceOutput;
  /** Output from slice 1. */
  readonly slice1: SliceOutput;
}

/**
 * Configurable Logic Block -- contains 2 slices.
 *
 * The carry chain connects slice 0's carryOut to slice 1's carryIn,
 * enabling fast multi-bit arithmetic.
 *
 * @example
 * const clb = new CLB(4);
 * // Configure each slice's LUTs
 * const xorTt = Array(16).fill(0) as Bit[];
 * xorTt[1] = 1; xorTt[2] = 1;
 * const andTt = Array(16).fill(0) as Bit[];
 * andTt[3] = 1;
 * clb.slice0.configure(xorTt, andTt, false, false, true);
 * clb.slice1.configure(xorTt, andTt, false, false, true);
 */
export class CLB {
  private readonly _slice0: Slice;
  private readonly _slice1: Slice;
  private readonly _k: number;

  /**
   * @param lutInputs - Number of inputs per LUT (2 to 6, default 4)
   */
  constructor(lutInputs: number = 4) {
    this._slice0 = new Slice(lutInputs);
    this._slice1 = new Slice(lutInputs);
    this._k = lutInputs;
  }

  /** First slice. */
  get slice0(): Slice {
    return this._slice0;
  }

  /** Second slice. */
  get slice1(): Slice {
    return this._slice1;
  }

  /** Number of LUT inputs per slice. */
  get k(): number {
    return this._k;
  }

  /**
   * Evaluate both slices in the CLB.
   *
   * The carry chain flows: carryIn -> slice0 -> slice1.
   *
   * @param slice0InputsA - Inputs to slice 0's LUT A
   * @param slice0InputsB - Inputs to slice 0's LUT B
   * @param slice1InputsA - Inputs to slice 1's LUT A
   * @param slice1InputsB - Inputs to slice 1's LUT B
   * @param clock - Clock signal (0 or 1)
   * @param carryIn - External carry input (default 0)
   * @returns CLBOutput containing both slices' outputs.
   */
  evaluate(
    slice0InputsA: Bit[],
    slice0InputsB: Bit[],
    slice1InputsA: Bit[],
    slice1InputsB: Bit[],
    clock: Bit,
    carryIn: Bit = 0,
  ): CLBOutput {
    // Evaluate slice 0 first (carry chain starts here)
    const out0 = this._slice0.evaluate(
      slice0InputsA,
      slice0InputsB,
      clock,
      carryIn,
    );

    // Slice 1 receives carry from slice 0
    const out1 = this._slice1.evaluate(
      slice1InputsA,
      slice1InputsB,
      clock,
      out0.carryOut,
    );

    return { slice0: out0, slice1: out1 };
  }
}
