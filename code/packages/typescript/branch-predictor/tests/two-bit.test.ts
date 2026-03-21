/**
 * Tests for the two-bit saturating counter predictor.
 *
 * The two-bit predictor is the gold standard of introductory computer architecture.
 * These tests verify state transitions, loop behavior, comparison with one-bit,
 * and configurable initial state.
 */

import { describe, expect, it } from "vitest";
import { OneBitPredictor } from "../src/one-bit.js";
import {
  TwoBitPredictor,
  TwoBitState,
  notTakenOutcome,
  predictsTaken,
  takenOutcome,
} from "../src/two-bit.js";

// ─── TwoBitState transitions ─────────────────────────────────────────────────

describe("TwoBitState", () => {
  it("state values", () => {
    /** States have integer values 0-3. */
    expect(TwoBitState.STRONGLY_NOT_TAKEN).toBe(0);
    expect(TwoBitState.WEAKLY_NOT_TAKEN).toBe(1);
    expect(TwoBitState.WEAKLY_TAKEN).toBe(2);
    expect(TwoBitState.STRONGLY_TAKEN).toBe(3);
  });

  it("taken increments", () => {
    /** 'taken' outcome moves the counter toward STRONGLY_TAKEN. */
    expect(takenOutcome(TwoBitState.STRONGLY_NOT_TAKEN)).toBe(
      TwoBitState.WEAKLY_NOT_TAKEN,
    );
    expect(takenOutcome(TwoBitState.WEAKLY_NOT_TAKEN)).toBe(
      TwoBitState.WEAKLY_TAKEN,
    );
    expect(takenOutcome(TwoBitState.WEAKLY_TAKEN)).toBe(
      TwoBitState.STRONGLY_TAKEN,
    );
    // Saturates at the top
    expect(takenOutcome(TwoBitState.STRONGLY_TAKEN)).toBe(
      TwoBitState.STRONGLY_TAKEN,
    );
  });

  it("not taken decrements", () => {
    /** 'not taken' outcome moves the counter toward STRONGLY_NOT_TAKEN. */
    expect(notTakenOutcome(TwoBitState.STRONGLY_TAKEN)).toBe(
      TwoBitState.WEAKLY_TAKEN,
    );
    expect(notTakenOutcome(TwoBitState.WEAKLY_TAKEN)).toBe(
      TwoBitState.WEAKLY_NOT_TAKEN,
    );
    expect(notTakenOutcome(TwoBitState.WEAKLY_NOT_TAKEN)).toBe(
      TwoBitState.STRONGLY_NOT_TAKEN,
    );
    // Saturates at the bottom
    expect(notTakenOutcome(TwoBitState.STRONGLY_NOT_TAKEN)).toBe(
      TwoBitState.STRONGLY_NOT_TAKEN,
    );
  });

  it("predicts taken threshold", () => {
    /** States >= WEAKLY_TAKEN predict taken; others predict not-taken. */
    expect(predictsTaken(TwoBitState.STRONGLY_TAKEN)).toBe(true);
    expect(predictsTaken(TwoBitState.WEAKLY_TAKEN)).toBe(true);
    expect(predictsTaken(TwoBitState.WEAKLY_NOT_TAKEN)).toBe(false);
    expect(predictsTaken(TwoBitState.STRONGLY_NOT_TAKEN)).toBe(false);
  });

  it("saturation at STRONGLY_TAKEN", () => {
    /** Multiple taken outcomes don't exceed STRONGLY_TAKEN. */
    let state = TwoBitState.STRONGLY_TAKEN;
    for (let i = 0; i < 10; i++) {
      state = takenOutcome(state);
    }
    expect(state).toBe(TwoBitState.STRONGLY_TAKEN);
  });

  it("saturation at STRONGLY_NOT_TAKEN", () => {
    /** Multiple not-taken outcomes don't go below STRONGLY_NOT_TAKEN. */
    let state = TwoBitState.STRONGLY_NOT_TAKEN;
    for (let i = 0; i < 10; i++) {
      state = notTakenOutcome(state);
    }
    expect(state).toBe(TwoBitState.STRONGLY_NOT_TAKEN);
  });
});

// ─── TwoBitPredictor basics ──────────────────────────────────────────────────

describe("TwoBitPredictor basics", () => {
  it("default initial state", () => {
    /** Default initial state is WEAKLY_NOT_TAKEN -> predict not-taken. */
    const p = new TwoBitPredictor();
    const pred = p.predict(0x100);
    expect(pred.taken).toBe(false);
  });

  it("custom initial state: taken", () => {
    /** Starting at WEAKLY_TAKEN -> predict taken from the start. */
    const p = new TwoBitPredictor(1024, TwoBitState.WEAKLY_TAKEN);
    const pred = p.predict(0x100);
    expect(pred.taken).toBe(true);
  });

  it("custom initial state: strongly taken", () => {
    const p = new TwoBitPredictor(1024, TwoBitState.STRONGLY_TAKEN);
    const pred = p.predict(0x100);
    expect(pred.taken).toBe(true);
    expect(pred.confidence).toBe(1.0);
  });

  it("one taken flips from weakly not taken", () => {
    /** WNT + taken -> WT (predicts taken). One taken outcome flips. */
    const p = new TwoBitPredictor(); // starts at WNT
    p.update(0x100, true);
    expect(p.predict(0x100).taken).toBe(true);
  });

  it("two not taken needed from weakly taken", () => {
    /**
     * WT -> needs 2 not-taken outcomes to flip to predicting not-taken.
     *
     * WT + not-taken -> WNT (predicts not-taken after one)
     * But this is the key: the first not-taken only moves to WNT.
     */
    const p = new TwoBitPredictor(1024, TwoBitState.WEAKLY_TAKEN);
    p.update(0x100, false);
    // Now at WNT -> predicts not-taken
    expect(p.predict(0x100).taken).toBe(false);
  });

  it("strongly taken needs two to flip", () => {
    /**
     * ST -> needs 2 not-taken to flip to not-taken prediction.
     *
     * ST + NT -> WT (still predicts taken)
     * WT + NT -> WNT (now predicts not-taken)
     */
    const p = new TwoBitPredictor(1024, TwoBitState.STRONGLY_TAKEN);
    p.update(0x100, false);
    // ST -> WT: still predicts taken
    expect(p.predict(0x100).taken).toBe(true);

    p.update(0x100, false);
    // WT -> WNT: now predicts not-taken
    expect(p.predict(0x100).taken).toBe(false);
  });

  it("getState debug method", () => {
    /** The getState() method exposes internal state for testing. */
    const p = new TwoBitPredictor();
    expect(p.getState(0x100)).toBe(TwoBitState.WEAKLY_NOT_TAKEN);
    p.update(0x100, true);
    expect(p.getState(0x100)).toBe(TwoBitState.WEAKLY_TAKEN);
  });

  it("confidence: strongly vs weakly", () => {
    /** Strong states have confidence 1.0; weak states have 0.5. */
    let p = new TwoBitPredictor(1024, TwoBitState.STRONGLY_TAKEN);
    expect(p.predict(0x100).confidence).toBe(1.0);

    p = new TwoBitPredictor(1024, TwoBitState.WEAKLY_TAKEN);
    expect(p.predict(0x100).confidence).toBe(0.5);

    p = new TwoBitPredictor(1024, TwoBitState.WEAKLY_NOT_TAKEN);
    expect(p.predict(0x100).confidence).toBe(0.5);

    p = new TwoBitPredictor(1024, TwoBitState.STRONGLY_NOT_TAKEN);
    expect(p.predict(0x100).confidence).toBe(1.0);
  });
});

// ─── Loop behavior ────────────────────────────────────────────────────────────

describe("TwoBitPredictor loop behavior", () => {
  it("loop mispredicts once not twice", () => {
    /**
     * A loop running 10 times: only 1 misprediction (the exit).
     *
     * Starting from default WEAKLY_NOT_TAKEN:
     * Iter 1: WNT -> predict NT, actual T -> WRONG, move to WT
     * Iter 2: WT -> predict T, actual T -> correct, move to ST
     * Iter 3-9: ST -> predict T, actual T -> correct (7x), saturated at ST
     * Iter 10: ST -> predict T, actual NT -> WRONG, move to WT
     *
     * Total: 8 correct, 2 incorrect.
     * BUT on second invocation, state is WT:
     * Iter 1: WT -> predict T, actual T -> correct! (only 1 miss per invocation)
     */
    const p = new TwoBitPredictor();
    const pc = 0x100;

    // First invocation (cold start)
    for (let i = 0; i < 10; i++) {
      p.update(pc, i < 9);
    }

    expect(p.stats.incorrect).toBe(2); // cold start + exit
    expect(p.stats.correct).toBe(8);
  });

  it("loop second invocation: one miss", () => {
    /**
     * Second loop invocation: only misses the exit.
     *
     * After first run, state is WT (weakly taken).
     * Second run:
     * Iter 1: WT -> predict T, actual T -> correct! -> ST
     * Iter 2-9: ST -> predict T -> correct (8x)
     * Iter 10: ST -> predict T, actual NT -> WRONG -> WT
     *
     * Only 1 misprediction in the second invocation.
     */
    const p = new TwoBitPredictor();
    const pc = 0x100;

    // First invocation
    for (let i = 0; i < 10; i++) {
      p.update(pc, i < 9);
    }

    // Record stats so far
    const firstRunIncorrect = p.stats.incorrect;

    // Second invocation
    for (let i = 0; i < 10; i++) {
      p.update(pc, i < 9);
    }

    const secondRunIncorrect = p.stats.incorrect - firstRunIncorrect;
    expect(secondRunIncorrect).toBe(1); // only the exit miss
  });
});

// ─── TwoBit vs OneBit ───────────────────────────────────────────────────────

describe("TwoBit vs OneBit", () => {
  it("two bit beats one bit on repeated loops", () => {
    /**
     * On repeated loop invocations, 2-bit is strictly better than 1-bit.
     *
     * After warmup, 2-bit has 1 miss/invocation vs 1-bit's 2.
     */
    const oneBit = new OneBitPredictor();
    const twoBit = new TwoBitPredictor();
    const pc = 0x100;

    // Run the loop 5 times, 10 iterations each
    for (let run = 0; run < 5; run++) {
      for (let i = 0; i < 10; i++) {
        const taken = i < 9;
        oneBit.update(pc, taken);
        twoBit.update(pc, taken);
      }
    }

    // 2-bit should have better accuracy
    expect(twoBit.stats.accuracy).toBeGreaterThan(oneBit.stats.accuracy);
  });

  it("both handle always taken", () => {
    /** On always-taken sequences, both converge to 100% after warmup. */
    const oneBit = new OneBitPredictor();
    const twoBit = new TwoBitPredictor();
    const pc = 0x100;

    // Warmup
    oneBit.update(pc, true);
    twoBit.update(pc, true);

    // Reset stats after warmup
    oneBit.stats.reset();
    twoBit.stats.reset();

    // Run 100 always-taken branches
    for (let i = 0; i < 100; i++) {
      oneBit.update(pc, true);
      twoBit.update(pc, true);
    }

    expect(oneBit.stats.accuracy).toBe(100.0);
    expect(twoBit.stats.accuracy).toBe(100.0);
  });
});

// ─── Table size effects ──────────────────────────────────────────────────────

describe("TwoBitPredictor table size", () => {
  it("small table causes aliasing", () => {
    /** table_size=2: branch at 0 and 2 alias to the same slot. */
    const p = new TwoBitPredictor(2);
    // Branch at 0: always taken -> should converge to STRONGLY_TAKEN
    for (let i = 0; i < 5; i++) {
      p.update(0, true);
    }
    expect(p.getState(0)).toBe(TwoBitState.STRONGLY_TAKEN);

    // Branch at 2 aliases to same slot (2 % 2 = 0)
    // Reading state at pc=2 shows the same entry
    expect(p.getState(2)).toBe(TwoBitState.STRONGLY_TAKEN);
  });

  it("large table avoids aliasing", () => {
    /** With table_size=4096, branches 0 and 2 are independent. */
    const p = new TwoBitPredictor(4096);
    for (let i = 0; i < 5; i++) {
      p.update(0, true);
    }
    // Branch at 2 is in a different slot
    expect(p.getState(2)).toBe(TwoBitState.WEAKLY_NOT_TAKEN); // default
  });
});

// ─── Reset ────────────────────────────────────────────────────────────────────

describe("TwoBitPredictor reset", () => {
  it("reset clears table", () => {
    const p = new TwoBitPredictor();
    p.update(0x100, true);
    p.update(0x100, true);
    p.reset();
    // After reset, back to initial state
    expect(p.getState(0x100)).toBe(TwoBitState.WEAKLY_NOT_TAKEN);
  });

  it("reset clears stats", () => {
    const p = new TwoBitPredictor();
    p.update(0x100, true);
    p.reset();
    expect(p.stats.predictions).toBe(0);
  });
});

// ─── Full state transition walkthrough ────────────────────────────────────────

describe("TwoBitPredictor full transition walkthrough", () => {
  it("walk up and down", () => {
    /**
     * Start at SNT, walk up to ST, then back down to SNT.
     *
     * SNT ->(T)-> WNT ->(T)-> WT ->(T)-> ST ->(NT)-> WT ->(NT)-> WNT ->(NT)-> SNT
     */
    const p = new TwoBitPredictor(1024, TwoBitState.STRONGLY_NOT_TAKEN);
    const pc = 0x100;

    expect(p.getState(pc)).toBe(TwoBitState.STRONGLY_NOT_TAKEN);

    p.update(pc, true);
    expect(p.getState(pc)).toBe(TwoBitState.WEAKLY_NOT_TAKEN);

    p.update(pc, true);
    expect(p.getState(pc)).toBe(TwoBitState.WEAKLY_TAKEN);

    p.update(pc, true);
    expect(p.getState(pc)).toBe(TwoBitState.STRONGLY_TAKEN);

    // Now walk back down
    p.update(pc, false);
    expect(p.getState(pc)).toBe(TwoBitState.WEAKLY_TAKEN);

    p.update(pc, false);
    expect(p.getState(pc)).toBe(TwoBitState.WEAKLY_NOT_TAKEN);

    p.update(pc, false);
    expect(p.getState(pc)).toBe(TwoBitState.STRONGLY_NOT_TAKEN);
  });
});
