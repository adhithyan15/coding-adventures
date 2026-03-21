/**
 * Tests for AMD GPU device simulator.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";

import { AmdGPU } from "../src/amd-gpu.js";
import {
  makeKernelDescriptor,
  makeAmdGPUConfig,
  makeShaderEngineConfig,
  formatDeviceTrace,
} from "../src/protocols.js";

describe("construction", () => {
  it("should construct with default options", () => {
    const gpu = new AmdGPU({ numCUs: 4 });
    expect(gpu.name).toContain("AMD");
    expect(gpu.computeUnits.length).toBe(4);
  });

  it("should construct with AMD-specific config", () => {
    const config = makeAmdGPUConfig({
      name: "Test AMD",
      numComputeUnits: 4,
      l2CacheSize: 4096,
      l2CacheAssociativity: 4,
      l2CacheLineSize: 64,
      globalMemorySize: 1024 * 1024,
      numShaderEngines: 2,
      seConfig: makeShaderEngineConfig({ cusPerEngine: 2 }),
    });
    const gpu = new AmdGPU({ config });
    expect(gpu.name).toBe("Test AMD");
    expect(gpu.shaderEngines.length).toBe(2);
    expect(gpu.computeUnits.length).toBe(4);
  });

  it("should start idle", () => {
    const gpu = new AmdGPU({ numCUs: 2 });
    expect(gpu.idle).toBe(true);
  });

  it("should group CUs into shader engines", () => {
    const config = makeAmdGPUConfig({
      name: "Test AMD",
      numComputeUnits: 6,
      l2CacheSize: 4096,
      l2CacheAssociativity: 4,
      l2CacheLineSize: 64,
      globalMemorySize: 1024 * 1024,
      numShaderEngines: 3,
      seConfig: makeShaderEngineConfig({ cusPerEngine: 2 }),
    });
    const gpu = new AmdGPU({ config });
    expect(gpu.shaderEngines.length).toBe(3);
    for (const se of gpu.shaderEngines) {
      expect(se.cus.length).toBe(2);
    }
  });
});

describe("kernel execution", () => {
  it("should launch and run to completion", () => {
    const gpu = new AmdGPU({ numCUs: 2 });
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
    const gpu = new AmdGPU({ numCUs: 4 });
    const kernel = makeKernelDescriptor({
      name: "multi_block",
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
    const gpu = new AmdGPU({ numCUs: 2 });
    const addr = gpu.malloc(256);
    const cycles = gpu.memcpyHostToDevice(addr, new Uint8Array(256).fill(0x42));
    expect(cycles).toBeGreaterThan(0);
    const [data] = gpu.memcpyDeviceToHost(addr, 256);
    expect(data).toEqual(new Uint8Array(256).fill(0x42));
  });
});

describe("traces", () => {
  it("should format trace with AMD name", () => {
    const gpu = new AmdGPU({ numCUs: 2 });
    const trace = gpu.step();
    const formatted = formatDeviceTrace(trace);
    expect(formatted).toContain("AMD");
  });

  it("should show shader engines as idle", () => {
    const config = makeAmdGPUConfig({
      name: "Test AMD",
      numComputeUnits: 4,
      l2CacheSize: 4096,
      l2CacheAssociativity: 4,
      l2CacheLineSize: 64,
      globalMemorySize: 1024 * 1024,
      numShaderEngines: 2,
      seConfig: makeShaderEngineConfig({ cusPerEngine: 2 }),
    });
    const gpu = new AmdGPU({ config });
    for (const se of gpu.shaderEngines) {
      expect(se.idle).toBe(true);
    }
  });
});

describe("reset", () => {
  it("should reset to idle", () => {
    const gpu = new AmdGPU({ numCUs: 2 });
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
    const gpu = new AmdGPU({ numCUs: 2 });
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
