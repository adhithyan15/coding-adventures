/**
 * Tests for structural hazard detection — resource conflict handling.
 *
 * These tests verify that the StructuralHazardDetector correctly identifies
 * when two instructions compete for the same hardware resource.
 */

import { describe, it, expect } from "vitest";
import { StructuralHazardDetector } from "../src/structuralHazard.js";
import { HazardAction, PipelineSlot } from "../src/types.js";

describe("ALU conflict", () => {
  /** Two ALU instructions in adjacent stages with limited ALUs. */

  it("should stall with 1 ALU when two ALU instructions overlap", () => {
    /**
     * With 1 ALU, two ALU instructions at once → stall.
     *
     * ADD R1, R2, R3   ← in EX (using the ALU)
     * SUB R4, R5, R6   ← in ID (about to enter EX, needs ALU)
     * Only 1 ALU → SUB must wait.
     */
    const detector = new StructuralHazardDetector({ numAlus: 1 });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: true,
      sourceRegs: [5, 6],
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: true,
      destReg: 1,
    });

    const result = detector.detect(idStage, exStage);

    expect(result.action).toBe(HazardAction.STALL);
    expect(result.stallCycles).toBe(1);
    expect(result.reason).toContain("ALU");
  });

  it("should not stall with 2 ALUs when two ALU instructions overlap", () => {
    /** With 2 ALUs, two ALU instructions can execute in parallel. */
    const detector = new StructuralHazardDetector({ numAlus: 2 });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: true,
      sourceRegs: [5, 6],
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: true,
      destReg: 1,
    });

    const result = detector.detect(idStage, exStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should not conflict when one uses ALU and other uses FP", () => {
    /** ALU + FP instruction at the same time — different units, no conflict. */
    const detector = new StructuralHazardDetector({
      numAlus: 1,
      numFpUnits: 1,
    });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: true,
      usesFp: false,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: false,
      usesFp: true,
    });

    const result = detector.detect(idStage, exStage);

    expect(result.action).toBe(HazardAction.NONE);
  });
});

describe("FP unit conflict", () => {
  /** Two FP instructions with limited FP units. */

  it("should stall with 1 FP unit when two FP instructions overlap", () => {
    /** With 1 FP unit, two FP instructions → stall. */
    const detector = new StructuralHazardDetector({ numFpUnits: 1 });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesFp: true,
      usesAlu: false,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesFp: true,
      usesAlu: false,
    });

    const result = detector.detect(idStage, exStage);

    expect(result.action).toBe(HazardAction.STALL);
    expect(result.reason).toContain("FP unit");
  });

  it("should not stall with 2 FP units when two FP instructions overlap", () => {
    /** With 2 FP units, two FP instructions → no stall. */
    const detector = new StructuralHazardDetector({ numFpUnits: 2 });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesFp: true,
      usesAlu: false,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesFp: true,
      usesAlu: false,
    });

    const result = detector.detect(idStage, exStage);

    expect(result.action).toBe(HazardAction.NONE);
  });
});

describe("memory port conflict", () => {
  /** Fetch and data access competing for a shared memory bus. */

  it("should not conflict with split L1I/L1D caches", () => {
    /** With split L1I/L1D caches, IF and MEM never conflict. */
    const detector = new StructuralHazardDetector({ splitCaches: true });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: false,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: false,
    });
    const ifStage = new PipelineSlot({ valid: true, pc: 0x100c });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x0ff8,
      memRead: true,
      usesAlu: false,
    });

    const result = detector.detect(idStage, exStage, ifStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should stall when shared cache and IF + MEM load conflict", () => {
    /** With shared cache, IF and MEM (load) → stall. */
    const detector = new StructuralHazardDetector({ splitCaches: false });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: false,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: false,
    });
    const ifStage = new PipelineSlot({ valid: true, pc: 0x100c });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x0ff8,
      memRead: true,
      usesAlu: false,
    });

    const result = detector.detect(idStage, exStage, ifStage, memStage);

    expect(result.action).toBe(HazardAction.STALL);
    expect(result.reason.toLowerCase()).toContain("memory bus");
  });

  it("should stall when shared cache and IF + MEM store conflict", () => {
    /** With shared cache, IF and MEM (store) → stall. */
    const detector = new StructuralHazardDetector({ splitCaches: false });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: false,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: false,
    });
    const ifStage = new PipelineSlot({ valid: true, pc: 0x100c });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x0ff8,
      memWrite: true,
      usesAlu: false,
    });

    const result = detector.detect(idStage, exStage, ifStage, memStage);

    expect(result.action).toBe(HazardAction.STALL);
  });

  it("should not conflict when shared cache but MEM has no memory access", () => {
    /** Shared cache but MEM isn't doing a load/store — no conflict. */
    const detector = new StructuralHazardDetector({ splitCaches: false });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: false,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: false,
    });
    const ifStage = new PipelineSlot({ valid: true, pc: 0x100c });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x0ff8,
      memRead: false,
      memWrite: false,
    });

    const result = detector.detect(idStage, exStage, ifStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });
});

describe("edge cases", () => {
  /** Edge cases — empty stages, no if/mem provided. */

  it("should return NONE when ID stage is empty", () => {
    /** ID stage is a bubble — can't have a structural hazard. */
    const detector = new StructuralHazardDetector({ numAlus: 1 });
    const idStage = new PipelineSlot({ valid: false });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: true,
    });

    const result = detector.detect(idStage, exStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should return NONE when EX stage is empty", () => {
    /** EX stage is a bubble — resource is free. */
    const detector = new StructuralHazardDetector({ numAlus: 1 });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: true,
    });
    const exStage = new PipelineSlot({ valid: false });

    const result = detector.detect(idStage, exStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should skip memory check when if/mem stages not provided", () => {
    /** If ifStage and memStage are not provided, skip memory check. */
    const detector = new StructuralHazardDetector({
      numAlus: 2,
      splitCaches: false,
    });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: true,
    });

    // Only checking execution unit conflict (2 ALUs → no stall)
    const result = detector.detect(idStage, exStage);

    expect(result.action).toBe(HazardAction.NONE);
  });

  it("should return NONE when shared cache but MEM stage is empty", () => {
    /** Shared cache but MEM stage is a bubble — no conflict. */
    const detector = new StructuralHazardDetector({ splitCaches: false });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      usesAlu: false,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      usesAlu: false,
    });
    const ifStage = new PipelineSlot({ valid: true, pc: 0x100c });
    const memStage = new PipelineSlot({ valid: false });

    const result = detector.detect(idStage, exStage, ifStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });
});
