/**
 * Tests for AMDComputeUnit -- AMD CU (GCN/RDNA) simulator.
 */

import { describe, it, expect } from "vitest";
import { limm, fmul, fadd, load, halt } from "@coding-adventures/gpu-core";

import {
  Architecture,
  SchedulingPolicy,
  makeWorkItem,
  ResourceError,
} from "../src/index.js";
import {
  AMDComputeUnit,
  makeAMDCUConfig,
} from "../src/amd-compute-unit.js";

// ---------------------------------------------------------------------------
// AMDCUConfig tests
// ---------------------------------------------------------------------------

describe("AMDCUConfig", () => {
  it("has correct defaults", () => {
    const config = makeAMDCUConfig();
    expect(config.numSimdUnits).toBe(4);
    expect(config.waveWidth).toBe(64);
    expect(config.maxWavefronts).toBe(40);
    expect(config.maxWorkGroups).toBe(16);
    expect(config.schedulingPolicy).toBe(SchedulingPolicy.LRR);
    expect(config.vgprPerSimd).toBe(256);
    expect(config.sgprCount).toBe(104);
    expect(config.ldsSize).toBe(65536);
    expect(config.memoryLatencyCycles).toBe(200);
  });

  it("allows customization", () => {
    const config = makeAMDCUConfig({
      numSimdUnits: 2,
      waveWidth: 32,
      maxWavefronts: 16,
    });
    expect(config.numSimdUnits).toBe(2);
    expect(config.waveWidth).toBe(32);
    expect(config.maxWavefronts).toBe(16);
  });
});

// ---------------------------------------------------------------------------
// AMDComputeUnit tests
// ---------------------------------------------------------------------------

describe("AMDComputeUnit", () => {
  const simpleProgram = () => [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()];

  it("creates correctly", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 8, waveWidth: 4 }),
    );
    expect(cu.name).toBe("CU");
    expect(cu.architecture).toBe(Architecture.AMD_CU);
    expect(cu.idle).toBe(true);
    expect(cu.occupancy).toBe(0.0);
  });

  it("dispatch creates wavefronts", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 16, waveWidth: 4, numSimdUnits: 2 }),
    );
    cu.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8, // 2 wavefronts of 4 lanes
    }));
    expect(cu.idle).toBe(false);
    expect(cu.wavefrontSlots.length).toBe(2);
  });

  it("decomposes wavefronts correctly", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 16, waveWidth: 64 }),
    );
    cu.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 128,
    }));
    expect(cu.wavefrontSlots.length).toBe(2);
  });

  it("runs a simple program", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 8, waveWidth: 4, numSimdUnits: 1 }),
    );
    cu.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 4,
    }));
    const traces = cu.run();
    expect(traces.length).toBeGreaterThan(0);
    expect(cu.idle).toBe(true);
  });

  it("computes occupancy", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 8, waveWidth: 4 }),
    );
    cu.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8, // 2 wavefronts
    }));
    expect(cu.occupancy).toBeCloseTo(2 / 8);
  });

  it("throws ResourceError on wavefront exhaustion", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 2, waveWidth: 4 }),
    );
    cu.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8, // 2 wavefronts -- fills capacity
    }));
    expect(() =>
      cu.dispatch(makeWorkItem({
        workId: 1,
        program: simpleProgram(),
        threadCount: 4,
      })),
    ).toThrow(ResourceError);
  });

  it("throws ResourceError on LDS exhaustion", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 16, ldsSize: 1024 }),
    );
    expect(() =>
      cu.dispatch(makeWorkItem({
        workId: 0,
        program: simpleProgram(),
        threadCount: 32,
        sharedMemBytes: 2048,
      })),
    ).toThrow(ResourceError);
  });

  it("traces have correct architecture", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 4, waveWidth: 4, numSimdUnits: 1 }),
    );
    cu.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 4,
    }));
    const traces = cu.run();
    for (const trace of traces) {
      expect(trace.architecture).toBe(Architecture.AMD_CU);
      expect(trace.unitName).toBe("CU");
    }
  });

  it("supports per-lane data", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 4, waveWidth: 4, numSimdUnits: 1 }),
    );
    cu.dispatch(makeWorkItem({
      workId: 0,
      program: [fadd(2, 0, 1), halt()],
      threadCount: 4,
      perThreadData: {
        0: { 0: 1.0, 1: 10.0 },
        1: { 0: 2.0, 1: 20.0 },
        2: { 0: 3.0, 1: 30.0 },
        3: { 0: 4.0, 1: 40.0 },
      },
    }));
    cu.run();
    expect(cu.idle).toBe(true);
  });

  it("dispatches multiple work groups", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 16, waveWidth: 4, numSimdUnits: 2 }),
    );
    cu.dispatch(makeWorkItem({ workId: 0, program: simpleProgram(), threadCount: 8 }));
    cu.dispatch(makeWorkItem({ workId: 1, program: simpleProgram(), threadCount: 8 }));
    expect(cu.wavefrontSlots.length).toBe(4);
    cu.run();
    expect(cu.idle).toBe(true);
  });

  it("LDS is accessible", () => {
    const cu = new AMDComputeUnit(makeAMDCUConfig());
    const lds = cu.lds;
    lds.write(0, 42.0, 0);
    expect(Math.abs(lds.read(0, 0) - 42.0)).toBeLessThan(0.01);
  });

  it("resets correctly", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 8, waveWidth: 4 }),
    );
    cu.dispatch(makeWorkItem({ workId: 0, program: simpleProgram(), threadCount: 4 }));
    cu.run();
    cu.reset();
    expect(cu.idle).toBe(true);
    expect(cu.wavefrontSlots.length).toBe(0);
    expect(cu.occupancy).toBe(0.0);
  });

  it("toString includes key info", () => {
    const cu = new AMDComputeUnit(makeAMDCUConfig());
    const r = cu.toString();
    expect(r).toContain("AMDComputeUnit");
  });

  it("simulates memory stalls", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({
        maxWavefronts: 8,
        waveWidth: 4,
        numSimdUnits: 1,
        memoryLatencyCycles: 3,
      }),
    );
    const prog = [limm(0, 0.0), load(1, 0), halt()];
    cu.dispatch(makeWorkItem({ workId: 0, program: prog, threadCount: 4 }));
    cu.run(50);
    expect(cu.idle).toBe(true);
  });
});
