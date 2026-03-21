/**
 * Tests for SystolicArray -- dataflow execution (Google TPU style).
 */

import { describe, it, expect } from "vitest";
import { floatToBits, bitsToFloat, FP32 } from "@coding-adventures/fp-arithmetic";
import {
  SystolicArray,
  makeSystolicConfig,
  SystolicPE,
  ExecutionModel,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// SystolicConfig
// ---------------------------------------------------------------------------

describe("SystolicConfig", () => {
  it("has sensible defaults", () => {
    const config = makeSystolicConfig();
    expect(config.rows).toBe(4);
    expect(config.cols).toBe(4);
  });

  it("can be customized", () => {
    const config = makeSystolicConfig({ rows: 8, cols: 8 });
    expect(config.rows).toBe(8);
    expect(config.cols).toBe(8);
  });
});

// ---------------------------------------------------------------------------
// SystolicPE
// ---------------------------------------------------------------------------

describe("SystolicPE", () => {
  it("creation", () => {
    const zero = floatToBits(0.0, FP32);
    const pe = new SystolicPE(0, 0, zero, zero);
    expect(pe.row).toBe(0);
    expect(pe.col).toBe(0);
    expect(pe.inputBuffer).toBeNull();
  });

  it("compute with no input returns null", () => {
    const zero = floatToBits(0.0, FP32);
    const pe = new SystolicPE(0, 0, zero, zero);
    expect(pe.compute()).toBeNull();
  });

  it("compute with input performs MAC", () => {
    const weight = floatToBits(3.0, FP32);
    const zero = floatToBits(0.0, FP32);
    const inputVal = floatToBits(2.0, FP32);
    const pe = new SystolicPE(0, 0, weight, zero, inputVal);

    const output = pe.compute();
    expect(output).not.toBeNull();

    const acc = bitsToFloat(pe.accumulator);
    // acc = 0 + 2.0 * 3.0 = 6.0
    expect(Math.abs(acc - 6.0)).toBeLessThan(0.01);
  });

  it("compute accumulates across multiple calls", () => {
    const weight = floatToBits(1.0, FP32);
    const zero = floatToBits(0.0, FP32);
    const pe = new SystolicPE(0, 0, weight, zero);

    // First MAC: acc = 0 + 2.0 * 1.0 = 2.0
    pe.inputBuffer = floatToBits(2.0, FP32);
    pe.compute();

    // Second MAC: acc = 2.0 + 3.0 * 1.0 = 5.0
    pe.inputBuffer = floatToBits(3.0, FP32);
    pe.compute();

    expect(Math.abs(bitsToFloat(pe.accumulator) - 5.0)).toBeLessThan(0.01);
  });
});

// ---------------------------------------------------------------------------
// SystolicArray -- basic properties
// ---------------------------------------------------------------------------

describe("SystolicArray properties", () => {
  it("has correct name", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    expect(array.name).toBe("SystolicArray");
  });

  it("has correct width", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 3, cols: 4 }));
    expect(array.width).toBe(12);
  });

  it("reports SYSTOLIC execution model", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    expect(array.executionModel).toBe(ExecutionModel.SYSTOLIC);
  });

  it("starts not halted", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    expect(array.halted).toBe(false);
  });

  it("exposes config", () => {
    const config = makeSystolicConfig({ rows: 3, cols: 3 });
    const array = new SystolicArray(config);
    expect(array.config).toBe(config);
  });

  it("exposes grid", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    expect(array.grid.length).toBe(2);
    expect(array.grid[0].length).toBe(2);
  });

  it("has readable toString", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    expect(array.toString()).toContain("SystolicArray");
    expect(array.toString()).toContain("2x2");
  });
});

// ---------------------------------------------------------------------------
// SystolicArray -- weight loading
// ---------------------------------------------------------------------------

describe("SystolicArray weights", () => {
  it("loadWeights", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    array.loadWeights([
      [1.0, 2.0],
      [3.0, 4.0],
    ]);

    expect(bitsToFloat(array.grid[0][0].weight)).toBe(1.0);
    expect(bitsToFloat(array.grid[0][1].weight)).toBe(2.0);
    expect(bitsToFloat(array.grid[1][0].weight)).toBe(3.0);
    expect(bitsToFloat(array.grid[1][1].weight)).toBe(4.0);
  });
});

// ---------------------------------------------------------------------------
// SystolicArray -- input feeding
// ---------------------------------------------------------------------------

describe("SystolicArray input", () => {
  it("feedInput queues data", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    array.feedInput(0, 5.0);
    // No error = success
  });

  it("feedInput rejects out-of-range row", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    expect(() => array.feedInput(2, 1.0)).toThrow();
    expect(() => array.feedInput(-1, 1.0)).toThrow();
  });

  it("feedInputVector", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    array.feedInputVector([1.0, 2.0]);
    // No error = success
  });
});

// ---------------------------------------------------------------------------
// SystolicArray -- matrix multiplication
// ---------------------------------------------------------------------------

describe("SystolicArray matmul", () => {
  it("identity weights return input", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    const result = array.runMatmul(
      [
        [1.0, 0.0],
        [0.0, 1.0],
      ],
      [
        [1.0, 0.0],
        [0.0, 1.0],
      ],
    );
    expect(Math.abs(result[0][0] - 1.0)).toBeLessThan(0.01);
    expect(Math.abs(result[0][1] - 0.0)).toBeLessThan(0.01);
    expect(Math.abs(result[1][0] - 0.0)).toBeLessThan(0.01);
    expect(Math.abs(result[1][1] - 1.0)).toBeLessThan(0.01);
  });

  it("2x2 matmul", () => {
    // A = [[1, 2], [3, 4]], W = [[5, 6], [7, 8]]
    // C = A x W = [[19, 22], [43, 50]]
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    const result = array.runMatmul(
      [
        [1.0, 2.0],
        [3.0, 4.0],
      ],
      [
        [5.0, 6.0],
        [7.0, 8.0],
      ],
    );
    expect(Math.abs(result[0][0] - 19.0)).toBeLessThan(0.1);
    expect(Math.abs(result[0][1] - 22.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][0] - 43.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][1] - 50.0)).toBeLessThan(0.1);
  });

  it("3x3 matmul", () => {
    const A = [
      [1.0, 0.0, 0.0],
      [0.0, 2.0, 0.0],
      [0.0, 0.0, 3.0],
    ];
    const W = [
      [1.0, 2.0, 3.0],
      [4.0, 5.0, 6.0],
      [7.0, 8.0, 9.0],
    ];
    // C = A x W = [[1, 2, 3], [8, 10, 12], [21, 24, 27]]
    const array = new SystolicArray(makeSystolicConfig({ rows: 3, cols: 3 }));
    const result = array.runMatmul(A, W);
    expect(Math.abs(result[0][0] - 1.0)).toBeLessThan(0.1);
    expect(Math.abs(result[0][1] - 2.0)).toBeLessThan(0.1);
    expect(Math.abs(result[0][2] - 3.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][0] - 8.0)).toBeLessThan(0.1);
    expect(Math.abs(result[1][1] - 10.0)).toBeLessThan(0.1);
    expect(Math.abs(result[2][2] - 27.0)).toBeLessThan(0.1);
  });

  it("drainOutputs returns correct shape", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 3 }));
    const result = array.drainOutputs();
    expect(result.length).toBe(2);
    expect(result[0].length).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// SystolicArray -- stepping and traces
// ---------------------------------------------------------------------------

describe("SystolicArray stepping", () => {
  it("step produces trace", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    array.loadWeights([
      [1.0, 0.0],
      [0.0, 1.0],
    ]);
    array.feedInput(0, 2.0);

    const trace = array.step({ cycle: 1 });
    expect(trace.cycle).toBe(1);
    expect(trace.engineName).toBe("SystolicArray");
    expect(trace.executionModel).toBe(ExecutionModel.SYSTOLIC);
    expect(trace.dataflowInfo).toBeDefined();
  });

  it("halts when no data", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    array.loadWeights([
      [1.0, 0.0],
      [0.0, 1.0],
    ]);
    array.feedInput(0, 1.0);

    for (let i = 0; i < 20; i++) {
      array.step({ cycle: i + 1 });
      if (array.halted) break;
    }
    expect(array.halted).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// SystolicArray -- reset
// ---------------------------------------------------------------------------

describe("SystolicArray reset", () => {
  it("resets to initial state", () => {
    const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
    array.loadWeights([
      [1.0, 2.0],
      [3.0, 4.0],
    ]);
    array.runMatmul(
      [
        [1.0, 0.0],
        [0.0, 1.0],
      ],
      [
        [1.0, 2.0],
        [3.0, 4.0],
      ],
    );

    array.reset();
    expect(array.halted).toBe(false);
    expect(bitsToFloat(array.grid[0][0].accumulator)).toBe(0.0);
  });
});
