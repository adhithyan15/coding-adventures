/**
 * Cross-engine tests -- verify same computation produces same results on all engines.
 *
 * This is the educational payoff of having multiple engines: you can run the
 * SAME computation on NVIDIA-style SIMT, AMD-style SIMD, Google-style systolic,
 * and Apple-style MAC arrays, and verify they all produce the same numerical
 * results -- just with different execution traces, cycle counts, and utilization.
 */

import { describe, it, expect } from "vitest";
import { limm, fmul, halt } from "@coding-adventures/gpu-core";
import {
  WarpEngine,
  makeWarpConfig,
  WavefrontEngine,
  makeWavefrontConfig,
  SystolicArray,
  makeSystolicConfig,
  MACArrayEngine,
  makeMACArrayConfig,
  makeMACScheduleEntry,
  MACOperation,
  ExecutionModel,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Scalar multiply across all engines
// ---------------------------------------------------------------------------

describe("cross-engine scalar multiply", () => {
  it("SIMT: each thread computes 3.0 * 4.0 = 12.0", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()]);
    engine.run();
    for (const t of engine.threads) {
      expect(t.core.registers.readFloat(2)).toBe(12.0);
    }
  });

  it("SIMD: all lanes compute 3.0 * 4.0 = 12.0", () => {
    const engine = new WavefrontEngine(
      makeWavefrontConfig({ waveWidth: 4 }),
    );
    engine.loadProgram([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()]);
    engine.run();
    for (let lane = 0; lane < 4; lane++) {
      expect(engine.vrf.read(2, lane)).toBe(12.0);
    }
  });

  it("Systolic: 1x1 matmul is just a multiply", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 1, cols: 1 }));
    const result = array.runMatmul([[3.0]], [[4.0]]);
    expect(Math.abs(result[0][0] - 12.0)).toBeLessThan(0.01);
  });

  it("MAC: one MAC unit computes 3.0 * 4.0 = 12.0", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 1 }));
    engine.loadInputs([3.0]);
    engine.loadWeights([4.0]);
    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0],
        weightIndices: [0],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 3,
        operation: MACOperation.STORE_OUTPUT,
        outputIndex: 0,
      }),
    ]);
    engine.run();
    expect(Math.abs(engine.readOutputs()[0] - 12.0)).toBeLessThan(0.01);
  });
});

// ---------------------------------------------------------------------------
// Dot product across engines
// ---------------------------------------------------------------------------

describe("cross-engine dot product", () => {
  const a = [1.0, 2.0, 3.0, 4.0];
  const b = [5.0, 6.0, 7.0, 8.0];
  // dot(a,b) = 1*5 + 2*6 + 3*7 + 4*8 = 5+12+21+32 = 70

  it("SIMT: each thread multiplies one pair", () => {
    const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    engine.loadProgram([fmul(2, 0, 1), halt()]);
    for (let t = 0; t < 4; t++) {
      engine.setThreadRegister(t, 0, a[t]);
      engine.setThreadRegister(t, 1, b[t]);
    }
    engine.run();

    let total = 0;
    for (let t = 0; t < 4; t++) {
      total += engine.threads[t].core.registers.readFloat(2);
    }
    expect(Math.abs(total - 70.0)).toBeLessThan(0.1);
  });

  it("SIMD: all lanes multiply in parallel", () => {
    const engine = new WavefrontEngine(
      makeWavefrontConfig({ waveWidth: 4 }),
    );
    engine.loadProgram([fmul(2, 0, 1), halt()]);
    for (let lane = 0; lane < 4; lane++) {
      engine.setLaneRegister(lane, 0, a[lane]);
      engine.setLaneRegister(lane, 1, b[lane]);
    }
    engine.run();

    let total = 0;
    for (let lane = 0; lane < 4; lane++) {
      total += engine.vrf.read(2, lane);
    }
    expect(Math.abs(total - 70.0)).toBeLessThan(0.1);
  });

  it("MAC: parallel MACs + reduce", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadInputs(a);
    engine.loadWeights(b);
    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0, 1, 2, 3],
        weightIndices: [0, 1, 2, 3],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
    ]);
    engine.run();
    expect(Math.abs(engine.readOutputs()[0] - 70.0)).toBeLessThan(0.1);
  });
});

// ---------------------------------------------------------------------------
// Matrix multiplication: systolic vs MAC
// ---------------------------------------------------------------------------

describe("cross-engine matmul", () => {
  it("systolic 2x2 matmul matches MAC 2x2", () => {
    const A = [
      [1.0, 2.0],
      [3.0, 4.0],
    ];
    const W = [
      [5.0, 6.0],
      [7.0, 8.0],
    ];
    // Expected: [[19, 22], [43, 50]]

    // Systolic
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    const systolicResult = array.runMatmul(A, W);

    // MAC: C[0][0] = A[0][0]*W[0][0] + A[0][1]*W[1][0] = 1*5+2*7 = 19
    const mac = new MACArrayEngine(makeMACArrayConfig({ numMacs: 2 }));
    mac.loadInputs([1.0, 2.0]);
    mac.loadWeights([5.0, 7.0]);
    mac.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0, 1],
        weightIndices: [0, 1],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
    ]);
    mac.run();

    expect(Math.abs(systolicResult[0][0] - 19.0)).toBeLessThan(0.1);
    expect(Math.abs(mac.readOutputs()[0] - 19.0)).toBeLessThan(0.1);
  });
});

// ---------------------------------------------------------------------------
// Execution model verification
// ---------------------------------------------------------------------------

describe("cross-engine execution models", () => {
  it("all engines report correct model", () => {
    const warp = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    const wave = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    const systolic = new SystolicArray(
      makeSystolicConfig({ rows: 2, cols: 2 }),
    );
    const mac = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));

    expect(warp.executionModel).toBe(ExecutionModel.SIMT);
    expect(wave.executionModel).toBe(ExecutionModel.SIMD);
    expect(systolic.executionModel).toBe(ExecutionModel.SYSTOLIC);
    expect(mac.executionModel).toBe(ExecutionModel.SCHEDULED_MAC);
  });

  it("all engines have names", () => {
    const warp = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
    const wave = new WavefrontEngine(makeWavefrontConfig({ waveWidth: 4 }));
    const systolic = new SystolicArray(
      makeSystolicConfig({ rows: 2, cols: 2 }),
    );
    const mac = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));

    expect(warp.name).toBe("WarpEngine");
    expect(wave.name).toBe("WavefrontEngine");
    expect(systolic.name).toBe("SystolicArray");
    expect(mac.name).toBe("MACArrayEngine");
  });
});
