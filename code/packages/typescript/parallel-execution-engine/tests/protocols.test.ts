/**
 * Tests for protocols -- ExecutionModel, EngineTrace, DivergenceInfo.
 */

import { describe, it, expect } from "vitest";
import {
  ExecutionModel,
  type DivergenceInfo,
  makeDivergenceInfo,
  type DataflowInfo,
  makeDataflowInfo,
  type EngineTrace,
  formatEngineTrace,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// ExecutionModel enum
// ---------------------------------------------------------------------------

describe("ExecutionModel", () => {
  it("all five models exist", () => {
    expect(ExecutionModel.SIMT).toBe("simt");
    expect(ExecutionModel.SIMD).toBe("simd");
    expect(ExecutionModel.SYSTOLIC).toBe("systolic");
    expect(ExecutionModel.SCHEDULED_MAC).toBe("scheduled_mac");
    expect(ExecutionModel.VLIW).toBe("vliw");
  });

  it("has exactly 5 members", () => {
    const values = Object.values(ExecutionModel);
    expect(values.length).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// DivergenceInfo
// ---------------------------------------------------------------------------

describe("DivergenceInfo", () => {
  it("creation with all fields", () => {
    const info: DivergenceInfo = {
      activeMaskBefore: [true, true, true, true],
      activeMaskAfter: [true, true, false, false],
      reconvergencePc: 10,
      divergenceDepth: 1,
    };
    expect(info.activeMaskBefore).toEqual([true, true, true, true]);
    expect(info.activeMaskAfter).toEqual([true, true, false, false]);
    expect(info.reconvergencePc).toBe(10);
    expect(info.divergenceDepth).toBe(1);
  });

  it("defaults via makeDivergenceInfo", () => {
    const info = makeDivergenceInfo({
      activeMaskBefore: [true],
      activeMaskAfter: [true],
    });
    expect(info.reconvergencePc).toBe(-1);
    expect(info.divergenceDepth).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// DataflowInfo
// ---------------------------------------------------------------------------

describe("DataflowInfo", () => {
  it("creation with PE states", () => {
    const info = makeDataflowInfo({
      peStates: [
        ["acc=1.0", "acc=2.0"],
        ["acc=3.0", "acc=4.0"],
      ],
      dataPositions: { input_0: [0, 1] },
    });
    expect(info.peStates[0][0]).toBe("acc=1.0");
    expect(info.dataPositions["input_0"]).toEqual([0, 1]);
  });

  it("defaults via makeDataflowInfo", () => {
    const info = makeDataflowInfo({ peStates: [["x"]] });
    expect(info.dataPositions).toEqual({});
  });
});

// ---------------------------------------------------------------------------
// EngineTrace
// ---------------------------------------------------------------------------

describe("EngineTrace", () => {
  function makeTrace(): EngineTrace {
    return {
      cycle: 3,
      engineName: "WarpEngine",
      executionModel: ExecutionModel.SIMT,
      description: "FADD R2, R0, R1 -- 3/4 threads active",
      unitTraces: {
        0: "R2 = 1.0 + 2.0 = 3.0",
        1: "R2 = 3.0 + 4.0 = 7.0",
        2: "(masked)",
        3: "R2 = 5.0 + 6.0 = 11.0",
      },
      activeMask: [true, true, false, true],
      activeCount: 3,
      totalCount: 4,
      utilization: 0.75,
    };
  }

  it("creation with all fields", () => {
    const trace = makeTrace();
    expect(trace.cycle).toBe(3);
    expect(trace.engineName).toBe("WarpEngine");
    expect(trace.executionModel).toBe(ExecutionModel.SIMT);
    expect(trace.activeCount).toBe(3);
    expect(trace.totalCount).toBe(4);
    expect(trace.utilization).toBe(0.75);
  });

  it("optional fields default to undefined", () => {
    const trace = makeTrace();
    expect(trace.divergenceInfo).toBeUndefined();
    expect(trace.dataflowInfo).toBeUndefined();
  });

  it("can include divergence info", () => {
    const div: DivergenceInfo = {
      activeMaskBefore: [true, true, true, true],
      activeMaskAfter: [true, true, false, false],
      reconvergencePc: 10,
      divergenceDepth: 1,
    };
    const trace: EngineTrace = {
      cycle: 1,
      engineName: "WarpEngine",
      executionModel: ExecutionModel.SIMT,
      description: "branch",
      unitTraces: {},
      activeMask: [true, true, false, false],
      activeCount: 2,
      totalCount: 4,
      utilization: 0.5,
      divergenceInfo: div,
    };
    expect(trace.divergenceInfo).not.toBeNull();
    expect(trace.divergenceInfo!.divergenceDepth).toBe(1);
  });

  it("can include dataflow info", () => {
    const df: DataflowInfo = {
      peStates: [["acc=0.0"]],
      dataPositions: {},
    };
    const trace: EngineTrace = {
      cycle: 1,
      engineName: "SystolicArray",
      executionModel: ExecutionModel.SYSTOLIC,
      description: "step",
      unitTraces: {},
      activeMask: [true],
      activeCount: 1,
      totalCount: 1,
      utilization: 1.0,
      dataflowInfo: df,
    };
    expect(trace.dataflowInfo).not.toBeNull();
  });

  it("formatEngineTrace produces readable output", () => {
    const trace = makeTrace();
    const text = formatEngineTrace(trace);
    expect(text).toContain("Cycle 3");
    expect(text).toContain("WarpEngine");
    expect(text).toContain("SIMT");
    expect(text).toContain("75.0%");
    expect(text).toContain("3/4 active");
  });

  it("formatEngineTrace includes divergence info", () => {
    const div: DivergenceInfo = {
      activeMaskBefore: [true, true, true, true],
      activeMaskAfter: [true, true, false, false],
      reconvergencePc: 10,
      divergenceDepth: 1,
    };
    const trace: EngineTrace = {
      cycle: 1,
      engineName: "Test",
      executionModel: ExecutionModel.SIMT,
      description: "test",
      unitTraces: {},
      activeMask: [true, true, false, false],
      activeCount: 2,
      totalCount: 4,
      utilization: 0.5,
      divergenceInfo: div,
    };
    const text = formatEngineTrace(trace);
    expect(text).toContain("Divergence");
    expect(text).toContain("depth=1");
  });
});
