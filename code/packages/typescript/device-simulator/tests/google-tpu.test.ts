/**
 * Tests for Google TPU device simulator.
 */

import { describe, it, expect } from "vitest";

import { GoogleTPU } from "../src/google-tpu.js";
import { makeKernelDescriptor, makeTPUConfig, formatDeviceTrace } from "../src/protocols.js";

describe("construction", () => {
  it("should construct with default options", () => {
    const tpu = new GoogleTPU({ mxuSize: 4 });
    expect(tpu.name).toContain("TPU");
    expect(tpu.computeUnits.length).toBe(1);
  });

  it("should construct with config", () => {
    const config = makeTPUConfig({
      name: "Test TPU",
      numComputeUnits: 1,
      globalMemorySize: 1024 * 1024,
      vectorUnitWidth: 4,
    });
    const tpu = new GoogleTPU({ config });
    expect(tpu.name).toBe("Test TPU");
  });

  it("should start idle", () => {
    const tpu = new GoogleTPU({ mxuSize: 4 });
    expect(tpu.idle).toBe(true);
  });
});

describe("matmul execution", () => {
  it("should mark device not idle after launch", () => {
    const tpu = new GoogleTPU({ mxuSize: 2 });
    const kernel = makeKernelDescriptor({
      name: "matmul",
      operation: "matmul",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    });
    tpu.launchKernel(kernel);
    expect(tpu.idle).toBe(false);
  });

  it("should run matmul to completion", () => {
    const tpu = new GoogleTPU({ mxuSize: 2 });
    const kernel = makeKernelDescriptor({
      name: "matmul",
      operation: "matmul",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    });
    tpu.launchKernel(kernel);
    const traces = tpu.run(500);
    expect(traces.length).toBeGreaterThan(0);
    expect(tpu.idle).toBe(true);
  });

  it("should tile large matmul", () => {
    const tpu = new GoogleTPU({ mxuSize: 2 });
    const kernel = makeKernelDescriptor({
      name: "big_matmul",
      operation: "matmul",
      inputData: Array.from({ length: 4 }, () => Array.from({ length: 4 }, () => 1.0)),
      weightData: Array.from({ length: 4 }, () => Array.from({ length: 4 }, () => 1.0)),
    });
    tpu.launchKernel(kernel);
    const traces = tpu.run(1000);
    expect(tpu.idle).toBe(true);
  });
});

describe("memory", () => {
  it("should malloc and transfer with latency", () => {
    const tpu = new GoogleTPU({ mxuSize: 4 });
    const addr = tpu.malloc(256);
    const cycles = tpu.memcpyHostToDevice(addr, new Uint8Array(256));
    expect(cycles).toBeGreaterThan(0);
  });
});

describe("traces", () => {
  it("should show pipeline actions", () => {
    const tpu = new GoogleTPU({ mxuSize: 2 });
    const kernel = makeKernelDescriptor({
      name: "matmul",
      operation: "matmul",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    });
    tpu.launchKernel(kernel);
    const trace = tpu.step();
    expect(trace.distributorActions.length).toBeGreaterThan(0);
  });

  it("should format trace with TPU name", () => {
    const tpu = new GoogleTPU({ mxuSize: 2 });
    const trace = tpu.step();
    const formatted = formatDeviceTrace(trace);
    expect(formatted).toContain("TPU");
  });
});

describe("reset", () => {
  it("should reset to idle", () => {
    const tpu = new GoogleTPU({ mxuSize: 2 });
    const kernel = makeKernelDescriptor({
      name: "matmul",
      operation: "matmul",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    tpu.launchKernel(kernel);
    tpu.run(500);
    tpu.reset();
    expect(tpu.idle).toBe(true);
  });

  it("should track stats", () => {
    const tpu = new GoogleTPU({ mxuSize: 2 });
    const kernel = makeKernelDescriptor({
      name: "matmul",
      operation: "matmul",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    tpu.launchKernel(kernel);
    tpu.run(500);
    const stats = tpu.stats;
    expect(stats.totalKernelsLaunched).toBe(1);
  });
});
