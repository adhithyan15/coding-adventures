/**
 * Tests for control hazard detection — branch misprediction handling.
 *
 * These tests verify that the ControlHazardDetector correctly identifies
 * branch mispredictions and signals the pipeline to flush.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { ControlHazardDetector } from "../src/controlHazard.js";
import { HazardAction, PipelineSlot } from "../src/types.js";

describe("correctly predicted branch", () => {
  /** When the branch predictor guessed right — no hazard. */

  let detector: ControlHazardDetector;

  beforeEach(() => {
    detector = new ControlHazardDetector();
  });

  it("should return NONE when predicted taken and actually taken", () => {
    /** Predictor said 'taken', branch IS taken — correct, no flush. */
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      isBranch: true,
      branchPredictedTaken: true,
      branchTaken: true,
    });

    const result = detector.detect(exStage);

    expect(result.action).toBe(HazardAction.NONE);
    expect(result.flushCount).toBe(0);
    expect(result.reason).toContain("correctly predicted");
  });

  it("should return NONE when predicted not taken and actually not taken", () => {
    /** Predictor said 'not taken', branch is NOT taken — correct. */
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x2000,
      isBranch: true,
      branchPredictedTaken: false,
      branchTaken: false,
    });

    const result = detector.detect(exStage);

    expect(result.action).toBe(HazardAction.NONE);
    expect(result.flushCount).toBe(0);
  });
});

describe("mispredicted branch", () => {
  /** When the branch predictor guessed wrong — must flush. */

  let detector: ControlHazardDetector;

  beforeEach(() => {
    detector = new ControlHazardDetector();
  });

  it("should FLUSH when predicted not taken but actually taken", () => {
    /**
     * Predictor said 'not taken', but branch IS taken → FLUSH.
     *
     * The pipeline fetched fall-through instructions, but it should
     * have jumped to the branch target. Flush IF and ID.
     */
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      isBranch: true,
      branchPredictedTaken: false,
      branchTaken: true,
    });

    const result = detector.detect(exStage);

    expect(result.action).toBe(HazardAction.FLUSH);
    expect(result.flushCount).toBe(2); // IF and ID
    expect(result.reason).toContain("not-taken, actually taken");
  });

  it("should FLUSH when predicted taken but actually not taken", () => {
    /**
     * Predictor said 'taken', but branch is NOT taken → FLUSH.
     *
     * The pipeline fetched from the branch target, but it should
     * have continued with the fall-through path.
     */
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x3000,
      isBranch: true,
      branchPredictedTaken: true,
      branchTaken: false,
    });

    const result = detector.detect(exStage);

    expect(result.action).toBe(HazardAction.FLUSH);
    expect(result.flushCount).toBe(2);
    expect(result.reason).toContain("taken, actually not-taken");
  });
});

describe("non-branch instruction", () => {
  /** Non-branch instructions can never cause a control hazard. */

  let detector: ControlHazardDetector;

  beforeEach(() => {
    detector = new ControlHazardDetector();
  });

  it("should return NONE for ALU instruction", () => {
    /** An ADD instruction in EX — no control hazard possible. */
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      isBranch: false,
      sourceRegs: [2, 3],
      destReg: 1,
      usesAlu: true,
    });

    const result = detector.detect(exStage);

    expect(result.action).toBe(HazardAction.NONE);
    expect(result.reason).toContain("not a branch");
  });

  it("should return NONE for load instruction", () => {
    /** A load instruction in EX — not a branch, no control hazard. */
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      isBranch: false,
      destReg: 1,
      memRead: true,
      usesAlu: false,
    });

    const result = detector.detect(exStage);

    expect(result.action).toBe(HazardAction.NONE);
  });
});

describe("empty EX stage", () => {
  /** Empty EX stage (bubble) — nothing to check. */

  let detector: ControlHazardDetector;

  beforeEach(() => {
    detector = new ControlHazardDetector();
  });

  it("should return NONE when EX stage is empty", () => {
    /** EX stage is a bubble — no instruction, no hazard. */
    const exStage = new PipelineSlot({ valid: false });

    const result = detector.detect(exStage);

    expect(result.action).toBe(HazardAction.NONE);
    expect(result.reason.toLowerCase()).toContain("empty");
  });
});
