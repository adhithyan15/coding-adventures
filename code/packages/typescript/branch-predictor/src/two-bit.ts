/**
 * Two-bit saturating counter predictor — the classic, used in most textbooks.
 *
 * The two-bit predictor improves on the one-bit predictor by adding hysteresis.
 * Instead of flipping the prediction on every misprediction, it takes TWO
 * consecutive mispredictions to change the predicted direction. This is achieved
 * with a 2-bit saturating counter — a counter that counts up to 3 and down to 0,
 * but never wraps around (it "saturates" at the boundaries).
 *
 * The four states and their meanings:
 *
 * ```
 *     +------------------------------------------------------------------------+
 *     |  STRONGLY      WEAKLY        WEAKLY        STRONGLY                    |
 *     |  NOT TAKEN     NOT TAKEN     TAKEN         TAKEN                       |
 *     |    (00)          (01)         (10)          (11)                       |
 *     |                                                                        |
 *     |  Predict:      Predict:      Predict:      Predict:                    |
 *     |  NOT TAKEN     NOT TAKEN     TAKEN         TAKEN                       |
 *     |                                                                        |
 *     |  Confidence:   Confidence:   Confidence:   Confidence:                 |
 *     |  HIGH          LOW           LOW           HIGH                        |
 *     +------------------------------------------------------------------------+
 * ```
 *
 * State transition diagram:
 *
 * ```
 *     taken                taken               taken               taken
 *     ------>              ------>              ------>              ------>
 *     (sat)   SNT <-------- WNT <-------- WT <-------- ST   (sat)
 *             ------>              ------>              ------>
 *           not taken          not taken           not taken
 * ```
 *
 *     SNT = Strongly Not Taken (0)
 *     WNT = Weakly Not Taken (1)
 *     WT  = Weakly Taken (2)
 *     ST  = Strongly Taken (3)
 *
 * The prediction threshold is at the midpoint:
 *     states 0, 1 -> predict NOT TAKEN
 *     states 2, 3 -> predict TAKEN
 *
 * Why this solves the double-misprediction problem:
 *     Consider the same loop as in one-bit.ts (10 iterations):
 *
 *     First invocation:
 *     Iter 1: state=WNT(1) -> predict NOT TAKEN -> actual TAKEN -> WRONG, state->WT(2)
 *     Iter 2: state=WT(2)  -> predict TAKEN     -> actual TAKEN -> correct, state->ST(3)
 *     ...
 *     Iter 9: state=ST(3)  -> predict TAKEN     -> actual TAKEN -> correct (saturated)
 *     Iter 10: state=ST(3) -> predict TAKEN     -> actual NOT TAKEN -> WRONG, state->WT(2)
 *
 *     Second invocation:
 *     Iter 1: state=WT(2)  -> predict TAKEN     -> actual TAKEN -> correct! state->ST(3)
 *
 *     Only 1 misprediction on re-entry (vs 2 for the one-bit predictor).
 *     The "weakly taken" state acts as a buffer — one not-taken doesn't flip it.
 *
 * Historical usage:
 *     - Alpha 21064: 2-bit counters with 2048 entries
 *     - Intel Pentium: 2-bit counters with 256 entries, indexed by branch history
 *     - Early ARM (ARM7): 2-bit counters with 64 entries
 *     - MIPS R10000: 2-bit counters as base predictor in a tournament scheme
 */

import { DFA } from "../../state-machine/src/index.js";
import type { BranchPredictor, Prediction } from "./base.js";
import { PredictionStats } from "./stats.js";

// ─── TwoBitState ──────────────────────────────────────────────────────────────
//
// We use a const enum so the states have integer values (0-3) that correspond to
// the 2-bit counter value. This makes the increment/decrement logic natural:
//   taken -> min(state + 1, 3)
//   not taken -> max(state - 1, 0)
//
// The "saturating" part means we clamp at the boundaries rather than wrapping.
// In hardware, this is implemented with a simple 2-bit adder and saturation
// logic — about 4 gates per entry.

/**
 * The 4 states of a 2-bit saturating counter.
 *
 * State transitions:
 * ```
 *     STRONGLY_NOT_TAKEN <-> WEAKLY_NOT_TAKEN <-> WEAKLY_TAKEN <-> STRONGLY_TAKEN
 *           (0)                   (1)                (2)              (3)
 * ```
 *
 * On 'taken' outcome: increment (move right), saturate at STRONGLY_TAKEN.
 * On 'not taken' outcome: decrement (move left), saturate at STRONGLY_NOT_TAKEN.
 *
 * Predict taken if state >= WEAKLY_TAKEN (bit 1 is set).
 *
 * Why this works: a loop that runs 10 times mispredicts only ONCE (the exit).
 * After the first taken, the counter moves to STRONGLY_TAKEN. It takes TWO
 * not-taken outcomes to flip the prediction. The single not-taken at loop exit
 * only moves it to WEAKLY_TAKEN, which still predicts taken next time.
 */
export enum TwoBitState {
  STRONGLY_NOT_TAKEN = 0,
  WEAKLY_NOT_TAKEN = 1,
  WEAKLY_TAKEN = 2,
  STRONGLY_TAKEN = 3,
}

/**
 * Transition on a 'taken' branch outcome (increment, saturate at 3).
 *
 * @example
 * ```ts
 * let state = TwoBitState.WEAKLY_NOT_TAKEN;  // 1
 * state = takenOutcome(state);                // -> WEAKLY_TAKEN (2)
 * state = takenOutcome(state);                // -> STRONGLY_TAKEN (3)
 * state = takenOutcome(state);                // -> STRONGLY_TAKEN (3) — saturated!
 * ```
 */
export function takenOutcome(state: TwoBitState): TwoBitState {
  return Math.min(state + 1, TwoBitState.STRONGLY_TAKEN) as TwoBitState;
}

/**
 * Transition on a 'not taken' branch outcome (decrement, saturate at 0).
 *
 * @example
 * ```ts
 * let state = TwoBitState.WEAKLY_TAKEN;      // 2
 * state = notTakenOutcome(state);             // -> WEAKLY_NOT_TAKEN (1)
 * state = notTakenOutcome(state);             // -> STRONGLY_NOT_TAKEN (0)
 * state = notTakenOutcome(state);             // -> STRONGLY_NOT_TAKEN (0) — saturated!
 * ```
 */
export function notTakenOutcome(state: TwoBitState): TwoBitState {
  return Math.max(state - 1, TwoBitState.STRONGLY_NOT_TAKEN) as TwoBitState;
}

/**
 * Whether this state predicts 'taken'.
 *
 * The threshold is at WEAKLY_TAKEN (2). States 2 and 3 predict taken;
 * states 0 and 1 predict not-taken. In hardware, this is just bit 1
 * of the 2-bit counter — a single wire, zero logic.
 *
 * @example
 * ```ts
 * predictsTaken(TwoBitState.STRONGLY_TAKEN)     // true
 * predictsTaken(TwoBitState.WEAKLY_TAKEN)        // true
 * predictsTaken(TwoBitState.WEAKLY_NOT_TAKEN)    // false
 * predictsTaken(TwoBitState.STRONGLY_NOT_TAKEN)  // false
 * ```
 */
export function predictsTaken(state: TwoBitState): boolean {
  return state >= TwoBitState.WEAKLY_TAKEN;
}

// ─── DFA representation ──────────────────────────────────────────────────────
//
// The 2-bit saturating counter is a textbook DFA. We define it here using the
// state-machine library's DFA class. This serves two purposes:
//
// 1. **Formal verification**: we can prove that the manual takenOutcome /
//    notTakenOutcome functions produce the same transitions as the DFA.
//
// 2. **Visualization**: the DFA can render itself as Graphviz DOT or an ASCII
//    transition table, which is invaluable for teaching.
//
// The DFA's states are string names ("SNT", "WNT", "WT", "ST") that correspond
// to the TwoBitState enum values. We provide bidirectional mappings below.

/**
 * Maps DFA state names to TwoBitState enum values.
 *
 * Used when reading the DFA's current state and converting back to the
 * numeric representation used by the predictor table.
 */
export const DFA_STATE_TO_ENUM: ReadonlyMap<string, TwoBitState> = new Map([
  ["SNT", TwoBitState.STRONGLY_NOT_TAKEN],
  ["WNT", TwoBitState.WEAKLY_NOT_TAKEN],
  ["WT", TwoBitState.WEAKLY_TAKEN],
  ["ST", TwoBitState.STRONGLY_TAKEN],
]);

/**
 * Maps TwoBitState enum values to DFA state names.
 *
 * Used when setting the DFA's initial state from the predictor table.
 */
export const ENUM_TO_DFA_STATE: ReadonlyMap<TwoBitState, string> = new Map([
  [TwoBitState.STRONGLY_NOT_TAKEN, "SNT"],
  [TwoBitState.WEAKLY_NOT_TAKEN, "WNT"],
  [TwoBitState.WEAKLY_TAKEN, "WT"],
  [TwoBitState.STRONGLY_TAKEN, "ST"],
]);

/**
 * The 2-bit saturating counter expressed as a formal DFA.
 *
 * States:
 *   SNT = Strongly Not Taken (0)
 *   WNT = Weakly Not Taken (1)
 *   WT  = Weakly Taken (2)
 *   ST  = Strongly Taken (3)
 *
 * Alphabet: { "taken", "not_taken" }
 *
 * Transitions:
 * ```
 *   SNT --taken--> WNT    SNT --not_taken--> SNT  (saturated)
 *   WNT --taken--> WT     WNT --not_taken--> SNT
 *   WT  --taken--> ST     WT  --not_taken--> WNT
 *   ST  --taken--> ST     ST  --not_taken--> WT   (saturated)
 * ```
 *
 * Initial state: WNT (conservative start, one taken flips to predict taken)
 *
 * Accepting states: { WT, ST } — states that predict "taken".
 * The DFA "accepts" an input sequence iff the final state predicts taken.
 */
export const TWO_BIT_DFA = new DFA(
  new Set(["SNT", "WNT", "WT", "ST"]),
  new Set(["taken", "not_taken"]),
  new Map([
    ["SNT\0taken", "WNT"],
    ["SNT\0not_taken", "SNT"],
    ["WNT\0taken", "WT"],
    ["WNT\0not_taken", "SNT"],
    ["WT\0taken", "ST"],
    ["WT\0not_taken", "WNT"],
    ["ST\0taken", "ST"],
    ["ST\0not_taken", "WT"],
  ]),
  "WNT",
  new Set(["WT", "ST"]),
);

/**
 * Compute the next TwoBitState using the DFA transition function.
 *
 * This is an alternative to the manual takenOutcome/notTakenOutcome functions.
 * It creates a temporary DFA, sets it to the given state, processes the event,
 * and returns the resulting TwoBitState. This is less efficient than the direct
 * functions (it constructs a DFA each time), but proves the equivalence.
 *
 * @param state - The current TwoBitState.
 * @param taken - Whether the branch was taken.
 * @returns The next TwoBitState after the transition.
 */
export function transitionViaDFA(
  state: TwoBitState,
  taken: boolean,
): TwoBitState {
  // Build a fresh DFA starting from the given state
  const startName = ENUM_TO_DFA_STATE.get(state)!;
  const dfa = new DFA(
    new Set(["SNT", "WNT", "WT", "ST"]),
    new Set(["taken", "not_taken"]),
    new Map([
      ["SNT\0taken", "WNT"],
      ["SNT\0not_taken", "SNT"],
      ["WNT\0taken", "WT"],
      ["WNT\0not_taken", "SNT"],
      ["WT\0taken", "ST"],
      ["WT\0not_taken", "WNT"],
      ["ST\0taken", "ST"],
      ["ST\0not_taken", "WT"],
    ]),
    startName,
    new Set(["WT", "ST"]),
  );

  const event = taken ? "taken" : "not_taken";
  dfa.process(event);
  return DFA_STATE_TO_ENUM.get(dfa.currentState)!;
}

// ─── TwoBitPredictor ─────────────────────────────────────────────────────────
//
// The predictor maintains a table of 2-bit saturating counters, one per entry.
// Each branch maps to an entry via (pc % tableSize). On predict(), we read
// the counter; on update(), we increment or decrement it.
//
// The initial state is configurable. Common choices:
//   - WEAKLY_NOT_TAKEN (01): conservative start, requires 1 taken to flip
//   - WEAKLY_TAKEN (10): optimistic start (like "always taken" initially)
//
// Most real processors use WEAKLY_NOT_TAKEN as the initial state, because
// it only takes one taken branch to move to WEAKLY_TAKEN and start predicting
// correctly. Starting at STRONGLY_NOT_TAKEN would require TWO taken branches.

/**
 * 2-bit saturating counter predictor — the classic, used in most textbooks.
 *
 * This was used in real processors: Alpha 21064, early MIPS, early ARM.
 * Modern CPUs use more sophisticated predictors (TAGE, perceptron) but
 * the 2-bit counter is the foundation that all advanced predictors build on.
 *
 * @param tableSize - Number of entries in the prediction table. Default: 1024.
 * @param initialState - Starting state for all counter entries.
 *     Default: WEAKLY_NOT_TAKEN — a good balance between responsiveness
 *     and stability.
 *
 * @example
 * ```ts
 * const predictor = new TwoBitPredictor(256);
 *
 * // First encounter — starts at WEAKLY_NOT_TAKEN -> predicts NOT TAKEN
 * let pred = predictor.predict(0x100);
 * // pred.taken === false
 *
 * // After one 'taken' outcome -> moves to WEAKLY_TAKEN -> predicts TAKEN
 * predictor.update(0x100, true);
 * pred = predictor.predict(0x100);
 * // pred.taken === true
 * ```
 */
export class TwoBitPredictor implements BranchPredictor {
  private _tableSize: number;
  private _initialState: TwoBitState;

  // ── Prediction table ──────────────────────────────────────────────
  // Maps (index) -> TwoBitState. Entries start at initialState.
  // We use a Map and fill on first access (lazy initialization).
  private _table = new Map<number, TwoBitState>();

  private _stats = new PredictionStats();

  constructor(
    tableSize = 1024,
    initialState: TwoBitState = TwoBitState.WEAKLY_NOT_TAKEN,
  ) {
    this._tableSize = tableSize;
    this._initialState = initialState;
  }

  /**
   * Compute the table index for a given PC.
   *
   * Same as OneBitPredictor — uses the lower bits of the PC.
   *
   * @param pc - The program counter of the branch instruction.
   * @returns An integer in [0, tableSize) used to index the prediction table.
   */
  private _index(pc: number): number {
    return pc % this._tableSize;
  }

  /**
   * Get the state for a table entry, initializing if needed.
   *
   * @param index - The table index.
   * @returns The current TwoBitState for this entry.
   */
  private _getState(index: number): TwoBitState {
    return this._table.get(index) ?? this._initialState;
  }

  /**
   * Predict based on the 2-bit counter for this branch.
   *
   * Reads the counter state and returns taken/not-taken based on the
   * threshold (states 2-3 -> taken, states 0-1 -> not-taken).
   *
   * Confidence mapping:
   *     STRONGLY states -> 1.0 (high confidence)
   *     WEAKLY states   -> 0.5 (low confidence)
   *
   * @param pc - The program counter of the branch instruction.
   * @returns Prediction with taken and confidence based on counter state.
   */
  predict(pc: number): Prediction {
    const index = this._index(pc);
    const state = this._getState(index);

    // Confidence: strong states are more confident than weak states.
    // This is useful for tournament predictors that pick the most
    // confident sub-predictor.
    let confidence: number;
    if (
      state === TwoBitState.STRONGLY_TAKEN ||
      state === TwoBitState.STRONGLY_NOT_TAKEN
    ) {
      confidence = 1.0;
    } else {
      confidence = 0.5;
    }

    return { taken: predictsTaken(state), confidence, target: null };
  }

  /**
   * Update the 2-bit counter based on the actual outcome.
   *
   * Increments on taken, decrements on not-taken, saturating at boundaries.
   *
   * @param pc - The program counter of the branch instruction.
   * @param taken - Whether the branch was actually taken.
   * @param _target - The actual target address (unused by this predictor).
   */
  update(pc: number, taken: boolean, _target?: number | null): void {
    const index = this._index(pc);
    const state = this._getState(index);

    // Record accuracy BEFORE updating
    this._stats.record(predictsTaken(state) === taken);

    // Transition the state
    if (taken) {
      this._table.set(index, takenOutcome(state));
    } else {
      this._table.set(index, notTakenOutcome(state));
    }
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

  /**
   * Inspect the current state for a branch address (for testing/debugging).
   *
   * @param pc - The program counter of the branch instruction.
   * @returns The current TwoBitState for this branch's table entry.
   */
  getState(pc: number): TwoBitState {
    return this._getState(this._index(pc));
  }
}
