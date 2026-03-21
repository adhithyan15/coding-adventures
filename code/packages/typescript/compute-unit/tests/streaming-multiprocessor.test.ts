/**
 * Tests for StreamingMultiprocessor -- NVIDIA SM simulator.
 */

import { describe, it, expect } from "vitest";
import { limm, fmul, fadd, load, halt } from "@coding-adventures/gpu-core";
import { WarpEngine, makeWarpConfig } from "@coding-adventures/parallel-execution-engine";

import {
  Architecture,
  SchedulingPolicy,
  WarpState,
  makeWorkItem,
  StreamingMultiprocessor,
  makeSMConfig,
  WarpScheduler,
  ResourceError,
  type WarpSlot,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// SMConfig tests
// ---------------------------------------------------------------------------

describe("SMConfig", () => {
  it("has correct defaults", () => {
    const config = makeSMConfig();
    expect(config.numSchedulers).toBe(4);
    expect(config.warpWidth).toBe(32);
    expect(config.maxWarps).toBe(48);
    expect(config.maxThreads).toBe(1536);
    expect(config.maxBlocks).toBe(16);
    expect(config.registerFileSize).toBe(65536);
    expect(config.sharedMemorySize).toBe(98304);
    expect(config.memoryLatencyCycles).toBe(200);
    expect(config.schedulingPolicy).toBe(SchedulingPolicy.GTO);
  });

  it("allows customization", () => {
    const config = makeSMConfig({
      numSchedulers: 2,
      maxWarps: 16,
      schedulingPolicy: SchedulingPolicy.ROUND_ROBIN,
    });
    expect(config.numSchedulers).toBe(2);
    expect(config.maxWarps).toBe(16);
    expect(config.schedulingPolicy).toBe(SchedulingPolicy.ROUND_ROBIN);
  });
});

// ---------------------------------------------------------------------------
// WarpScheduler tests
// ---------------------------------------------------------------------------

describe("WarpScheduler", () => {
  function makeSlot(
    warpId: number,
    state: WarpState = WarpState.READY,
    age: number = 0,
  ): WarpSlot {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    return {
      warpId,
      workId: 0,
      state,
      engine,
      stallCounter: 0,
      age,
      registersUsed: 0,
    };
  }

  it("round-robin picks in order", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN);
    sched.addWarp(makeSlot(0));
    sched.addWarp(makeSlot(1));
    sched.addWarp(makeSlot(2));
    const picked = sched.pickWarp();
    expect(picked).not.toBeNull();
    expect(picked!.warpId).toBe(0);
  });

  it("round-robin wraps around", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN);
    sched.addWarp(makeSlot(0));
    sched.addWarp(makeSlot(1));
    const p1 = sched.pickWarp();
    expect(p1!.warpId).toBe(0);
    const p2 = sched.pickWarp();
    expect(p2!.warpId).toBe(1);
    const p3 = sched.pickWarp();
    expect(p3!.warpId).toBe(0);
  });

  it("round-robin skips non-ready warps", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN);
    sched.addWarp(makeSlot(0, WarpState.STALLED_MEMORY));
    sched.addWarp(makeSlot(1, WarpState.READY));
    const picked = sched.pickWarp();
    expect(picked).not.toBeNull();
    expect(picked!.warpId).toBe(1);
  });

  it("GTO stays with same warp", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.GTO);
    sched.addWarp(makeSlot(0));
    sched.addWarp(makeSlot(1));
    const p1 = sched.pickWarp();
    expect(p1).not.toBeNull();
    sched.markIssued(p1!.warpId);
    const p2 = sched.pickWarp();
    expect(p2).not.toBeNull();
    expect(p2!.warpId).toBe(p1!.warpId);
  });

  it("GTO switches when stalled", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.GTO);
    const s0 = makeSlot(0, WarpState.READY, 5);
    const s1 = makeSlot(1, WarpState.READY, 10);
    sched.addWarp(s0);
    sched.addWarp(s1);
    sched.markIssued(0);
    s0.state = WarpState.STALLED_MEMORY;
    const picked = sched.pickWarp();
    expect(picked).not.toBeNull();
    expect(picked!.warpId).toBe(1);
  });

  it("oldest-first picks the oldest", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.OLDEST_FIRST);
    sched.addWarp(makeSlot(0, WarpState.READY, 5));
    sched.addWarp(makeSlot(1, WarpState.READY, 10));
    sched.addWarp(makeSlot(2, WarpState.READY, 3));
    const picked = sched.pickWarp();
    expect(picked).not.toBeNull();
    expect(picked!.warpId).toBe(1); // age=10 is oldest
  });

  it("returns null when no ready warps", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN);
    sched.addWarp(makeSlot(0, WarpState.STALLED_MEMORY));
    sched.addWarp(makeSlot(1, WarpState.COMPLETED));
    expect(sched.pickWarp()).toBeNull();
  });

  it("tick stalls decrements counter", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN);
    const slot = makeSlot(0, WarpState.STALLED_MEMORY);
    slot.stallCounter = 3;
    sched.addWarp(slot);
    sched.tickStalls();
    expect(slot.stallCounter).toBe(2);
    expect(slot.state).toBe(WarpState.STALLED_MEMORY);
    sched.tickStalls();
    expect(slot.stallCounter).toBe(1);
    sched.tickStalls();
    expect(slot.stallCounter).toBe(0);
    expect(slot.state).toBe(WarpState.READY);
  });

  it("tick stalls increments age", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN);
    const slot = makeSlot(0);
    slot.age = 0;
    sched.addWarp(slot);
    sched.tickStalls();
    expect(slot.age).toBe(1);
  });

  it("reset clears warps", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN);
    sched.addWarp(makeSlot(0));
    sched.reset();
    expect(sched.warps.length).toBe(0);
  });

  it("LRR scheduling works", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.LRR);
    sched.addWarp(makeSlot(0, WarpState.STALLED_MEMORY));
    sched.addWarp(makeSlot(1));
    sched.addWarp(makeSlot(2));
    const picked = sched.pickWarp();
    expect(picked).not.toBeNull();
    expect([1, 2]).toContain(picked!.warpId);
  });

  it("greedy scheduling works", () => {
    const sched = new WarpScheduler(0, SchedulingPolicy.GREEDY);
    sched.addWarp(makeSlot(0, WarpState.READY, 2));
    sched.addWarp(makeSlot(1, WarpState.READY, 5));
    const picked = sched.pickWarp();
    expect(picked).not.toBeNull();
    expect(picked!.warpId).toBe(1); // older
  });
});

// ---------------------------------------------------------------------------
// StreamingMultiprocessor tests
// ---------------------------------------------------------------------------

describe("StreamingMultiprocessor", () => {
  const simpleProgram = () => [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()];

  it("creates correctly", () => {
    const sm = new StreamingMultiprocessor(makeSMConfig({ maxWarps: 8 }));
    expect(sm.name).toBe("SM");
    expect(sm.architecture).toBe(Architecture.NVIDIA_SM);
    expect(sm.idle).toBe(true);
    expect(sm.occupancy).toBe(0.0);
  });

  it("dispatch creates warps", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 16, warpWidth: 4 }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8, // 2 warps of 4 threads
    }));
    expect(sm.idle).toBe(false);
    expect(sm.warpSlots.length).toBe(2);
  });

  it("decomposes thread block correctly", () => {
    const sm = new StreamingMultiprocessor(makeSMConfig({ maxWarps: 16 }));
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 128,
    }));
    expect(sm.warpSlots.length).toBe(4); // 128/32 = 4
  });

  it("handles partial warps", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 16, warpWidth: 32 }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 40,
    }));
    expect(sm.warpSlots.length).toBe(2);
  });

  it("runs a simple program", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 8, warpWidth: 4, numSchedulers: 1 }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 4,
    }));
    const traces = sm.run();
    expect(traces.length).toBeGreaterThan(0);
    expect(sm.idle).toBe(true);
  });

  it("produces correct traces", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 8, warpWidth: 4, numSchedulers: 1 }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 4,
    }));
    const traces = sm.run();
    for (const trace of traces) {
      expect(trace.unitName).toBe("SM");
      expect(trace.architecture).toBe(Architecture.NVIDIA_SM);
      expect(trace.occupancy).toBeGreaterThanOrEqual(0);
      expect(trace.occupancy).toBeLessThanOrEqual(1);
    }
  });

  it("computes occupancy correctly", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 8, warpWidth: 4 }),
    );
    // 8 threads = 2 warps. occupancy = 2/8 = 0.25
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8,
    }));
    expect(sm.occupancy).toBeCloseTo(0.25);
  });

  it("computes static occupancy: register limited", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({
        maxWarps: 48,
        registerFileSize: 65536,
        sharedMemorySize: 98304,
      }),
    );
    // 64 regs/thread * 32 threads = 2048 regs/warp
    // 65536 / 2048 = 32 warps max by registers
    const occ = sm.computeOccupancy(64, 0, 256);
    // 32/48 = 0.667
    expect(occ).toBeCloseTo(32 / 48, 1);
  });

  it("computes static occupancy: smem limited", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({
        maxWarps: 48,
        registerFileSize: 65536,
        sharedMemorySize: 98304,
        warpWidth: 32,
      }),
    );
    // 49152 bytes/block. 98304/49152 = 2 blocks.
    // 256 threads/block = 8 warps. 2*8 = 16 warps.
    const occ = sm.computeOccupancy(16, 49152, 256);
    expect(occ).toBeCloseTo(16 / 48, 1);
  });

  it("computes static occupancy: hardware limited", () => {
    const sm = new StreamingMultiprocessor(makeSMConfig({ maxWarps: 8 }));
    const occ = sm.computeOccupancy(4, 0, 32);
    expect(occ).toBeCloseTo(1.0);
  });

  it("throws ResourceError on warp slot exhaustion", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 2, warpWidth: 4 }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8,
    }));
    expect(() =>
      sm.dispatch(makeWorkItem({
        workId: 1,
        program: simpleProgram(),
        threadCount: 4,
      })),
    ).toThrow(ResourceError);
  });

  it("throws ResourceError on register exhaustion", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 100, warpWidth: 4, registerFileSize: 100 }),
    );
    expect(() =>
      sm.dispatch(makeWorkItem({
        workId: 0,
        program: simpleProgram(),
        threadCount: 4,
        registersPerThread: 32, // 32 * 4 = 128 > 100
      })),
    ).toThrow(ResourceError);
  });

  it("throws ResourceError on shared memory exhaustion", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 100, sharedMemorySize: 1024 }),
    );
    expect(() =>
      sm.dispatch(makeWorkItem({
        workId: 0,
        program: simpleProgram(),
        threadCount: 32,
        sharedMemBytes: 2048,
      })),
    ).toThrow(ResourceError);
  });

  it("simulates memory stalls", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({
        maxWarps: 8,
        warpWidth: 4,
        numSchedulers: 1,
        memoryLatencyCycles: 5,
      }),
    );
    const prog = [limm(0, 0.0), load(1, 0), halt()];
    sm.dispatch(makeWorkItem({ workId: 0, program: prog, threadCount: 4 }));
    sm.run(50);
    expect(sm.idle).toBe(true);
  });

  it("supports per-thread data", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 8, warpWidth: 4, numSchedulers: 1 }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: [fadd(2, 0, 1), halt()],
      threadCount: 4,
      perThreadData: {
        0: { 0: 1.0, 1: 2.0 },
        1: { 0: 3.0, 1: 4.0 },
        2: { 0: 5.0, 1: 6.0 },
        3: { 0: 7.0, 1: 8.0 },
      },
    }));
    sm.run();
    expect(sm.idle).toBe(true);
  });

  it("dispatches multiple blocks", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 16, warpWidth: 4, numSchedulers: 2 }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8,
    }));
    sm.dispatch(makeWorkItem({
      workId: 1,
      program: simpleProgram(),
      threadCount: 8,
    }));
    expect(sm.warpSlots.length).toBe(4); // 2 warps per block * 2 blocks
    sm.run();
    expect(sm.idle).toBe(true);
  });

  it("resets correctly", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 8, warpWidth: 4 }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 4,
    }));
    sm.run();
    sm.reset();
    expect(sm.idle).toBe(true);
    expect(sm.warpSlots.length).toBe(0);
    expect(sm.occupancy).toBe(0.0);
  });

  it("toString includes key info", () => {
    const sm = new StreamingMultiprocessor(makeSMConfig({ maxWarps: 8 }));
    const r = sm.toString();
    expect(r).toContain("StreamingMultiprocessor");
    expect(r).toContain("policy=");
  });

  it("GTO scheduling integration", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({
        maxWarps: 8,
        warpWidth: 4,
        numSchedulers: 1,
        schedulingPolicy: SchedulingPolicy.GTO,
      }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 4,
    }));
    const traces = sm.run();
    expect(traces.length).toBeGreaterThan(0);
    expect(sm.idle).toBe(true);
  });

  it("round-robin scheduling integration", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({
        maxWarps: 8,
        warpWidth: 4,
        numSchedulers: 1,
        schedulingPolicy: SchedulingPolicy.ROUND_ROBIN,
      }),
    );
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8,
    }));
    sm.run();
    expect(sm.idle).toBe(true);
  });

  it("shared memory is accessible", () => {
    const sm = new StreamingMultiprocessor(makeSMConfig());
    const smem = sm.sharedMemory;
    smem.write(0, 42.0, 0);
    expect(Math.abs(smem.read(0, 0) - 42.0)).toBeLessThan(0.01);
  });
});
