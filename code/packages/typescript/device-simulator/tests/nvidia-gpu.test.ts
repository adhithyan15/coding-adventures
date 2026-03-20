/**
 * Tests for NVIDIA GPU device simulator.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";

import { NvidiaGPU } from "../src/nvidia-gpu.js";
import { makeKernelDescriptor, makeDeviceConfig, formatDeviceTrace } from "../src/protocols.js";

describe("construction", () => {
  it("should construct with default options", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    expect(gpu.name).toContain("NVIDIA");
    expect(gpu.computeUnits.length).toBe(2);
  });

  it("should construct with config", () => {
    const config = makeDeviceConfig({
      name: "Test GPU",
      numComputeUnits: 3,
      l2CacheSize: 4096,
      l2CacheAssociativity: 4,
      l2CacheLineSize: 64,
      globalMemorySize: 1024 * 1024,
    });
    const gpu = new NvidiaGPU({ config });
    expect(gpu.name).toBe("Test GPU");
    expect(gpu.computeUnits.length).toBe(3);
  });

  it("should start idle", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    expect(gpu.idle).toBe(true);
  });
});

describe("memory management", () => {
  it("should malloc and free", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const addr = gpu.malloc(256);
    expect(addr).toBeGreaterThanOrEqual(0);
    gpu.free(addr);
  });

  it("should give sequential allocations non-overlapping addresses", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const a1 = gpu.malloc(256);
    const a2 = gpu.malloc(256);
    expect(a2).toBeGreaterThan(a1);
  });

  it("should transfer host to device with latency", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const addr = gpu.malloc(128);
    const cycles = gpu.memcpyHostToDevice(addr, new Uint8Array(128).fill(0x42));
    expect(cycles).toBeGreaterThan(0);
  });

  it("should round-trip data through device", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const addr = gpu.malloc(64);
    gpu.memcpyHostToDevice(addr, new Uint8Array(64).fill(0xaa));
    const [data, cycles] = gpu.memcpyDeviceToHost(addr, 64);
    expect(data).toEqual(new Uint8Array(64).fill(0xaa));
    expect(cycles).toBeGreaterThan(0);
  });
});

describe("kernel launch", () => {
  it("should mark device as not idle after launch", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 42.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    gpu.launchKernel(kernel);
    expect(gpu.idle).toBe(false);
  });

  it("should run to completion", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 42.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    gpu.launchKernel(kernel);
    const traces = gpu.run(1000);
    expect(traces.length).toBeGreaterThan(0);
    expect(gpu.idle).toBe(true);
  });

  it("should handle multi-block kernel", () => {
    const gpu = new NvidiaGPU({ numSMs: 4 });
    const kernel = makeKernelDescriptor({
      name: "multi_block",
      program: [limm(0, 1.0), halt()],
      gridDim: [8, 1, 1],
      blockDim: [32, 1, 1],
    });
    gpu.launchKernel(kernel);
    const traces = gpu.run(2000);
    expect(gpu.idle).toBe(true);
    expect(traces.length).toBeGreaterThan(0);
  });
});

describe("traces", () => {
  it("should have correct cycle number", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const trace = gpu.step();
    expect(trace.cycle).toBe(1);
  });

  it("should have device name", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const trace = gpu.step();
    expect(trace.deviceName).toContain("NVIDIA");
  });

  it("should format trace", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 42.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    gpu.launchKernel(kernel);
    const trace = gpu.step();
    const formatted = formatDeviceTrace(trace);
    expect(formatted).toContain("NVIDIA");
    expect(formatted).toContain("Cycle");
  });

  it("should show pending blocks", () => {
    const gpu = new NvidiaGPU({ numSMs: 1 });
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 1.0), halt()],
      gridDim: [4, 1, 1],
      blockDim: [32, 1, 1],
    });
    gpu.launchKernel(kernel);
    const trace = gpu.step();
    expect(trace.pendingBlocks).toBeGreaterThanOrEqual(0);
  });
});

describe("stats", () => {
  it("should track kernel launches", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 42.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    gpu.launchKernel(kernel);
    gpu.run(500);
    const stats = gpu.stats;
    expect(stats.totalKernelsLaunched).toBe(1);
    expect(stats.totalBlocksDispatched).toBeGreaterThanOrEqual(1);
  });

  it("should track memory stats", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const addr = gpu.malloc(128);
    gpu.memcpyHostToDevice(addr, new Uint8Array(128));
    const stats = gpu.stats;
    expect(stats.globalMemoryStats.hostToDeviceBytes).toBe(128);
  });
});

describe("reset", () => {
  it("should reset to idle state", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const kernel = makeKernelDescriptor({
      name: "test",
      program: [limm(0, 42.0), halt()],
      gridDim: [2, 1, 1],
      blockDim: [32, 1, 1],
    });
    gpu.launchKernel(kernel);
    gpu.run(500);
    gpu.reset();
    expect(gpu.idle).toBe(true);
  });

  it("should reset memory stats", () => {
    const gpu = new NvidiaGPU({ numSMs: 2 });
    const addr = gpu.malloc(64);
    gpu.memcpyHostToDevice(addr, new Uint8Array(64).fill(0xff));
    gpu.reset();
    const stats = gpu.stats;
    expect(stats.globalMemoryStats.hostToDeviceBytes).toBe(0);
  });
});
