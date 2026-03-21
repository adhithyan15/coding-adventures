/**
 * Tests for NeuralEngineCore -- Apple ANE Core simulator.
 */

import { describe, it, expect } from "vitest";

import { Architecture, makeWorkItem } from "../src/index.js";
import {
  NeuralEngineCore,
  makeANECoreConfig,
} from "../src/neural-engine-core.js";

// ---------------------------------------------------------------------------
// ANECoreConfig tests
// ---------------------------------------------------------------------------

describe("ANECoreConfig", () => {
  it("has correct defaults", () => {
    const config = makeANECoreConfig();
    expect(config.numMacs).toBe(16);
    expect(config.sramSize).toBe(4194304);
    expect(config.activationBuffer).toBe(131072);
    expect(config.weightBuffer).toBe(524288);
    expect(config.outputBuffer).toBe(131072);
    expect(config.dmaBandwidth).toBe(10);
  });

  it("allows customization", () => {
    const config = makeANECoreConfig({ numMacs: 8, dmaBandwidth: 20 });
    expect(config.numMacs).toBe(8);
    expect(config.dmaBandwidth).toBe(20);
  });
});

// ---------------------------------------------------------------------------
// NeuralEngineCore tests
// ---------------------------------------------------------------------------

describe("NeuralEngineCore", () => {
  it("creates correctly", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig({ numMacs: 4 }));
    expect(ane.name).toBe("ANECore");
    expect(ane.architecture).toBe(Architecture.APPLE_ANE_CORE);
    expect(ane.idle).toBe(true);
  });

  it("computes dot product", () => {
    /** [1, 2, 3, 4] . [0.5, 0.5, 0.5, 0.5] = 5.0 */
    const ane = new NeuralEngineCore(makeANECoreConfig({ numMacs: 4 }));
    const result = ane.runInference(
      [[1.0, 2.0, 3.0, 4.0]],
      [[0.5], [0.5], [0.5], [0.5]],
      "none",
    );
    expect(result.length).toBe(1);
    expect(Math.abs(result[0][0] - 5.0)).toBeLessThan(0.01);
  });

  it("computes 2x2 matmul", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference(
      [[1.0, 2.0], [3.0, 4.0]],
      [[5.0, 6.0], [7.0, 8.0]],
      "none",
    );
    expect(Math.abs(result[0][0] - 19.0)).toBeLessThan(0.01);
    expect(Math.abs(result[0][1] - 22.0)).toBeLessThan(0.01);
    expect(Math.abs(result[1][0] - 43.0)).toBeLessThan(0.01);
    expect(Math.abs(result[1][1] - 50.0)).toBeLessThan(0.01);
  });

  it("applies relu: zeroes negatives", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference(
      [[1.0, -2.0]],
      [[1.0], [1.0]],
      "relu",
    );
    // 1*1 + (-2)*1 = -1, ReLU(-1) = 0
    expect(Math.abs(result[0][0])).toBeLessThan(0.01);
  });

  it("applies relu: passes positives", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference(
      [[3.0, 2.0]],
      [[1.0], [1.0]],
      "relu",
    );
    // 3+2=5, ReLU(5)=5
    expect(Math.abs(result[0][0] - 5.0)).toBeLessThan(0.01);
  });

  it("applies sigmoid", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference([[0.0]], [[1.0]], "sigmoid");
    expect(Math.abs(result[0][0] - 0.5)).toBeLessThan(0.01);
  });

  it("applies tanh", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference([[0.0]], [[1.0]], "tanh");
    expect(Math.abs(result[0][0])).toBeLessThan(0.01);
  });

  it("sigmoid handles large positive input", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference([[100.0]], [[1.0]], "sigmoid");
    expect(result[0][0]).toBeGreaterThan(0.99);
  });

  it("sigmoid handles large negative input", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference([[-100.0]], [[1.0]], "sigmoid");
    expect(result[0][0]).toBeLessThan(0.01);
  });

  it("dispatch and run works", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    ane.dispatch(makeWorkItem({
      workId: 0,
      inputData: [[1.0, 2.0]],
      weightData: [[3.0], [4.0]],
    }));
    const traces = ane.run();
    expect(traces.length).toBeGreaterThan(0);
    expect(ane.idle).toBe(true);
    // 1*3 + 2*4 = 11
    expect(Math.abs(ane.result[0][0] - 11.0)).toBeLessThan(0.01);
  });

  it("dispatch without data works", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    ane.dispatch(makeWorkItem({ workId: 0 }));
    ane.run();
    expect(ane.idle).toBe(true);
    expect(ane.result).toEqual([]);
  });

  it("traces have correct architecture", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    ane.dispatch(makeWorkItem({
      workId: 0,
      inputData: [[1.0]],
      weightData: [[2.0]],
    }));
    const traces = ane.run();
    for (const trace of traces) {
      expect(trace.architecture).toBe(Architecture.APPLE_ANE_CORE);
      expect(trace.unitName).toBe("ANECore");
    }
  });

  it("produces idle trace", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const trace = ane.step({ cycle: 1 });
    expect(trace.schedulerAction).toBe("idle");
    expect(trace.occupancy).toBe(0.0);
  });

  it("handles multiple dispatches", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    ane.dispatch(makeWorkItem({ workId: 0, inputData: [[1.0]], weightData: [[2.0]] }));
    ane.dispatch(makeWorkItem({ workId: 1, inputData: [[3.0]], weightData: [[4.0]] }));
    ane.run();
    expect(ane.idle).toBe(true);
  });

  it("MAC engine is accessible", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    expect(ane.macEngine).toBeDefined();
  });

  it("resets correctly", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    ane.runInference([[1.0]], [[2.0]], "relu");
    ane.reset();
    expect(ane.idle).toBe(true);
    expect(ane.result).toEqual([]);
  });

  it("toString includes key info", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig({ numMacs: 8 }));
    const r = ane.toString();
    expect(r).toContain("NeuralEngineCore");
    expect(r).toContain("macs=8");
  });

  it("unknown activation passes through", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference([[5.0]], [[1.0]], "unknown");
    expect(Math.abs(result[0][0] - 5.0)).toBeLessThan(0.01);
  });

  it("computes 3x2 matmul", () => {
    const ane = new NeuralEngineCore(makeANECoreConfig());
    const result = ane.runInference(
      [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]],
      [[7.0], [8.0]],
      "none",
    );
    expect(result.length).toBe(3);
    expect(Math.abs(result[0][0] - 23.0)).toBeLessThan(0.01); // 1*7+2*8
    expect(Math.abs(result[1][0] - 53.0)).toBeLessThan(0.01); // 3*7+4*8
    expect(Math.abs(result[2][0] - 83.0)).toBeLessThan(0.01); // 5*7+6*8
  });
});
