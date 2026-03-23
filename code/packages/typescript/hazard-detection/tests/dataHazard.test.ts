/**
 * Tests for data hazard detection — RAW hazards, forwarding, and stalling.
 *
 * These tests verify that the DataHazardDetector correctly identifies
 * data dependencies between pipeline stages and chooses the right
 * resolution strategy (forward vs. stall).
 */

import { describe, it, expect, beforeEach } from "vitest";
import { DataHazardDetector } from "../src/dataHazard.js";
import { HazardAction, PipelineSlot } from "../src/types.js";

describe("RAW forwarding from EX", () => {
  /**
   * RAW hazard where the value can be forwarded from the EX stage.
   *
   * Scenario:
   *     ADD R1, R2, R3    ← in EX stage (just computed R1 = 42)
   *     SUB R4, R1, R5    ← in ID stage (reads R1)
   *
   * The value of R1 is available in the EX stage (the ALU just produced it).
   * We forward it directly to ID — zero stall cycles.
   */

  let detector: DataHazardDetector;

  beforeEach(() => {
    detector = new DataHazardDetector();
  });

  it("should forward from EX when single source reg matches EX dest", () => {
    /** SUB reads R1, ADD in EX writes R1 → forward from EX. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 42,
      usesAlu: true,
    });
    const memStage = new PipelineSlot(); // empty

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.FORWARD_FROM_EX);
    expect(result.forwardedValue).toBe(42);
    expect(result.forwardedFrom).toBe("EX");
    expect(result.stallCycles).toBe(0);
  });

  it("should forward from EX when second source reg matches EX dest", () => {
    /** ADD R4, R5, R1 — R1 is the second source reg, still matches EX. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [5, 1],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 99,
      usesAlu: true,
    });
    const memStage = new PipelineSlot();

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.FORWARD_FROM_EX);
    expect(result.forwardedValue).toBe(99);
  });
});

describe("RAW forwarding from MEM", () => {
  /**
   * RAW hazard where the value is forwarded from the MEM stage.
   *
   * Scenario (2-instruction gap):
   *     ADD R1, R2, R3    ← in MEM stage (R1 computed 2 cycles ago)
   *     NOP               ← in EX stage  (no conflict)
   *     SUB R4, R1, R5    ← in ID stage  (reads R1)
   *
   * The value of R1 has passed through EX and is now in MEM.
   * We forward from MEM.
   */

  let detector: DataHazardDetector;

  beforeEach(() => {
    detector = new DataHazardDetector();
  });

  it("should forward from MEM when source reg matches MEM dest", () => {
    /** R1 available in MEM stage → forward from MEM. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x100c,
      sourceRegs: [1],
      usesAlu: true,
    });
    const exStage = new PipelineSlot(); // NOP or different instruction
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 77,
      usesAlu: true,
    });

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.FORWARD_FROM_MEM);
    expect(result.forwardedValue).toBe(77);
    expect(result.forwardedFrom).toBe("MEM");
  });

  it("should prefer EX over MEM when both write the same register", () => {
    /**
     * If both EX and MEM write R1, EX is newer — use EX's value.
     *
     * This happens with back-to-back writes to the same register:
     *     ADD R1, R2, R3    ← in MEM (old value of R1)
     *     MUL R1, R4, R5    ← in EX  (new value of R1)
     *     SUB R6, R1, R7    ← in ID  (should get MUL's value)
     */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x100c,
      sourceRegs: [1],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1008,
      destReg: 1,
      destValue: 200, // newer value
      usesAlu: true,
    });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 100, // older value
      usesAlu: true,
    });

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.FORWARD_FROM_EX);
    expect(result.forwardedValue).toBe(200); // newer value from EX
  });
});

describe("load-use hazard", () => {
  /**
   * Load-use hazard: a load in EX followed by a use in ID — must stall.
   *
   * Scenario:
   *     LW R1, [addr]    ← in EX stage (value not available until after MEM)
   *     ADD R4, R1, R5   ← in ID stage (needs R1 in EX) — must stall!
   *
   * The load instruction's result won't be available until the MEM stage
   * completes. But ADD needs it one cycle earlier, in EX. No amount of
   * forwarding can bridge this 1-cycle gap. We must stall.
   */

  let detector: DataHazardDetector;

  beforeEach(() => {
    detector = new DataHazardDetector();
  });

  it("should stall when load is followed by immediate use", () => {
    /** LW R1 in EX, ADD using R1 in ID → stall 1 cycle. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      memRead: true, // this is a load instruction
      usesAlu: false,
    });
    const memStage = new PipelineSlot();

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.STALL);
    expect(result.stallCycles).toBe(1);
    expect(result.reason.toLowerCase()).toContain("load-use");
  });

  it("should forward from MEM when load has a gap (no stall)", () => {
    /**
     * Load in MEM (not EX) + use in ID → forward from MEM, no stall.
     *
     * When there's a 1-instruction gap between load and use:
     *     LW R1, [addr]    ← now in MEM stage (load completing)
     *     NOP               ← in EX stage
     *     ADD R4, R1, R5   ← in ID stage
     *
     * The load value is available from MEM — forward it.
     */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x100c,
      sourceRegs: [1],
      usesAlu: true,
    });
    const exStage = new PipelineSlot(); // NOP or unrelated
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 55,
      memRead: true,
      usesAlu: false,
    });

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.FORWARD_FROM_MEM);
    expect(result.forwardedValue).toBe(55);
  });
});

describe("no data hazard", () => {
  /** Cases where no data hazard exists. */

  let detector: DataHazardDetector;

  beforeEach(() => {
    detector = new DataHazardDetector();
  });

  it("should return NONE when instructions use different registers", () => {
    /** Instructions use completely different registers — no conflict. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [2, 3],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 42,
      usesAlu: true,
    });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x0ffc,
      destReg: 4,
      destValue: 10,
      usesAlu: true,
    });

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should return NONE when ID stage is empty (bubble)", () => {
    /** ID stage is a bubble (empty) — nothing to check. */
    const idStage = new PipelineSlot({ valid: false });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      usesAlu: true,
    });
    const memStage = new PipelineSlot();

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should return NONE when ID has no source registers", () => {
    /** Instruction reads no registers (e.g., NOP) — no dependency. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      usesAlu: true,
    });
    const memStage = new PipelineSlot();

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should return NONE when EX has no dest register", () => {
    /** EX instruction writes no register (e.g., store) — no conflict. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: null, // store doesn't write a register
      memWrite: true,
      usesAlu: false,
    });
    const memStage = new PipelineSlot();

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should return NONE when EX is an empty bubble", () => {
    /** EX stage is a bubble — no conflict possible. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({ valid: false });
    const memStage = new PipelineSlot({ valid: false });

    const result = detector.detect(idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });
});

describe("multiple source registers", () => {
  /** Instructions with multiple source registers — mixed hazard cases. */

  let detector: DataHazardDetector;

  beforeEach(() => {
    detector = new DataHazardDetector();
  });

  it("should detect hazard when one source has hazard and other does not", () => {
    /** ADD R4, R1, R5 — R1 has a hazard (EX writes R1), R5 is fine. */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1, 5],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 42,
      usesAlu: true,
    });
    const memStage = new PipelineSlot();

    const result = detector.detect(idStage, exStage, memStage);

    // Should forward because of R1 hazard
    expect(result.action).toBe(HazardAction.FORWARD_FROM_EX);
    expect(result.forwardedValue).toBe(42);
  });

  it("should pick stall over forward when both are needed", () => {
    /**
     * If one source needs a stall and another needs forward, stall wins.
     *
     * LW R1, [addr]    ← in EX (load, must stall for R1)
     * ADD R4, R1, R2   ← in ID (R1 needs stall, R2 needs forward from MEM)
     *
     * Even though R2 can be forwarded, R1 forces a stall. The stall is
     * the higher-priority action.
     */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1008,
      sourceRegs: [1, 2],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      destReg: 1,
      memRead: true, // load — must stall
      usesAlu: false,
    });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 2,
      destValue: 88,
      usesAlu: true,
    });

    const result = detector.detect(idStage, exStage, memStage);

    // Stall takes priority over forward
    expect(result.action).toBe(HazardAction.STALL);
    expect(result.stallCycles).toBe(1);
  });

  it("should pick FORWARD_FROM_EX when both sources forward from different stages", () => {
    /**
     * R1 forwards from EX, R2 forwards from MEM.
     *
     * EX is higher priority than MEM, so the result is FORWARD_FROM_EX.
     */
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x100c,
      sourceRegs: [1, 2],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1008,
      destReg: 1,
      destValue: 10,
      usesAlu: true,
    });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 2,
      destValue: 20,
      usesAlu: true,
    });

    const result = detector.detect(idStage, exStage, memStage);

    // FORWARD_FROM_EX has higher priority than FORWARD_FROM_MEM
    expect(result.action).toBe(HazardAction.FORWARD_FROM_EX);
    expect(result.forwardedValue).toBe(10);
  });
});
