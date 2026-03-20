/**
 * Tests for MatrixMultiplyUnit -- Google TPU MXU simulator.
 */

import { describe, it, expect } from "vitest";

import {
  Architecture,
  makeWorkItem,
} from "../src/index.js";
import {
  MatrixMultiplyUnit,
  makeMXUConfig,
} from "../src/matrix-multiply-unit.js";

// ---------------------------------------------------------------------------
// MXUConfig tests
// ---------------------------------------------------------------------------

describe("MXUConfig", () => {
  it("has correct defaults", () => {
    const config = makeMXUConfig();
    expect(config.arrayRows).toBe(128);
    expect(config.arrayCols).toBe(128);
    expect(config.vectorWidth).toBe(128);
    expect(config.accumulatorCount).toBe(128);
    expect(config.weightBufferSize).toBe(4194304);
    expect(config.activationBufferSize).toBe(2097152);
  });

  it("allows customization", () => {
    const config = makeMXUConfig({ arrayRows: 4, arrayCols: 4 });
    expect(config.arrayRows).toBe(4);
    expect(config.arrayCols).toBe(4);
  });
});

// ---------------------------------------------------------------------------
// MatrixMultiplyUnit tests
// ---------------------------------------------------------------------------

describe("MatrixMultiplyUnit", () => {
  it("creates correctly", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    expect(mxu.name).toBe("MXU");
    expect(mxu.architecture).toBe(Architecture.GOOGLE_MXU);
    expect(mxu.idle).toBe(true);
  });

  it("computes 2x2 matmul", () => {
    /**
     * [1, 2]   [5, 6]   [1*5+2*7, 1*6+2*8]   [19, 22]
     * [3, 4] x [7, 8] = [3*5+4*7, 3*6+4*8] = [43, 50]
     */
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul(
      [[1.0, 2.0], [3.0, 4.0]],
      [[5.0, 6.0], [7.0, 8.0]],
    );
    expect(result.length).toBe(2);
    expect(result[0].length).toBe(2);
    expect(Math.abs(result[0][0] - 19.0)).toBeLessThan(0.1);
    expect(Math.abs(result[0][1] - 22.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][0] - 43.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][1] - 50.0)).toBeLessThan(0.1);
  });

  it("identity matmul returns original", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul(
      [[1.0, 2.0], [3.0, 4.0]],
      [[1.0, 0.0], [0.0, 1.0]],
    );
    expect(Math.abs(result[0][0] - 1.0)).toBeLessThan(0.1);
    expect(Math.abs(result[0][1] - 2.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][0] - 3.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][1] - 4.0)).toBeLessThan(0.1);
  });

  it("applies relu activation", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul(
      [[1.0, -2.0]],
      [[1.0], [1.0]],
      "relu",
    );
    // 1*1 + (-2)*1 = -1.0, ReLU(-1.0) = 0.0
    expect(Math.abs(result[0][0] - 0.0)).toBeLessThan(0.1);
  });

  it("applies sigmoid activation", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul([[0.0]], [[1.0]], "sigmoid");
    // sigmoid(0) = 0.5
    expect(Math.abs(result[0][0] - 0.5)).toBeLessThan(0.01);
  });

  it("applies tanh activation", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul([[0.0]], [[1.0]], "tanh");
    // tanh(0) = 0.0
    expect(Math.abs(result[0][0])).toBeLessThan(0.01);
  });

  it("no activation passes through", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul([[1.0, -2.0]], [[1.0], [1.0]], "none");
    expect(Math.abs(result[0][0] - (-1.0))).toBeLessThan(0.1);
  });

  it("dispatch and run works", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    mxu.dispatch(makeWorkItem({
      workId: 0,
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    }));
    const traces = mxu.run();
    expect(traces.length).toBeGreaterThan(0);
    expect(mxu.idle).toBe(true);
    expect(mxu.result.length).toBe(2);
  });

  it("dispatch without data works", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    mxu.dispatch(makeWorkItem({ workId: 0 }));
    mxu.run();
    expect(mxu.idle).toBe(true);
    expect(mxu.result).toEqual([]);
  });

  it("traces have correct architecture", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    mxu.dispatch(makeWorkItem({
      workId: 0,
      inputData: [[1.0]],
      weightData: [[2.0]],
    }));
    const traces = mxu.run();
    for (const trace of traces) {
      expect(trace.architecture).toBe(Architecture.GOOGLE_MXU);
      expect(trace.unitName).toBe("MXU");
    }
  });

  it("produces idle trace", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const trace = mxu.step({ cycle: 1 });
    expect(trace.schedulerAction).toBe("idle");
    expect(trace.occupancy).toBe(0.0);
  });

  it("handles multiple dispatches", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    mxu.dispatch(makeWorkItem({ workId: 0, inputData: [[1.0]], weightData: [[2.0]] }));
    mxu.dispatch(makeWorkItem({ workId: 1, inputData: [[3.0]], weightData: [[4.0]] }));
    mxu.run();
    expect(mxu.idle).toBe(true);
  });

  it("resets correctly", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    mxu.runMatmul([[1.0]], [[2.0]]);
    mxu.reset();
    expect(mxu.idle).toBe(true);
    expect(mxu.result).toEqual([]);
  });

  it("systolic array is accessible", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    expect(mxu.systolicArray).toBeDefined();
  });

  it("toString includes key info", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const r = mxu.toString();
    expect(r).toContain("MatrixMultiplyUnit");
    expect(r).toContain("4x4");
  });

  it("computes non-square matmul (3x2 times 2x3)", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul(
      [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]],
      [[7.0, 8.0, 9.0], [10.0, 11.0, 12.0]],
    );
    expect(result.length).toBe(3);
    expect(result[0].length).toBe(3);
    expect(Math.abs(result[0][0] - 27.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][0] - 61.0)).toBeLessThan(0.1);
    expect(Math.abs(result[2][2] - 117.0)).toBeLessThan(0.1);
  });

  it("unknown activation passes through", () => {
    const mxu = new MatrixMultiplyUnit(
      makeMXUConfig({ arrayRows: 4, arrayCols: 4 }),
    );
    const result = mxu.runMatmul([[5.0]], [[1.0]], "unknown_fn");
    expect(Math.abs(result[0][0] - 5.0)).toBeLessThan(0.1);
  });
});
