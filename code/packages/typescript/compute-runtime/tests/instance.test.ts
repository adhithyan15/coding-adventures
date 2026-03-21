/**
 * Tests for RuntimeInstance and device discovery.
 */

import { describe, it, expect } from "vitest";
import { NvidiaGPU, AppleANE } from "@coding-adventures/device-simulator";
import {
  RuntimeInstance,
  DeviceType,
  QueueType,
  MemoryType,
} from "../src/index.js";

describe("RuntimeInstance", () => {
  it("has version 0.1.0", () => {
    const instance = new RuntimeInstance();
    expect(instance.version).toBe("0.1.0");
  });

  it("enumerates 5 default devices", () => {
    const instance = new RuntimeInstance();
    const devices = instance.enumeratePhysicalDevices();
    expect(devices.length).toBe(5);
  });

  it("includes expected device names", () => {
    const instance = new RuntimeInstance();
    const names = instance.enumeratePhysicalDevices().map((d) => d.name);
    expect(names.some((n) => n.includes("NVIDIA"))).toBe(true);
    expect(names.some((n) => n.includes("AMD"))).toBe(true);
    expect(names.some((n) => /TPU|Google/.test(n))).toBe(true);
    expect(names.some((n) => n.includes("Intel"))).toBe(true);
    expect(names.some((n) => /Apple|ANE/.test(n))).toBe(true);
  });

  it("includes all device types", () => {
    const instance = new RuntimeInstance();
    const types = new Set(instance.enumeratePhysicalDevices().map((d) => d.deviceType));
    expect(types.has(DeviceType.GPU)).toBe(true);
    expect(types.has(DeviceType.TPU)).toBe(true);
    expect(types.has(DeviceType.NPU)).toBe(true);
  });

  it("includes all vendors", () => {
    const instance = new RuntimeInstance();
    const vendors = new Set(instance.enumeratePhysicalDevices().map((d) => d.vendor));
    expect(vendors.has("nvidia")).toBe(true);
    expect(vendors.has("amd")).toBe(true);
    expect(vendors.has("google")).toBe(true);
    expect(vendors.has("intel")).toBe(true);
    expect(vendors.has("apple")).toBe(true);
  });

  it("supports custom devices", () => {
    const nvidia = new NvidiaGPU({ numSMs: 4 });
    const instance = new RuntimeInstance([[nvidia, DeviceType.GPU, "nvidia"]]);
    const devices = instance.enumeratePhysicalDevices();
    expect(devices.length).toBe(1);
    expect(devices[0].vendor).toBe("nvidia");
  });

  it("assigns unique device IDs", () => {
    const instance = new RuntimeInstance();
    const ids = instance.enumeratePhysicalDevices().map((d) => d.deviceId);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe("PhysicalDevice", () => {
  it("discrete GPUs have separate VRAM and staging heaps", () => {
    const instance = new RuntimeInstance();
    const nvidia = instance.enumeratePhysicalDevices().find((d) => d.vendor === "nvidia")!;
    const mem = nvidia.memoryProperties;
    expect(mem.isUnified).toBe(false);
    expect(mem.heaps.length).toBeGreaterThanOrEqual(2);
  });

  it("Apple has unified memory", () => {
    const instance = new RuntimeInstance();
    const apple = instance.enumeratePhysicalDevices().find((d) => d.vendor === "apple")!;
    const mem = apple.memoryProperties;
    expect(mem.isUnified).toBe(true);
    expect(mem.heaps.length).toBeGreaterThanOrEqual(1);
  });

  it("has queue families with compute", () => {
    const instance = new RuntimeInstance();
    const nvidia = instance.enumeratePhysicalDevices().find((d) => d.vendor === "nvidia")!;
    expect(nvidia.queueFamilies.some((f) => f.queueType === QueueType.COMPUTE)).toBe(true);
  });

  it("discrete GPUs have transfer queue", () => {
    const instance = new RuntimeInstance();
    const nvidia = instance.enumeratePhysicalDevices().find((d) => d.vendor === "nvidia")!;
    expect(nvidia.queueFamilies.some((f) => f.queueType === QueueType.TRANSFER)).toBe(true);
  });

  it("Apple unified memory has no separate transfer queue", () => {
    const instance = new RuntimeInstance();
    const apple = instance.enumeratePhysicalDevices().find((d) => d.vendor === "apple")!;
    expect(apple.queueFamilies.some((f) => f.queueType === QueueType.TRANSFER)).toBe(false);
  });

  it("supports fp32 feature", () => {
    const instance = new RuntimeInstance();
    const nvidia = instance.enumeratePhysicalDevices().find((d) => d.vendor === "nvidia")!;
    expect(nvidia.supportsFeature("fp32")).toBe(true);
    expect(nvidia.supportsFeature("unified_memory")).toBe(false);
  });

  it("Apple supports unified_memory feature", () => {
    const instance = new RuntimeInstance();
    const apple = instance.enumeratePhysicalDevices().find((d) => d.vendor === "apple")!;
    expect(apple.supportsFeature("unified_memory")).toBe(true);
  });

  it("has positive limits", () => {
    const instance = new RuntimeInstance();
    const nvidia = instance.enumeratePhysicalDevices().find((d) => d.vendor === "nvidia")!;
    expect(nvidia.limits.maxWorkgroupSize[0]).toBeGreaterThan(0);
    expect(nvidia.limits.maxBufferSize).toBeGreaterThan(0);
    expect(nvidia.limits.maxPushConstantSize).toBeGreaterThan(0);
  });
});

describe("LogicalDevice", () => {
  it("can be created from physical device", () => {
    const instance = new RuntimeInstance();
    const physical = instance.enumeratePhysicalDevices()[0];
    const device = instance.createLogicalDevice(physical);
    expect(device.physicalDevice).toBe(physical);
    expect(device.queues["compute"]).toBeDefined();
  });

  it("creates one default compute queue", () => {
    const instance = new RuntimeInstance();
    const physical = instance.enumeratePhysicalDevices()[0];
    const device = instance.createLogicalDevice(physical);
    expect(device.queues["compute"].length).toBe(1);
  });

  it("creates multiple queues on request", () => {
    const instance = new RuntimeInstance();
    const physical = instance.enumeratePhysicalDevices()[0];
    const device = instance.createLogicalDevice(physical, [{ type: "compute", count: 3 }]);
    expect(device.queues["compute"].length).toBe(3);
  });

  it("has a memory manager", () => {
    const instance = new RuntimeInstance();
    const physical = instance.enumeratePhysicalDevices()[0];
    const device = instance.createLogicalDevice(physical);
    expect(device.memoryManager).toBeDefined();
  });

  it("factory methods work", () => {
    const instance = new RuntimeInstance();
    const physical = instance.enumeratePhysicalDevices()[0];
    const device = instance.createLogicalDevice(physical);
    expect(device.createCommandBuffer()).toBeDefined();
    expect(device.createFence().signaled).toBe(false);
    expect(device.createSemaphore().signaled).toBe(false);
    expect(device.createEvent().signaled).toBe(false);
  });

  it("creates fence in signaled state", () => {
    const instance = new RuntimeInstance();
    const physical = instance.enumeratePhysicalDevices()[0];
    const device = instance.createLogicalDevice(physical);
    expect(device.createFence(true).signaled).toBe(true);
  });

  it("waitIdle does not throw", () => {
    const instance = new RuntimeInstance();
    const physical = instance.enumeratePhysicalDevices()[0];
    const device = instance.createLogicalDevice(physical);
    expect(() => device.waitIdle()).not.toThrow();
  });

  it("reset does not throw", () => {
    const instance = new RuntimeInstance();
    const physical = instance.enumeratePhysicalDevices()[0];
    const device = instance.createLogicalDevice(physical);
    expect(() => device.reset()).not.toThrow();
  });

  it("every device type produces valid logical device", () => {
    const instance = new RuntimeInstance();
    for (const physical of instance.enumeratePhysicalDevices()) {
      const device = instance.createLogicalDevice(physical);
      expect(device.physicalDevice.name).toBe(physical.name);
      expect(device.queues["compute"]).toBeDefined();
    }
  });
});
