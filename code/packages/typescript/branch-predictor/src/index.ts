/**
 * Branch Predictor — teaching CPUs to guess the future.
 *
 * This package simulates the branch prediction algorithms used in real CPU cores.
 * Branch prediction is one of the most critical performance features in modern
 * processors — without it, a deeply pipelined CPU would stall on every branch
 * instruction, losing 10-15 cycles each time.
 *
 * The package provides a pluggable architecture:
 *
 * - **BranchPredictor** interface — the interface all predictors implement
 * - **Static predictors** — AlwaysTaken, AlwaysNotTaken, BTFNT
 * - **Dynamic predictors** — OneBit (1-bit flip-flop), TwoBit (saturating counter)
 * - **BranchTargetBuffer** — caches WHERE branches go (used alongside any predictor)
 * - **PredictionStats** — tracks accuracy metrics for benchmarking
 *
 * All predictors implement the same predict/update interface, so they can be
 * swapped into any CPU core design without changing the core's code.
 *
 * @example
 * ```ts
 * import { TwoBitPredictor, BranchTargetBuffer } from "@coding-adventures/branch-predictor";
 *
 * const predictor = new TwoBitPredictor(1024);
 * const btb = new BranchTargetBuffer(256);
 *
 * // Simulate a branch at PC=0x100
 * const prediction = predictor.predict(0x100);
 * if (prediction.taken) {
 *     const target = btb.lookup(0x100);
 * }
 *
 * // After execution, update both structures
 * predictor.update(0x100, true, 0x200);
 * btb.update(0x100, 0x200);
 *
 * // Check accuracy
 * console.log(`Accuracy: ${predictor.stats.accuracy.toFixed(1)}%`);
 * ```
 */

export type { BranchPredictor, Prediction } from "./base.js";
export { BTBEntry, BranchTargetBuffer, createBTBEntry } from "./btb.js";
export {
  ONE_BIT_BOOL_TO_DFA_STATE,
  ONE_BIT_DFA,
  ONE_BIT_DFA_STATE_TO_BOOL,
  OneBitPredictor,
  oneBitTransitionViaDFA,
} from "./one-bit.js";
export {
  AlwaysNotTakenPredictor,
  AlwaysTakenPredictor,
  BackwardTakenForwardNotTaken,
} from "./static.js";
export { PredictionStats } from "./stats.js";
export {
  DFA_STATE_TO_ENUM,
  ENUM_TO_DFA_STATE,
  TWO_BIT_DFA,
  TwoBitPredictor,
  TwoBitState,
  notTakenOutcome,
  predictsTaken,
  takenOutcome,
  transitionViaDFA,
} from "./two-bit.js";
