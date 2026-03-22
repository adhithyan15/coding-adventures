/**
 * Slice -- the building block of a Configurable Logic Block (CLB).
 *
 * === What is a Slice? ===
 *
 * A slice is one "lane" inside a CLB. It combines:
 * - 2 LUTs (A and B) for combinational logic
 * - 2 D flip-flops for registered (sequential) outputs
 * - 2 output MUXes that choose between combinational or registered output
 * - Carry chain logic for fast arithmetic
 *
 * The output MUX is critical: it lets the same slice be used for both
 * combinational circuits (bypass the flip-flop) and sequential circuits
 * (register the LUT output on the clock edge).
 *
 * === Slice Architecture ===
 *
 *     inputs_a --> [LUT A] --> +----------+
 *                              | MUX_A    |--> output_a
 *                  +-> [FF A]->|(sel=ff_a)|
 *                  |           +----------+
 *                  |
 *     inputs_b --> [LUT B] --> +----------+
 *                              | MUX_B    |--> output_b
 *                  +-> [FF B]->|(sel=ff_b)|
 *                  |           +----------+
 *                  |
 *     carry_in --> [CARRY] -----------------> carry_out
 *
 *     clock -----> [FF A] [FF B]
 *
 * === Carry Chain ===
 *
 * For arithmetic operations, the carry chain connects adjacent slices
 * to propagate carry bits without going through the general routing
 * fabric. This is what makes FPGA arithmetic fast -- dedicated carry
 * logic is hardwired between slices.
 *
 * Our carry chain computes:
 *     carry_out = (LUT_A_out AND LUT_B_out) OR (carry_in AND (LUT_A_out XOR LUT_B_out))
 *
 * This is the standard full-adder carry equation where LUT_A computes
 * the generate signal and LUT_B computes the propagate signal.
 */

import {
  type Bit,
  AND,
  OR,
  XOR,
  mux2,
  dFlipFlop,
  type FlipFlopState,
} from "@coding-adventures/logic-gates";
import { LUT } from "./lut.js";

/**
 * Output from a single slice evaluation.
 */
export interface SliceOutput {
  /** LUT A result (combinational or registered). */
  readonly outputA: Bit;
  /** LUT B result (combinational or registered). */
  readonly outputB: Bit;
  /** Carry chain output (0 if carry disabled). */
  readonly carryOut: Bit;
}

/**
 * One slice of a CLB: 2 LUTs + 2 flip-flops + output MUXes + carry chain.
 *
 * @example
 * const s = new Slice(4);
 * const andTt = Array(16).fill(0) as Bit[];
 * andTt[3] = 1;
 * const xorTt = Array(16).fill(0) as Bit[];
 * xorTt[1] = 1; xorTt[2] = 1;
 * s.configure(andTt, xorTt);
 * const out = s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], 0);
 * // out.outputA === 1 (AND(1,1))
 * // out.outputB === 1 (XOR(1,0))
 */
export class Slice {
  private readonly _lutA: LUT;
  private readonly _lutB: LUT;
  private readonly _k: number;

  // Flip-flop state
  private _ffAState: FlipFlopState;
  private _ffBState: FlipFlopState;

  // Configuration
  private _ffAEnabled: boolean = false;
  private _ffBEnabled: boolean = false;
  private _carryEnabled: boolean = false;

  /**
   * @param lutInputs - Number of inputs per LUT (2 to 6, default 4)
   */
  constructor(lutInputs: number = 4) {
    this._lutA = new LUT(lutInputs);
    this._lutB = new LUT(lutInputs);
    this._k = lutInputs;
    this._ffAState = { masterQ: 0, masterQBar: 1, slaveQ: 0, slaveQBar: 1 };
    this._ffBState = { masterQ: 0, masterQBar: 1, slaveQ: 0, slaveQBar: 1 };
  }

  /**
   * Configure the slice's LUTs, flip-flops, and carry chain.
   *
   * @param lutATable - Truth table for LUT A (2^k entries)
   * @param lutBTable - Truth table for LUT B (2^k entries)
   * @param ffAEnabled - Route LUT A output through flip-flop A
   * @param ffBEnabled - Route LUT B output through flip-flop B
   * @param carryEnabled - Enable carry chain computation
   */
  configure(
    lutATable: Bit[],
    lutBTable: Bit[],
    ffAEnabled: boolean = false,
    ffBEnabled: boolean = false,
    carryEnabled: boolean = false,
  ): void {
    this._lutA.configure(lutATable);
    this._lutB.configure(lutBTable);
    this._ffAEnabled = ffAEnabled;
    this._ffBEnabled = ffBEnabled;
    this._carryEnabled = carryEnabled;

    // Reset flip-flop state on reconfiguration
    this._ffAState = { masterQ: 0, masterQBar: 1, slaveQ: 0, slaveQBar: 1 };
    this._ffBState = { masterQ: 0, masterQBar: 1, slaveQ: 0, slaveQBar: 1 };
  }

  /**
   * Evaluate the slice for one half-cycle.
   *
   * @param inputsA - Input bits for LUT A (length k)
   * @param inputsB - Input bits for LUT B (length k)
   * @param clock - Clock signal (0 or 1)
   * @param carryIn - Carry input from previous slice (default 0)
   * @returns SliceOutput with outputA, outputB, and carryOut.
   */
  evaluate(
    inputsA: Bit[],
    inputsB: Bit[],
    clock: Bit,
    carryIn: Bit = 0,
  ): SliceOutput {
    // Evaluate LUTs (combinational -- always computed)
    const lutAOut = this._lutA.evaluate(inputsA);
    const lutBOut = this._lutB.evaluate(inputsB);

    // Flip-flop A: route through if enabled
    let outputA: Bit;
    if (this._ffAEnabled) {
      const [qA, , newState] = dFlipFlop(lutAOut, clock, this._ffAState);
      this._ffAState = newState;
      // MUX: select registered (sel=1) or combinational (sel=0)
      outputA = mux2(lutAOut, qA, 1);
    } else {
      outputA = lutAOut;
    }

    // Flip-flop B: route through if enabled
    let outputB: Bit;
    if (this._ffBEnabled) {
      const [qB, , newState] = dFlipFlop(lutBOut, clock, this._ffBState);
      this._ffBState = newState;
      outputB = mux2(lutBOut, qB, 1);
    } else {
      outputB = lutBOut;
    }

    // Carry chain: standard full-adder carry equation
    //   carry_out = (A AND B) OR (carry_in AND (A XOR B))
    let carryOut: Bit;
    if (this._carryEnabled) {
      carryOut = OR(
        AND(lutAOut, lutBOut),
        AND(carryIn, XOR(lutAOut, lutBOut)),
      );
    } else {
      carryOut = 0;
    }

    return { outputA, outputB, carryOut };
  }

  /** LUT A (for inspection). */
  get lutA(): LUT {
    return this._lutA;
  }

  /** LUT B (for inspection). */
  get lutB(): LUT {
    return this._lutB;
  }

  /** Number of LUT inputs. */
  get k(): number {
    return this._k;
  }
}
