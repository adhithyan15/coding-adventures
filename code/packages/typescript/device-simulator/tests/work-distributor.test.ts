/**
 * Tests for work distributors -- GPU, TPU, and ANE strategies.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";
import { Clock } from "@coding-adventures/clock";
import {
  StreamingMultiprocessor,
  makeSMConfig,
  MatrixMultiplyUnit,
  makeMXUConfig,
  NeuralEngineCore,
  makeANECoreConfig,
} from "@coding-adventures/compute-unit";

import { GPUWorkDistributor, TPUSequencer, ANEScheduleReplayer } from "../src/work-distributor.js";
import { makeKernelDescriptor, totalBlocks, threadsPerBlock, totalThreads } from "../src/protocols.js";

// =========================================================================
// Helper: create small CUs for testing
// =========================================================================

function makeSMs(n: number = 4) {
  const config = makeSMConfig({
    maxWarps: 4,
    numSchedulers: 1,
    sharedMemorySize: 1024,
    registerFileSize: 2048,
  });
  const sms = Array.from({ length: n }, () => new StreamingMultiprocessor(config));
  return sms;
}

function makeMXU() {
  return new MatrixMultiplyUnit(makeMXUConfig());
}

function makeANECores(n: number = 4) {
  const config = makeANECoreConfig();
  return Array.from({ length: n }, () => new NeuralEngineCore(config));
}

// =========================================================================
// GPU Work Distributor
// =========================================================================

describe("GPUWorkDistributor", () => {
  it("should create blocks from kernel submit", () => {
    const sms = makeSMs(2);
    const dist = new GPUWorkDistributor(sms);
    const kernel = makeKernelDescriptor({
      name: "test",
      gridDim: [4, 1, 1],
      blockDim: [32, 1, 1],
    });
    dist.submitKernel(kernel);
    expect(dist.pendingCount).toBe(4);
  });

  it("should dispatch blocks on step", () => {
    const sms = makeSMs(2);
    const dist = new GPUWorkDistributor(sms);
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 1.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    dist.submitKernel(kernel);
    const actions = dist.step();
    expect(actions.length).toBeGreaterThanOrEqual(1);
    expect(dist.pendingCount).toBeLessThan(2);
  });

  it("should distribute evenly with round_robin", () => {
    const sms = makeSMs(4);
    const dist = new GPUWorkDistributor(sms, "round_robin");
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 1.0), halt()],
      gridDim: [4, 1, 1],
      blockDim: [32, 1, 1],
    });
    dist.submitKernel(kernel);
    dist.step();
    expect(dist.totalDispatched).toBeGreaterThan(0);
  });

  it("should track total dispatched", () => {
    const sms = makeSMs(2);
    const dist = new GPUWorkDistributor(sms);
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 1.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    dist.submitKernel(kernel);
    dist.step();
    expect(dist.totalDispatched).toBeGreaterThanOrEqual(1);
  });

  it("should return empty array when no pending work", () => {
    const sms = makeSMs(2);
    const dist = new GPUWorkDistributor(sms);
    const actions = dist.step();
    expect(actions).toEqual([]);
  });

  it("should reset pending and dispatched", () => {
    const sms = makeSMs(2);
    const dist = new GPUWorkDistributor(sms);
    const kernel = makeKernelDescriptor({
      name: "test",
      gridDim: [4, 1, 1],
      blockDim: [32, 1, 1],
    });
    dist.submitKernel(kernel);
    dist.reset();
    expect(dist.pendingCount).toBe(0);
    expect(dist.totalDispatched).toBe(0);
  });

  it("should support fill_first policy", () => {
    const sms = makeSMs(2);
    const dist = new GPUWorkDistributor(sms, "fill_first");
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 1.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    dist.submitKernel(kernel);
    dist.step();
    expect(dist.totalDispatched).toBeGreaterThanOrEqual(1);
  });

  it("should support least_loaded policy", () => {
    const sms = makeSMs(2);
    const dist = new GPUWorkDistributor(sms, "least_loaded");
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 1.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    dist.submitKernel(kernel);
    dist.step();
    expect(dist.totalDispatched).toBeGreaterThanOrEqual(1);
  });
});

// =========================================================================
// KernelDescriptor helpers
// =========================================================================

describe("KernelDescriptor helpers", () => {
  it("should compute totalBlocks, threadsPerBlock, totalThreads", () => {
    const k = makeKernelDescriptor({
      gridDim: [4, 2, 1],
      blockDim: [16, 16, 1],
    });
    expect(totalBlocks(k)).toBe(8);
    expect(threadsPerBlock(k)).toBe(256);
    expect(totalThreads(k)).toBe(2048);
  });
});

// =========================================================================
// TPU Sequencer
// =========================================================================

describe("TPUSequencer", () => {
  it("should create tiles from operation submit", () => {
    const mxu = makeMXU();
    const seq = new TPUSequencer(mxu, {
      mxuSize: 2,
      scalarLatency: 2,
      mxuLatency: 5,
      vectorLatency: 3,
    });
    const kernel = makeKernelDescriptor({
      operation: "matmul",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    });
    seq.submitOperation(kernel);
    expect(seq.pendingCount).toBeGreaterThanOrEqual(1);
  });

  it("should advance pipeline on step", () => {
    const mxu = makeMXU();
    const seq = new TPUSequencer(mxu, {
      mxuSize: 2,
      scalarLatency: 1,
      mxuLatency: 2,
      vectorLatency: 1,
    });
    const kernel = makeKernelDescriptor({
      operation: "matmul",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    });
    seq.submitOperation(kernel);
    const actions = seq.step();
    expect(actions.length).toBeGreaterThanOrEqual(1);
  });

  it("should run to completion", () => {
    const mxu = makeMXU();
    const seq = new TPUSequencer(mxu, {
      mxuSize: 2,
      scalarLatency: 1,
      mxuLatency: 2,
      vectorLatency: 1,
    });
    const kernel = makeKernelDescriptor({
      operation: "matmul",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    });
    seq.submitOperation(kernel);
    for (let i = 0; i < 100; i++) {
      seq.step();
      if (seq.idle) break;
    }
    expect(seq.idle).toBe(true);
  });

  it("should start idle", () => {
    const mxu = makeMXU();
    const seq = new TPUSequencer(mxu, { mxuSize: 2 });
    expect(seq.idle).toBe(true);
  });

  it("should reset properly", () => {
    const mxu = makeMXU();
    const seq = new TPUSequencer(mxu, {
      mxuSize: 2,
      scalarLatency: 1,
      mxuLatency: 2,
      vectorLatency: 1,
    });
    const kernel = makeKernelDescriptor({
      operation: "matmul",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    seq.submitOperation(kernel);
    seq.step();
    seq.reset();
    expect(seq.idle).toBe(true);
    expect(seq.pendingCount).toBe(0);
  });
});

// =========================================================================
// ANE Schedule Replayer
// =========================================================================

describe("ANEScheduleReplayer", () => {
  it("should generate schedule from operation", () => {
    const cores = makeANECores(2);
    const replayer = new ANEScheduleReplayer(cores, {
      dmaLatency: 1,
      computeLatency: 2,
      activateLatency: 1,
    });
    const kernel = makeKernelDescriptor({
      operation: "conv2d",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[0.5, 0.5], [0.5, 0.5]],
    });
    replayer.submitOperation(kernel);
    expect(replayer.pendingCount).toBeGreaterThan(0);
  });

  it("should replay schedule on step", () => {
    const cores = makeANECores(2);
    const replayer = new ANEScheduleReplayer(cores, {
      dmaLatency: 1,
      computeLatency: 2,
      activateLatency: 1,
    });
    const kernel = makeKernelDescriptor({
      operation: "conv2d",
      inputData: [[1.0, 2.0]],
      weightData: [[0.5, 0.5]],
    });
    replayer.submitOperation(kernel);
    const actions = replayer.step();
    expect(actions.length).toBeGreaterThanOrEqual(1);
  });

  it("should run to completion", () => {
    const cores = makeANECores(2);
    const replayer = new ANEScheduleReplayer(cores, {
      dmaLatency: 1,
      computeLatency: 2,
      activateLatency: 1,
    });
    const kernel = makeKernelDescriptor({
      operation: "inference",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    replayer.submitOperation(kernel);
    for (let i = 0; i < 100; i++) {
      replayer.step();
      if (replayer.idle) break;
    }
    expect(replayer.idle).toBe(true);
  });

  it("should start idle", () => {
    const cores = makeANECores(2);
    const replayer = new ANEScheduleReplayer(cores);
    expect(replayer.idle).toBe(true);
  });

  it("should reset properly", () => {
    const cores = makeANECores(2);
    const replayer = new ANEScheduleReplayer(cores, {
      dmaLatency: 1,
      computeLatency: 1,
      activateLatency: 1,
    });
    const kernel = makeKernelDescriptor({
      operation: "test",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    replayer.submitOperation(kernel);
    replayer.step();
    replayer.reset();
    expect(replayer.idle).toBe(true);
    expect(replayer.pendingCount).toBe(0);
  });

  it("should track total dispatched", () => {
    const cores = makeANECores(2);
    const replayer = new ANEScheduleReplayer(cores, {
      dmaLatency: 1,
      computeLatency: 1,
      activateLatency: 1,
    });
    const kernel = makeKernelDescriptor({
      operation: "test",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    replayer.submitOperation(kernel);
    for (let i = 0; i < 100; i++) {
      replayer.step();
      if (replayer.idle) break;
    }
    expect(replayer.totalDispatched).toBeGreaterThan(0);
  });
});
