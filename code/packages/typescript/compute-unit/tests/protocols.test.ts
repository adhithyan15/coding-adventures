/**
 * Tests for protocols.ts -- shared types and SharedMemory.
 */

import { describe, it, expect } from "vitest";
import { ExecutionModel } from "@coding-adventures/parallel-execution-engine";

import {
  Architecture,
  WarpState,
  SchedulingPolicy,
  makeWorkItem,
  makeComputeUnitTrace,
  formatComputeUnitTrace,
  SharedMemory,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Architecture enum
// ---------------------------------------------------------------------------

describe("Architecture", () => {
  it("has all five vendor architectures", () => {
    expect(Architecture.NVIDIA_SM).toBe("nvidia_sm");
    expect(Architecture.AMD_CU).toBe("amd_cu");
    expect(Architecture.GOOGLE_MXU).toBe("google_mxu");
    expect(Architecture.INTEL_XE_CORE).toBe("intel_xe_core");
    expect(Architecture.APPLE_ANE_CORE).toBe("apple_ane_core");
  });

  it("has unique values", () => {
    const values = Object.values(Architecture);
    expect(new Set(values).size).toBe(values.length);
  });

  it("has exactly 5 members", () => {
    const values = Object.values(Architecture);
    expect(values.length).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// WarpState enum
// ---------------------------------------------------------------------------

describe("WarpState", () => {
  it("has all six states", () => {
    expect(WarpState.READY).toBe("ready");
    expect(WarpState.RUNNING).toBe("running");
    expect(WarpState.STALLED_MEMORY).toBe("stalled_memory");
    expect(WarpState.STALLED_BARRIER).toBe("stalled_barrier");
    expect(WarpState.STALLED_DEPENDENCY).toBe("stalled_dependency");
    expect(WarpState.COMPLETED).toBe("completed");
  });

  it("has exactly 6 members", () => {
    const values = Object.values(WarpState);
    expect(values.length).toBe(6);
  });
});

// ---------------------------------------------------------------------------
// SchedulingPolicy enum
// ---------------------------------------------------------------------------

describe("SchedulingPolicy", () => {
  it("has all five policies", () => {
    expect(SchedulingPolicy.ROUND_ROBIN).toBe("round_robin");
    expect(SchedulingPolicy.GREEDY).toBe("greedy");
    expect(SchedulingPolicy.OLDEST_FIRST).toBe("oldest_first");
    expect(SchedulingPolicy.GTO).toBe("gto");
    expect(SchedulingPolicy.LRR).toBe("lrr");
  });

  it("has exactly 5 members", () => {
    const values = Object.values(SchedulingPolicy);
    expect(values.length).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// WorkItem
// ---------------------------------------------------------------------------

describe("WorkItem", () => {
  it("has correct defaults via makeWorkItem", () => {
    const wi = makeWorkItem({ workId: 0 });
    expect(wi.workId).toBe(0);
    expect(wi.program).toBeNull();
    expect(wi.threadCount).toBe(32);
    expect(wi.perThreadData).toEqual({});
    expect(wi.inputData).toBeNull();
    expect(wi.weightData).toBeNull();
    expect(wi.schedule).toBeNull();
    expect(wi.sharedMemBytes).toBe(0);
    expect(wi.registersPerThread).toBe(32);
  });

  it("accepts custom values", () => {
    const wi = makeWorkItem({
      workId: 42,
      threadCount: 128,
      sharedMemBytes: 4096,
      registersPerThread: 64,
    });
    expect(wi.workId).toBe(42);
    expect(wi.threadCount).toBe(128);
    expect(wi.sharedMemBytes).toBe(4096);
    expect(wi.registersPerThread).toBe(64);
  });

  it("supports dataflow work items", () => {
    const wi = makeWorkItem({
      workId: 1,
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    });
    expect(wi.inputData).not.toBeNull();
    expect(wi.inputData!.length).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// ComputeUnitTrace
// ---------------------------------------------------------------------------

describe("ComputeUnitTrace", () => {
  function makeTrace(cycle: number = 1) {
    return makeComputeUnitTrace({
      cycle,
      unitName: "SM",
      architecture: Architecture.NVIDIA_SM,
      schedulerAction: "issued warp 3",
      activeWarps: 48,
      totalWarps: 64,
      engineTraces: {},
      sharedMemoryUsed: 49152,
      sharedMemoryTotal: 98304,
      registerFileUsed: 32768,
      registerFileTotal: 65536,
      occupancy: 0.75,
    });
  }

  it("creates with correct fields", () => {
    const trace = makeTrace();
    expect(trace.cycle).toBe(1);
    expect(trace.unitName).toBe("SM");
    expect(trace.architecture).toBe(Architecture.NVIDIA_SM);
    expect(trace.occupancy).toBe(0.75);
  });

  it("defaults cache stats to 0", () => {
    const trace = makeTrace();
    expect(trace.l1Hits).toBe(0);
    expect(trace.l1Misses).toBe(0);
  });

  it("formats output correctly", () => {
    const trace = makeTrace(5);
    const formatted = formatComputeUnitTrace(trace);
    expect(formatted).toContain("[Cycle 5]");
    expect(formatted).toContain("SM");
    expect(formatted).toContain("nvidia_sm");
    expect(formatted).toContain("75.0%");
    expect(formatted).toContain("issued warp 3");
    expect(formatted).toContain("Shared memory");
    expect(formatted).toContain("Registers");
  });

  it("formats with engine traces", () => {
    const engineTrace = {
      cycle: 5,
      engineName: "WarpEngine",
      executionModel: ExecutionModel.SIMT,
      description: "FMUL R2, R0, R1 -- 32/32 active",
      unitTraces: { 0: "ok" },
      activeMask: [true],
      activeCount: 32,
      totalCount: 32,
      utilization: 1.0,
    };
    const trace = makeComputeUnitTrace({
      cycle: 5,
      unitName: "SM",
      architecture: Architecture.NVIDIA_SM,
      schedulerAction: "issued warp 0",
      activeWarps: 1,
      totalWarps: 48,
      engineTraces: { 0: engineTrace },
      sharedMemoryUsed: 0,
      sharedMemoryTotal: 98304,
      registerFileUsed: 1024,
      registerFileTotal: 65536,
      occupancy: 1 / 48,
    });
    const formatted = formatComputeUnitTrace(trace);
    expect(formatted).toContain("Engine 0");
    expect(formatted).toContain("FMUL");
  });
});

// ---------------------------------------------------------------------------
// SharedMemory
// ---------------------------------------------------------------------------

describe("SharedMemory", () => {
  it("creates with correct parameters", () => {
    const smem = new SharedMemory(1024);
    expect(smem.size).toBe(1024);
    expect(smem.numBanks).toBe(32);
    expect(smem.bankWidth).toBe(4);
  });

  it("writes and reads floats", () => {
    const smem = new SharedMemory(1024);
    smem.write(0, 3.14, 0);
    const val = smem.read(0, 0);
    expect(Math.abs(val - 3.14)).toBeLessThan(0.01);
  });

  it("writes to multiple addresses", () => {
    const smem = new SharedMemory(1024);
    smem.write(0, 1.0, 0);
    smem.write(4, 2.0, 1);
    smem.write(8, 3.0, 2);
    expect(Math.abs(smem.read(0, 0) - 1.0)).toBeLessThan(0.001);
    expect(Math.abs(smem.read(4, 1) - 2.0)).toBeLessThan(0.001);
    expect(Math.abs(smem.read(8, 2) - 3.0)).toBeLessThan(0.001);
  });

  it("throws on read out of range", () => {
    const smem = new SharedMemory(64);
    expect(() => smem.read(64, 0)).toThrow();
  });

  it("throws on write out of range", () => {
    const smem = new SharedMemory(64);
    expect(() => smem.write(64, 1.0, 0)).toThrow();
  });

  it("throws on negative address", () => {
    const smem = new SharedMemory(64);
    expect(() => smem.read(-1, 0)).toThrow();
  });

  it("detects no bank conflicts", () => {
    const smem = new SharedMemory(1024, 32, 4);
    // Addresses 0, 4, 8, 12 -> banks 0, 1, 2, 3
    const conflicts = smem.checkBankConflicts([0, 4, 8, 12]);
    expect(conflicts).toEqual([]);
  });

  it("detects 2-way bank conflict", () => {
    const smem = new SharedMemory(1024, 32, 4);
    // Address 0 -> bank 0, address 128 -> bank 0 (32*4=128 wraps)
    const conflicts = smem.checkBankConflicts([0, 4, 128, 12]);
    expect(conflicts.length).toBe(1);
    expect(conflicts[0].sort()).toEqual([0, 2]);
  });

  it("detects 3-way bank conflict", () => {
    const smem = new SharedMemory(1024, 32, 4);
    // Addresses 0, 128, 256 all map to bank 0
    const conflicts = smem.checkBankConflicts([0, 128, 256]);
    expect(conflicts.length).toBe(1);
    expect(conflicts[0].sort()).toEqual([0, 1, 2]);
  });

  it("detects multiple conflict groups", () => {
    const smem = new SharedMemory(1024, 32, 4);
    // Bank 0: addr 0 and 128 (threads 0, 2)
    // Bank 1: addr 4 and 132 (threads 1, 3)
    const conflicts = smem.checkBankConflicts([0, 4, 128, 132]);
    expect(conflicts.length).toBe(2);
  });

  it("counts accesses", () => {
    const smem = new SharedMemory(1024);
    smem.write(0, 1.0, 0);
    smem.read(0, 0);
    smem.read(0, 1);
    expect(smem.totalAccesses).toBe(3);
  });

  it("counts conflicts", () => {
    const smem = new SharedMemory(1024, 32, 4);
    smem.checkBankConflicts([0, 128]);
    expect(smem.totalConflicts).toBe(1);
  });

  it("resets state", () => {
    const smem = new SharedMemory(1024);
    smem.write(0, 42.0, 0);
    smem.checkBankConflicts([0, 128]);
    smem.reset();
    expect(smem.totalAccesses).toBe(0);
    expect(smem.totalConflicts).toBe(0);
    const val = smem.read(0, 0);
    expect(val).toBe(0.0);
  });

  it("supports custom bank config", () => {
    const smem = new SharedMemory(256, 8, 8);
    expect(smem.numBanks).toBe(8);
    expect(smem.bankWidth).toBe(8);
    // Bank = (addr // 8) % 8
    // addr 0 -> bank 0, addr 64 -> bank 0 (64//8=8, 8%8=0)
    const conflicts = smem.checkBankConflicts([0, 64]);
    expect(conflicts.length).toBe(1);
  });
});
