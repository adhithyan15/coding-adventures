/**
 * One-bit branch predictor — one flip-flop per branch.
 *
 * The one-bit predictor is the simplest dynamic predictor. Unlike static
 * predictors (AlwaysTaken, BTFNT), it actually learns from the branch's
 * history. Each branch address maps to a single bit of state that records
 * the last outcome:
 *
 *     bit = 0 -> predict NOT TAKEN
 *     bit = 1 -> predict TAKEN
 *
 * After each branch resolves, the bit is updated to match the actual outcome.
 * This means the predictor always predicts "whatever happened last time."
 *
 * Hardware implementation:
 *     A small SRAM table indexed by the lower bits of the PC.
 *     Each entry is a single flip-flop (1 bit of storage).
 *     Total storage: table_size x 1 bit.
 *     For a 1024-entry table: 1024 bits = 128 bytes.
 *
 * The aliasing problem:
 *     Since the table is indexed by (pc % table_size), two different branches
 *     can map to the same entry. This is called "aliasing" or "interference."
 *     When branches alias, they corrupt each other's predictions.
 *
 *     Example with table_size=4:
 *         Branch at 0x100 -> index 0 (0x100 % 4 = 0)
 *         Branch at 0x104 -> index 0 (0x104 % 4 = 0)   <- COLLISION!
 *
 *     With larger tables (1024+), aliasing is rare for most programs.
 *
 * The double-misprediction problem:
 *     Consider a loop that runs N times then exits:
 *
 *         for (let i = 0; i < 10; i++) {
 *             body();      // branch at end: taken 9 times, not-taken once
 *         }
 *
 *     Iteration 1: bit=0 (cold) -> predict NOT TAKEN -> actual TAKEN -> WRONG, set bit=1
 *     Iteration 2: bit=1 -> predict TAKEN -> actual TAKEN -> correct
 *     ...
 *     Iteration 9: bit=1 -> predict TAKEN -> actual TAKEN -> correct
 *     Iteration 10: bit=1 -> predict TAKEN -> actual NOT TAKEN -> WRONG, set bit=0
 *
 *     Next time the loop runs:
 *     Iteration 1: bit=0 -> predict NOT TAKEN -> actual TAKEN -> WRONG, set bit=1
 *
 *     Result: 2 mispredictions per loop invocation (first and last iterations).
 *     For a loop running 10 times, that's 2/10 = 20% misprediction rate.
 *     The two-bit predictor solves this — see two-bit.ts.
 */

import { DFA } from "../../state-machine/src/index.js";
import type { BranchPredictor, Prediction } from "./base.js";
import { PredictionStats } from "./stats.js";

// ─── DFA representation ──────────────────────────────────────────────────────
//
// The 1-bit predictor is a 2-state DFA — the simplest possible dynamic
// branch predictor. It has just two states (predict taken or not taken)
// and flips on every misprediction.
//
// Expressing it as a formal DFA makes the structure explicit and enables
// equivalence testing with the manual implementation above.

/**
 * The 1-bit predictor expressed as a formal DFA.
 *
 * States:
 *   NT = Not Taken (bit = 0)
 *   T  = Taken (bit = 1)
 *
 * Alphabet: { "taken", "not_taken" }
 *
 * Transitions:
 * ```
 *   NT --taken--> T       NT --not_taken--> NT
 *   T  --taken--> T       T  --not_taken--> NT
 * ```
 *
 * Initial state: NT (cold start, defaults to not taken)
 *
 * Accepting states: { T } — the state that predicts "taken".
 */
export const ONE_BIT_DFA = new DFA(
  new Set(["NT", "T"]),
  new Set(["taken", "not_taken"]),
  new Map([
    ["NT\0taken", "T"],
    ["NT\0not_taken", "NT"],
    ["T\0taken", "T"],
    ["T\0not_taken", "NT"],
  ]),
  "NT",
  new Set(["T"]),
);

/**
 * Maps DFA state names to boolean prediction values.
 */
export const ONE_BIT_DFA_STATE_TO_BOOL: ReadonlyMap<string, boolean> = new Map([
  ["NT", false],
  ["T", true],
]);

/**
 * Maps boolean prediction values to DFA state names.
 */
export const ONE_BIT_BOOL_TO_DFA_STATE: ReadonlyMap<boolean, string> = new Map([
  [false, "NT"],
  [true, "T"],
]);

/**
 * Compute the next prediction state using the DFA transition function.
 *
 * @param currentlyTaken - The current prediction (true = taken, false = not taken).
 * @param actualTaken - Whether the branch was actually taken.
 * @returns The next prediction state after the transition.
 */
export function oneBitTransitionViaDFA(
  currentlyTaken: boolean,
  actualTaken: boolean,
): boolean {
  const startName = ONE_BIT_BOOL_TO_DFA_STATE.get(currentlyTaken)!;
  const dfa = new DFA(
    new Set(["NT", "T"]),
    new Set(["taken", "not_taken"]),
    new Map([
      ["NT\0taken", "T"],
      ["NT\0not_taken", "NT"],
      ["T\0taken", "T"],
      ["T\0not_taken", "NT"],
    ]),
    startName,
    new Set(["T"]),
  );

  const event = actualTaken ? "taken" : "not_taken";
  dfa.process(event);
  return ONE_BIT_DFA_STATE_TO_BOOL.get(dfa.currentState)!;
}

/**
 * 1-bit predictor — one flip-flop per branch address.
 *
 * Maintains a table of 1-bit entries indexed by (pc % tableSize).
 * Each entry remembers the LAST outcome of that branch.
 *
 * The fundamental state diagram:
 * ```
 *     +-------------------+     taken      +-------------------+
 *     | Predict NOT TAKEN | ------------> |  Predict TAKEN     |
 *     |    (bit = 0)      | <------------ |    (bit = 1)       |
 *     +-------------------+   not taken   +-------------------+
 * ```
 *
 * Every misprediction flips the bit. This is too aggressive — a single
 * anomalous outcome changes the prediction. The 2-bit predictor adds
 * hysteresis to fix this.
 *
 * @param tableSize - Number of entries in the prediction table. Must be a
 *     power of 2 for efficient hardware implementation (though this
 *     simulator doesn't enforce that). Larger tables reduce aliasing
 *     but cost more silicon. Default: 1024 entries = 128 bytes.
 *
 * @example
 * ```ts
 * const predictor = new OneBitPredictor(1024);
 *
 * // First encounter — cold start, defaults to NOT TAKEN
 * let pred = predictor.predict(0x100);
 * // pred.taken === false
 *
 * // Update with actual outcome: branch was taken
 * predictor.update(0x100, true);
 *
 * // Now predicts TAKEN (remembers last outcome)
 * pred = predictor.predict(0x100);
 * // pred.taken === true
 * ```
 */
export class OneBitPredictor implements BranchPredictor {
  // ── Table size ────────────────────────────────────────────────────
  // In hardware, this would be the number of rows in a small SRAM.
  // Common sizes: 256, 512, 1024, 2048, 4096.
  private _tableSize: number;

  // ── Prediction table ──────────────────────────────────────────────
  // Maps (index) -> last_outcome. We use a Map rather than an Array
  // to avoid pre-allocating memory for entries that are never accessed.
  // In hardware, all entries exist physically but start at 0 (not-taken).
  private _table = new Map<number, boolean>();

  // ── Statistics tracker ────────────────────────────────────────────
  private _stats = new PredictionStats();

  constructor(tableSize = 1024) {
    this._tableSize = tableSize;
  }

  /**
   * Compute the table index for a given PC.
   *
   * In hardware, this is just the lower log2(tableSize) bits of the PC.
   * Using modulo achieves the same result in software.
   *
   * @param pc - The program counter of the branch instruction.
   * @returns An integer in [0, tableSize) used to index the prediction table.
   */
  private _index(pc: number): number {
    return pc % this._tableSize;
  }

  /**
   * Predict based on the last outcome of this branch.
   *
   * On a cold start (branch not yet seen), defaults to NOT TAKEN.
   * This is a common design choice — the bit starts at 0.
   *
   * @param pc - The program counter of the branch instruction.
   * @returns Prediction with taken matching the stored bit for this branch.
   */
  predict(pc: number): Prediction {
    const index = this._index(pc);
    const taken = this._table.get(index) ?? false; // default: not taken
    // Confidence: 0.5 because we only have 1 bit of history.
    // We know the last outcome, but that's weak evidence.
    return { taken, confidence: 0.5, target: null };
  }

  /**
   * Update the prediction table with the actual outcome.
   *
   * Simply sets the bit to match the actual outcome. This is the "flip"
   * that gives the 1-bit predictor its characteristic behavior.
   *
   * @param pc - The program counter of the branch instruction.
   * @param taken - Whether the branch was actually taken.
   * @param _target - The actual target address (unused by this predictor).
   */
  update(pc: number, taken: boolean, _target?: number | null): void {
    const index = this._index(pc);
    // Record accuracy BEFORE updating the table, so we compare against
    // what the predictor would have predicted.
    const predicted = this._table.get(index) ?? false;
    this._stats.record(predicted === taken);
    // Now update the table to remember this outcome for next time.
    this._table.set(index, taken);
  }

  /** Get prediction accuracy statistics. */
  get stats(): PredictionStats {
    return this._stats;
  }

  /** Reset the prediction table and statistics. */
  reset(): void {
    this._table.clear();
    this._stats.reset();
  }
}
