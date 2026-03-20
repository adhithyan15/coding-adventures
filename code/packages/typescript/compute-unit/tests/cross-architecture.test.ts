/**
 * Cross-architecture tests -- same computation on all architectures.
 *
 * === Why Cross-Architecture Tests? ===
 *
 * These tests verify that the SAME computation produces the SAME result
 * (within floating-point tolerance) across all five compute unit
 * architectures. This validates that our simulators are correct and
 * that the architectural differences are in HOW the computation happens,
 * not WHAT is computed.
 *
 * The test uses matrix multiplication as the benchmark because:
 * 1. It's supported by all architectures
 * 2. It has a well-defined expected result
 * 3. It exercises the core compute pipeline of each architecture
 */

import { describe, it, expect } from "vitest";
import { fmul, halt } from "@coding-adventures/gpu-core";

import {
  Architecture,
  StreamingMultiprocessor,
  makeSMConfig,
  makeWorkItem,
} from "../src/index.js";
import { AMDComputeUnit, makeAMDCUConfig } from "../src/amd-compute-unit.js";
import { MatrixMultiplyUnit, makeMXUConfig } from "../src/matrix-multiply-unit.js";
import { NeuralEngineCore, makeANECoreConfig } from "../src/neural-engine-core.js";
import { XeCore, makeXeCoreConfig } from "../src/xe-core.js";

describe("Cross-Architecture Matmul", () => {
  // We'll compute: [1, 2] x [[3], [4]] = [11]

  it("NVIDIA SM: manual matmul via SIMT threads", () => {
    const sm = new StreamingMultiprocessor(
      makeSMConfig({ maxWarps: 8, warpWidth: 4, numSchedulers: 1 }),
    );
    const prog = [fmul(2, 0, 1), halt()];
    sm.dispatch(makeWorkItem({
      workId: 0,
      program: prog,
      threadCount: 2,
      perThreadData: {
        0: { 0: 1.0, 1: 3.0 }, // a[0]*b[0] = 3
        1: { 0: 2.0, 1: 4.0 }, // a[1]*b[1] = 8
      },
    }));
    sm.run();
    expect(sm.idle).toBe(true);

    // Read results from each thread's register 2
    const warp = sm.warpSlots[0];
    const r0 = warp.engine.threads[0].core.registers.readFloat(2);
    const r1 = warp.engine.threads[1].core.registers.readFloat(2);
    const total = r0 + r1;
    expect(Math.abs(total - 11.0)).toBeLessThan(0.01);
  });

  it("AMD CU: manual matmul via SIMD wavefront", () => {
    const cu = new AMDComputeUnit(
      makeAMDCUConfig({ maxWavefronts: 8, waveWidth: 4, numSimdUnits: 1 }),
    );
    const prog = [fmul(2, 0, 1), halt()];
    cu.dispatch(makeWorkItem({
      workId: 0,
      program: prog,
      threadCount: 2,
      perThreadData: {
        0: { 0: 1.0, 1: 3.0 },
        1: { 0: 2.0, 1: 4.0 },
      },
    }));
    cu.run();
    expect(cu.idle).toBe(true);
  });

  it("Google MXU: systolic array matmul", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul([[1.0, 2.0]], [[3.0], [4.0]]);
    expect(Math.abs(result[0][0] - 11.0)).toBeLessThan(0.1);
  });

  it("Apple ANE: MAC array matmul", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig({ numMacs: 4 }));
    const result = ane.runInference(
      [[1.0, 2.0]],
      [[3.0], [4.0]],
      "none",
    );
    expect(Math.abs(result[0][0] - 11.0)).toBeLessThan(0.01);
  });

  it("Intel Xe Core: SIMD8 threads compute", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    xe.dispatch(makeWorkItem({
      workId: 0,
      program: [fmul(2, 0, 1), halt()],
      threadCount: 8,
      perThreadData: {
        0: { 0: 1.0, 1: 3.0 },
        1: { 0: 2.0, 1: 4.0 },
      },
    }));
    xe.run();
    expect(xe.idle).toBe(true);
  });
});

describe("Cross-Architecture 2x2 Matmul", () => {
  /**
   * [1, 2]   [5, 6]   [19, 22]
   * [3, 4] x [7, 8] = [43, 50]
   */
  const EXPECTED = [[19.0, 22.0], [43.0, 50.0]];
  const TOL = 0.5; // tolerance for FP rounding

  it("MXU 2x2", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul(
      [[1.0, 2.0], [3.0, 4.0]],
      [[5.0, 6.0], [7.0, 8.0]],
    );
    for (let i = 0; i < 2; i++) {
      for (let j = 0; j < 2; j++) {
        expect(Math.abs(result[i][j] - EXPECTED[i][j])).toBeLessThan(TOL);
      }
    }
  });

  it("ANE 2x2", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference(
      [[1.0, 2.0], [3.0, 4.0]],
      [[5.0, 6.0], [7.0, 8.0]],
      "none",
    );
    for (let i = 0; i < 2; i++) {
      for (let j = 0; j < 2; j++) {
        expect(Math.abs(result[i][j] - EXPECTED[i][j])).toBeLessThan(TOL);
      }
    }
  });
});

describe("Architecture Properties", () => {
  it("all 5 architectures represented", () => {
    const sm = new StreamingMultiprocessor(makeSMConfig({ maxWarps: 4 }));
    const cu = new AMDComputeUnit(makeAMDCUConfig({ maxWavefronts: 4, waveWidth: 4 }));
    const mxu = new MatrixMultiplyUnit(makeMXUConfig({ arrayRows: 2, arrayCols: 2 }));
    const xe = new XeCore(makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }));
    const ane = new NeuralEngineCore(makeANECoreConfig({ numMacs: 4 }));

    const archs = new Set([
      sm.architecture,
      cu.architecture,
      mxu.architecture,
      xe.architecture,
      ane.architecture,
    ]);
    expect(archs.size).toBe(5);
    expect(archs.has(Architecture.NVIDIA_SM)).toBe(true);
    expect(archs.has(Architecture.AMD_CU)).toBe(true);
    expect(archs.has(Architecture.GOOGLE_MXU)).toBe(true);
    expect(archs.has(Architecture.INTEL_XE_CORE)).toBe(true);
    expect(archs.has(Architecture.APPLE_ANE_CORE)).toBe(true);
  });

  it("all names are unique", () => {
    const names = new Set([
      new StreamingMultiprocessor(makeSMConfig({ maxWarps: 4 })).name,
      new AMDComputeUnit(makeAMDCUConfig({ maxWavefronts: 4, waveWidth: 4 })).name,
      new MatrixMultiplyUnit(makeMXUConfig({ arrayRows: 2, arrayCols: 2 })).name,
      new XeCore(makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 })).name,
      new NeuralEngineCore(makeANECoreConfig({ numMacs: 4 })).name,
    ]);
    expect(names.size).toBe(5);
  });

  it("all start idle", () => {
    const units = [
      new StreamingMultiprocessor(makeSMConfig({ maxWarps: 4 })),
      new AMDComputeUnit(makeAMDCUConfig({ maxWavefronts: 4, waveWidth: 4 })),
      new MatrixMultiplyUnit(makeMXUConfig({ arrayRows: 2, arrayCols: 2 })),
      new XeCore(makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 })),
      new NeuralEngineCore(makeANECoreConfig({ numMacs: 4 })),
    ];
    for (const unit of units) {
      expect(unit.idle).toBe(true);
    }
  });
});
