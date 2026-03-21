/**
 * Tests for Intel GPU device simulator.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";

import { IntelGPU } from "../src/intel-gpu.js";
import {
  makeKernelDescriptor,
  makeIntelGPUConfig,
  makeXeSliceConfig,
  formatDeviceTrace,
} from "../src/protocols.js";

describe("construction", () => {
  it("should construct with default options", () => {
    const gpu = new IntelGPU({ numCores: 4 });
    expect(gpu.name).toContain("Intel");
    expect(gpu.computeUnits.length).toBe(4);
  });

  it("should construct with Intel-specific config", () => {
    const config = makeIntelGPUConfig({
      name: "Test Intel",
      numComputeUnits: 4,
      l2CacheSize: 4096,
      l2CacheAssociativity: 4,
      l2CacheLineSize: 64,
      globalMemorySize: 1024 * 1024,
      numXeSlices: 2,
      sliceConfig: makeXeSliceConfig({ xeCoresPerSlice: 2 }),
    });
    const gpu = new IntelGPU({ config });
    expect(gpu.name).toBe("Test Intel");
    expect(gpu.xeSlices.length).toBe(2);
  });

  it("should start idle", () => {
    const gpu = new IntelGPU({ numCores: 2 });
    expect(gpu.idle).toBe(true);
  });

  it("should group cores into Xe-Slices", () => {
    const config = makeIntelGPUConfig({
      name: "Test Intel",
      numComputeUnits: 8,
      l2CacheSize: 4096,
      l2CacheAssociativity: 4,
      l2CacheLineSize: 64,
      globalMemorySize: 1024 * 1024,
      numXeSlices: 4,
      sliceConfig: makeXeSliceConfig({ xeCoresPerSlice: 2 }),
    });
    const gpu = new IntelGPU({ config });
    expect(gpu.xeSlices.length).toBe(4);
    for (const s of gpu.xeSlices) {
      expect(s.xeCores.length).toBe(2);
    }
  });
});

describe("kernel execution", () => {
  it("should launch and run to completion", () => {
    const gpu = new IntelGPU({ numCores: 2 });
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
    const gpu = new IntelGPU({ numCores: 4 });
    const kernel = makeKernelDescriptor({
      name: "multi",
      program: [limm(0, 1.0), halt()],
      gridDim: [4, 1, 1],
      blockDim: [32, 1, 1],
    });
    gpu.launchKernel(kernel);
    const traces = gpu.run(2000);
    expect(gpu.idle).toBe(true);
  });
});

describe("memory", () => {
  it("should malloc and transfer data", () => {
    const gpu = new IntelGPU({ numCores: 2 });
    const addr = gpu.malloc(256);
    const cycles = gpu.memcpyHostToDevice(addr, new Uint8Array(256).fill(0x42));
    expect(cycles).toBeGreaterThan(0);
    const [data] = gpu.memcpyDeviceToHost(addr, 256);
    expect(data).toEqual(new Uint8Array(256).fill(0x42));
  });
});

describe("traces", () => {
  it("should format trace with Intel name", () => {
    const gpu = new IntelGPU({ numCores: 2 });
    const trace = gpu.step();
    const formatted = formatDeviceTrace(trace);
    expect(formatted).toContain("Intel");
  });

  it("should show Xe-Slices as idle", () => {
    const config = makeIntelGPUConfig({
      name: "Test Intel",
      numComputeUnits: 4,
      l2CacheSize: 4096,
      l2CacheAssociativity: 4,
      l2CacheLineSize: 64,
      globalMemorySize: 1024 * 1024,
      numXeSlices: 2,
      sliceConfig: makeXeSliceConfig({ xeCoresPerSlice: 2 }),
    });
    const gpu = new IntelGPU({ config });
    for (const s of gpu.xeSlices) {
      expect(s.idle).toBe(true);
    }
  });
});

describe("reset", () => {
  it("should reset to idle", () => {
    const gpu = new IntelGPU({ numCores: 2 });
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

  it("should track stats", () => {
    const gpu = new IntelGPU({ numCores: 2 });
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
  });
});
