/**
 * Tests for static branch predictors — AlwaysTaken, AlwaysNotTaken, BTFNT.
 *
 * Static predictors are simple strategies that don't learn from history.
 * These tests verify their fixed prediction behavior and accuracy tracking.
 */

import { describe, expect, it } from "vitest";
import {
  AlwaysNotTakenPredictor,
  AlwaysTakenPredictor,
  BackwardTakenForwardNotTaken,
} from "../src/static.js";

// ─── AlwaysTakenPredictor ─────────────────────────────────────────────────────

describe("AlwaysTakenPredictor", () => {
  it("always predicts taken", () => {
    const p = new AlwaysTakenPredictor();
    for (const pc of [0x0, 0x100, 0xdead, 0xffff_ffff]) {
      const pred = p.predict(pc);
      expect(pred.taken).toBe(true);
    }
  });

  it("confidence is zero", () => {
    /** Static predictors have no confidence — they're just guessing. */
    const p = new AlwaysTakenPredictor();
    expect(p.predict(0x100).confidence).toBe(0.0);
  });

  it("target is null", () => {
    /** Static predictors don't know target addresses. */
    const p = new AlwaysTakenPredictor();
    expect(p.predict(0x100).target).toBeNull();
  });

  it("100% accuracy on all taken", () => {
    /** Perfect accuracy when every branch is taken. */
    const p = new AlwaysTakenPredictor();
    for (let i = 0; i < 100; i++) {
      p.update(i * 4, true);
    }
    expect(p.stats.accuracy).toBe(100.0);
  });

  it("0% accuracy on all not taken", () => {
    /** Zero accuracy when no branch is taken. */
    const p = new AlwaysTakenPredictor();
    for (let i = 0; i < 100; i++) {
      p.update(i * 4, false);
    }
    expect(p.stats.accuracy).toBe(0.0);
  });

  it("mixed sequence", () => {
    /** 60% taken -> 60% accuracy for always-taken. */
    const p = new AlwaysTakenPredictor();
    for (let i = 0; i < 60; i++) {
      p.update(0x100, true);
    }
    for (let i = 0; i < 40; i++) {
      p.update(0x100, false);
    }
    expect(p.stats.accuracy).toBe(60.0);
  });

  it("reset clears stats", () => {
    const p = new AlwaysTakenPredictor();
    p.update(0x100, true);
    p.reset();
    expect(p.stats.predictions).toBe(0);
  });
});

// ─── AlwaysNotTakenPredictor ──────────────────────────────────────────────────

describe("AlwaysNotTakenPredictor", () => {
  it("always predicts not taken", () => {
    const p = new AlwaysNotTakenPredictor();
    for (const pc of [0x0, 0x100, 0xdead]) {
      const pred = p.predict(pc);
      expect(pred.taken).toBe(false);
    }
  });

  it("100% accuracy on all not taken", () => {
    const p = new AlwaysNotTakenPredictor();
    for (let i = 0; i < 100; i++) {
      p.update(i * 4, false);
    }
    expect(p.stats.accuracy).toBe(100.0);
  });

  it("0% accuracy on all taken", () => {
    const p = new AlwaysNotTakenPredictor();
    for (let i = 0; i < 100; i++) {
      p.update(i * 4, true);
    }
    expect(p.stats.accuracy).toBe(0.0);
  });

  it("inverse of always taken", () => {
    /** AlwaysNotTaken's accuracy is 100% - AlwaysTaken's accuracy. */
    const takenPred = new AlwaysTakenPredictor();
    const notTakenPred = new AlwaysNotTakenPredictor();
    // 70% taken sequence
    const outcomes = [
      ...Array.from({ length: 70 }, () => true),
      ...Array.from({ length: 30 }, () => false),
    ];
    for (let i = 0; i < outcomes.length; i++) {
      takenPred.update(i * 4, outcomes[i]);
      notTakenPred.update(i * 4, outcomes[i]);
    }
    expect(takenPred.stats.accuracy).toBe(70.0);
    expect(notTakenPred.stats.accuracy).toBe(30.0);
    expect(
      Math.abs(takenPred.stats.accuracy + notTakenPred.stats.accuracy - 100.0),
    ).toBeLessThan(1e-10);
  });

  it("reset clears stats", () => {
    const p = new AlwaysNotTakenPredictor();
    p.update(0x100, false);
    p.reset();
    expect(p.stats.predictions).toBe(0);
  });
});

// ─── BackwardTakenForwardNotTaken ─────────────────────────────────────────────

describe("BackwardTakenForwardNotTaken", () => {
  it("cold start predicts not taken", () => {
    /** Before seeing a target, default to not-taken. */
    const p = new BackwardTakenForwardNotTaken();
    const pred = p.predict(0x108);
    expect(pred.taken).toBe(false);
  });

  it("backward branch predicts taken", () => {
    /** Backward branch (target < pc) -> predict taken. */
    const p = new BackwardTakenForwardNotTaken();
    // First, teach the predictor the target
    p.update(0x108, true, 0x100);
    // Now it knows target=0x100 < pc=0x108 -> backward -> taken
    const pred = p.predict(0x108);
    expect(pred.taken).toBe(true);
  });

  it("forward branch predicts not taken", () => {
    /** Forward branch (target > pc) -> predict not-taken. */
    const p = new BackwardTakenForwardNotTaken();
    p.update(0x200, false, 0x20c);
    const pred = p.predict(0x200);
    expect(pred.taken).toBe(false);
  });

  it("equal target predicts taken", () => {
    /** target == pc (degenerate loop) -> predict taken. */
    const p = new BackwardTakenForwardNotTaken();
    p.update(0x100, true, 0x100);
    const pred = p.predict(0x100);
    expect(pred.taken).toBe(true);
  });

  it("backward branch accuracy on loop", () => {
    /**
     * A loop (backward branch, taken 9 times, not-taken once).
     *
     * After the first update (which teaches the target), the predictor
     * knows this is a backward branch -> predicts taken.
     */
    const p = new BackwardTakenForwardNotTaken();
    const pc = 0x108;
    const target = 0x100; // backward

    // Run the loop 10 times (9 taken + 1 not-taken)
    for (let i = 0; i < 10; i++) {
      const taken = i < 9; // taken on iterations 0-8, not-taken on 9
      p.update(pc, taken, target);
    }

    // BTFNT always predicts taken for backward branches (target < pc):
    // Updates 0-8: backward -> predicted taken, actual taken -> correct (9)
    // Update 9: backward -> predicted taken, actual not-taken -> WRONG (1)
    expect(p.stats.correct).toBe(9);
    expect(p.stats.incorrect).toBe(1);
  });

  it("forward branch accuracy", () => {
    /** Forward branch, not taken 8 out of 10 times. */
    const p = new BackwardTakenForwardNotTaken();
    const pc = 0x200;
    const target = 0x20c; // forward

    const outcomes = [
      ...Array.from({ length: 8 }, () => false),
      ...Array.from({ length: 2 }, () => true),
    ];
    for (const taken of outcomes) {
      p.update(pc, taken, target);
    }

    // Update 0: no prior target -> predicted not-taken, actual not-taken -> correct
    // Updates 1-7: forward -> predicted not-taken, actual not-taken -> correct (7)
    // Updates 8-9: forward -> predicted not-taken, actual taken -> WRONG (2)
    expect(p.stats.correct).toBe(8);
    expect(p.stats.incorrect).toBe(2);
  });

  it("confidence on known branch", () => {
    /** Known branches should have moderate confidence (0.5). */
    const p = new BackwardTakenForwardNotTaken();
    p.update(0x108, true, 0x100);
    const pred = p.predict(0x108);
    expect(pred.confidence).toBe(0.5);
  });

  it("confidence on unknown branch", () => {
    /** Unknown branches should have zero confidence. */
    const p = new BackwardTakenForwardNotTaken();
    const pred = p.predict(0x108);
    expect(pred.confidence).toBe(0.0);
  });

  it("target in prediction", () => {
    /** Known branches include target in prediction. */
    const p = new BackwardTakenForwardNotTaken();
    p.update(0x108, true, 0x100);
    const pred = p.predict(0x108);
    expect(pred.target).toBe(0x100);
  });

  it("reset clears targets and stats", () => {
    const p = new BackwardTakenForwardNotTaken();
    p.update(0x108, true, 0x100);
    p.reset();
    expect(p.stats.predictions).toBe(0);
    // After reset, should be cold start again
    const pred = p.predict(0x108);
    expect(pred.taken).toBe(false); // no target known
  });

  it("multiple branches", () => {
    /** Multiple branches with different directions. */
    const p = new BackwardTakenForwardNotTaken();
    // Branch A: backward (loop)
    p.update(0x108, true, 0x100);
    // Branch B: forward (if-else)
    p.update(0x200, false, 0x20c);

    expect(p.predict(0x108).taken).toBe(true); // backward -> taken
    expect(p.predict(0x200).taken).toBe(false); // forward -> not taken
  });

  it("update without target", () => {
    /** Update with target=undefined doesn't crash or overwrite existing target. */
    const p = new BackwardTakenForwardNotTaken();
    p.update(0x108, true, 0x100);
    // Update without target — should preserve old target
    p.update(0x108, true);
    const pred = p.predict(0x108);
    expect(pred.taken).toBe(true); // still knows it's backward
  });
});
