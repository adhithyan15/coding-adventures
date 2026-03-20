/**
 * Tests for Apple ANE device simulator.
 */

import { describe, it, expect } from "vitest";

import { AppleANE } from "../src/apple-ane.js";
import { makeKernelDescriptor, makeANEConfig, formatDeviceTrace } from "../src/protocols.js";

describe("construction", () => {
  it("should construct with default options", () => {
    const ane = new AppleANE({ numCores: 4 });
    expect(ane.name).toContain("Apple");
    expect(ane.computeUnits.length).toBe(4);
  });

  it("should construct with ANE-specific config", () => {
    const config = makeANEConfig({
      name: "Test ANE",
      numComputeUnits: 8,
      globalMemorySize: 1024 * 1024,
      unifiedMemory: true,
      hostLatency: 0,
    });
    const ane = new AppleANE({ config });
    expect(ane.name).toBe("Test ANE");
    expect(ane.computeUnits.length).toBe(8);
  });

  it("should start idle", () => {
    const ane = new AppleANE({ numCores: 4 });
    expect(ane.idle).toBe(true);
  });

  it("should report unified memory", () => {
    const ane = new AppleANE({ numCores: 4 });
    expect(ane.isUnifiedMemory).toBe(true);
  });
});

describe("unified memory", () => {
  it("should have zero-cost host-to-device transfers", () => {
    const ane = new AppleANE({ numCores: 4 });
    const addr = ane.malloc(256);
    const cycles = ane.memcpyHostToDevice(addr, new Uint8Array(256).fill(0x42));
    expect(cycles).toBe(0);
  });

  it("should have zero-cost device-to-host transfers", () => {
    const ane = new AppleANE({ numCores: 4 });
    const addr = ane.malloc(64);
    ane.memcpyHostToDevice(addr, new Uint8Array(64).fill(0xaa));
    const [data, cycles] = ane.memcpyDeviceToHost(addr, 64);
    expect(data).toEqual(new Uint8Array(64).fill(0xaa));
    expect(cycles).toBe(0);
  });

  it("should persist data after zero-copy transfer", () => {
    const ane = new AppleANE({ numCores: 4 });
    const addr = ane.malloc(128);
    ane.memcpyHostToDevice(addr, new Uint8Array(128).fill(0xff));
    const [data] = ane.memcpyDeviceToHost(addr, 128);
    expect(data).toEqual(new Uint8Array(128).fill(0xff));
  });
});

describe("inference execution", () => {
  it("should mark device not idle after launch", () => {
    const ane = new AppleANE({ numCores: 2 });
    const kernel = makeKernelDescriptor({
      name: "conv2d",
      operation: "conv2d",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[0.5, 0.5], [0.5, 0.5]],
    });
    ane.launchKernel(kernel);
    expect(ane.idle).toBe(false);
  });

  it("should run to completion", () => {
    const ane = new AppleANE({ numCores: 2 });
    const kernel = makeKernelDescriptor({
      name: "inference",
      operation: "matmul",
      inputData: [[1.0, 2.0], [3.0, 4.0]],
      weightData: [[5.0, 6.0], [7.0, 8.0]],
    });
    ane.launchKernel(kernel);
    const traces = ane.run(500);
    expect(traces.length).toBeGreaterThan(0);
    expect(ane.idle).toBe(true);
  });

  it("should replay schedule actions", () => {
    const ane = new AppleANE({ numCores: 4 });
    const kernel = makeKernelDescriptor({
      name: "inference",
      operation: "matmul",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    ane.launchKernel(kernel);
    const trace = ane.step();
    expect(trace.distributorActions.length).toBeGreaterThan(0);
  });
});

describe("traces", () => {
  it("should format trace with Apple name", () => {
    const ane = new AppleANE({ numCores: 2 });
    const trace = ane.step();
    const formatted = formatDeviceTrace(trace);
    expect(formatted).toContain("Apple");
  });

  it("should show active blocks count", () => {
    const ane = new AppleANE({ numCores: 4 });
    const trace = ane.step();
    expect(trace.activeBlocks).toBeGreaterThanOrEqual(0);
  });
});

describe("reset", () => {
  it("should reset to idle", () => {
    const ane = new AppleANE({ numCores: 2 });
    const kernel = makeKernelDescriptor({
      name: "test",
      operation: "matmul",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    ane.launchKernel(kernel);
    ane.run(500);
    ane.reset();
    expect(ane.idle).toBe(true);
  });

  it("should track stats", () => {
    const ane = new AppleANE({ numCores: 2 });
    const kernel = makeKernelDescriptor({
      name: "test",
      operation: "matmul",
      inputData: [[1.0]],
      weightData: [[1.0]],
    });
    ane.launchKernel(kernel);
    ane.run(500);
    const stats = ane.stats;
    expect(stats.totalKernelsLaunched).toBe(1);
  });
});
