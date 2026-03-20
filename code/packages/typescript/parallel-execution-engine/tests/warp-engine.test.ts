/**
 * Tests for WarpEngine -- SIMT parallel execution (NVIDIA/ARM Mali style).
 */

import { describe, it, expect } from "vitest";
import { limm, fmul, halt, blt, nop } from "@coding-adventures/gpu-core";
import {
  WarpEngine,
  makeWarpConfig,
  ExecutionModel,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

describe("WarpConfig", () => {
  it("has sensible defaults", () => {
    const config = makeWarpConfig();
    expect(config.warpWidth).toBe(32);
    expect(config.numRegisters).toBe(32);
    expect(config.memoryPerThread).toBe(1024);
    expect(config.maxDivergenceDepth).toBe(32);
    expect(config.independentThreadScheduling).toBe(false);
  });

  it("can be customized", () => {
    const config = makeWarpConfig({ warpWidth: 16, numRegisters: 64 });
    expect(config.warpWidth).toBe(16);
    expect(config.numRegisters).toBe(64);
  });
});

// ---------------------------------------------------------------------------
// WarpEngine -- basic properties
// ---------------------------------------------------------------------------

describe("WarpEngine properties", () => {
  it("has correct name", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    expect(engine.name).toBe("WarpEngine");
  });

  it("has correct width", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 16 }));
    expect(engine.width).toBe(16);
  });

  it("reports SIMT execution model", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    expect(engine.executionModel).toBe(ExecutionModel.SIMT);
  });

  it("starts not halted", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    expect(engine.halted).toBe(false);
  });

  it("all threads initially active", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    expect(engine.activeMask).toEqual([true, true, true, true]);
  });

  it("exposes config", () => {
    const config = makeWarpConfig({ warpWidth: 8 });
    const engine = new WarpEngine(config);
    expect(engine.config).toBe(config);
  });

  it("has readable toString", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    const r = engine.toString();
    expect(r).toContain("WarpEngine");
    expect(r).toContain("width=4");
  });
});

// ---------------------------------------------------------------------------
// WarpEngine -- program execution
// ---------------------------------------------------------------------------

describe("WarpEngine execution", () => {
  it("all threads execute simple program", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([limm(0, 42.0), halt()]);
    const traces = engine.run();
    expect(traces.length).toBeGreaterThanOrEqual(2);

    // All threads should have R0 = 42.0
    for (const t of engine.threads) {
      expect(t.core.registers.readFloat(0)).toBe(42.0);
    }
  });

  it("per-thread data computes independently", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([limm(1, 2.0), fmul(2, 0, 1), halt()]);

    // Give each thread a different R0
    for (let t = 0; t < 4; t++) {
      engine.setThreadRegister(t, 0, t + 1);
    }

    engine.run();

    // Thread t should have R2 = (t+1) * 2.0
    for (let t = 0; t < 4; t++) {
      expect(engine.threads[t].core.registers.readFloat(2)).toBe(
        (t + 1) * 2.0,
      );
    }
  });

  it("halts when all threads done", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([halt()]);
    engine.run();
    expect(engine.halted).toBe(true);
  });

  it("rejects out-of-range thread register", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    expect(() => engine.setThreadRegister(4, 0, 1.0)).toThrow();
    expect(() => engine.setThreadRegister(-1, 0, 1.0)).toThrow();
  });

  it("step produces traces with correct fields", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([limm(0, 1.0), halt()]);

    const trace = engine.step({ cycle: 1 });
    expect(trace.cycle).toBe(1);
    expect(trace.engineName).toBe("WarpEngine");
    expect(trace.executionModel).toBe(ExecutionModel.SIMT);
    expect(trace.totalCount).toBe(4);
    expect(trace.activeCount).toBeGreaterThan(0);
    expect(trace.utilization).toBeGreaterThanOrEqual(0.0);
    expect(trace.utilization).toBeLessThanOrEqual(1.0);
  });

  it("utilization is activeCount / totalCount", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([limm(0, 1.0), halt()]);

    const trace = engine.step({ cycle: 1 });
    const expected = trace.activeCount / trace.totalCount;
    expect(Math.abs(trace.utilization - expected)).toBeLessThan(0.001);
  });
});

// ---------------------------------------------------------------------------
// WarpEngine -- divergence
// ---------------------------------------------------------------------------

describe("WarpEngine divergence", () => {
  it("no divergence on uniform branch", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([
      limm(0, 0.0),
      limm(1, 10.0),
      blt(0, 1, 2),
      nop(),
      nop(),
      halt(),
    ]);
    engine.run();
    expect(engine.halted).toBe(true);
  });

  it("handles divergent branch", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([
      limm(1, 2.0),
      blt(0, 1, 2),
      limm(2, 99.0),
      halt(),
      limm(2, 42.0),
      halt(),
    ]);

    // Threads 0,1 have R0=0 (< 2), threads 2,3 have R0=5 (>= 2)
    engine.setThreadRegister(0, 0, 0.0);
    engine.setThreadRegister(1, 0, 0.0);
    engine.setThreadRegister(2, 0, 5.0);
    engine.setThreadRegister(3, 0, 5.0);

    engine.run();
    expect(engine.halted).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// WarpEngine -- reset
// ---------------------------------------------------------------------------

describe("WarpEngine reset", () => {
  it("restores initial state", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([limm(0, 42.0), halt()]);
    engine.run();
    expect(engine.halted).toBe(true);

    engine.reset();
    expect(engine.halted).toBe(false);
    expect(engine.threads.every((t) => t.active)).toBe(true);

    // Can run again after reset
    engine.run();
    expect(engine.halted).toBe(true);
  });

  it("clears registers", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([limm(0, 42.0), halt()]);
    engine.run();

    engine.reset();
    for (const t of engine.threads) {
      expect(t.core.registers.readFloat(0)).toBe(0.0);
    }
  });
});

// ---------------------------------------------------------------------------
// WarpEngine -- clock integration
// ---------------------------------------------------------------------------

describe("WarpEngine clock integration", () => {
  it("step works with clock edge", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([limm(0, 1.0), halt()]);
    const trace = engine.step({ cycle: 1 });
    expect(trace.cycle).toBe(1);
  });

  it("halted step returns trace", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([halt()]);
    engine.run();
    expect(engine.halted).toBe(true);

    const trace = engine.step({ cycle: 99 });
    expect(trace.activeCount).toBe(0);
    expect(
      trace.description.toLowerCase().includes("halted") ||
        trace.utilization === 0.0,
    ).toBe(true);
  });
});
