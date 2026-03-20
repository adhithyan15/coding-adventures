/**
 * Cross-device tests -- same workloads on all architectures.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";

import { NvidiaGPU } from "../src/nvidia-gpu.js";
import { AmdGPU } from "../src/amd-gpu.js";
import { GoogleTPU } from "../src/google-tpu.js";
import { IntelGPU } from "../src/intel-gpu.js";
import { AppleANE } from "../src/apple-ane.js";
import { makeKernelDescriptor, formatDeviceTrace } from "../src/protocols.js";

// =========================================================================
// Helpers
// =========================================================================

type AnyDevice = NvidiaGPU | AmdGPU | GoogleTPU | IntelGPU | AppleANE;

function allGPUDevices(): Record<string, AnyDevice> {
  return {
    NVIDIA: new NvidiaGPU({ numSMs: 2 }),
    AMD: new AmdGPU({ numCUs: 2 }),
    Intel: new IntelGPU({ numCores: 2 }),
  };
}

function allDataflowDevices(): Record<string, AnyDevice> {
  return {
    TPU: new GoogleTPU({ mxuSize: 2 }),
    ANE: new AppleANE({ numCores: 2 }),
  };
}

function allDevices(): Record<string, AnyDevice> {
  return { ...allGPUDevices(), ...allDataflowDevices() };
}

// =========================================================================
// Basic lifecycle tests
// =========================================================================

describe("all devices start idle", () => {
  it("should be idle initially", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      expect(device.idle).toBe(true);
    }
  });
});

describe("all devices have names", () => {
  it("should have non-empty names", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      expect(device.name.length).toBeGreaterThan(0);
    }
  });
});

describe("all devices have compute units", () => {
  it("should have at least one compute unit", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      expect(device.computeUnits.length).toBeGreaterThan(0);
    }
  });
});

describe("all devices can step when idle", () => {
  it("should produce a trace with valid cycle", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      const trace = device.step();
      expect(trace.cycle).toBeGreaterThan(0);
    }
  });
});

describe("all devices can reset", () => {
  it("should be idle after reset", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      device.step();
      device.step();
      device.reset();
      expect(device.idle).toBe(true);
    }
  });
});

// =========================================================================
// GPU-style kernel execution
// =========================================================================

describe("GPU kernel execution", () => {
  it("should run a simple kernel on all GPU devices", () => {
    for (const [name, device] of Object.entries(allGPUDevices())) {
      const kernel = makeKernelDescriptor({
        name: "test_simple",
        program: [limm(0, 42.0), halt()],
        gridDim: [2, 1, 1],
        blockDim: [32, 1, 1],
      });
      device.launchKernel(kernel);
      const traces = device.run(2000);
      expect(traces.length).toBeGreaterThan(0);
      expect(device.idle).toBe(true);
    }
  });
});

// =========================================================================
// Dataflow-style execution
// =========================================================================

describe("dataflow execution", () => {
  it("should process matmul on TPU and ANE", () => {
    for (const [name, device] of Object.entries(allDataflowDevices())) {
      const kernel = makeKernelDescriptor({
        name: "matmul",
        operation: "matmul",
        inputData: [[1.0, 2.0], [3.0, 4.0]],
        weightData: [[5.0, 6.0], [7.0, 8.0]],
      });
      device.launchKernel(kernel);
      const traces = device.run(1000);
      expect(traces.length).toBeGreaterThan(0);
      expect(device.idle).toBe(true);
    }
  });
});

// =========================================================================
// Memory management
// =========================================================================

describe("all devices can malloc and free", () => {
  it("should allocate valid addresses", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      const addr = device.malloc(256);
      expect(addr).toBeGreaterThanOrEqual(0);
      device.free(addr);
    }
  });
});

describe("all devices can transfer data", () => {
  it("should round-trip data", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      const addr = device.malloc(64);
      device.memcpyHostToDevice(addr, new Uint8Array(64).fill(0x42));
      const [data] = device.memcpyDeviceToHost(addr, 64);
      expect(data).toEqual(new Uint8Array(64).fill(0x42));
    }
  });
});

describe("unified vs discrete transfer cost", () => {
  it("should have zero cost for ANE, nonzero for NVIDIA", () => {
    const ane = new AppleANE({ numCores: 2 });
    const nvidia = new NvidiaGPU({ numSMs: 2 });

    const aneAddr = ane.malloc(256);
    const nvidiaAddr = nvidia.malloc(256);

    const aneCycles = ane.memcpyHostToDevice(aneAddr, new Uint8Array(256));
    const nvidiaCycles = nvidia.memcpyHostToDevice(nvidiaAddr, new Uint8Array(256));

    expect(aneCycles).toBe(0);
    expect(nvidiaCycles).toBeGreaterThan(0);
  });
});

// =========================================================================
// Stats
// =========================================================================

describe("all devices track kernel launches", () => {
  it("should count kernel launches", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      let kernel;
      if (name === "NVIDIA" || name === "AMD" || name === "Intel") {
        kernel = makeKernelDescriptor({
          name: "test",
          program: [limm(0, 1.0), halt()],
          gridDim: [1, 1, 1],
          blockDim: [32, 1, 1],
        });
      } else {
        kernel = makeKernelDescriptor({
          name: "test",
          operation: "matmul",
          inputData: [[1.0]],
          weightData: [[1.0]],
        });
      }
      device.launchKernel(kernel);
      device.run(1000);
      const stats = device.stats;
      expect(stats.totalKernelsLaunched).toBe(1);
    }
  });
});

// =========================================================================
// Trace format
// =========================================================================

describe("all devices produce readable traces", () => {
  it("should return non-empty formatted strings", () => {
    for (const [name, device] of Object.entries(allDevices())) {
      const trace = device.step();
      const formatted = formatDeviceTrace(trace);
      expect(typeof formatted).toBe("string");
      expect(formatted.length).toBeGreaterThan(0);
    }
  });
});
