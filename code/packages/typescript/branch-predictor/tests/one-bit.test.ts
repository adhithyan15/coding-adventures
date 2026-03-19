/**
 * Tests for the one-bit branch predictor.
 *
 * The one-bit predictor stores a single bit per branch: the last outcome.
 * These tests verify its learning behavior, the double-misprediction problem
 * on loops, and the aliasing problem with small tables.
 */

import { describe, expect, it } from "vitest";
import { OneBitPredictor } from "../src/one-bit.js";

// ─── Basics ─────────────────────────────────────────────────────────────────

describe("OneBitPredictor basics", () => {
  it("cold start predicts not taken", () => {
    /** Uninitialized entries default to not-taken (bit = 0). */
    const p = new OneBitPredictor();
    const pred = p.predict(0x100);
    expect(pred.taken).toBe(false);
  });

  it("predicts last outcome: taken", () => {
    /** After a taken branch, predicts taken next time. */
    const p = new OneBitPredictor();
    p.update(0x100, true);
    const pred = p.predict(0x100);
    expect(pred.taken).toBe(true);
  });

  it("predicts last outcome: not taken", () => {
    /** After a not-taken branch, predicts not-taken. */
    const p = new OneBitPredictor();
    p.update(0x100, true);
    p.update(0x100, false);
    const pred = p.predict(0x100);
    expect(pred.taken).toBe(false);
  });

  it("confidence is 0.5", () => {
    /** One bit of history -> moderate confidence. */
    const p = new OneBitPredictor();
    expect(p.predict(0x100).confidence).toBe(0.5);
  });

  it("different branches are independent", () => {
    /** Two branches at different PCs have independent state. */
    const p = new OneBitPredictor(4096); // large table to avoid aliasing
    p.update(0x100, true);
    p.update(0x200, false);
    expect(p.predict(0x100).taken).toBe(true);
    expect(p.predict(0x200).taken).toBe(false);
  });
});

// ─── Loop Pattern ───────────────────────────────────────────────────────────

describe("OneBitPredictor loop pattern", () => {
  it("loop mispredicts first and last", () => {
    /**
     * A loop running 10 times: TTTTTTTTTN (9 taken, 1 not-taken).
     *
     * Expected mispredictions:
     * - Iteration 1 (cold start): predict NT, actual T -> WRONG
     * - Iterations 2-9: predict T, actual T -> correct (8x)
     * - Iteration 10: predict T, actual NT -> WRONG
     *
     * Total: 8 correct, 2 incorrect = 80% accuracy
     */
    const p = new OneBitPredictor();
    const pc = 0x100;

    for (let i = 0; i < 10; i++) {
      const taken = i < 9;
      p.update(pc, taken);
    }

    expect(p.stats.correct).toBe(8);
    expect(p.stats.incorrect).toBe(2);
    expect(p.stats.accuracy).toBe(80.0);
  });

  it("loop repeated invocations", () => {
    /**
     * Running the same loop twice -> mispredicts on re-entry too.
     *
     * First run: TTTTTTTTTN (cold start miss + exit miss = 2 wrong)
     * Second run: TTTTTTTTTN (re-entry miss + exit miss = 2 wrong)
     *
     * After first run, bit = 0 (last outcome was NT).
     * Second run iter 1: predict NT, actual T -> WRONG.
     */
    const p = new OneBitPredictor();
    const pc = 0x100;

    // First invocation: 10 iterations
    for (let i = 0; i < 10; i++) {
      p.update(pc, i < 9);
    }

    // Second invocation: 10 more iterations
    for (let i = 0; i < 10; i++) {
      p.update(pc, i < 9);
    }

    // 2 mispredictions per invocation x 2 invocations = 4 wrong
    expect(p.stats.incorrect).toBe(4);
    expect(p.stats.correct).toBe(16);
  });
});

// ─── Aliasing ───────────────────────────────────────────────────────────────

describe("OneBitPredictor aliasing", () => {
  it("aliasing with small table", () => {
    /**
     * With tableSize=4, PCs differing by 4 alias to the same slot.
     *
     * Branch A at 0x100 -> index 0 (0x100 % 4 = 0)
     * Branch B at 0x104 -> index 0 (0x104 % 4 = 0)
     *
     * They corrupt each other's predictions.
     */
    const p = new OneBitPredictor(4);

    // Branch A: taken
    p.update(0x100, true);
    expect(p.predict(0x100).taken).toBe(true);

    // Branch B overwrites the same slot: not-taken
    p.update(0x104, false);

    // Branch A now sees B's state (not-taken) -> WRONG
    expect(p.predict(0x100).taken).toBe(false);
  });

  it("no aliasing with large table", () => {
    /** With a large enough table, nearby branches don't alias. */
    const p = new OneBitPredictor(4096);
    p.update(0x100, true);
    p.update(0x104, false);
    // Different indices -> independent
    expect(p.predict(0x100).taken).toBe(true);
    expect(p.predict(0x104).taken).toBe(false);
  });
});

// ─── Reset ──────────────────────────────────────────────────────────────────

describe("OneBitPredictor reset", () => {
  it("reset clears table", () => {
    const p = new OneBitPredictor();
    p.update(0x100, true);
    p.reset();
    // After reset, cold start again
    expect(p.predict(0x100).taken).toBe(false);
  });

  it("reset clears stats", () => {
    const p = new OneBitPredictor();
    p.update(0x100, true);
    p.reset();
    expect(p.stats.predictions).toBe(0);
  });
});

// ─── Alternating Pattern ────────────────────────────────────────────────────

describe("OneBitPredictor alternating pattern", () => {
  it("alternating is worst case", () => {
    /**
     * Alternating T/NT: every prediction is wrong after the first.
     *
     * Seq: T, N, T, N, T, N
     * Pred: NT(cold), T, N, T, N, T  -> all wrong!
     * Every prediction is wrong because it always predicts the opposite.
     */
    const p = new OneBitPredictor();
    const pc = 0x100;
    for (let i = 0; i < 100; i++) {
      const taken = i % 2 === 0; // alternating T, N, T, N, ...
      p.update(pc, taken);
    }

    // Every prediction is wrong: 0% accuracy
    expect(p.stats.accuracy).toBe(0.0);
  });
});
