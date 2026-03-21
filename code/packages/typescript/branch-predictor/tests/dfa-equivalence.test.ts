/**
 * DFA equivalence tests — verify that the state-machine DFA definitions
 * produce identical transitions to the manual implementations.
 *
 * These tests serve as a bridge between two representations of the same logic:
 *
 * 1. **Manual functions**: takenOutcome(), notTakenOutcome() for the 2-bit
 *    predictor, and simple boolean assignment for the 1-bit predictor.
 *    These are fast and used in the actual predictor classes.
 *
 * 2. **DFA objects**: TWO_BIT_DFA and ONE_BIT_DFA from the state-machine
 *    library. These are formal, traceable, and visualizable.
 *
 * By proving equivalence, we gain confidence that the DFA definitions are
 * correct representations of the predictor logic, AND that the manual
 * implementations match the formal specification.
 */

import { describe, expect, it } from "vitest";
import {
  ONE_BIT_BOOL_TO_DFA_STATE,
  ONE_BIT_DFA,
  ONE_BIT_DFA_STATE_TO_BOOL,
  oneBitTransitionViaDFA,
} from "../src/one-bit.js";
import {
  DFA_STATE_TO_ENUM,
  ENUM_TO_DFA_STATE,
  TWO_BIT_DFA,
  TwoBitState,
  notTakenOutcome,
  predictsTaken,
  takenOutcome,
  transitionViaDFA,
} from "../src/two-bit.js";

// ─── TWO_BIT_DFA structure tests ────────────────────────────────────────────

describe("TWO_BIT_DFA structure", () => {
  it("has 4 states", () => {
    expect(TWO_BIT_DFA.states.size).toBe(4);
    expect(TWO_BIT_DFA.states.has("SNT")).toBe(true);
    expect(TWO_BIT_DFA.states.has("WNT")).toBe(true);
    expect(TWO_BIT_DFA.states.has("WT")).toBe(true);
    expect(TWO_BIT_DFA.states.has("ST")).toBe(true);
  });

  it("has 2 input symbols", () => {
    expect(TWO_BIT_DFA.alphabet.size).toBe(2);
    expect(TWO_BIT_DFA.alphabet.has("taken")).toBe(true);
    expect(TWO_BIT_DFA.alphabet.has("not_taken")).toBe(true);
  });

  it("starts at WNT (weakly not taken)", () => {
    expect(TWO_BIT_DFA.initial).toBe("WNT");
  });

  it("accepts WT and ST (predict taken)", () => {
    expect(TWO_BIT_DFA.accepting.size).toBe(2);
    expect(TWO_BIT_DFA.accepting.has("WT")).toBe(true);
    expect(TWO_BIT_DFA.accepting.has("ST")).toBe(true);
  });

  it("has 8 transitions (complete DFA: 4 states x 2 inputs)", () => {
    expect(TWO_BIT_DFA.transitions.size).toBe(8);
  });

  it("is a complete DFA", () => {
    expect(TWO_BIT_DFA.isComplete()).toBe(true);
  });

  it("all states are reachable", () => {
    const reachable = TWO_BIT_DFA.reachableStates();
    expect(reachable.size).toBe(4);
  });

  it("has no validation warnings", () => {
    expect(TWO_BIT_DFA.validate()).toEqual([]);
  });
});

// ─── TWO_BIT_DFA transition equivalence ─────────────────────────────────────

describe("TWO_BIT_DFA transition equivalence", () => {
  /**
   * For every (state, outcome) pair, verify that the DFA transition matches
   * the manual takenOutcome/notTakenOutcome function.
   */
  const allStates: TwoBitState[] = [
    TwoBitState.STRONGLY_NOT_TAKEN,
    TwoBitState.WEAKLY_NOT_TAKEN,
    TwoBitState.WEAKLY_TAKEN,
    TwoBitState.STRONGLY_TAKEN,
  ];

  for (const state of allStates) {
    const stateName = ENUM_TO_DFA_STATE.get(state)!;

    it(`takenOutcome(${stateName}) matches DFA`, () => {
      const manual = takenOutcome(state);
      const viaDfa = transitionViaDFA(state, true);
      expect(viaDfa).toBe(manual);
    });

    it(`notTakenOutcome(${stateName}) matches DFA`, () => {
      const manual = notTakenOutcome(state);
      const viaDfa = transitionViaDFA(state, false);
      expect(viaDfa).toBe(manual);
    });
  }
});

// ─── TWO_BIT_DFA accepts() equivalence ──────────────────────────────────────

describe("TWO_BIT_DFA accepts() matches predictsTaken()", () => {
  it("empty sequence: DFA starts at WNT -> not accepting", () => {
    /** WNT is not in the accepting set {WT, ST}. */
    expect(TWO_BIT_DFA.accepts([])).toBe(false);
    expect(predictsTaken(TwoBitState.WEAKLY_NOT_TAKEN)).toBe(false);
  });

  it("single taken: WNT -> WT -> accepting", () => {
    expect(TWO_BIT_DFA.accepts(["taken"])).toBe(true);
  });

  it("single not_taken: WNT -> SNT -> not accepting", () => {
    expect(TWO_BIT_DFA.accepts(["not_taken"])).toBe(false);
  });

  it("taken, taken: WNT -> WT -> ST -> accepting", () => {
    expect(TWO_BIT_DFA.accepts(["taken", "taken"])).toBe(true);
  });

  it("taken, not_taken: WNT -> WT -> WNT -> not accepting", () => {
    expect(TWO_BIT_DFA.accepts(["taken", "not_taken"])).toBe(false);
  });

  it("taken, taken, not_taken: WNT -> WT -> ST -> WT -> accepting", () => {
    /** This is the hysteresis in action: one not_taken doesn't flip from taken. */
    expect(TWO_BIT_DFA.accepts(["taken", "taken", "not_taken"])).toBe(true);
  });

  it("loop pattern: 9 taken + 1 not_taken -> still accepting", () => {
    /**
     * Simulates a loop that runs 10 times. After 9 taken, the counter is
     * at ST. One not_taken drops it to WT, which still predicts taken.
     * This is the key advantage over the 1-bit predictor.
     */
    const events = Array(9).fill("taken").concat(["not_taken"]);
    expect(TWO_BIT_DFA.accepts(events)).toBe(true);
  });
});

// ─── State mapping round-trips ──────────────────────────────────────────────

describe("two-bit state mappings", () => {
  it("DFA_STATE_TO_ENUM and ENUM_TO_DFA_STATE are inverses", () => {
    for (const [name, enumVal] of DFA_STATE_TO_ENUM) {
      expect(ENUM_TO_DFA_STATE.get(enumVal)).toBe(name);
    }
    for (const [enumVal, name] of ENUM_TO_DFA_STATE) {
      expect(DFA_STATE_TO_ENUM.get(name)).toBe(enumVal);
    }
  });

  it("all 4 states are mapped", () => {
    expect(DFA_STATE_TO_ENUM.size).toBe(4);
    expect(ENUM_TO_DFA_STATE.size).toBe(4);
  });
});

// ─── ONE_BIT_DFA structure tests ────────────────────────────────────────────

describe("ONE_BIT_DFA structure", () => {
  it("has 2 states", () => {
    expect(ONE_BIT_DFA.states.size).toBe(2);
    expect(ONE_BIT_DFA.states.has("NT")).toBe(true);
    expect(ONE_BIT_DFA.states.has("T")).toBe(true);
  });

  it("has 2 input symbols", () => {
    expect(ONE_BIT_DFA.alphabet.size).toBe(2);
    expect(ONE_BIT_DFA.alphabet.has("taken")).toBe(true);
    expect(ONE_BIT_DFA.alphabet.has("not_taken")).toBe(true);
  });

  it("starts at NT (not taken)", () => {
    expect(ONE_BIT_DFA.initial).toBe("NT");
  });

  it("accepts only T (predict taken)", () => {
    expect(ONE_BIT_DFA.accepting.size).toBe(1);
    expect(ONE_BIT_DFA.accepting.has("T")).toBe(true);
  });

  it("has 4 transitions (complete DFA: 2 states x 2 inputs)", () => {
    expect(ONE_BIT_DFA.transitions.size).toBe(4);
  });

  it("is a complete DFA", () => {
    expect(ONE_BIT_DFA.isComplete()).toBe(true);
  });

  it("all states are reachable", () => {
    expect(ONE_BIT_DFA.reachableStates().size).toBe(2);
  });

  it("has no validation warnings", () => {
    expect(ONE_BIT_DFA.validate()).toEqual([]);
  });
});

// ─── ONE_BIT_DFA transition equivalence ─────────────────────────────────────

describe("ONE_BIT_DFA transition equivalence", () => {
  /**
   * The 1-bit predictor simply records the last outcome. So:
   *   NT + taken -> T      (update to taken)
   *   NT + not_taken -> NT (stays not taken)
   *   T  + taken -> T      (stays taken)
   *   T  + not_taken -> NT (update to not taken)
   *
   * The manual implementation is: new_state = actual_outcome.
   */
  const cases: [boolean, boolean, boolean][] = [
    // [current, actual_taken, expected_next]
    [false, true, true],
    [false, false, false],
    [true, true, true],
    [true, false, false],
  ];

  for (const [current, actual, expected] of cases) {
    const currentName = current ? "T" : "NT";
    const actualName = actual ? "taken" : "not_taken";
    const expectedName = expected ? "T" : "NT";

    it(`${currentName} + ${actualName} -> ${expectedName}`, () => {
      const result = oneBitTransitionViaDFA(current, actual);
      expect(result).toBe(expected);
      // Also verify it matches the trivial manual logic: new_state = actual
      expect(result).toBe(actual);
    });
  }
});

// ─── ONE_BIT_DFA accepts() ──────────────────────────────────────────────────

describe("ONE_BIT_DFA accepts()", () => {
  it("empty sequence: starts at NT -> not accepting", () => {
    expect(ONE_BIT_DFA.accepts([])).toBe(false);
  });

  it("single taken -> accepting (T)", () => {
    expect(ONE_BIT_DFA.accepts(["taken"])).toBe(true);
  });

  it("single not_taken -> not accepting (NT)", () => {
    expect(ONE_BIT_DFA.accepts(["not_taken"])).toBe(false);
  });

  it("taken, not_taken -> not accepting (flips back)", () => {
    expect(ONE_BIT_DFA.accepts(["taken", "not_taken"])).toBe(false);
  });

  it("loop pattern: 9 taken + 1 not_taken -> NOT accepting", () => {
    /**
     * Unlike the 2-bit predictor, the 1-bit predictor flips immediately.
     * After 9 taken, the state is T. One not_taken flips it to NT.
     * This is the double-misprediction problem.
     */
    const events = Array(9).fill("taken").concat(["not_taken"]);
    expect(ONE_BIT_DFA.accepts(events)).toBe(false);
  });
});

// ─── One-bit state mapping round-trips ──────────────────────────────────────

describe("one-bit state mappings", () => {
  it("round-trip: bool -> name -> bool", () => {
    for (const [bool, name] of ONE_BIT_BOOL_TO_DFA_STATE) {
      expect(ONE_BIT_DFA_STATE_TO_BOOL.get(name)).toBe(bool);
    }
  });

  it("all 2 states are mapped", () => {
    expect(ONE_BIT_DFA_STATE_TO_BOOL.size).toBe(2);
    expect(ONE_BIT_BOOL_TO_DFA_STATE.size).toBe(2);
  });
});

// ─── Cross-predictor comparison via DFA ─────────────────────────────────────

describe("1-bit vs 2-bit DFA comparison", () => {
  it("both start by predicting not-taken", () => {
    /** Both initial states are not in the accepting set. */
    expect(ONE_BIT_DFA.accepts([])).toBe(false);
    expect(TWO_BIT_DFA.accepts([])).toBe(false);
  });

  it("both predict taken after a single taken", () => {
    expect(ONE_BIT_DFA.accepts(["taken"])).toBe(true);
    expect(TWO_BIT_DFA.accepts(["taken"])).toBe(true);
  });

  it("loop exit: 1-bit flips, 2-bit holds (hysteresis)", () => {
    /**
     * After many taken followed by one not_taken:
     * - 1-bit: T -> NT (flips immediately)
     * - 2-bit: ST -> WT (still predicts taken due to hysteresis)
     *
     * This is THE fundamental difference between the two predictors.
     */
    const loopPattern = Array(5).fill("taken").concat(["not_taken"]);
    expect(ONE_BIT_DFA.accepts(loopPattern)).toBe(false);
    expect(TWO_BIT_DFA.accepts(loopPattern)).toBe(true);
  });
});
