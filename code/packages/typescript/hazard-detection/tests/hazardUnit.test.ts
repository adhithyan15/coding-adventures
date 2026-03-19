/**
 * Tests for the combined hazard unit — priority, history, and stats.
 *
 * These tests verify that the HazardUnit correctly combines results from
 * all three detectors and applies the priority system:
 * FLUSH > STALL > FORWARD > NONE.
 */

import { describe, it, expect } from "vitest";
import { HazardUnit, pickHighestPriority } from "../src/hazardUnit.js";
import { HazardAction, HazardResult, PipelineSlot } from "../src/types.js";

describe("priority system", () => {
  /** Verify that higher-priority hazards override lower ones. */

  it("should let FLUSH override STALL", () => {
    /**
     * Branch misprediction (flush) + load-use (stall) → flush wins.
     *
     * Even though there's a data hazard, the branch misprediction
     * means the instruction with the data hazard is WRONG and will
     * be flushed anyway. No point stalling for it.
     */
    const unit = new HazardUnit({ numAlus: 1 });

    // EX: mispredicted branch (will cause FLUSH)
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      isBranch: true,
      branchPredictedTaken: false,
      branchTaken: true,
      usesAlu: true,
      destReg: null,
    });
    // ID: instruction that would have a load-use hazard with EX
    // (but it's going to be flushed anyway)
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1],
      usesAlu: true,
    });
    // MEM: a load that just completed
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x0ffc,
      destReg: 1,
      destValue: 42,
      memRead: true,
      usesAlu: false,
    });
    const ifStage = new PipelineSlot({ valid: true, pc: 0x1008 });

    const result = unit.check(ifStage, idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.FLUSH);
  });

  it("should let STALL override FORWARD", () => {
    /**
     * Load-use stall + forwarding available → stall wins.
     *
     * When a load-use hazard requires a stall, it doesn't matter that
     * another register could be forwarded. The stall is mandatory.
     */
    const unit = new HazardUnit();

    // EX: load instruction (will cause stall if ID reads its dest)
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      memRead: true,
      usesAlu: false,
    });
    // ID: reads R1 (stall) and R2 (could forward from MEM)
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1, 2],
      usesAlu: true,
    });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x0ffc,
      destReg: 2,
      destValue: 99,
      usesAlu: true,
    });
    const ifStage = new PipelineSlot({ valid: true, pc: 0x1008 });

    const result = unit.check(ifStage, idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.STALL);
  });

  it("should let FORWARD override NONE", () => {
    /** Forwarding available + no other hazard → forward. */
    const unit = new HazardUnit({ numAlus: 2 }); // avoid structural hazard on ALU

    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 42,
      usesAlu: true,
    });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1],
      usesAlu: true,
    });
    const memStage = new PipelineSlot();
    const ifStage = new PipelineSlot({ valid: true, pc: 0x1008 });

    const result = unit.check(ifStage, idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.FORWARD_FROM_EX);
    expect(result.forwardedValue).toBe(42);
  });

  it("should return NONE when no hazards exist", () => {
    /** All clear — no hazards of any type. */
    const unit = new HazardUnit({ numAlus: 2 }); // avoid structural hazard on ALU

    // All stages valid but no conflicts.
    const ifStage = new PipelineSlot({ valid: true, pc: 0x100c });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1008,
      sourceRegs: [5, 6],
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      destReg: 1,
      destValue: 10,
      usesAlu: true,
    });
    const memStage = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 2,
      usesAlu: true,
    });

    const result = unit.check(ifStage, idStage, exStage, memStage);

    expect(result.action).toBe(HazardAction.NONE);
  });
});

describe("history tracking", () => {
  /** Verify that the hazard unit records history of all checks. */

  it("should record each check in history", () => {
    /** Each call to check() adds one entry to history. */
    const unit = new HazardUnit();

    const empty = new PipelineSlot();
    unit.check(empty, empty, empty, empty);
    unit.check(empty, empty, empty, empty);
    unit.check(empty, empty, empty, empty);

    expect(unit.history).toHaveLength(3);
  });

  it("should return a copy of history (not the internal list)", () => {
    /** The history property returns a copy, not the internal list. */
    const unit = new HazardUnit();
    const empty = new PipelineSlot();
    unit.check(empty, empty, empty, empty);

    const history = unit.history;
    history.length = 0; // modifying the copy

    expect(unit.history).toHaveLength(1); // internal list unaffected
  });
});

describe("statistics", () => {
  /** Verify stallCount, flushCount, and forwardCount stats. */

  it("should sum stall cycles in stallCount", () => {
    /** stallCount sums all stallCycles across history. */
    const unit = new HazardUnit();

    // Scenario 1: load-use stall (1 cycle)
    const exLoad = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      memRead: true,
      usesAlu: false,
    });
    const idUse = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1],
      usesAlu: true,
    });
    const empty = new PipelineSlot();

    unit.check(empty, idUse, exLoad, empty);
    unit.check(empty, idUse, exLoad, empty);

    expect(unit.stallCount).toBe(2); // 1 + 1
  });

  it("should track mispredictions in flushCount", () => {
    /** flushCount counts the number of FLUSH actions. */
    const unit = new HazardUnit();

    const branchMispredict = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      isBranch: true,
      branchPredictedTaken: false,
      branchTaken: true,
    });
    const empty = new PipelineSlot();

    unit.check(empty, empty, branchMispredict, empty);
    unit.check(empty, empty, empty, empty); // no hazard
    unit.check(empty, empty, branchMispredict, empty);

    expect(unit.flushCount).toBe(2);
  });

  it("should track forwarding operations in forwardCount", () => {
    /** forwardCount counts FORWARD_FROM_EX and FORWARD_FROM_MEM. */
    const unit = new HazardUnit({ numAlus: 2 }); // avoid structural hazard on ALU

    // Forward from EX
    const exAlu = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 1,
      destValue: 42,
      usesAlu: true,
    });
    const idRead = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      sourceRegs: [1],
      usesAlu: true,
    });
    const empty = new PipelineSlot();

    unit.check(empty, idRead, exAlu, empty);

    // Forward from MEM
    const memAlu = new PipelineSlot({
      valid: true,
      pc: 0x1000,
      destReg: 2,
      destValue: 99,
      usesAlu: true,
    });
    const idRead2 = new PipelineSlot({
      valid: true,
      pc: 0x100c,
      sourceRegs: [2],
      usesAlu: true,
    });
    unit.check(empty, idRead2, empty, memAlu);

    expect(unit.forwardCount).toBe(2);
  });

  it("should track all stats together in a mixed scenario", () => {
    /** Run a mix of scenarios and verify all stats. */
    const unit = new HazardUnit({ numAlus: 2 }); // avoid structural hazard on ALU
    const empty = new PipelineSlot();

    // Cycle 1: forward from EX
    unit.check(
      empty,
      new PipelineSlot({
        valid: true,
        pc: 0x04,
        sourceRegs: [1],
        usesAlu: true,
      }),
      new PipelineSlot({
        valid: true,
        pc: 0x00,
        destReg: 1,
        destValue: 10,
        usesAlu: true,
      }),
      empty,
    );

    // Cycle 2: stall (load-use)
    unit.check(
      empty,
      new PipelineSlot({
        valid: true,
        pc: 0x0c,
        sourceRegs: [3],
        usesAlu: true,
      }),
      new PipelineSlot({
        valid: true,
        pc: 0x08,
        destReg: 3,
        memRead: true,
        usesAlu: false,
      }),
      empty,
    );

    // Cycle 3: flush (misprediction)
    unit.check(
      empty,
      empty,
      new PipelineSlot({
        valid: true,
        pc: 0x10,
        isBranch: true,
        branchPredictedTaken: true,
        branchTaken: false,
      }),
      empty,
    );

    // Cycle 4: no hazard
    unit.check(empty, empty, empty, empty);

    expect(unit.forwardCount).toBe(1);
    expect(unit.stallCount).toBe(1);
    expect(unit.flushCount).toBe(1);
    expect(unit.history).toHaveLength(4);
  });
});

describe("pickHighestPriority", () => {
  /** Unit tests for the pickHighestPriority helper function. */

  it("should pick FLUSH over everything", () => {
    /** FLUSH is the highest priority action. */
    const flush = new HazardResult({
      action: HazardAction.FLUSH,
      flushCount: 2,
    });
    const stall = new HazardResult({
      action: HazardAction.STALL,
      stallCycles: 1,
    });
    const forward = new HazardResult({
      action: HazardAction.FORWARD_FROM_EX,
    });
    const none = new HazardResult({ action: HazardAction.NONE });

    expect(pickHighestPriority(flush, stall, forward, none)).toBe(flush);
    expect(pickHighestPriority(none, stall, flush, forward)).toBe(flush);
  });

  it("should pick STALL over FORWARD and NONE", () => {
    /** STALL beats FORWARD and NONE. */
    const stall = new HazardResult({
      action: HazardAction.STALL,
      stallCycles: 1,
    });
    const forward = new HazardResult({
      action: HazardAction.FORWARD_FROM_EX,
    });
    const none = new HazardResult({ action: HazardAction.NONE });

    expect(pickHighestPriority(stall, forward, none)).toBe(stall);
  });

  it("should pick FORWARD_FROM_EX over FORWARD_FROM_MEM", () => {
    /** FORWARD_FROM_EX beats FORWARD_FROM_MEM. */
    const fwdEx = new HazardResult({
      action: HazardAction.FORWARD_FROM_EX,
    });
    const fwdMem = new HazardResult({
      action: HazardAction.FORWARD_FROM_MEM,
    });

    expect(pickHighestPriority(fwdMem, fwdEx)).toBe(fwdEx);
  });

  it("should return single result unchanged", () => {
    /** Single result is returned unchanged. */
    const none = new HazardResult({ action: HazardAction.NONE });
    const result = pickHighestPriority(none);
    expect(result).toBe(none);
  });

  it("should pick first result when priorities are equal (tie)", () => {
    /** When priorities are equal, first result wins. */
    const stall1 = new HazardResult({
      action: HazardAction.STALL,
      reason: "first stall",
    });
    const stall2 = new HazardResult({
      action: HazardAction.STALL,
      reason: "second stall",
    });

    const result = pickHighestPriority(stall1, stall2);
    expect(result).toBe(stall1);
  });
});

describe("structural and data combined", () => {
  /** Test interaction between structural and data hazards. */

  it("should detect structural stall with no data hazard", () => {
    /** Structural hazard (ALU conflict) with no data dependency. */
    const unit = new HazardUnit({ numAlus: 1 });

    const ifStage = new PipelineSlot({ valid: true, pc: 0x100c });
    const idStage = new PipelineSlot({
      valid: true,
      pc: 0x1008,
      sourceRegs: [5, 6], // different regs than EX dest
      usesAlu: true,
    });
    const exStage = new PipelineSlot({
      valid: true,
      pc: 0x1004,
      destReg: 1,
      destValue: 10,
      usesAlu: true,
    });
    const memStage = new PipelineSlot();

    const result = unit.check(ifStage, idStage, exStage, memStage);

    // Data detector sees forward_from_ex? No — different regs.
    // Structural detector sees ALU conflict → stall.
    expect(result.action).toBe(HazardAction.STALL);
  });
});
