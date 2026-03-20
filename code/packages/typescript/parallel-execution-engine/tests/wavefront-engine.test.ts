/**
 * Tests for WavefrontEngine -- SIMD parallel execution (AMD GCN/RDNA style).
 */

import { describe, it, expect } from "vitest";
import { limm, fmul, halt } from "@coding-adventures/gpu-core";
import {
  WavefrontEngine,
  makeWavefrontConfig,
  VectorRegisterFile,
  ScalarRegisterFile,
  ExecutionModel,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// VectorRegisterFile
// ---------------------------------------------------------------------------

describe("VectorRegisterFile", () => {
  it("creation", () => {
    const vrf = new VectorRegisterFile(8, 4);
    expect(vrf.numVgprs).toBe(8);
    expect(vrf.waveWidth).toBe(4);
  });

  it("read and write", () => {
    const vrf = new VectorRegisterFile(8, 4);
    vrf.write(0, 2, 3.14);
    expect(Math.abs(vrf.read(0, 2) - 3.14)).toBeLessThan(0.01);
  });

  it("lanes are independent", () => {
    const vrf = new VectorRegisterFile(4, 4);
    vrf.write(0, 0, 1.0);
    vrf.write(0, 1, 2.0);
    vrf.write(0, 2, 3.0);
    vrf.write(0, 3, 4.0);
    expect(vrf.read(0, 0)).toBe(1.0);
    expect(vrf.read(0, 1)).toBe(2.0);
    expect(vrf.read(0, 2)).toBe(3.0);
    expect(vrf.read(0, 3)).toBe(4.0);
  });

  it("readAllLanes", () => {
    const vrf = new VectorRegisterFile(4, 4);
    for (let lane = 0; lane < 4; lane++) {
      vrf.write(0, lane, lane + 1);
    }
    expect(vrf.readAllLanes(0)).toEqual([1.0, 2.0, 3.0, 4.0]);
  });
});

// ---------------------------------------------------------------------------
// ScalarRegisterFile
// ---------------------------------------------------------------------------

describe("ScalarRegisterFile", () => {
  it("creation", () => {
    const srf = new ScalarRegisterFile(8);
    expect(srf.numSgprs).toBe(8);
  });

  it("read and write", () => {
    const srf = new ScalarRegisterFile(8);
    srf.write(3, 42.0);
    expect(srf.read(3)).toBe(42.0);
  });

  it("initial zero", () => {
    const srf = new ScalarRegisterFile(8);
    expect(srf.read(0)).toBe(0.0);
  });
});

// ---------------------------------------------------------------------------
// WavefrontConfig
// ---------------------------------------------------------------------------

describe("WavefrontConfig", () => {
  it("has sensible defaults", () => {
    const config = makeWavefrontConfig();
    expect(config.waveWidth).toBe(32);
    expect(config.numVgprs).toBe(256);
    expect(config.numSgprs).toBe(104);
    expect(config.ldsSize).toBe(65536);
  });
});

// ---------------------------------------------------------------------------
// WavefrontEngine -- basic properties
// ---------------------------------------------------------------------------

describe("WavefrontEngine properties", () => {
  it("has correct name", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    expect(engine.name).toBe("WavefrontEngine");
  });

  it("has correct width", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 8 }));
    expect(engine.width).toBe(8);
  });

  it("reports SIMD execution model", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    expect(engine.executionModel).toBe(ExecutionModel.SIMD);
  });

  it("starts not halted", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    expect(engine.halted).toBe(false);
  });

  it("exec mask all true initially", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    expect(engine.execMask).toEqual([true, true, true, true]);
  });

  it("exposes config", () => {
    const config = makeWavefrontConfig({ waveWidth: 4 });
    const engine = new WavefrontEngine(config);
    expect(engine.config).toBe(config);
  });

  it("exposes vrf and srf", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    expect(engine.vrf).toBeDefined();
    expect(engine.srf).toBeDefined();
  });

  it("has readable toString", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    expect(engine.toString()).toContain("WavefrontEngine");
  });
});

// ---------------------------------------------------------------------------
// WavefrontEngine -- execution
// ---------------------------------------------------------------------------

describe("WavefrontEngine execution", () => {
  it("all lanes execute simple program", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    engine.loadProgram([limm(0, 42.0), halt()]);
    const traces = engine.run();
    expect(traces.length).toBeGreaterThanOrEqual(2);
    expect(engine.halted).toBe(true);
  });

  it("per-lane data via vector registers", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    engine.loadProgram([limm(1, 2.0), fmul(2, 0, 1), halt()]);

    for (let lane = 0; lane < 4; lane++) {
      engine.setLaneRegister(lane, 0, lane + 1);
    }

    engine.run();

    // Check VRF for results
    for (let lane = 0; lane < 4; lane++) {
      expect(engine.vrf.read(2, lane)).toBe((lane + 1) * 2.0);
    }
  });

  it("rejects out-of-range lane register", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    expect(() => engine.setLaneRegister(4, 0, 1.0)).toThrow();
    expect(() => engine.setLaneRegister(-1, 0, 1.0)).toThrow();
  });

  it("rejects out-of-range scalar register", () => {
    const engine = new WavefrontEngine(
      makeWavefrontConfig({ waveWidth: 4, numSgprs: 8 }),
    );
    expect(() => engine.setScalarRegister(8, 1.0)).toThrow();
    expect(() => engine.setScalarRegister(-1, 1.0)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// WavefrontEngine -- EXEC mask
// ---------------------------------------------------------------------------

describe("WavefrontEngine EXEC mask", () => {
  it("set exec mask", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    engine.setExecMask([true, false, true, false]);
    expect(engine.execMask).toEqual([true, false, true, false]);
  });

  it("rejects wrong-length mask", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    expect(() => engine.setExecMask([true, false])).toThrow();
  });

  it("masked lanes don't update VRF", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    engine.loadProgram([limm(0, 99.0), halt()]);
    engine.setExecMask([true, true, false, false]);
    engine.run();

    // Lanes 0,1 should have R0=99 in VRF
    expect(engine.vrf.read(0, 0)).toBe(99.0);
    expect(engine.vrf.read(0, 1)).toBe(99.0);
    // Masked lanes: VRF should still be 0.0
    expect(engine.vrf.read(0, 2)).toBe(0.0);
    expect(engine.vrf.read(0, 3)).toBe(0.0);
  });

  it("utilization reflects exec mask", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    engine.loadProgram([limm(0, 1.0), halt()]);
    engine.setExecMask([true, true, false, false]);

    const trace = engine.step({ cycle: 1 });
    expect(trace.activeCount).toBe(2);
    expect(Math.abs(trace.utilization - 0.5)).toBeLessThan(0.01);
  });
});

// ---------------------------------------------------------------------------
// WavefrontEngine -- reset
// ---------------------------------------------------------------------------

describe("WavefrontEngine reset", () => {
  it("restores initial state", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    engine.loadProgram([limm(0, 42.0), halt()]);
    engine.run();
    expect(engine.halted).toBe(true);

    engine.reset();
    expect(engine.halted).toBe(false);
    expect(engine.execMask).toEqual([true, true, true, true]);

    engine.run();
    expect(engine.halted).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// WavefrontEngine -- traces
// ---------------------------------------------------------------------------

describe("WavefrontEngine traces", () => {
  it("trace includes divergence info", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    engine.loadProgram([limm(0, 1.0), halt()]);

    const trace = engine.step({ cycle: 1 });
    expect(trace.divergenceInfo).toBeDefined();
    expect(trace.divergenceInfo).not.toBeNull();
  });

  it("halted step returns trace", () => {
    const engine = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    engine.loadProgram([halt()]);
    engine.run();

    const trace = engine.step({ cycle: 99 });
    expect(trace.activeCount).toBe(0);
  });
});
